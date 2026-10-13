//! Fixed-width bit sets (2048 bits = 32 u64 words) and sparse codes (32 positions).
use crate::rng::Rng;

pub const D: usize = 2048;
pub const WORDS: usize = D / 64;
pub const CK: usize = 32;

pub type Bits = [u64; WORDS];
pub type Code = [u16; CK];

pub const ZERO: Bits = [0; WORDS];

#[inline]
pub fn get(b: &Bits, i: usize) -> bool {
    (b[i >> 6] >> (i & 63)) & 1 == 1
}

#[inline]
pub fn set(b: &mut Bits, i: usize) {
    b[i >> 6] |= 1 << (i & 63);
}

#[inline]
pub fn clear(b: &mut Bits, i: usize) {
    b[i >> 6] &= !(1 << (i & 63));
}

#[inline]
pub fn and_count(a: &Bits, b: &Bits) -> u32 {
    let mut n = 0;
    for i in 0..WORDS {
        n += (a[i] & b[i]).count_ones();
    }
    n
}

pub fn count(a: &Bits) -> u32 {
    a.iter().map(|w| w.count_ones()).sum()
}

pub fn from_positions(p: &[u16]) -> Bits {
    let mut b = ZERO;
    for &i in p {
        set(&mut b, i as usize);
    }
    b
}

pub fn positions(b: &Bits) -> Vec<u16> {
    let mut out = Vec::new();
    for (w, &x) in b.iter().enumerate() {
        let mut v = x;
        while v != 0 {
            let t = v.trailing_zeros() as usize;
            out.push((w * 64 + t) as u16);
            v &= v - 1;
        }
    }
    out
}

/// A random code: CK distinct positions out of D.
pub fn rand_code(rng: &mut Rng) -> Code {
    let v = rng.choose_distinct(D, CK);
    let mut c = [0u16; CK];
    for (i, x) in v.into_iter().enumerate() {
        c[i] = x as u16;
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut r = Rng::new(1);
        let c = rand_code(&mut r);
        let b = from_positions(&c);
        assert_eq!(count(&b), CK as u32);
        let mut p = positions(&b);
        let mut q: Vec<u16> = c.to_vec();
        p.sort();
        q.sort();
        assert_eq!(p, q);
        assert_eq!(and_count(&b, &b), 32);
    }
}
