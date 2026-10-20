//! A heteroassociative store (experiment 31b): the hippocampal store with no tables of words, roles or
//! channels. Everything is a sparse block code (d bits in `s` blocks of `l`, one active bit per block), bound by
//! adding positions block by block (mod l) and unbound by subtracting them.
//!
//! - filler f(w): a fixed random block code per word, computed from its id (not stored)
//! - role r_t: the binding of the meaning block codes of the two words before t (similar contexts, similar roles)
//! - memory M (d x d counters, fading with half-life `half` words): each word t is written against the words
//!   of a fading window before it, both ways: address f(s) -> content f(t) * r_t, and address f(t) -> content
//!   f(s) * r_s
//! - recall: the cue (the last `cue` words, weighted by recency and rarity) reads v = sum of M's rows at its
//!   address bits, relative to how busy each content bit is; a gate memory G (d x d) maps the context code
//!   (the meaning codes of the last `ctx` words, each permuted by position, thinned to one bit per block, each
//!   block's from one of the words) to the role wanted, ĝ; unbinding v by ĝ and reading the result through an
//!   inverted index of fillers gives scored candidate words. A second hop recalls from the first hop's
//!   candidates, with its own gate.
//! - the gates learn from every word: once the word w is seen, unbinding v by f(w) shows the role w was bound
//!   under (if it was recalled); that role, centred, is added to G's rows at the context's address bits.
use crate::meaning::{Meaning, MeaningCfg};
use crate::rng::hash2;
use std::collections::{HashMap, VecDeque};

#[derive(Clone, Debug)]
pub struct AssocCfg {
    pub s: usize,
    pub l: usize,
    pub win: usize,
    /// Weight of a pair `d` words apart: fade^(d-1).
    pub fade: f64,
    pub half: f64,
    pub cue: usize,
    pub ctx: usize,
    pub k: usize,
    pub hops: usize,
    /// Gate learning rate, and the power the gate's role weights are raised to before use.
    pub glr: f32,
    pub sharpen: f32,
    /// Cue weight: fade^j * ln(n / count)^cue_pow.
    pub cue_pow: f64,
    /// Leave the cue words out of the candidates.
    pub exclude: bool,
    /// Bind contents with their roles (and learn the gates); without, contents are plain fillers.
    pub roles: bool,
    /// Learned attention over the cue: a memory from the context's address to a weight per cue position,
    /// learned (delta rule, rate `cue_lr`) toward how well each position's recall scored the word that came.
    pub cue_gate: bool,
    pub cue_lr: f32,
}

pub struct Assoc {
    pub cfg: AssocCfg,
    d: usize,
    mn: Meaning,
    m: Vec<f32>,
    /// How much each content bit has been written over the whole run (not fading): recall is read relative
    /// to it.
    busy: Vec<f64>,
    g: f64,
    t: u64,
    gates: Vec<Vec<f32>>,
    recent: VecDeque<(u32, Vec<u16>)>,
    hist: VecDeque<u32>,
    cache: HashMap<u32, (u64, Vec<u16>)>,
    index: Vec<Vec<u32>>,
    seen: Vec<bool>,
    last_v: Vec<Vec<f32>>,
    last_addr: Vec<u32>,
    pub gate_writes: u64,
    cue_g: Vec<f32>,
    last_cue: Vec<(Vec<f32>, f32)>,
}

impl Assoc {
    pub fn new(cfg: AssocCfg, nw: usize) -> Self {
        let d = cfg.s * cfg.l;
        let mut mc = MeaningCfg::default();
        mc.d = d;
        mc.skip = 0;
        mc.seed = 7;
        Assoc {
            d,
            mn: Meaning::new(mc, nw),
            m: vec![0.0; d * d],
            busy: vec![0.0; d],
            g: 1.0,
            t: 0,
            gates: vec![vec![0.0; d * d]; cfg.hops],
            recent: VecDeque::new(),
            hist: VecDeque::new(),
            cache: HashMap::new(),
            index: vec![vec![]; d],
            seen: vec![false; nw],
            last_v: vec![],
            last_addr: vec![],
            gate_writes: 0,
            cue_g: vec![0.0; d * cfg.cue],
            last_cue: vec![],
            cfg,
        }
    }

    fn filler(&self, w: u32) -> Vec<u16> {
        (0..self.cfg.s).map(|b| (hash2(w as u64 + 1, b as u64) % self.cfg.l as u64) as u16).collect()
    }

    /// A word's meaning block code (its filler while it has no meaning yet), refreshed every 256 words.
    fn mblock(&mut self, w: u32) -> Vec<u16> {
        if w == u32::MAX {
            return (0..self.cfg.s).map(|b| (hash2(u64::MAX, b as u64) % self.cfg.l as u64) as u16).collect();
        }
        if let Some((t0, c)) = self.cache.get(&w) {
            if self.t - t0 < 256 {
                return c.clone();
            }
        }
        let c = self.mn.block_code(w, self.cfg.s).unwrap_or_else(|| self.filler(w));
        self.cache.insert(w, (self.t, c.clone()));
        c
    }

    fn bind(&self, a: &[u16], b: &[u16]) -> Vec<u16> {
        let l = self.cfg.l as u16;
        a.iter().zip(b).map(|(&x, &y)| (x + y) % l).collect()
    }

    /// Permutation j: block b moves to block b + j, its position shifts by 13 j.
    fn permute(&self, a: &[u16], j: usize) -> Vec<u16> {
        let (s, l) = (self.cfg.s, self.cfg.l);
        let mut out = vec![0u16; s];
        for (b, &p) in a.iter().enumerate() {
            out[(b + j) % s] = ((p as usize + 13 * j) % l) as u16;
        }
        out
    }

    fn back(&self, j: usize) -> u32 {
        let n = self.hist.len();
        if j < n { self.hist[n - 1 - j] } else { u32::MAX }
    }

    fn bits(&self, c: &[u16]) -> Vec<usize> {
        let l = self.cfg.l;
        c.iter().enumerate().map(|(b, &p)| b * l + p as usize).collect()
    }

    /// The context's address: the meaning codes of the last `ctx` words, each permuted by its distance back.
    fn gate_addr(&mut self) -> Vec<u32> {
        // thinned: block b takes its bit from context word (b mod ctx), so the address has one bit per block
        let mut codes = vec![];
        for j in 0..self.cfg.ctx {
            let w = self.back(j);
            let mb = self.mblock(w);
            codes.push(self.permute(&mb, j + 2));
        }
        let l = self.cfg.l;
        (0..self.cfg.s).map(|b| (b * l + codes[b % codes.len()][b] as usize) as u32).collect()
    }

    /// Learn from word w (after it has been predicted).
    pub fn observe(&mut self, w: u32) {
        let (d, l, s) = (self.d, self.cfg.l, self.cfg.s);
        // the gates: the role w was recalled under, centred, added at the context's address
        let fw = self.filler(w);
        if self.cfg.cue_gate && !self.last_cue.is_empty() {
            let busy: Vec<f32> = self.busy.iter().map(|&b| b as f32).collect();
            let lc = std::mem::take(&mut self.last_cue);
            let q: Vec<f32> = lc.iter().map(|(v, _)| self.score_of(v, &busy, &fw)).collect();
            let mean = q.iter().sum::<f32>() / q.len() as f32;
            let a = self.cue_logits(&self.last_addr);
            let c = self.cfg.cue;
            let na = self.last_addr.len().max(1) as f32;
            for (j, &qj) in q.iter().enumerate() {
                let target = ((qj + 0.1) / (mean + 0.1)).ln();
                let step = self.cfg.cue_lr * (target - a[j]) / na;
                for &i in &self.last_addr {
                    self.cue_g[i as usize * c + j] += step;
                }
            }
        }
        let vs = std::mem::take(&mut self.last_v);
        for (h, v) in vs.iter().enumerate().filter(|_| self.cfg.roles) {
            let mut rho = vec![0f32; d];
            let mut any = false;
            for b in 0..s {
                let mut sum = 0f32;
                for q in 0..l {
                    let x = v[b * l + (fw[b] as usize + q) % l].max(0.0);
                    rho[b * l + q] = x;
                    sum += x;
                }
                if sum > 0.0 {
                    any = true;
                    for q in 0..l {
                        rho[b * l + q] = rho[b * l + q] / sum - 1.0 / l as f32;
                    }
                } else {
                    for q in 0..l {
                        rho[b * l + q] = 0.0;
                    }
                }
            }
            if !any {
                continue;
            }
            let lr = self.cfg.glr;
            let gt = &mut self.gates[h];
            for &i in &self.last_addr {
                let row = &mut gt[i as usize * d..(i as usize + 1) * d];
                for (x, &r) in row.iter_mut().zip(&rho) {
                    *x += lr * r;
                }
            }
            self.gate_writes += 1;
        }
        // the word's role and content code (from the words before it)
        let (p1, p2) = (self.back(0), self.back(1));
        let r = {
            let a = self.mblock(p1);
            let b = self.mblock(p2);
            let b = self.permute(&b, 1);
            self.bind(&a, &b)
        };
        let ct = if self.cfg.roles { self.bind(&fw, &r) } else { fw.clone() };
        self.mn.observe(w);
        self.t += 1;
        self.g *= 2f64.powf(1.0 / self.cfg.half);
        if self.g > 1e20 {
            let sc = (1.0 / self.g) as f32;
            self.m.iter_mut().for_each(|x| *x *= sc);
            self.g = 1.0;
        }
        // write against the window, both ways
        let mut wt = self.g;
        let recent: Vec<(u32, Vec<u16>)> = self.recent.iter().rev().cloned().collect();
        for (sw, cs) in recent {
            if sw != w {
                let fs = self.filler(sw);
                let wf = wt as f32;
                for (ra, cc) in [(&fs, &ct), (&fw, &cs)] {
                    for (b2, &pc) in cc.iter().enumerate() {
                        self.busy[b2 * l + pc as usize] += wt / self.g * s as f64;
                    }
                    for (b, &pa) in ra.iter().enumerate() {
                        let row = (b * l + pa as usize) * d;
                        for (b2, &pc) in cc.iter().enumerate() {
                            self.m[row + b2 * l + pc as usize] += wf;
                        }
                    }
                }
            }
            wt *= self.cfg.fade;
        }
        self.recent.push_back((w, ct));
        if self.recent.len() > self.cfg.win {
            self.recent.pop_front();
        }
        self.hist.push_back(w);
        if self.hist.len() > 16 {
            self.hist.pop_front();
        }
        if !self.seen[w as usize] {
            self.seen[w as usize] = true;
            for x in self.bits(&fw) {
                self.index[x].push(w);
            }
        }
    }

    /// Sum of M's rows at the fillers of the weighted cue words.
    fn read(&self, cue: &[(u32, f64)]) -> Vec<f32> {
        let d = self.d;
        let mut v = vec![0f32; d];
        for &(c, wc) in cue {
            let wc = (wc / self.g) as f32;
            for x in self.bits(&self.filler(c)) {
                let row = &self.m[x * d..(x + 1) * d];
                for (o, &y) in v.iter_mut().zip(row) {
                    *o += wc * y;
                }
            }
        }
        v
    }

    /// How well a recall v scores word w (filler fw): the mean over blocks of its share of the block's recall
    /// over its share of the block's baseline.
    fn score_of(&self, v: &[f32], busy: &[f32], fw: &[u16]) -> f32 {
        let l = self.cfg.l;
        let mut tot = 0f32;
        for (b, &p) in fw.iter().enumerate() {
            let tv: f32 = v[b * l..(b + 1) * l].iter().sum();
            let tb: f32 = busy[b * l..(b + 1) * l].iter().sum();
            if tv > 0.0 && tb > 0.0 {
                tot += (v[b * l + p as usize] / tv) / (busy[b * l + p as usize] / tb + 1.0 / l as f32);
            }
        }
        tot / fw.len() as f32
    }

    /// The cue attention's logits for this context, one per cue position.
    fn cue_logits(&self, addr: &[u32]) -> Vec<f32> {
        let c = self.cfg.cue;
        let mut a = vec![0f32; c];
        for &i in addr {
            for (o, &y) in a.iter_mut().zip(&self.cue_g[i as usize * c..(i as usize + 1) * c]) {
                *o += y;
            }
        }
        a
    }

    /// The gate's role weights for this context (per block, clipped, sharpened, summing to 1), as up to 8
    /// (position, weight) pairs per block.
    fn role_wanted(&self, h: usize, addr: &[u32]) -> Vec<Vec<(usize, f32)>> {
        let (d, l, s) = (self.d, self.cfg.l, self.cfg.s);
        let mut u = vec![0f32; d];
        for &i in addr {
            for (o, &y) in u.iter_mut().zip(&self.gates[h][i as usize * d..(i as usize + 1) * d]) {
                *o += y;
            }
        }
        (0..s)
            .map(|b| {
                let mut v: Vec<(usize, f32)> =
                    (0..l).map(|q| (q, u[b * l + q].max(0.0).powf(self.cfg.sharpen))).filter(|x| x.1 > 0.0).collect();
                if v.is_empty() {
                    return (0..l).map(|q| (q, 1.0 / l as f32)).collect();
                }
                v.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));
                v.truncate(8);
                let t: f32 = v.iter().map(|x| x.1).sum();
                v.iter_mut().for_each(|x| x.1 /= t);
                v
            })
            .collect()
    }

    /// Unbind v by the wanted role and score words through the inverted index; the k best (above the
    /// (k+1)-th), normalised, excluding `skip`.
    fn candidates(&self, v: &[f32], busy: &[f32], role: &[Vec<(usize, f32)>], skip: &[u32]) -> Vec<(u32, f64)> {
        let (l, s) = (self.cfg.l, self.cfg.s);
        let mut sc: HashMap<u32, f64> = HashMap::new();
        // unbind the recall and the cue-free baseline (how busy each content bit is) by the wanted role, and
        // score each filler bit by recall over baseline
        let mut phi = vec![0f32; l];
        let mut phi0 = vec![0f32; l];
        for b in 0..s {
            for p in 0..l {
                let (mut x, mut x0) = (0f32, 0f32);
                for &(q, gq) in &role[b] {
                    x += gq * v[b * l + (p + q) % l];
                    x0 += gq * busy[b * l + (p + q) % l];
                }
                phi[p] = x;
                phi0[p] = x0;
            }
            let (tv, t0) = (phi.iter().sum::<f32>(), phi0.iter().sum::<f32>());
            if tv <= 0.0 || t0 <= 0.0 {
                continue;
            }
            for p in 0..l {
                let ws = &self.index[b * l + p];
                if ws.is_empty() {
                    continue;
                }
                let r = (phi[p] / tv) / (phi0[p] / t0 + 1.0 / l as f32);
                for &w in ws {
                    *sc.entry(w).or_insert(0.0) += r as f64;
                }
            }
        }
        let mut out: Vec<(u32, f64)> = sc.into_iter().filter(|x| !skip.contains(&x.0)).collect();
        out.sort_unstable_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let floor = out.get(self.cfg.k).map(|x| x.1).unwrap_or(0.0);
        out.truncate(self.cfg.k);
        let mut v: Vec<(u32, f64)> = out.into_iter().map(|(w, x)| (w, x - floor)).filter(|x| x.1 > 0.0).collect();
        let t: f64 = v.iter().map(|x| x.1).sum();
        if t > 0.0 {
            v.iter_mut().for_each(|x| x.1 /= t);
        }
        v
    }

    /// The candidates recalled for the next word, per hop.
    pub fn recall(&mut self) -> Vec<Vec<(u32, f64)>> {
        let addr = self.gate_addr();
        let n = (self.t as f64).max(1.0);
        let mut cue: Vec<(u32, f64)> = vec![];
        let mut f = 1.0;
        for j in 0..self.cfg.cue.min(self.hist.len()) {
            let w = self.back(j);
            let c = self.mn.fc[w as usize].max(1) as f64;
            let wt = f * (n / c).ln().max(0.0).powf(self.cfg.cue_pow);
            match cue.iter_mut().find(|x| x.0 == w) {
                Some(x) => x.1 += wt,
                None => cue.push((w, wt)),
            }
            f *= self.cfg.fade;
        }
        let skip: Vec<u32> = if self.cfg.exclude { cue.iter().map(|x| x.0).collect() } else { vec![] };
        let mut out = vec![];
        let mut vs = vec![];
        let mut front = cue;
        let busy: Vec<f32> = self.busy.iter().map(|&b| b as f32).collect();
        let mean = busy.iter().sum::<f32>() / self.d as f32;
        let mut first: Option<Vec<f32>> = None;
        if self.cfg.cue_gate {
            // each cue position read alone, weighted by its prior and the learned attention
            let a = self.cue_logits(&addr);
            let mut v = vec![0f32; self.d];
            let mut lc = vec![];
            let mut f = 1.0;
            for j in 0..self.cfg.cue.min(self.hist.len()) {
                let w = self.back(j);
                let c = self.mn.fc[w as usize].max(1) as f64;
                let prior = f * (n / c).ln().max(0.0).powf(self.cfg.cue_pow);
                f *= self.cfg.fade;
                let vj = self.read(&[(w, 1.0)]);
                let wt = (prior * (a[j] as f64).clamp(-20.0, 20.0).exp()) as f32;
                for (o, &x) in v.iter_mut().zip(&vj) {
                    *o += wt * x;
                }
                lc.push((vj, wt));
            }
            self.last_cue = lc;
            first = Some(v);
        }
        for h in 0..self.cfg.hops {
            let v = match (h, first.take()) {
                (0, Some(v)) => v,
                _ => self.read(&front),
            };
            let role = if self.cfg.roles { self.role_wanted(h, &addr) } else { vec![vec![(0, 1.0)]; self.cfg.s] };
            let c = self.candidates(&v, &busy, &role, &skip);
            // for learning the gate: the recall relative to how busy each content bit is
            let vn: Vec<f32> = v.iter().zip(&busy).map(|(&x, &b)| x / (b + mean + 1e-30)).collect();
            vs.push(vn);
            front = c.clone();
            out.push(c);
        }
        self.last_v = vs;
        self.last_addr = addr;
        out
    }
}
