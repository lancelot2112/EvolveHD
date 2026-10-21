//! A bitwise hippocampal store (experiment 31c): learned segmentation decides what gets bound, direction and a
//! learned choice of cue word and hop decide how it is read back. Only bits and small integers: no floats inside
//! the store (the word level's own mixing of what it offers stays as it is).
//!
//! Codes. Every word has a fixed block code f(w): `s` blocks of `l` bits, one bit set per block (computed from its
//! id by a hash, not stored).
//!
//! Segmentation (learned). For each word u, three bit rows hold the code of the word seen just before u at one
//! earlier occurrence, just after it, and of that (before, after) pair; a row is rewritten with probability
//! 1 / `seg_r` per occurrence (a stochastic write), so it holds a random earlier draw. A "hit" is an item whose
//! bits are all present, i.e. the same as that draw, so over n occurrences the hit rates are collision
//! probabilities, 2^-H2 (Rényi entropy). u is a boundary when what follows u carries little of what came before:
//!   I / min(H_prev, H_next) < 1/3   <=>   lo^3 hi^2 > n^2 h_pair^3      (lo, hi = the smaller, larger of the
//! before and after hit counts), an integer test; never when one side is nearly fixed.
//!
//! Memory. Two bit matrices (d x d, d = s l), forward (address f(earlier) -> content f(later)) and backward,
//! each a ring of `planes` planes; the head plane takes the writes and the ring advances every `epoch` words,
//! clearing the plane it reuses. Each word w is bound to the words before it, back to and including the first
//! boundary (at most `win`). Reading a word's code: for each address row, each content bit counts from its
//! newest plane only (so a later binding overrides an older one), weighted planes - age.
//!
//! Recall. Channels: for each of the last `cue` words (position j) and direction, hop 1 (its partners); and for
//! each of those, hop 2 (the partners, in either direction, of its `k2` best partners). Words are scored through
//! their codes (the sum of their content bits' counts), the cue words left out. Attention: 4-bit counters, one
//! per bit of the context's address (the codes of the words 1, 3 and 4 back, each permuted by position, thinned
//! to one bit per block) and channel (8-bit counters); learned from errors only: when another channel ranked the
//! word that came higher than the chosen channel did, the best-ranking ones go up and the chosen one down. The best hop-1
//! and the best hop-2 channel are offered.
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct HippoCfg {
    pub s: usize,
    pub l: usize,
    pub win: usize,
    pub planes: usize,
    pub epoch: usize,
    pub cue: usize,
    pub k: usize,
    pub k2: usize,
    pub khit: usize,
    /// Context words for the attention, as distances back (0 = the last word).
    pub ctx: Vec<usize>,
    /// Segmentation rows' capacity, in items (two generations of r / 2).
    pub seg_r: u32,
    /// The attention's context address bound (exact context) instead of thinned (shared by contexts with words
    /// in common).
    pub ctx_bind: bool,
    /// Rank recalled words by recall over how often they are bound anywhere.
    pub lift: bool,
    /// Recency weights halve per plane of age (a shift) instead of falling by one.
    pub steep: bool,
    /// Channels offered per group (hop 1, hop 2), best by attention first.
    pub offer: usize,
    /// Offer every channel, each in its own slot.
    pub offer_all: bool,
    /// Weight hop-2 fronts by their rank.
    pub front_rank: bool,
    /// Attention counters track each channel's hit rate (a running average) instead of moving on errors.
    pub att_rate: bool,
    /// Rank by cancelling what all channels of a group recall (instead of the lift ratio).
    pub cancel: bool,
    /// Rank by excess share over the long-run baseline (subtraction) instead of the ratio.
    pub excess: bool,
    /// Roles as phase: contents are bound to their word's phase (steps since the last boundary, below `phase`)
    /// by shifting each block's position by phase * m_b (mod l); reading unbinds by shifting back. 0 = off.
    pub phase: usize,
    /// With phases: hop 2 also from these phases' hop-1 channels (besides any phase).
    pub hop2_ph: Vec<usize>,
    /// Learn which words follow each context (a bit row per address bit) and offer only those.
    pub typed: bool,
    /// Memory as quantized complex holographic traces (src/holo.rs) of this many elements (0 = bit planes),
    /// with `holo_levels` states per part.
    pub holo_d: usize,
    pub holo_levels: usize,
    /// Halve the holographic traces every this many words (0 = never; saturation alone forgets).
    pub holo_decay: usize,
    /// Write magnitude per binding (so halving leaves something), and the superseding threshold in percent of
    /// a fresh binding's score (0 = off).
    pub holo_mag: usize,
    pub holo_sup: usize,
    /// Sparse contextified holographic memory (src/ctxholo.rs): elements, active elements per key, context
    /// slots redrawn per word and per boundary (0 elements = off).
    pub ch_d: usize,
    pub ch_k: usize,
    pub ch_drift: usize,
    pub ch_bdrift: usize,
    /// A second, slow memory beside it: no context (the same keys in every story), many levels, larger, written
    /// with probability 1 / ch2_every. 0 elements = off. The fast one's levels are `ch_levels`.
    pub ch_levels: usize,
    pub ch2_d: usize,
    pub ch2_k: usize,
    pub ch2_levels: usize,
    pub ch2_every: usize,
}

struct SegRow {
    rows: Vec<Vec<u64>>, // stream * 2 + generation
    cur: usize,
    items: u32,
    hits: [u64; 3],
    n: u64,
    boundary: bool,
}

pub struct Hippo {
    pub cfg: HippoCfg,
    d: usize,
    wpr: usize,
    mem: Vec<u64>,
    /// Type memory: for each context address bit, the bits of the words that have come next (ORed).
    tmem: Vec<u64>,
    /// Write counts per content bit, over the whole run.
    busy: Vec<u64>,
    head: usize,
    tick: u64,
    seg: Vec<Option<SegRow>>,
    hist: Vec<u32>,
    /// Each word's phase: steps since the last boundary (a free-running count, reset at boundaries).
    hist_ph: Vec<u32>,
    cur_phase: u32,
    nch: usize,
    att: Vec<u8>,
    last: Option<(Vec<usize>, Vec<Vec<u32>>, [usize; 2])>,
    seen: Vec<bool>,
    words: Vec<u32>,
    pub att_updates: u64,
    /// Every channel's candidates at the last recall (for diagnostics).
    pub chan_lists: Vec<Vec<u32>>,
    pub nch_pub: usize,
    holo: Option<crate::holo::Holo>,
    ch: Option<crate::ctxholo::CtxHolo>,
    ch2: Option<crate::ctxholo::CtxHolo>,
    pub superseded: u64,
}

fn hash(a: u64, b: u64) -> u64 {
    crate::rng::hash2(a, b)
}

impl Hippo {
    pub fn new(cfg: HippoCfg, nw: usize) -> Self {
        let d = cfg.s * cfg.l;
        assert!(d % 64 == 0);
        let wpr = d / 64;
        let np = if cfg.phase > 0 { cfg.phase + 1 } else { 1 };
        let nm = if cfg.ch_d > 0 && cfg.ch2_d > 0 { 2 } else { 1 };
        let nch = (cfg.cue * 2 * np + cfg.cue * 4 * (1 + cfg.hop2_ph.len())) * nm;
        let holo = if cfg.holo_d > 0 {
            Some(crate::holo::Holo::new(cfg.holo_d, cfg.holo_levels, nw, cfg.phase.max(1)))
        } else {
            None
        };
        Hippo {
            d,
            wpr,
            holo,
            ch: if cfg.ch_d > 0 {
                let lv = if cfg.ch_levels > 0 { cfg.ch_levels } else { cfg.holo_levels };
                Some(crate::ctxholo::CtxHolo::new(cfg.ch_d, cfg.ch_k, lv, cfg.ch_drift, cfg.ch_bdrift, nw, true))
            } else {
                None
            },
            ch2: if cfg.ch_d > 0 && cfg.ch2_d > 0 {
                Some(crate::ctxholo::CtxHolo::new(cfg.ch2_d, cfg.ch2_k, cfg.ch2_levels, 0, 0, nw, false))
            } else {
                None
            },
            superseded: 0,
            mem: vec![0; if cfg.holo_d > 0 || cfg.ch_d > 0 { 0 } else { 2 * cfg.planes * d * wpr }],
            busy: vec![0; d],
            tmem: vec![0; if cfg.typed { d * wpr } else { 0 }],
            head: 0,
            tick: 0,
            seg: (0..nw).map(|_| None).collect(),
            hist: vec![],
            hist_ph: vec![],
            cur_phase: 0,
            nch,
            att: vec![128; d * nch],
            last: None,
            seen: vec![false; nw],
            words: vec![],
            att_updates: 0,
            chan_lists: vec![],
            nch_pub: nch,
            cfg,
        }
    }

    fn code(&self, w: u32) -> Vec<u16> {
        (0..self.cfg.s)
            .map(|b| (hash(w as u64 + 1, b as u64) % self.cfg.l as u64) as u16)
            .collect()
    }
    fn bits(&self, c: &[u16]) -> Vec<usize> {
        c.iter()
            .enumerate()
            .map(|(b, &p)| b * self.cfg.l + p as usize)
            .collect()
    }
    /// A code bound to a phase: each block's position moves by ph * m_b (m_b odd, fixed per block).
    fn shift(&self, c: &[u16], ph: u32) -> Vec<u16> {
        let l = self.cfg.l;
        c.iter()
            .enumerate()
            .map(|(b, &p)| {
                ((p as usize + ph as usize * (2 * (hash(77, b as u64) as usize % l) + 1)) % l)
                    as u16
            })
            .collect()
    }
    /// The content bits a word would have in phase ph (None: in any phase, all of them).
    fn wbits(&self, w: u32, ph: Option<u32>) -> Vec<usize> {
        let c = self.code(w);
        match ph {
            Some(p) => self.bits(&self.shift(&c, p)),
            None if self.cfg.phase == 0 => self.bits(&c),
            None => (0..self.cfg.phase as u32)
                .flat_map(|p| self.bits(&self.shift(&c, p)))
                .collect(),
        }
    }
    fn back(&self, j: usize) -> u32 {
        let n = self.hist.len();
        if j < n {
            self.hist[n - 1 - j]
        } else {
            u32::MAX
        }
    }

    pub fn is_boundary(&self, w: u32) -> bool {
        self.seg
            .get(w as usize)
            .and_then(|r| r.as_ref())
            .map(|r| r.boundary)
            .unwrap_or(false)
    }
    /// Segmentation statistics per word: (word, n, hits before, after, pair, boundary).
    pub fn seg_stats(&self) -> Vec<(u32, u64, [u64; 3], bool)> {
        self.seg
            .iter()
            .enumerate()
            .filter_map(|(w, r)| r.as_ref().map(|r| (w as u32, r.n, r.hits, r.boundary)))
            .collect()
    }
    pub fn boundaries(&self) -> Vec<u32> {
        (0..self.seg.len() as u32)
            .filter(|&w| self.is_boundary(w))
            .collect()
    }

    // ---- memory ----
    fn row_at(&self, dir: usize, plane: usize, row: usize) -> usize {
        ((dir * self.cfg.planes + plane) * self.d + row) * self.wpr
    }
    fn write(&mut self, dir: usize, a: &[u16], b: &[u16]) {
        let cb = self.bits(b);
        for ra in self.bits(a) {
            let base = self.row_at(dir, self.head, ra);
            for &x in &cb {
                self.mem[base + x / 64] |= 1u64 << (x % 64);
            }
        }
        for &x in &cb {
            self.busy[x] += 1;
        }
    }
    /// Content counts for a code: each content bit from its newest plane, weighted planes - age.
    fn read(&self, dir: usize, a: &[u16], out: &mut [u32]) {
        let p = self.cfg.planes;
        let mut seen = vec![0u64; self.wpr];
        for ra in self.bits(a) {
            seen.iter_mut().for_each(|x| *x = 0);
            for k in 0..p {
                let plane = (self.head + p - k) % p;
                let base = self.row_at(dir, plane, ra);
                let wt = if self.cfg.steep {
                    1u32 << (p - 1 - k)
                } else {
                    (p - k) as u32
                };
                for wi in 0..self.wpr {
                    let mut x = self.mem[base + wi] & !seen[wi];
                    seen[wi] |= x;
                    while x != 0 {
                        let tz = x.trailing_zeros() as usize;
                        out[wi * 64 + tz] += wt;
                        x &= x - 1;
                    }
                }
            }
        }
    }
    /// Words scored through their codes, best first, the excluded words left out.
    fn rank(&self, sc: &[u32], skip: &[u32], k: usize, ph: Option<u32>) -> Vec<(u32, u32)> {
        // (word, score, baseline): the baseline is how often the word has been bound at all (its content bits'
        // write counts) plus the mean, so a word is ranked by how much more it is recalled here than anywhere;
        // compared by cross-multiplying (no division)
        let mut v: Vec<(u32, u32, u64)> = self
            .words
            .iter()
            .filter(|w| !skip.contains(w))
            .map(|&w| {
                let b = self.wbits(w, ph);
                (
                    w,
                    b.iter().map(|&x| sc[x]).sum::<u32>(),
                    b.iter().map(|&x| self.busy[x]).sum::<u64>(),
                )
            })
            .filter(|x| x.1 > 0)
            .collect();
        if self.cfg.excess {
            // excess share, by subtraction: s_w * B - b_w * S (S, B the totals over the candidates), so a word
            // ranks by how much more of this recall it takes than of all bindings; drops those at or below
            let st: u128 = v.iter().map(|x| x.1 as u128).sum();
            let bt: u128 = v.iter().map(|x| x.2 as u128).sum::<u128>().max(1);
            let mut e: Vec<(u32, u32, i128)> = v
                .iter()
                .map(|x| {
                    (
                        x.0,
                        x.1,
                        (x.1 as u128 * bt) as i128 - (x.2 as u128 * st) as i128,
                    )
                })
                .filter(|x| x.2 > 0)
                .collect();
            e.sort_unstable_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
            e.truncate(k);
            return e.into_iter().map(|x| (x.0, x.1)).collect();
        }
        if self.cfg.lift {
            let mean = self
                .words
                .iter()
                .map(|&w| {
                    self.bits(&self.code(w))
                        .iter()
                        .map(|&x| self.busy[x])
                        .sum::<u64>()
                })
                .sum::<u64>()
                / self.words.len().max(1) as u64;
            v.sort_unstable_by(|a, b| {
                let (x, y) = (
                    a.1 as u128 * (b.2 + mean) as u128,
                    b.1 as u128 * (a.2 + mean) as u128,
                );
                y.cmp(&x).then(a.0.cmp(&b.0))
            });
        } else {
            v.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        }
        v.truncate(k);
        v.into_iter().map(|x| (x.0, x.1)).collect()
    }

    /// Rank each channel's words by cancellation: the word's count in this channel times the number of
    /// channels, minus its counts summed over all of them (what every cue recalls cancels out). Integer add
    /// and subtract only.
    fn rank_cancel(
        &self,
        scs: &[(Vec<u32>, Option<u32>)],
        skip: &[u32],
        k: usize,
    ) -> Vec<Vec<(u32, u32)>> {
        let words: Vec<u32> = self
            .words
            .iter()
            .copied()
            .filter(|w| !skip.contains(w))
            .collect();
        let ws: Vec<Vec<i64>> = scs
            .iter()
            .map(|(sc, ph)| {
                words
                    .iter()
                    .map(|&w| self.wbits(w, *ph).iter().map(|&x| sc[x] as i64).sum())
                    .collect()
            })
            .collect();
        let n = scs.len() as i64;
        let tot: Vec<i64> = (0..words.len())
            .map(|i| ws.iter().map(|v| v[i]).sum())
            .collect();
        ws.iter()
            .map(|v| {
                let mut r: Vec<(u32, i64)> = (0..words.len())
                    .map(|i| (words[i], n * v[i] - tot[i]))
                    .filter(|x| x.1 > 0)
                    .collect();
                r.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                r.truncate(k);
                r.into_iter()
                    .map(|(w, x)| (w, x.min(u32::MAX as i64) as u32))
                    .collect()
            })
            .collect()
    }

    /// The channels' lists from the holographic traces: hop 1 per (cue word, direction, phase or any), hop 2
    /// from the best words of each (cue word, direction) in any phase and in the phases of `hop2_ph`, either
    /// direction. Words are ranked by excess share over the background (subtraction).
    fn holo_lists(&self, cue: &[u32], skipf: &[u32], kk: usize, np: usize, lists: &mut [Vec<(u32, u32)>]) {
        let h = self.holo.as_ref().unwrap();
        let nr = self.cfg.phase.max(1);
        let c = self.cfg.cue;
        let n1 = c * 2 * np;
        let ph_of = |p: usize| -> Option<usize> {
            if self.cfg.phase == 0 { Some(0) } else if p == self.cfg.phase { None } else { Some(p) }
        };
        let rank = |z: &(Vec<i32>, Vec<i32>), dir: usize, skip: &[u32], k: usize, r: Option<usize>| -> Vec<(u32, u32)> {
            let v: Vec<(u32, i64, i64)> = self
                .words
                .iter()
                .copied()
                .filter(|w| !skip.contains(w))
                .map(|w| (w, h.score(z, w, r, nr), h.base(dir, w, r, nr).max(0)))
                .filter(|x| x.1 > 0)
                .collect();
            let st: i128 = v.iter().map(|x| x.1 as i128).sum();
            let bt: i128 = v.iter().map(|x| x.2 as i128).sum::<i128>().max(1);
            let mut e: Vec<(u32, i64, i128)> = v.iter().map(|x| (x.0, x.1, x.1 as i128 * bt - x.2 as i128 * st)).filter(|x| x.2 > 0).collect();
            e.sort_unstable_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
            e.truncate(k);
            e.into_iter().map(|x| (x.0, x.1.min(u32::MAX as i64) as u32)).collect()
        };
        let mut zs: HashMap<(u32, usize), (Vec<i32>, Vec<i32>)> = HashMap::new();
        let mut fronts: HashMap<(usize, usize), Vec<u32>> = HashMap::new();
        for (j, &w) in cue.iter().enumerate() {
            for dir in 0..2 {
                let z = zs.entry((w, dir)).or_insert_with(|| h.unbind(dir, w)).clone();
                for p in 0..np {
                    lists[(j * 2 + dir) * np + p] = rank(&z, dir, skipf, kk, ph_of(p));
                    fronts.insert((j * 2 + dir, p), rank(&z, dir, cue, self.cfg.k2, ph_of(p)).into_iter().map(|x| x.0).collect());
                }
            }
        }
        let srcs: Vec<usize> = std::iter::once(np - 1).chain(self.cfg.hop2_ph.iter().copied()).collect();
        for (si, &p1) in srcs.iter().enumerate() {
            for jd in 0..cue.len() * 2 {
                let fr = fronts.get(&(jd, p1)).cloned().unwrap_or_default();
                for d2 in 0..2 {
                    // the unbound traces are linear: hop 2 sums the fronts', halved per rank place
                    let mut z = (vec![0i32; h.d], vec![0i32; h.d]);
                    for (r, &f) in fr.iter().enumerate() {
                        let zf = zs.entry((f, d2)).or_insert_with(|| h.unbind(d2, f));
                        let sh = if self.cfg.front_rank { (r as u32).min(16) } else { 0 };
                        for e in 0..h.d {
                            z.0[e] += (zf.0[e] << 8) >> sh;
                            z.1[e] += (zf.1[e] << 8) >> sh;
                        }
                    }
                    let any = if self.cfg.phase == 0 { Some(0) } else { None };
                    lists[n1 + si * c * 4 + jd * 2 + d2] = rank(&z, d2, skipf, kk, any);
                }
            }
        }
    }

    /// Channel lists from the contextified trace (no phases): hop 1 per (cue word, direction), hop 2 from each
    /// one's best words (scores summed, halved per rank place), either direction. Ranked by excess share over
    /// the background, by subtraction.
    fn ch_lists(&self, cue: &[u32], skipf: &[u32], kk: usize, lists: &mut [Vec<(u32, u32)>]) {
        // memories: the fast one, and the slow one when there is one; channels per memory
        let mems: Vec<&crate::ctxholo::CtxHolo> = std::iter::once(self.ch.as_ref().unwrap()).chain(self.ch2.as_ref()).collect();
        let nm = mems.len();
        let c = self.cfg.cue;
        let n1 = c * 2 * nm;
        let words: Vec<u32> = self.words.clone();
        // per read: every word's (score, baseline)
        let mut reads: HashMap<(u32, usize, usize), Vec<(i64, i64)>> = HashMap::new();
        let get = |w: u32, dir: usize, m: usize, reads: &mut HashMap<(u32, usize, usize), Vec<(i64, i64)>>| -> Vec<(i64, i64)> {
            reads
                .entry((w, dir, m))
                .or_insert_with(|| {
                    let z = mems[m].read(dir, w);
                    words.iter().map(|&x| mems[m].score(&z, x)).collect()
                })
                .clone()
        };
        let rank = |sb: &[(i64, i64)], skip: &[u32], k: usize| -> Vec<(u32, u32)> {
            let v: Vec<(u32, i64, i64)> = self
                .words
                .iter()
                .zip(sb)
                .filter(|(w, _)| !skip.contains(w))
                .map(|(&w, &(s, b))| (w, s, b.max(0)))
                .filter(|x| x.1 > 0)
                .collect();
            let st: i128 = v.iter().map(|x| x.1 as i128).sum();
            let bt: i128 = v.iter().map(|x| x.2 as i128).sum::<i128>().max(1);
            let mut e: Vec<(u32, i64, i128)> = v.iter().map(|x| (x.0, x.1, x.1 as i128 * bt - x.2 as i128 * st)).filter(|x| x.2 > 0).collect();
            e.sort_unstable_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
            e.truncate(k);
            e.into_iter().map(|x| (x.0, x.1.min(u32::MAX as i64) as u32)).collect()
        };
        // hop 1 per (cue word, direction, memory); hop-2 fronts from the fast memory's hop 1
        let mut fronts: Vec<Vec<u32>> = vec![vec![]; c * 2];
        for (j, &w) in cue.iter().enumerate() {
            for dir in 0..2 {
                for m in 0..nm {
                    let sb = get(w, dir, m, &mut reads);
                    lists[(j * 2 + dir) * nm + m] = rank(&sb, skipf, kk);
                    if m == 0 {
                        fronts[j * 2 + dir] = rank(&sb, cue, self.cfg.k2).into_iter().map(|x| x.0).collect();
                    }
                }
            }
        }
        // hop 2: the fronts' partners, either direction, in each memory
        for jd in 0..cue.len() * 2 {
            for d2 in 0..2 {
                for m2 in 0..nm {
                    let mut sb = vec![(0i64, 0i64); self.words.len()];
                    for (r, &f) in fronts[jd].iter().enumerate() {
                        let sh = if self.cfg.front_rank { (r as u32).min(16) } else { 0 };
                        for (o, x) in sb.iter_mut().zip(get(f, d2, m2, &mut reads)) {
                            o.0 += (x.0 << 8) >> sh;
                            o.1 += (x.1 << 8) >> sh;
                        }
                    }
                    lists[n1 + (jd * 2 + d2) * nm + m2] = rank(&sb, skipf, kk);
                }
            }
        }
    }

    // ---- segmentation ----
    fn seg_update(&mut self, a: u32, u: u32, c: u32) {
        let (s, l, wpr) = (self.cfg.s, self.cfg.l, self.wpr);
        let fa = self.code(a);
        let fc = self.code(c);
        let fp: Vec<u16> = (0..s)
            .map(|b| ((fa[b] as usize + 7 * fc[b] as usize + 13) % l) as u16)
            .collect();
        let codes = [self.bits(&fa), self.bits(&fc), self.bits(&fp)];
        let r = self.cfg.seg_r;
        let row = self.seg[u as usize].get_or_insert_with(|| SegRow {
            rows: vec![vec![0u64; wpr]; 6],
            cur: 0,
            items: 0,
            hits: [0; 3],
            n: 0,
            boundary: false,
        });
        // the rows are rewritten only now and then (a stochastic write, probability 1 / r), so each holds an
        // item from a random earlier occurrence (about r back), not the last: hits are then collisions of
        // independent draws rather than of neighbours
        let write = hash(self.tick, u as u64 + 0x5151) % r as u64 == 0 || row.n == 0;
        for (k, cb) in codes.iter().enumerate() {
            let hit = row.n > 0
                && cb
                    .iter()
                    .all(|&x| row.rows[2 * k][x / 64] >> (x % 64) & 1 == 1);
            if hit {
                row.hits[k] += 1;
            }
            if write {
                row.rows[2 * k].iter_mut().for_each(|x| *x = 0);
                for &x in cb {
                    row.rows[2 * k][x / 64] |= 1u64 << (x % 64);
                }
            }
        }
        let _ = (&mut row.cur, &mut row.items);
        row.n += 1;
        if row.n >= 4096 {
            row.n /= 2;
            row.hits.iter_mut().for_each(|h| *h /= 2);
        }
        if row.n >= 64 && row.n % 32 == 0 {
            // with r = 2 each row holds just the last item, so a hit rate is a collision probability, 2^-H2
            // exactly; boundary when I / min(H_prev, H_next) < 1/3:
            //   H_pair - H_max > (2/3) H_min   <=>   lo^3 hi^2 > n^2 pair^3
            let [hp, hn, hj] = row.hits;
            let (lo, hi) = (hp.min(hn) as u128, hp.max(hn) as u128);
            let (n, hj) = (row.n as u128, hj as u128);
            let _ = r;
            row.boundary = 20 * hi <= 19 * n && lo * lo * lo * hi * hi > n * n * hj * hj * hj;
        }
    }

    // ---- attention ----
    fn ctx_addr(&self) -> Vec<usize> {
        let (s, l) = (self.cfg.s, self.cfg.l);
        let codes: Vec<Vec<u16>> = self
            .cfg
            .ctx
            .iter()
            .enumerate()
            .map(|(i, &j)| {
                let c = self.code(self.back(j));
                let mut out = vec![0u16; s];
                for (b, &p) in c.iter().enumerate() {
                    out[(b + i + 1) % s] = ((p as usize + 37 * (i + 1)) % l) as u16;
                }
                out
            })
            .collect();
        if self.cfg.ctx_bind {
            // bound: each block's position is the sum of the words' (permuted) positions, so only the same
            // context words in the same places share an address
            (0..s)
                .map(|b| b * l + codes.iter().map(|c| c[b] as usize).sum::<usize>() % l)
                .collect()
        } else {
            (0..s)
                .map(|b| b * l + codes[b % codes.len()][b] as usize)
                .collect()
        }
    }

    /// The candidates offered for the next word: the best hop-1 channel's and the best hop-2 channel's.
    pub fn recall(&mut self) -> Vec<Vec<(u32, f64)>> {
        let cue: Vec<u32> = (0..self.cfg.cue.min(self.hist.len()))
            .map(|j| self.back(j))
            .collect();
        let kk = self.cfg.k.max(self.cfg.khit).max(self.cfg.k2);
        let mut lists: Vec<Vec<(u32, u32)>> = vec![vec![]; self.nch];
        // the learned type filter: words that have followed this context before (its address rows hold all
        // of the word's bits at half of the address bits or more); offered lists keep only those, fronts for
        // hop 2 are taken before it
        let addr = self.ctx_addr();
        let mut skipf = cue.clone();
        if self.cfg.typed {
            let plaus: Vec<u32> = self
                .words
                .iter()
                .copied()
                .filter(|&w| {
                    let b = self.bits(&self.code(w));
                    let n = addr
                        .iter()
                        .filter(|&&i| b.iter().all(|&x| self.tmem[i * self.wpr + x / 64] >> (x % 64) & 1 == 1))
                        .count();
                    2 * n >= addr.len()
                })
                .collect();
            if !plaus.is_empty() {
                skipf.extend(self.words.iter().copied().filter(|w| !plaus.contains(w)));
            }
        }
        let mut fronts_of: Vec<Vec<u32>> = vec![vec![]; self.nch];
        let mut cache: HashMap<(u32, usize), Vec<u32>> = HashMap::new();
        let d = self.d;
        let get = |me: &Self,
                   w: u32,
                   dir: usize,
                   cache: &mut HashMap<(u32, usize), Vec<u32>>|
         -> Vec<u32> {
            cache
                .entry((w, dir))
                .or_insert_with(|| {
                    let mut sc = vec![0u32; d];
                    me.read(dir, &me.code(w), &mut sc);
                    sc
                })
                .clone()
        };
        let c = self.cfg.cue;
        let np = if self.cfg.phase > 0 {
            self.cfg.phase + 1
        } else {
            1
        };
        let n1 = c * 2 * np * if self.ch2.is_some() { 2 } else { 1 };
        if self.ch.is_some() {
            self.ch_lists(&cue, &skipf, kk, &mut lists);
        } else if self.holo.is_some() {
            self.holo_lists(&cue, &skipf, kk, np, &mut lists);
        } else {
        // hop-1 channel (j, dir, phase): the phase wanted, or any (the last)
        let ph_of = |p: usize| -> Option<u32> {
            if self.cfg.phase == 0 {
                Some(0)
            } else if p == self.cfg.phase {
                None
            } else {
                Some(p as u32)
            }
        };
        let mut scs1: Vec<(usize, Vec<u32>, Option<u32>)> = vec![];
        for (j, &w) in cue.iter().enumerate() {
            for dir in 0..2 {
                let sc = get(self, w, dir, &mut cache);
                for p in 0..np {
                    let i1 = (j * 2 + dir) * np + p;
                    if self.cfg.cancel {
                        scs1.push((i1, sc.clone(), ph_of(p)));
                    } else {
                        fronts_of[i1] = self.rank(&sc, &cue, self.cfg.k2, ph_of(p)).into_iter().map(|x| x.0).collect();
                        lists[i1] = self.rank(&sc, &skipf, kk, ph_of(p));
                    }
                }
            }
        }
        if self.cfg.cancel && !scs1.is_empty() {
            let idx: Vec<usize> = scs1.iter().map(|x| x.0).collect();
            let v: Vec<(Vec<u32>, Option<u32>)> = scs1.into_iter().map(|x| (x.1, x.2)).collect();
            for (i, l) in idx.into_iter().zip(self.rank_cancel(&v, &cue, kk)) {
                lists[i] = l;
            }
        }
        // hop 2 from each (cue word, direction)'s best words in any phase, either direction, any phase
        let mut scs2: Vec<(usize, Vec<u32>, Option<u32>)> = vec![];
        let srcs: Vec<usize> = std::iter::once(np - 1)
            .chain(self.cfg.hop2_ph.iter().copied())
            .collect();
        for (si, &p1) in srcs.iter().enumerate() {
            for jd in 0..cue.len() * 2 {
                let i1 = jd * np + p1;
                let fronts: Vec<u32> = if self.cfg.cancel {
                    lists[i1].iter().take(self.cfg.k2).map(|x| x.0).collect()
                } else {
                    fronts_of[i1].clone()
                };
                for d2 in 0..2 {
                    let mut sc = vec![0u32; d];
                    for (r, &f) in fronts.iter().enumerate() {
                        let s1 = get(self, f, d2, &mut cache);
                        // fronts weighted by rank when asked (halving per place, a shift)
                        let sh = if self.cfg.front_rank {
                            (r as u32).min(16)
                        } else {
                            0
                        };
                        sc.iter_mut()
                            .zip(&s1)
                            .for_each(|(o, &x)| *o += (x << 16) >> sh);
                    }
                    let i2 = n1 + si * c * 4 + jd * 2 + d2;
                    let any = if self.cfg.phase == 0 { Some(0) } else { None };
                    if self.cfg.cancel {
                        scs2.push((i2, sc, any));
                    } else {
                        lists[i2] = self.rank(&sc, &skipf, kk, any);
                    }
                }
            }
        }
        if self.cfg.cancel && !scs2.is_empty() {
            let idx: Vec<usize> = scs2.iter().map(|x| x.0).collect();
            let v: Vec<(Vec<u32>, Option<u32>)> = scs2.into_iter().map(|x| (x.1, x.2)).collect();
            for (i, l) in idx.into_iter().zip(self.rank_cancel(&v, &cue, kk)) {
                lists[i] = l;
            }
        }
        }
        let att: Vec<u32> = (0..self.nch)
            .map(|ch| {
                addr.iter()
                    .map(|&i| self.att[i * self.nch + ch] as u32)
                    .sum()
            })
            .collect();
        let best = |r: std::ops::Range<usize>| {
            r.max_by(|&a, &b| att[a].cmp(&att[b]).then(b.cmp(&a)))
                .unwrap()
        };
        let (b1, b2) = (best(0..n1), best(n1..self.nch));
        let offer = |l: &Vec<(u32, u32)>| -> Vec<(u32, f64)> {
            // weights by rank (halving per place), the ranking being the store's own measure
            let mut o: Vec<(u32, f64)> = l
                .iter()
                .take(self.cfg.k)
                .enumerate()
                .map(|(r, &(w, _))| (w, (1u32 << (self.cfg.k - 1 - r)) as f64))
                .collect();
            let t: f64 = o.iter().map(|x| x.1).sum();
            if t > 0.0 {
                o.iter_mut().for_each(|x| x.1 /= t);
            }
            o
        };
        // offered: the `offer` best channels of each group, by attention (the word level weighs them)
        let order = |r: std::ops::Range<usize>| {
            let mut v: Vec<usize> = r.collect();
            v.sort_by(|&a, &b| att[b].cmp(&att[a]).then(a.cmp(&b)));
            v.truncate(self.cfg.offer);
            v
        };
        if self.cfg.offer_all {
            // every channel in a fixed slot, so the word level can learn which to trust where
            let out: Vec<Vec<(u32, f64)>> = lists.iter().map(|l| offer(l)).collect();
            self.chan_lists = lists
                .iter()
                .map(|l| l.iter().take(self.cfg.k).map(|x| x.0).collect())
                .collect();
            self.last = Some((
                addr,
                lists
                    .into_iter()
                    .map(|l| l.into_iter().take(self.cfg.k).map(|x| x.0).collect())
                    .collect(),
                [b1, b2],
            ));
            return out;
        }
        let mut out = vec![offer(&lists[b1]), offer(&lists[b2])];
        for ch in order(0..n1)
            .into_iter()
            .skip(1)
            .chain(order(n1..self.nch).into_iter().skip(1))
        {
            out.push(offer(&lists[ch]));
        }
        self.chan_lists = lists
            .iter()
            .map(|l| l.iter().take(self.cfg.k).map(|x| x.0).collect())
            .collect();
        self.last = Some((
            addr,
            lists
                .into_iter()
                .map(|l| l.into_iter().take(self.cfg.k).map(|x| x.0).collect())
                .collect(),
            [b1, b2],
        ));
        out
    }

    /// Learn from word w (after it has been predicted).
    pub fn observe(&mut self, w: u32) {
        // attention, learned from errors only: in each group (hop 1, hop 2), when some channel ranked w higher
        // than the chosen one did, the channels that ranked it best move up and the chosen one down (8-bit
        // counters)
        if let Some((addr, lists, chosen)) = self.last.take() {
            if self.cfg.typed {
                let b = self.bits(&self.code(w));
                for &i in &addr {
                    for &x in &b {
                        self.tmem[i * self.wpr + x / 64] |= 1u64 << (x % 64);
                    }
                }
            }
            // with `khit` set, a channel's rank only counts within its first khit words (inclusion, not order)
            let rank: Vec<usize> = lists
                .iter()
                .map(|l| match l.iter().position(|&x| x == w) {
                    Some(p) if self.cfg.khit > 0 && p < self.cfg.khit => 0,
                    Some(p) if self.cfg.khit == 0 => p,
                    _ => usize::MAX,
                })
                .collect();
            let c = self.cfg.cue;
            if self.cfg.att_rate {
                // each channel's counter tracks how often its first khit words held the word that came, in this
                // context: a running average in 8 bits, c += (255 hit - c) / 16 (a shift)
                let hit: Vec<bool> = rank.iter().map(|&r| r != usize::MAX).collect();
                if hit.iter().any(|&h| h) {
                    self.att_updates += 1;
                    for &i in &addr {
                        for ch in 0..self.nch {
                            let a = &mut self.att[i * self.nch + ch];
                            let t: i32 = if hit[ch] { 255 } else { 0 };
                            *a = (*a as i32 + ((t - *a as i32) >> 4)) as u8;
                        }
                    }
                }
            }
            let np = if self.cfg.phase > 0 {
                self.cfg.phase + 1
            } else {
                1
            };
            let n1 = self.cfg.cue * 2 * np * if self.ch2.is_some() { 2 } else { 1 };
            let _ = c;
            for (g, r) in [(0usize, 0..n1), (1, n1..self.nch)]
                .into_iter()
                .filter(|_| !self.cfg.att_rate)
            {
                let b = chosen[g];
                let top = r.clone().map(|ch| rank[ch]).min().unwrap();
                if top == usize::MAX || rank[b] <= top {
                    continue;
                }
                self.att_updates += 1;
                for &i in &addr {
                    for ch in r.clone() {
                        let a = &mut self.att[i * self.nch + ch];
                        if rank[ch] == top {
                            *a = a.saturating_add(1);
                        } else if ch == b {
                            *a = a.saturating_sub(1);
                        }
                    }
                }
            }
        }
        if self.hist.len() >= 2 {
            let (a, u) = (self.back(1), self.back(0));
            self.seg_update(a, u, w);
        }
        // bind w to the words before it, back to and including the first boundary
        let cands: Vec<u32> = if self.cfg.holo_sup > 0 { self.words.clone() } else { vec![] };
        let fw = self.code(w);
        let pw = self.cur_phase;
        let fwp = self.shift(&fw, pw);
        for j in 0..self.cfg.win.min(self.hist.len()) {
            let s = self.back(j);
            if s != w {
                let fs = self.code(s);
                let ps = self.hist_ph[self.hist_ph.len() - 1 - j];
                let fsp = self.shift(&fs, ps);
                if let Some(h) = self.ch.as_mut() {
                    let mag = self.cfg.holo_mag as i16;
                    h.write(0, s, w, mag);
                    h.write(1, w, s, mag);
                    // the slow memory takes each binding with probability 1 / ch2_every (a stochastic write)
                    if let Some(h2) = self.ch2.as_mut() {
                        if hash(self.tick * 131 + j as u64, s as u64 * 7 + w as u64) % self.cfg.ch2_every.max(1) as u64 == 0 {
                            h2.write(0, s, w, 1);
                            h2.write(1, w, s, 1);
                        }
                    }
                } else if let Some(h) = self.holo.as_mut() {
                    let mag = self.cfg.holo_mag as i16;
                    if self.cfg.holo_sup > 0 {
                        // supersede: the value the key held in this role, if it is held clearly and is not the
                        // new one, is taken out (in proportion to how strongly it is still there), then the
                        // new binding goes in
                        let th = (h.d as i64 * mag as i64 * self.cfg.holo_sup as i64) / 100;
                        for (dir, key, val, r) in [(0, s, w, pw as usize), (1, w, s, ps as usize)] {
                            if let Some((u, sc)) = h.best(dir, key, r, &cands) {
                                if u != val && sc > th {
                                    let m = ((sc / h.d as i64) as i16).clamp(1, mag);
                                    h.write_m(dir, key, u, r, -m);
                                    self.superseded += 1;
                                }
                            }
                        }
                    }
                    h.write_m(0, s, w, pw as usize, mag);
                    h.write_m(1, w, s, ps as usize, mag);
                } else {
                    self.write(0, &fs, &fwp);
                    self.write(1, &fw, &fsp);
                }
            }
            if self.is_boundary(s) {
                break;
            }
        }
        if let Some(h) = self.ch.as_mut() {
            let b = self.seg.get(w as usize).and_then(|r| r.as_ref()).map(|r| r.boundary).unwrap_or(false);
            h.step(b);
        }
        self.hist.push(w);
        self.hist_ph.push(pw);
        // the free-running phase: one step per word, back to 0 after a boundary
        if self.cfg.phase > 0 {
            self.cur_phase = if self.is_boundary(w) {
                0
            } else {
                (pw + 1).min(self.cfg.phase as u32 - 1)
            };
        }
        if self.hist.len() > 64 {
            self.hist.drain(..32);
            self.hist_ph.drain(..32);
        }
        if !self.seen[w as usize] {
            self.seen[w as usize] = true;
            self.words.push(w);
        }
        self.tick += 1;
        if self.cfg.holo_decay > 0 && self.tick % self.cfg.holo_decay as u64 == 0 {
            if let Some(h) = self.holo.as_mut() {
                h.halve();
            }
        }
        if self.holo.is_none() && self.ch.is_none() && self.tick % self.cfg.epoch as u64 == 0 {
            self.head = (self.head + 1) % self.cfg.planes;
            for dir in 0..2 {
                let a = self.row_at(dir, self.head, 0);
                let n = self.d * self.wpr;
                self.mem[a..a + n].iter_mut().for_each(|x| *x = 0);
            }
        }
    }
}
