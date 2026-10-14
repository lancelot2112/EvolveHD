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
/// Default bits per mask level, and most bits written per update (the Python reference's values).
pub const KW_DEFAULT: usize = 640;
pub const SWAPS_DEFAULT: usize = 8;

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

impl TargetIndex {
    /// The live target cell whose code this is (the owner of all its bits).
    pub fn cell_of_code(&self, code: &Code) -> Option<usize> {
        let o0 = &self.owners[code[0] as usize];
        o0.iter()
            .copied()
            .find(|&c| code.iter().all(|&b| self.owners[b as usize].contains(&c)))
            .map(|c| c as usize)
    }
}

/// Writes reach level k with probability base^-k (default 2, so counts resolve up to 2^8 = 256).
/// Overridable for experiments with EVOLVEHD_LEVEL_BASE.
fn level_base() -> f64 {
    static V: std::sync::OnceLock<f64> = std::sync::OnceLock::new();
    *V.get_or_init(|| {
        std::env::var("EVOLVEHD_LEVEL_BASE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2.0)
    })
}

/// Overlap counts of a row's masks with every live target code.
fn count_rows(masks: &[Bits], tcodes: &[Code], talive: &[bool]) -> Vec<i32> {
    let mut counts = vec![0; tcodes.len()];
    for (c, code) in tcodes.iter().enumerate() {
        if !talive[c] {
            continue;
        }
        let mut n = 0;
        for m in masks.iter() {
            for &b in code.iter() {
                n += bits::get(m, b as usize) as i32;
            }
        }
        counts[c] = n;
    }
    counts
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
    /// Writes into this row, and which target cells it has been written toward (cells below 1,024).
    pub n: u32,
    pub seen: [u64; 16],
}

impl Row {
    /// Witten-Bell confidence n / (n + T), T = distinct targets written.
    pub fn confidence(&self) -> f64 {
        let t: u32 = self.seen.iter().map(|w| w.count_ones()).sum();
        if self.n == 0 {
            0.0
        } else {
            self.n as f64 / (self.n + t) as f64
        }
    }
}

/// The shared starting masks for new rows of one edge, with their counts for one target version.
struct Template {
    masks: Vec<Bits>,
    counts: Vec<i32>,
    version: u64,
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
    /// New rows copy one random template instead of drawing their own masks.
    pub template: bool,
    /// New rows start empty and fill up to kw bits per level, instead of starting full of random bits.
    pub sparse: bool,
    /// If > 0, every this many writes a row ages: its levels shift down one (all counts halve).
    pub age_every: u32,
    /// Scale each row's scores by its Witten-Bell confidence n / (n + T).
    pub confidence: bool,
    tmpl: Option<Template>,
    /// Second gain, on confidence-scaled scores (used when `confidence` is on), and the current row's confidence.
    pub beta_c: f64,
    v_c: f64,
    lam: f64,
    /// Bits per mask level, and most bits written per update.
    pub kw: usize,
    pub swaps: usize,
}

impl Edge {
    pub fn new(src: usize, beta: f64) -> Self {
        let mut prob = [0.0; LEVELS];
        for (k, p) in prob.iter_mut().enumerate() {
            *p = (1.0 / level_base()).powi(k as i32);
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
            template: false,
            sparse: false,
            age_every: 0,
            confidence: false,
            tmpl: None,
            beta_c: 0.0,
            v_c: 0.01,
            lam: 0.0,
            kw: KW_DEFAULT,
            swaps: SWAPS_DEFAULT,
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
            let kw = self.kw;
            let rand_masks = |rng: &mut Rng| {
                let mut masks = vec![ZERO; LEVELS];
                for m in masks.iter_mut() {
                    for b in rng.choose_distinct(D, kw) {
                        bits::set(m, b);
                    }
                }
                masks
            };
            let row = if self.sparse {
                Row {
                    gen,
                    masks: vec![ZERO; LEVELS],
                    counts: vec![0; tcodes.len()],
                    version: tix.version,
                    n: 0,
                    seen: [0; 16],
                }
            } else if self.template {
                let t = self.tmpl.get_or_insert_with(|| Template {
                    masks: rand_masks(rng),
                    counts: Vec::new(),
                    version: u64::MAX,
                });
                if t.version != tix.version {
                    t.counts = count_rows(&t.masks, tcodes, talive);
                    t.version = tix.version;
                }
                Row {
                    gen,
                    masks: t.masks.clone(),
                    counts: t.counts.clone(),
                    version: t.version,
                    n: 0,
                    seen: [0; 16],
                }
            } else {
                Row {
                    gen,
                    masks: rand_masks(rng),
                    counts: Vec::new(),
                    version: u64::MAX,
                    n: 0,
                    seen: [0; 16],
                }
            };
            self.rows.insert(i, row);
        }
        let r = self.rows.get_mut(&i).unwrap();
        if r.version != tix.version {
            r.counts = count_rows(&r.masks, tcodes, talive);
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
        let lam = r.confidence();
        let s = 1.0 / (CK * LEVELS) as f64;
        let v: Vec<f64> = alive_idx.iter().map(|&c| r.counts[c] as f64 * s).collect();
        self.sc = Some(v);
        self.lam = lam;
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
        r.n += 1;
        if self.age_every > 0 && r.n % self.age_every == 0 {
            // age the row: every level takes the next one's bits, the top empties (all counts halve)
            for k in 0..LEVELS - 1 {
                r.masks[k] = r.masks[k + 1];
            }
            r.masks[LEVELS - 1] = ZERO;
            r.counts = count_rows(&r.masks, tcodes, talive);
        }
        if let Some(c) = tix.cell_of_code(ycode) {
            if c < 1024 {
                r.seen[c / 64] |= 1 << (c % 64);
            }
        }
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
            let w = self.swaps.min(add.len());
            if w < add.len() {
                add = rng.sample(&add, w);
            }
            // sparse rows fill freely up to kw bits and drop only what they must beyond that
            let room = if self.sparse {
                self.kw.saturating_sub(bits::count(m) as usize)
            } else {
                0
            };
            let need = add.len().saturating_sub(room);
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
                c.truncate(need);
                ra = c;
            } else {
                let mut tries = 0;
                while ra.len() < need && tries < 4 * w + 8 {
                    tries += 1;
                    let b = rng.below(D) as u16;
                    if bits::get(m, b as usize) && !bits::get(tgt, b as usize) && !ra.contains(&b) {
                        ra.push(b);
                    }
                }
            }
            add.truncate(room + ra.len());
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

    /// This edge's addition to the logits: beta * s, plus beta_c * confidence * s when enabled.
    pub fn contribution(&self) -> Option<Vec<f64>> {
        let sc = self.sc.as_ref()?;
        let w = self.beta
            + if self.confidence {
                self.beta_c * self.lam
            } else {
                0.0
            };
        Some(sc.iter().map(|s| w * s).collect())
    }

    /// Gradient step on the edge gains: d(log p(y))/d(beta) = s(y) - E_p[s], RMS-normalised;
    /// the confidence gain sees the same gradient times the row's confidence.
    pub fn learn_beta(&mut self, p: &[f64], yi: usize, lr: f64) {
        let sc = match &self.sc {
            Some(s) => s,
            None => return,
        };
        let e: f64 = p.iter().zip(sc.iter()).map(|(a, b)| a * b).sum();
        let g = sc[yi] - e;
        self.v = 0.999 * self.v + 0.001 * g * g;
        self.beta += lr * g / (self.v.sqrt() + 1e-4);
        if self.confidence {
            let gc = g * self.lam;
            self.v_c = 0.999 * self.v_c + 0.001 * gc * gc;
            self.beta_c += lr * gc / (self.v_c.sqrt() + 1e-4);
        }
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
