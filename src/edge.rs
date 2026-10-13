//! Prime edges: stochastic multi-level writer masks with a learned gain.
//!
//! Each source cell owns L masks over the target's 2048-bit code space. Level k is written with
//! probability 2^-k (a stochastic binary counter), up to SWAPS bits per write, with the mask's bit
//! count kept constant. The score of target cell j is (sum over levels of |mask_k AND code_j|) / (32 L).
//! It is kept as an integer count per (source cell, target cell), updated incrementally through
//! an owner table that maps each bit to the target cells whose code contains it.
use crate::bits::{self, Bits, Code, CK, D, ZERO};
use crate::rng::Rng;
use std::collections::HashMap;

pub const LEVELS: usize = 8;
pub const KW: usize = 640;
pub const SWAPS: usize = 8;

/// Which target cells own each bit. Rebuilt whenever the target pool's set of live cells changes.
pub struct TargetIndex {
    pub owners: Vec<Vec<u32>>,
    pub version: u64,
}

impl TargetIndex {
    pub fn new() -> Self {
        TargetIndex {
            owners: vec![Vec::new(); D],
            version: 0,
        }
    }

    pub fn refresh(&mut self, codes: &[Code], alive: &[bool]) {
        for o in self.owners.iter_mut() {
            o.clear();
        }
        for (c, code) in codes.iter().enumerate() {
            if alive[c] {
                for &b in code.iter() {
                    self.owners[b as usize].push(c as u32);
                }
            }
        }
        self.version += 1;
    }
}

impl Default for TargetIndex {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Row {
    pub gen: u32,
    pub masks: Vec<Bits>,
    pub counts: Vec<i32>,
    pub version: u64,
}

pub struct Edge {
    /// Index of the source node in the brain.
    pub src: usize,
    pub rows: HashMap<u32, Row>,
    pub beta: f64,
    v: f64,
    pub cur: i32,
    pub sc: Option<Vec<f64>>,
    pub sum: f64,
    pub sq: f64,
    pub n: u64,
    prob: [f64; LEVELS],
    /// Drop the lowest-numbered eligible bits (the Python reference's rule) instead of random ones.
    pub drop_lowest: bool,
}

impl Edge {
    pub fn new(src: usize, beta: f64) -> Self {
        let mut prob = [0.0; LEVELS];
        for (k, p) in prob.iter_mut().enumerate() {
            *p = 0.5f64.powi(k as i32);
        }
        Edge {
            src,
            rows: HashMap::new(),
            beta,
            v: 0.01,
            cur: -1,
            sc: None,
            sum: 0.0,
            sq: 0.0,
            n: 0,
            prob,
            drop_lowest: false,
        }
    }

    /// The row for source cell i, (re)made if its generation changed, counts refreshed if the target changed.
    pub fn row(
        &mut self,
        i: u32,
        gen: u32,
        tix: &TargetIndex,
        tcodes: &[Code],
        talive: &[bool],
        rng: &mut Rng,
    ) -> &mut Row {
        let fresh = match self.rows.get(&i) {
            Some(r) => r.gen != gen,
            None => true,
        };
        if fresh {
            let mut masks = vec![ZERO; LEVELS];
            for m in masks.iter_mut() {
                for b in rng.choose_distinct(D, KW) {
                    bits::set(m, b);
                }
            }
            self.rows.insert(
                i,
                Row {
                    gen,
                    masks,
                    counts: Vec::new(),
                    version: u64::MAX,
                },
            );
        }
        let r = self.rows.get_mut(&i).unwrap();
        if r.version != tix.version {
            r.counts = vec![0; tcodes.len()];
            for (c, code) in tcodes.iter().enumerate() {
                if !talive[c] {
                    continue;
                }
                let mut n = 0;
                for m in r.masks.iter() {
                    for &b in code.iter() {
                        n += bits::get(m, b as usize) as i32;
                    }
                }
                r.counts[c] = n;
            }
            r.version = tix.version;
        }
        r
    }

    /// Scores over the target's live cells (in `alive_idx` order), or None if the source has no state.
    #[allow(clippy::too_many_arguments)]
    pub fn score(
        &mut self,
        state: i32,
        gen: u32,
        alive_idx: &[usize],
        tix: &TargetIndex,
        tcodes: &[Code],
        talive: &[bool],
        rng: &mut Rng,
    ) {
        self.cur = state;
        if state < 0 {
            self.sc = None;
            return;
        }
        let r = self.row(state as u32, gen, tix, tcodes, talive, rng);
        let s = 1.0 / (CK * LEVELS) as f64;
        let v: Vec<f64> = alive_idx.iter().map(|&c| r.counts[c] as f64 * s).collect();
        self.sc = Some(v);
    }

    /// Write the target's winner code into the current source cell's masks.
    #[allow(clippy::too_many_arguments)]
    pub fn learn(
        &mut self,
        gen: u32,
        ycode: &Code,
        tgt: &Bits,
        tix: &TargetIndex,
        tcodes: &[Code],
        talive: &[bool],
        rng: &mut Rng,
    ) {
        if self.cur < 0 {
            return;
        }
        let prob = self.prob;
        let cur = self.cur as u32;
        // draw the levels first, so the row borrow below holds no rng
        let mut lv = [false; LEVELS];
        for k in 0..LEVELS {
            lv[k] = rng.f64() < prob[k];
        }
        self.row(cur, gen, tix, tcodes, talive, rng);
        let r = self.rows.get_mut(&cur).unwrap();
        for k in 0..LEVELS {
            if !lv[k] {
                continue;
            }
            let m = &mut r.masks[k];
            let mut add: Vec<u16> = ycode
                .iter()
                .copied()
                .filter(|&b| !bits::get(m, b as usize))
                .collect();
            if add.is_empty() {
                continue;
            }
            let w = SWAPS.min(add.len());
            if w < add.len() {
                add = rng.sample(&add, w);
            }
            // bits to drop: set in the mask, not in the target code; sampled without replacement
            let mut ra: Vec<u16> = Vec::with_capacity(w);
            if self.drop_lowest {
                // Python reference rule: 4w+8 draws, keep eligible, sort, take the lowest
                let mut c: Vec<u16> = (0..4 * w + 8)
                    .map(|_| rng.below(D) as u16)
                    .filter(|&b| bits::get(m, b as usize) && !bits::get(tgt, b as usize))
                    .collect();
                c.sort_unstable();
                c.dedup();
                c.truncate(add.len());
                ra = c;
            } else {
                let mut tries = 0;
                while ra.len() < add.len() && tries < 4 * w + 8 {
                    tries += 1;
                    let b = rng.below(D) as u16;
                    if bits::get(m, b as usize) && !bits::get(tgt, b as usize) && !ra.contains(&b) {
                        ra.push(b);
                    }
                }
            }
            add.truncate(ra.len());
            for &b in &ra {
                bits::clear(m, b as usize);
                for &c in &tix.owners[b as usize] {
                    r.counts[c as usize] -= 1;
                }
            }
            for &b in &add {
                bits::set(m, b as usize);
                for &c in &tix.owners[b as usize] {
                    r.counts[c as usize] += 1;
                }
            }
        }
    }

    /// Gradient step on the edge gain: d(log p(y))/d(beta) = s(y) - E_p[s], RMS-normalised.
    pub fn learn_beta(&mut self, p: &[f64], yi: usize, lr: f64) {
        let sc = match &self.sc {
            Some(s) => s,
            None => return,
        };
        let e: f64 = p.iter().zip(sc.iter()).map(|(a, b)| a * b).sum();
        let g = sc[yi] - e;
        self.v = 0.999 * self.v + 0.001 * g * g;
        self.beta += lr * g / (self.v.sqrt() + 1e-4);
    }

    pub fn reset_stats(&mut self) {
        self.sum = 0.0;
        self.sq = 0.0;
        self.n = 0;
    }

    /// Mean and z-score of the recorded paired gains.
    pub fn stats(&self) -> (f64, f64) {
        let n = self.n as f64;
        let m = self.sum / n;
        let sd = (self.sq / n - m * m).max(1e-12).sqrt();
        (m, m / (sd / n.sqrt()))
    }
}
