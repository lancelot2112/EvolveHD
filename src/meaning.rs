//! Meaning codes and the states built from them (the Rust port of `scripts/meaning_codes.py`, experiments 27-28).
//!
//! Every word has a fixed random identity code. Its meaning is learned as the text is read: a counter per bit,
//! to which each co-occurrence with another content word (within `win` content words, both directions) adds the
//! other word's identity code, weighted by the other word's rarity. The meaning code is the `k_word` most
//! distinctive bits (counts divided by how busy each bit is across all words).
//! - Fast mapping: on a word's first `fast_n` occurrences, the story code lit at that moment is added to its
//!   counters (`fast` per bit), so a new word starts out meaning what the story is about.
//! - Story state: a decaying sum (half-life `half` content words) of the meaning codes of recent content words;
//!   its code is the `k_story` most lit bits.
//! - Working memory: state <- fade * rotate(state) + identity code, over every word (half-life `wm_half` words);
//!   its code is the `wm_k` most active bits.
//! Content words are those with id >= `skip` (ids are frequency ranks).
use crate::rng::Rng;
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct MeaningCfg {
    pub d: usize,
    pub k_word: usize,
    pub k_story: usize,
    pub win: usize,
    pub skip: u32,
    pub half: f64,
    pub fast: f32,
    pub fast_n: u64,
    pub wm_half: f64,
    pub wm_k: usize,
    pub wm_rot: bool,
    /// Build the story state from random codes of the same size instead of meaning codes (the control).
    pub identity: bool,
    pub seed: u64,
}

impl Default for MeaningCfg {
    fn default() -> Self {
        MeaningCfg {
            d: 2048,
            k_word: 64,
            k_story: 64,
            win: 16,
            skip: 100,
            half: 200.0,
            fast: 0.0,
            fast_n: 8,
            wm_half: 0.0,
            wm_k: 64,
            wm_rot: true,
            identity: false,
            seed: 1,
        }
    }
}

pub struct Meaning {
    pub cfg: MeaningCfg,
    pub nw: usize,
    /// Identity codes, 32 bits per word; and the control's random codes, k_word bits per word.
    ident: Vec<Vec<u32>>,
    ident_k: Vec<Vec<u32>>,
    /// Meaning counters, nw x d, and how much each bit is used across all words.
    m: Vec<f32>,
    mcol: Vec<f64>,
    /// Occurrences so far, and content-word occurrences so far.
    pub fc: Vec<u64>,
    ctot: u64,
    recent: VecDeque<u32>,
    act: Vec<f64>,
    g: f64,
    /// The story code (bit positions) and a membership mask.
    pub code: Vec<u32>,
    pub incode: Vec<bool>,
    wm: Vec<f32>,
    wm_fade: f32,
    pub wm_code: Vec<u32>,
    scratch: Vec<f32>,
}

/// Indices of the k largest values (only strictly positive ones), in no particular order.
fn top_k(v: &[f32], k: usize, idx: &mut Vec<u32>) -> Vec<u32> {
    idx.clear();
    idx.extend((0..v.len() as u32).filter(|&i| v[i as usize] > 0.0));
    if idx.len() > k {
        idx.select_nth_unstable_by(k - 1, |&a, &b| v[b as usize].partial_cmp(&v[a as usize]).unwrap());
        idx.truncate(k);
    }
    idx.clone()
}

impl Meaning {
    pub fn new(cfg: MeaningCfg, nw: usize) -> Self {
        let mut rng = Rng::new(cfg.seed);
        let d = cfg.d;
        let ident: Vec<Vec<u32>> = (0..nw)
            .map(|_| rng.choose_distinct(d, 32).into_iter().map(|x| x as u32).collect())
            .collect();
        let ident_k: Vec<Vec<u32>> = if cfg.identity {
            (0..nw)
                .map(|_| rng.choose_distinct(d, cfg.k_word).into_iter().map(|x| x as u32).collect())
                .collect()
        } else {
            vec![]
        };
        let m = if cfg.identity { vec![] } else { vec![0.0f32; nw * d] };
        let wm_fade = if cfg.wm_half > 0.0 { 2f64.powf(-1.0 / cfg.wm_half) as f32 } else { 0.0 };
        Meaning {
            nw,
            ident,
            ident_k,
            m,
            mcol: vec![0.0; d],
            fc: vec![0; nw],
            ctot: 0,
            recent: VecDeque::new(),
            act: vec![0.0; d],
            g: 1.0,
            code: vec![],
            incode: vec![false; d],
            wm: vec![0.0; if cfg.wm_half > 0.0 { d } else { 0 }],
            wm_fade,
            wm_code: vec![],
            scratch: vec![0.0; d],
            cfg,
        }
    }

    /// A word's meaning code: its `k_word` most distinctive bits, or None if it has none yet.
    pub fn mcode(&mut self, w: u32) -> Option<Vec<u32>> {
        if w < self.cfg.skip {
            return None;
        }
        if self.cfg.identity {
            return Some(self.ident_k[w as usize].clone());
        }
        let d = self.cfg.d;
        let row = &self.m[w as usize * d..(w as usize + 1) * d];
        if !row.iter().any(|&x| x != 0.0) {
            return None;
        }
        let mean = self.mcol.iter().sum::<f64>() / d as f64;
        for i in 0..d {
            self.scratch[i] = (row[i] as f64 / (self.mcol[i] + mean + 1e-9)) as f32;
        }
        let mut idx = Vec::with_capacity(d);
        let s = std::mem::take(&mut self.scratch);
        let r = top_k(&s, self.cfg.k_word, &mut idx);
        self.scratch = s;
        Some(r)
    }

    /// Learn from word `w` (call after predicting it): occurrence counts, links, fast mapping, story state and
    /// working memory.
    pub fn observe(&mut self, w: u32) {
        let wi = w as usize;
        self.fc[wi] += 1;
        let d = self.cfg.d;
        if w >= self.cfg.skip {
            self.ctot += 1;
            if !self.cfg.identity {
                let ln = |c: u64| (self.ctot as f64 / c as f64).ln() + 1e-3;
                let wt_w = ln(self.fc[wi]) as f32;
                let recent: Vec<u32> = self.recent.iter().copied().collect();
                for c in recent {
                    if c == w {
                        continue;
                    }
                    let wc = ln(self.fc[c as usize]) as f32;
                    for &b in &self.ident[c as usize] {
                        self.m[wi * d + b as usize] += wc;
                        self.mcol[b as usize] += wc as f64;
                    }
                    for &b in &self.ident[wi] {
                        self.m[c as usize * d + b as usize] += wt_w;
                        self.mcol[b as usize] += wt_w as f64;
                    }
                }
                if self.cfg.fast > 0.0 && self.fc[wi] <= self.cfg.fast_n && !self.code.is_empty() {
                    for &b in &self.code {
                        self.m[wi * d + b as usize] += self.cfg.fast;
                        self.mcol[b as usize] += self.cfg.fast as f64;
                    }
                }
            }
            self.recent.push_back(w);
            if self.recent.len() > self.cfg.win {
                self.recent.pop_front();
            }
            let mc = self.mcode(w);
            self.g *= 2f64.powf(1.0 / self.cfg.half);
            if let Some(mc) = mc {
                for b in mc {
                    self.act[b as usize] += self.g;
                }
            }
            if self.g > 1e100 {
                self.act.iter_mut().for_each(|x| *x /= 1e100);
                self.g /= 1e100;
            }
            let a32: Vec<f32> = self.act.iter().map(|&x| (x / self.g) as f32).collect();
            let mut idx = Vec::with_capacity(d);
            for &b in &self.code {
                self.incode[b as usize] = false;
            }
            self.code = top_k(&a32, self.cfg.k_story, &mut idx);
            for &b in &self.code {
                self.incode[b as usize] = true;
            }
        }
        if self.cfg.wm_half > 0.0 {
            if self.cfg.wm_rot {
                self.wm.rotate_right(1);
            }
            let f = self.wm_fade;
            self.wm.iter_mut().for_each(|x| *x *= f);
            for &b in &self.ident[wi] {
                self.wm[b as usize] += 1.0;
            }
            let mut idx = Vec::with_capacity(d);
            let wm = std::mem::take(&mut self.wm);
            self.wm_code = top_k(&wm, self.cfg.wm_k, &mut idx);
            self.wm = wm;
        }
    }
}

/// Rows keyed by code bits, each counting the next word: a counts readout over a code (as in the prototype).
pub struct BitRows {
    d: usize,
    nw: usize,
    /// 32-bit counts, or 16-bit saturating ones to save memory.
    rows32: Vec<u32>,
    rows16: Vec<u16>,
    nb: Vec<u64>,
    pub saturated: u64,
}

impl BitRows {
    pub fn new(d: usize, nw: usize, small: bool) -> Self {
        BitRows {
            d,
            nw,
            rows32: if small { vec![] } else { vec![0; d * nw] },
            rows16: if small { vec![0; d * nw] } else { vec![] },
            nb: vec![0; d],
            saturated: 0,
        }
    }
    fn get(&self, i: usize) -> f64 {
        if self.rows16.is_empty() { self.rows32[i] as f64 } else { self.rows16[i] as f64 }
    }
    /// Probability of word w: the mean over the code's bits of (count + beta * backoff) / (total + beta).
    pub fn prob(&self, code: &[u32], w: u32, backoff: f64, beta: f64) -> f64 {
        if code.is_empty() {
            return backoff;
        }
        let mut s = 0.0;
        for &b in code {
            let r = self.get(b as usize * self.nw + w as usize);
            s += (r + beta * backoff) / (self.nb[b as usize] as f64 + beta);
        }
        s / code.len() as f64
    }
    pub fn learn(&mut self, code: &[u32], w: u32) {
        for &b in code {
            let i = b as usize * self.nw + w as usize;
            if self.rows16.is_empty() {
                self.rows32[i] += 1;
            } else if self.rows16[i] == u16::MAX {
                self.saturated += 1;
            } else {
                self.rows16[i] += 1;
            }
            self.nb[b as usize] += 1;
        }
        let _ = self.d;
    }
}

/// The meaning readout's snapshot: every word's meaning code, with frequency weights, so lit story bits can
/// name the words whose codes contain them.
pub struct Snapshot {
    codes: Vec<Vec<u32>>,
    u: Vec<f64>,
    big_u: Vec<f64>,
    /// Per bit, the frequency weight of words with id >= `oov_from` (words outside a vocabulary).
    big_u_oov: Vec<f64>,
    oov_from: u32,
}

impl Snapshot {
    pub fn take(mn: &mut Meaning) -> Self {
        Self::take_oov(mn, u32::MAX)
    }
    pub fn take_oov(mn: &mut Meaning, oov_from: u32) -> Self {
        let nw = mn.nw;
        let mut codes = vec![vec![]; nw];
        let mut big_u = vec![0.0; mn.cfg.d];
        let mut big_u_oov = vec![0.0; mn.cfg.d];
        let u: Vec<f64> = mn.fc.iter().map(|&c| c as f64).collect();
        for w in 0..nw {
            if mn.fc[w] >= 1 {
                if let Some(c) = mn.mcode(w as u32) {
                    for &b in &c {
                        big_u[b as usize] += u[w];
                        if w as u32 >= oov_from {
                            big_u_oov[b as usize] += u[w];
                        }
                    }
                    codes[w] = c;
                }
            }
        }
        Snapshot { codes, u, big_u, big_u_oov, oov_from }
    }
    /// The probability of word w among the words outside the vocabulary, given the lit story code: its share of
    /// the lit bits, renormalised over the outside words.
    pub fn prob_oov(&self, w: u32, code: &[u32], incode: &[bool]) -> f64 {
        let z: f64 = code
            .iter()
            .filter(|&&b| self.big_u[b as usize] > 0.0)
            .map(|&b| self.big_u_oov[b as usize] / self.big_u[b as usize])
            .sum();
        if z <= 0.0 || w < self.oov_from {
            return 0.0;
        }
        self.prob(w, incode, code.len()) * code.len() as f64 / z
    }
    /// The share of the lit story code that names word w.
    pub fn prob(&self, w: u32, incode: &[bool], nlit: usize) -> f64 {
        let wi = w as usize;
        if nlit == 0 || self.u[wi] <= 0.0 {
            return 0.0;
        }
        let mut s = 0.0;
        for &b in &self.codes[wi] {
            if incode[b as usize] && self.big_u[b as usize] > 0.0 {
                s += self.u[wi] / self.big_u[b as usize];
            }
        }
        s / nlit as f64
    }
}
