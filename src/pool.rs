//! Matcher pools: the Associate/Expand cell population.
//!
//! k-WTA with k = 1. A cell's mask has K bits; it matches an input by the overlap of its mask with the
//! input's bits. If the best overlap is below `th_new`, a cell is recruited (a free one, else the least
//! recently used) with a mask of K bits sampled from the input. Otherwise, on a partial match, the winner
//! swaps one mask bit toward the input (Hebbian), with probability `hebb`.
//!
//! Matching uses an inverted index (bit -> cells whose mask holds it), and eviction an ordered set of
//! (last use, cell), so both stay cheap at tens of thousands of cells. Winners are exactly those of a
//! linear scan: the highest overlap, lowest index on ties; the least recently used cell, lowest index on ties.
//!
//! A pool can grow: it records how long ago each evicted cell was last used. When most evictions remove
//! recently used cells, its working set does not fit, and the owner may call [`Pool::grow`].
use crate::bits::{self, Bits, Code, D, ZERO};
use crate::rng::Rng;
use std::collections::BTreeSet;

pub struct Pool {
    pub cap: usize,
    pub k: u32,
    pub th_new: u32,
    pub hebb: f64,
    pub masks: Vec<Bits>,
    pub alive: Vec<bool>,
    pub codes: Vec<Code>,
    pub gen: Vec<u32>,
    pub last: Vec<u64>,
    pub born: Vec<u64>,
    pub recruits: u64,
    pub evictions: u64,
    /// An eviction of a cell used within this many ticks counts as pressure to grow.
    pub young_age: u64,
    /// Evictions and young evictions since the last [`Pool::take_pressure`].
    ev_n: u64,
    ev_young: u64,
    post: Vec<Vec<u32>>,
    lru: BTreeSet<(u64, u32)>,
    free: BTreeSet<u32>,
    count: Vec<u16>,
    touched: Vec<u32>,
}

/// Result of one match.
pub struct Match {
    pub cell: usize,
    pub recruited: bool,
    pub overlap: u32,
}

impl Pool {
    pub fn new(cap: usize, k: u32, th_new: u32) -> Self {
        Pool {
            cap,
            k,
            th_new,
            hebb: 1.0,
            masks: vec![ZERO; cap],
            alive: vec![false; cap],
            codes: vec![[0; bits::CK]; cap],
            gen: vec![0; cap],
            last: vec![0; cap],
            born: vec![0; cap],
            recruits: 0,
            evictions: 0,
            young_age: 50_000,
            ev_n: 0,
            ev_young: 0,
            post: vec![Vec::new(); D],
            lru: BTreeSet::new(),
            free: (0..cap as u32).collect(),
            count: vec![0; cap],
            touched: Vec::new(),
        }
    }

    pub fn n_alive(&self) -> usize {
        self.cap - self.free.len()
    }

    /// Add cells, up to `new_cap` in all. They start free.
    pub fn grow(&mut self, new_cap: usize) {
        if new_cap <= self.cap {
            return;
        }
        let n = new_cap;
        self.masks.resize(n, ZERO);
        self.alive.resize(n, false);
        self.codes.resize(n, [0; bits::CK]);
        self.gen.resize(n, 0);
        self.last.resize(n, 0);
        self.born.resize(n, 0);
        self.count.resize(n, 0);
        self.free.extend(self.cap as u32..n as u32);
        self.cap = n;
    }

    /// (evictions, evictions of cells used within `young_age` ticks) since the last call.
    pub fn take_pressure(&mut self) -> (u64, u64) {
        let r = (self.ev_n, self.ev_young);
        self.ev_n = 0;
        self.ev_young = 0;
        r
    }

    /// Mark cell c used at tick t.
    pub fn touch(&mut self, c: usize, t: u64) {
        if self.alive[c] {
            self.lru.remove(&(self.last[c], c as u32));
            self.lru.insert((t, c as u32));
        }
        self.last[c] = t;
    }

    fn set_bit(&mut self, c: usize, b: usize) {
        if !bits::get(&self.masks[c], b) {
            bits::set(&mut self.masks[c], b);
            self.post[b].push(c as u32);
        }
    }

    fn clear_bit(&mut self, c: usize, b: usize) {
        if bits::get(&self.masks[c], b) {
            bits::clear(&mut self.masks[c], b);
            let p = &mut self.post[b];
            if let Some(i) = p.iter().position(|&x| x == c as u32) {
                p.swap_remove(i);
            }
        }
    }

    fn clear_mask(&mut self, c: usize) {
        for b in bits::positions(&self.masks[c]) {
            self.clear_bit(c, b as usize);
        }
    }

    fn retire(&mut self, c: usize) {
        self.clear_mask(c);
        if self.alive[c] {
            self.lru.remove(&(self.last[c], c as u32));
        }
        self.alive[c] = false;
        self.free.insert(c as u32);
    }

    /// Keep only what a frozen (removed) node is still read for: codes, generations and liveness.
    pub fn freeze(&mut self) {
        self.masks = Vec::new();
        self.last = Vec::new();
        self.born = Vec::new();
        self.post = Vec::new();
        self.lru = BTreeSet::new();
        self.free = BTreeSet::new();
        self.count = Vec::new();
        self.touched = Vec::new();
    }

    /// Give cell c a fresh output code and bump its generation (edges reset their masks for it).
    pub fn new_code(&mut self, c: usize, rng: &mut Rng) {
        self.codes[c] = bits::rand_code(rng);
        self.gen[c] += 1;
        if !self.alive[c] {
            self.free.remove(&(c as u32));
            self.lru.insert((self.last[c], c as u32));
        }
        self.alive[c] = true;
    }

    /// Match an input given as its set-bit positions (`idx`) and as a bit set (`inp`).
    pub fn match_input(&mut self, idx: &[u16], inp: &Bits, t: u64, rng: &mut Rng) -> Match {
        // overlap counts for every cell sharing a bit with the input
        for &b in idx {
            for &c in &self.post[b as usize] {
                if self.count[c as usize] == 0 {
                    self.touched.push(c);
                }
                self.count[c as usize] += 1;
            }
        }
        let mut best: i64 = -1;
        let mut b = 0usize;
        for &c in &self.touched {
            let ov = self.count[c as usize] as i64;
            if ov > best || (ov == best && (c as usize) < b) {
                best = ov;
                b = c as usize;
            }
        }
        for &c in &self.touched {
            self.count[c as usize] = 0;
        }
        self.touched.clear();
        if best < 0 && self.n_alive() > 0 {
            best = 0; // a live cell with no overlap; below any recruit bar
        }
        let mut recruited = false;
        let mut overlap = best.max(0) as u32;
        if best < self.th_new as i64 {
            match self.free.iter().next() {
                Some(&f) => b = f as usize,
                None => {
                    let &(lt, c) = self.lru.iter().next().unwrap();
                    b = c as usize;
                    self.evictions += 1;
                    self.ev_n += 1;
                    if t.saturating_sub(lt) < self.young_age {
                        self.ev_young += 1;
                    }
                    self.retire(b);
                }
            }
            self.clear_mask(b);
            for &i in rng.sample(idx, (self.k as usize).min(idx.len())).iter() {
                self.set_bit(b, i as usize);
            }
            overlap = bits::count(&self.masks[b]);
            self.new_code(b, rng);
            self.born[b] = t;
            self.recruits += 1;
            recruited = true;
        } else if best < self.k as i64 && (self.hebb >= 1.0 || rng.f64() < self.hebb) {
            let m = &self.masks[b];
            let add: Vec<u16> = idx
                .iter()
                .copied()
                .filter(|&i| !bits::get(m, i as usize))
                .collect();
            let mut rem: Vec<u16> = Vec::new();
            for w in 0..bits::WORDS {
                let mut v = m[w] & !inp[w];
                while v != 0 {
                    rem.push((w * 64 + v.trailing_zeros() as usize) as u16);
                    v &= v - 1;
                }
            }
            if !add.is_empty() && !rem.is_empty() {
                let r = rem[rng.below(rem.len())] as usize;
                let a = add[rng.below(add.len())] as usize;
                self.clear_bit(b, r);
                self.set_bit(b, a);
            }
        }
        self.touch(b, t);
        Match {
            cell: b,
            recruited,
            overlap,
        }
    }

    /// Sleep-style merge: cells whose masks overlap by at least `thr` bits recognise the same thing.
    /// Keep the older cell and retire the younger. Returns the number retired.
    pub fn consolidate(&mut self, thr: u32) -> usize {
        let live: Vec<usize> = (0..self.cap).filter(|&c| self.alive[c]).collect();
        let mut gone = 0;
        for a in 0..live.len() {
            for bb in 0..live.len() {
                if a == bb {
                    continue;
                }
                let (i, j) = (live[a], live[bb]);
                if !self.alive[i] || !self.alive[j] {
                    continue;
                }
                if bits::and_count(&self.masks[i], &self.masks[j]) >= thr {
                    let young = if self.born[j] >= self.born[i] { j } else { i };
                    self.retire(young);
                    gone += 1;
                }
            }
        }
        gone
    }
}

/// Positions of a code rotated by r (mod D): the shifted half of a shift-bind.
pub fn rotate(code: &Code, r: usize) -> Code {
    let mut c = *code;
    for x in c.iter_mut() {
        *x = ((*x as usize + r) % D) as u16;
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recruit_then_recognise() {
        let mut rng = Rng::new(3);
        let mut p = Pool::new(8, 24, 10);
        let a = bits::rand_code(&mut rng);
        let b = bits::rand_code(&mut rng);
        let (ba, bb) = (bits::from_positions(&a), bits::from_positions(&b));
        let m1 = p.match_input(&a, &ba, 0, &mut rng);
        assert!(m1.recruited);
        let m2 = p.match_input(&b, &bb, 1, &mut rng);
        assert!(m2.recruited && m2.cell != m1.cell);
        let m3 = p.match_input(&a, &ba, 2, &mut rng);
        assert!(!m3.recruited && m3.cell == m1.cell && m3.overlap == 24);
    }

    #[test]
    fn lru_eviction_and_pressure() {
        let mut rng = Rng::new(4);
        let mut p = Pool::new(2, 24, 10);
        let codes: Vec<Code> = (0..3).map(|_| bits::rand_code(&mut rng)).collect();
        for (t, c) in codes.iter().enumerate() {
            p.match_input(c, &bits::from_positions(c), t as u64, &mut rng);
        }
        assert_eq!(p.evictions, 1);
        assert_eq!(p.recruits, 3);
        assert_eq!(p.take_pressure(), (1, 1));
        // the first code was evicted (least recently used): it must be recruited again
        let m = p.match_input(&codes[0], &bits::from_positions(&codes[0]), 3, &mut rng);
        assert!(m.recruited);
        p.grow(4);
        let m = p.match_input(&codes[1], &bits::from_positions(&codes[1]), 4, &mut rng);
        assert!(
            m.recruited && m.cell >= 2,
            "grown cells are used before evicting"
        );
    }

    /// The index must pick the same winner as a linear scan.
    #[test]
    fn index_matches_linear_scan() {
        let mut rng = Rng::new(9);
        let mut p = Pool::new(64, 24, 10);
        let codes: Vec<Code> = (0..120).map(|_| bits::rand_code(&mut rng)).collect();
        for t in 0..3000u64 {
            let c = &codes[rng.below(codes.len())];
            // a noisy copy: drop some bits
            let keep: Vec<u16> = c.iter().copied().filter(|_| rng.f64() > 0.2).collect();
            let inp = bits::from_positions(&keep);
            let mut lin: (i64, usize) = (-1, 0);
            for cell in 0..p.cap {
                if p.alive[cell] {
                    let ov = bits::and_count(&p.masks[cell], &inp) as i64;
                    if ov > lin.0 {
                        lin = (ov, cell);
                    }
                }
            }
            let m = p.match_input(&keep, &inp, t, &mut rng);
            if lin.0 >= p.th_new as i64 {
                assert_eq!(m.cell, lin.1);
            }
        }
    }
}
