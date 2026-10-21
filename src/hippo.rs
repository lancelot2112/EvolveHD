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
    /// Write counts per content bit, over the whole run.
    busy: Vec<u64>,
    head: usize,
    tick: u64,
    seg: Vec<Option<SegRow>>,
    hist: Vec<u32>,
    nch: usize,
    att: Vec<u8>,
    last: Option<(Vec<usize>, Vec<Vec<u32>>, [usize; 2])>,
    seen: Vec<bool>,
    words: Vec<u32>,
    pub att_updates: u64,
    /// Every channel's candidates at the last recall (for diagnostics).
    pub chan_lists: Vec<Vec<u32>>,
    pub nch_pub: usize,
}

fn hash(a: u64, b: u64) -> u64 {
    crate::rng::hash2(a, b)
}

impl Hippo {
    pub fn new(cfg: HippoCfg, nw: usize) -> Self {
        let d = cfg.s * cfg.l;
        assert!(d % 64 == 0);
        let wpr = d / 64;
        let nch = cfg.cue * 2 + cfg.cue * 4;
        Hippo {
            d,
            wpr,
            mem: vec![0; 2 * cfg.planes * d * wpr],
            busy: vec![0; d],
            head: 0,
            tick: 0,
            seg: (0..nw).map(|_| None).collect(),
            hist: vec![],
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
        (0..self.cfg.s).map(|b| (hash(w as u64 + 1, b as u64) % self.cfg.l as u64) as u16).collect()
    }
    fn bits(&self, c: &[u16]) -> Vec<usize> {
        c.iter().enumerate().map(|(b, &p)| b * self.cfg.l + p as usize).collect()
    }
    fn back(&self, j: usize) -> u32 {
        let n = self.hist.len();
        if j < n { self.hist[n - 1 - j] } else { u32::MAX }
    }

    pub fn is_boundary(&self, w: u32) -> bool {
        self.seg.get(w as usize).and_then(|r| r.as_ref()).map(|r| r.boundary).unwrap_or(false)
    }
    /// Segmentation statistics per word: (word, n, hits before, after, pair, boundary).
    pub fn seg_stats(&self) -> Vec<(u32, u64, [u64; 3], bool)> {
        self.seg.iter().enumerate().filter_map(|(w, r)| r.as_ref().map(|r| (w as u32, r.n, r.hits, r.boundary))).collect()
    }
    pub fn boundaries(&self) -> Vec<u32> {
        (0..self.seg.len() as u32).filter(|&w| self.is_boundary(w)).collect()
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
                let wt = if self.cfg.steep { 1u32 << (p - 1 - k) } else { (p - k) as u32 };
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
    fn rank(&self, sc: &[u32], skip: &[u32], k: usize) -> Vec<(u32, u32)> {
        // (word, score, baseline): the baseline is how often the word has been bound at all (its content bits'
        // write counts) plus the mean, so a word is ranked by how much more it is recalled here than anywhere;
        // compared by cross-multiplying (no division)
        let mut v: Vec<(u32, u32, u64)> = self
            .words
            .iter()
            .filter(|w| !skip.contains(w))
            .map(|&w| {
                let b = self.bits(&self.code(w));
                (w, b.iter().map(|&x| sc[x]).sum::<u32>(), b.iter().map(|&x| self.busy[x]).sum::<u64>())
            })
            .filter(|x| x.1 > 0)
            .collect();
        if self.cfg.excess {
            // excess share, by subtraction: s_w * B - b_w * S (S, B the totals over the candidates), so a word
            // ranks by how much more of this recall it takes than of all bindings; drops those at or below
            let st: u128 = v.iter().map(|x| x.1 as u128).sum();
            let bt: u128 = v.iter().map(|x| x.2 as u128).sum::<u128>().max(1);
            let mut e: Vec<(u32, u32, i128)> = v.iter().map(|x| (x.0, x.1, (x.1 as u128 * bt) as i128 - (x.2 as u128 * st) as i128)).filter(|x| x.2 > 0).collect();
            e.sort_unstable_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
            e.truncate(k);
            return e.into_iter().map(|x| (x.0, x.1)).collect();
        }
        if self.cfg.lift {
            let mean = self.words.iter().map(|&w| self.bits(&self.code(w)).iter().map(|&x| self.busy[x]).sum::<u64>()).sum::<u64>()
                / self.words.len().max(1) as u64;
            v.sort_unstable_by(|a, b| {
                let (x, y) = (a.1 as u128 * (b.2 + mean) as u128, b.1 as u128 * (a.2 + mean) as u128);
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
    fn rank_cancel(&self, scs: &[Vec<u32>], skip: &[u32], k: usize) -> Vec<Vec<(u32, u32)>> {
        let words: Vec<u32> = self.words.iter().copied().filter(|w| !skip.contains(w)).collect();
        let bits: Vec<Vec<usize>> = words.iter().map(|&w| self.bits(&self.code(w))).collect();
        let ws: Vec<Vec<i64>> = scs.iter().map(|sc| bits.iter().map(|b| b.iter().map(|&x| sc[x] as i64).sum()).collect()).collect();
        let n = scs.len() as i64;
        let tot: Vec<i64> = (0..words.len()).map(|i| ws.iter().map(|v| v[i]).sum()).collect();
        ws.iter()
            .map(|v| {
                let mut r: Vec<(u32, i64)> = (0..words.len()).map(|i| (words[i], n * v[i] - tot[i])).filter(|x| x.1 > 0).collect();
                r.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                r.truncate(k);
                r.into_iter().map(|(w, x)| (w, x.min(u32::MAX as i64) as u32)).collect()
            })
            .collect()
    }

    // ---- segmentation ----
    fn seg_update(&mut self, a: u32, u: u32, c: u32) {
        let (s, l, wpr) = (self.cfg.s, self.cfg.l, self.wpr);
        let fa = self.code(a);
        let fc = self.code(c);
        let fp: Vec<u16> = (0..s).map(|b| ((fa[b] as usize + 7 * fc[b] as usize + 13) % l) as u16).collect();
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
            let hit = row.n > 0 && cb.iter().all(|&x| row.rows[2 * k][x / 64] >> (x % 64) & 1 == 1);
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
            (0..s).map(|b| b * l + codes.iter().map(|c| c[b] as usize).sum::<usize>() % l).collect()
        } else {
            (0..s).map(|b| b * l + codes[b % codes.len()][b] as usize).collect()
        }
    }

    /// The candidates offered for the next word: the best hop-1 channel's and the best hop-2 channel's.
    pub fn recall(&mut self) -> Vec<Vec<(u32, f64)>> {
        let cue: Vec<u32> = (0..self.cfg.cue.min(self.hist.len())).map(|j| self.back(j)).collect();
        let kk = self.cfg.k.max(self.cfg.khit).max(self.cfg.k2);
        let mut lists: Vec<Vec<(u32, u32)>> = vec![vec![]; self.nch];
        let mut cache: HashMap<(u32, usize), Vec<u32>> = HashMap::new();
        let d = self.d;
        let get = |me: &Self, w: u32, dir: usize, cache: &mut HashMap<(u32, usize), Vec<u32>>| -> Vec<u32> {
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
        let mut scs1: Vec<Vec<u32>> = vec![];
        for (j, &w) in cue.iter().enumerate() {
            for dir in 0..2 {
                let sc = get(self, w, dir, &mut cache);
                if self.cfg.cancel {
                    scs1.push(sc);
                } else {
                    lists[j * 2 + dir] = self.rank(&sc, &cue, kk);
                }
            }
        }
        if self.cfg.cancel {
            for (i, l) in self.rank_cancel(&scs1, &cue, kk).into_iter().enumerate() {
                lists[i] = l;
            }
        }
        let mut scs2: Vec<(usize, Vec<u32>)> = vec![];
        for j in 0..cue.len() {
            for d1 in 0..2 {
                let fronts: Vec<u32> = lists[j * 2 + d1].iter().take(self.cfg.k2).map(|x| x.0).collect();
                for d2 in 0..2 {
                    let mut sc = vec![0u32; d];
                    for (r, &f) in fronts.iter().enumerate() {
                        let s1 = get(self, f, d2, &mut cache);
                        // fronts weighted by rank when asked (halving per place, a shift)
                        let sh = if self.cfg.front_rank { (r as u32).min(16) } else { 0 };
                        sc.iter_mut().zip(&s1).for_each(|(o, &x)| *o += (x << 16) >> sh);
                    }
                    if self.cfg.cancel {
                        scs2.push((2 * c + j * 4 + d1 * 2 + d2, sc));
                    } else {
                        lists[2 * c + j * 4 + d1 * 2 + d2] = self.rank(&sc, &cue, kk);
                    }
                }
            }
        }
        if self.cfg.cancel && !scs2.is_empty() {
            let (idx, sc2): (Vec<usize>, Vec<Vec<u32>>) = scs2.into_iter().unzip();
            for (i, l) in idx.into_iter().zip(self.rank_cancel(&sc2, &cue, kk)) {
                lists[i] = l;
            }
        }
        let addr = self.ctx_addr();
        let att: Vec<u32> = (0..self.nch).map(|ch| addr.iter().map(|&i| self.att[i * self.nch + ch] as u32).sum()).collect();
        let best = |r: std::ops::Range<usize>| r.max_by(|&a, &b| att[a].cmp(&att[b]).then(b.cmp(&a))).unwrap();
        let (b1, b2) = (best(0..2 * c), best(2 * c..self.nch));
        let offer = |l: &Vec<(u32, u32)>| -> Vec<(u32, f64)> {
            // weights by rank (halving per place), the ranking being the store's own measure
            let mut o: Vec<(u32, f64)> = l.iter().take(self.cfg.k).enumerate().map(|(r, &(w, _))| (w, (1u32 << (self.cfg.k - 1 - r)) as f64)).collect();
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
            self.chan_lists = lists.iter().map(|l| l.iter().take(self.cfg.k).map(|x| x.0).collect()).collect();
            self.last = Some((addr, lists.into_iter().map(|l| l.into_iter().take(self.cfg.k).map(|x| x.0).collect()).collect(), [b1, b2]));
            return out;
        }
        let mut out = vec![offer(&lists[b1]), offer(&lists[b2])];
        for ch in order(0..2 * c).into_iter().skip(1).chain(order(2 * c..self.nch).into_iter().skip(1)) {
            out.push(offer(&lists[ch]));
        }
        self.chan_lists = lists.iter().map(|l| l.iter().take(self.cfg.k).map(|x| x.0).collect()).collect();
        self.last = Some((addr, lists.into_iter().map(|l| l.into_iter().take(self.cfg.k).map(|x| x.0).collect()).collect(), [b1, b2]));
        out
    }

    /// Learn from word w (after it has been predicted).
    pub fn observe(&mut self, w: u32) {
        // attention, learned from errors only: in each group (hop 1, hop 2), when some channel ranked w higher
        // than the chosen one did, the channels that ranked it best move up and the chosen one down (8-bit
        // counters)
        if let Some((addr, lists, chosen)) = self.last.take() {
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
            for (g, r) in [(0usize, 0..2 * c), (1, 2 * c..self.nch)].into_iter().filter(|_| !self.cfg.att_rate) {
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
        let fw = self.code(w);
        for j in 0..self.cfg.win.min(self.hist.len()) {
            let s = self.back(j);
            if s != w {
                let fs = self.code(s);
                self.write(0, &fs, &fw);
                self.write(1, &fw, &fs);
            }
            if self.is_boundary(s) {
                break;
            }
        }
        self.hist.push(w);
        if self.hist.len() > 64 {
            self.hist.drain(..32);
        }
        if !self.seen[w as usize] {
            self.seen[w as usize] = true;
            self.words.push(w);
        }
        self.tick += 1;
        if self.tick % self.cfg.epoch as u64 == 0 {
            self.head = (self.head + 1) % self.cfg.planes;
            for dir in 0..2 {
                let a = self.row_at(dir, self.head, 0);
                let n = self.d * self.wpr;
                self.mem[a..a + n].iter_mut().for_each(|x| *x = 0);
            }
        }
    }
}
