//! Matcher pools: the Associate/Expand cell population.
//!
//! k-WTA with k = 1. A cell's mask has K bits; it matches an input by the overlap of its mask with the
//! input's bits. If the best overlap is below `th_new`, a cell is recruited (a free one, else the least
//! recently used) with a mask of K bits sampled from the input. Otherwise, on a partial match, the winner
//! swaps one mask bit toward the input (Hebbian), with probability `hebb`.
use crate::bits::{self, Bits, Code, D, ZERO};
use crate::rng::Rng;

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
        }
    }

    pub fn n_alive(&self) -> usize {
        self.alive.iter().filter(|&&a| a).count()
    }

    /// Give cell c a fresh output code and bump its generation (edges reset their masks for it).
    pub fn new_code(&mut self, c: usize, rng: &mut Rng) {
        self.codes[c] = bits::rand_code(rng);
        self.gen[c] += 1;
        self.alive[c] = true;
    }

    /// Match an input given as its set-bit positions (`idx`) and as a bit set (`inp`).
    pub fn match_input(&mut self, idx: &[u16], inp: &Bits, t: u64, rng: &mut Rng) -> Match {
        let mut best: i64 = -1;
        let mut b = 0usize;
        for c in 0..self.cap {
            if !self.alive[c] {
                continue;
            }
            let ov = bits::and_count(&self.masks[c], inp) as i64;
            if ov > best {
                best = ov;
                b = c;
            }
        }
        let mut recruited = false;
        let mut overlap = best.max(0) as u32;
        if best < self.th_new as i64 {
            match self.alive.iter().position(|&a| !a) {
                Some(f) => b = f,
                None => {
                    b = (0..self.cap).min_by_key(|&c| self.last[c]).unwrap();
                    self.evictions += 1;
                }
            }
            let m = &mut self.masks[b];
            *m = ZERO;
            for &i in rng.sample(idx, (self.k as usize).min(idx.len())).iter() {
                bits::set(m, i as usize);
            }
            overlap = bits::count(m);
            self.new_code(b, rng);
            self.born[b] = t;
            self.recruits += 1;
            recruited = true;
        } else if best < self.k as i64 && (self.hebb >= 1.0 || rng.f64() < self.hebb) {
            let m = &mut self.masks[b];
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
                bits::clear(m, rem[rng.below(rem.len())] as usize);
                bits::set(m, add[rng.below(add.len())] as usize);
            }
        }
        self.last[b] = t;
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
                    self.alive[young] = false;
                    self.masks[young] = ZERO;
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
    fn lru_eviction() {
        let mut rng = Rng::new(4);
        let mut p = Pool::new(2, 24, 10);
        let codes: Vec<Code> = (0..3).map(|_| bits::rand_code(&mut rng)).collect();
        for (t, c) in codes.iter().enumerate() {
            p.match_input(c, &bits::from_positions(c), t as u64, &mut rng);
        }
        assert_eq!(p.evictions, 1);
        assert_eq!(p.recruits, 3);
    }
}
