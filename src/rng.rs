//! Seeded xoshiro256** generator. Every random choice in the engine draws from one of these,
//! so a run is reproducible from its seed.

#[derive(Clone, Debug)]
pub struct Rng {
    s: [u64; 4],
}

fn splitmix(x: &mut u64) -> u64 {
    *x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut x = seed;
        Rng {
            s: [
                splitmix(&mut x),
                splitmix(&mut x),
                splitmix(&mut x),
                splitmix(&mut x),
            ],
        }
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let r = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        r
    }

    /// Uniform integer in [0, n).
    #[inline]
    pub fn below(&mut self, n: usize) -> usize {
        debug_assert!(n > 0);
        ((self.next_u64() as u128 * n as u128) >> 64) as usize
    }

    /// Uniform integer in [lo, hi).
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + self.below(hi - lo)
    }

    /// Uniform float in [0, 1).
    #[inline]
    pub fn f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// k distinct values from [0, n), in random order (partial Fisher-Yates over a scratch index).
    pub fn choose_distinct(&mut self, n: usize, k: usize) -> Vec<usize> {
        let k = k.min(n);
        if k * 4 < n {
            let mut out: Vec<usize> = Vec::with_capacity(k);
            while out.len() < k {
                let v = self.below(n);
                if !out.contains(&v) {
                    out.push(v);
                }
            }
            out
        } else {
            let mut idx: Vec<usize> = (0..n).collect();
            for i in 0..k {
                let j = i + self.below(n - i);
                idx.swap(i, j);
            }
            idx.truncate(k);
            idx
        }
    }

    /// k distinct elements of `items`, in random order.
    pub fn sample<T: Copy>(&mut self, items: &[T], k: usize) -> Vec<T> {
        self.choose_distinct(items.len(), k)
            .into_iter()
            .map(|i| items[i])
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_in_range() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        for _ in 0..1000 {
            let x = a.below(13);
            assert_eq!(x, b.below(13));
            assert!(x < 13);
        }
        let d = a.choose_distinct(2048, 32);
        let mut s = d.clone();
        s.sort();
        s.dedup();
        assert_eq!(s.len(), 32);
    }
}

/// A fixed hash of two numbers (for codes computed on the fly instead of stored).
pub fn hash2(a: u64, b: u64) -> u64 {
    let mut x = a.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ b.wrapping_add(0x632B_E59B_D9B4_E019);
    splitmix(&mut x)
}
