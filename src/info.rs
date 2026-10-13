//! Plug-in information estimates over integer series: the screens.
//!
//! Every series is a slice of non-negative integers (a node's state + 1, so "no state" is 0).
//! Joint entropies are computed by packing the columns into one u64 key, sorting and counting runs.

/// Joint entropy in bits of one or more aligned columns.
pub fn ent(cols: &[&[i64]]) -> f64 {
    let n = cols[0].len();
    if n == 0 {
        return 0.0;
    }
    let mut keys: Vec<u64> = cols[0].iter().map(|&x| x as u64).collect();
    for c in &cols[1..] {
        let m = c.iter().copied().max().unwrap_or(0) as u64 + 1;
        for (k, &x) in keys.iter_mut().zip(c.iter()) {
            *k = *k * m + x as u64;
        }
    }
    keys.sort_unstable();
    let nf = n as f64;
    let mut h = 0.0;
    let mut i = 0;
    while i < n {
        let mut j = i + 1;
        while j < n && keys[j] == keys[i] {
            j += 1;
        }
        let p = (j - i) as f64 / nf;
        h -= p * p.log2();
        i = j;
    }
    h
}

fn plogp_runs<F: Fn(u64) -> u64>(keys: &[u64], f: F) -> (f64, usize) {
    let n = keys.len();
    let nf = n as f64;
    let (mut h, mut runs, mut i) = (0.0, 0, 0);
    while i < n {
        let k = f(keys[i]);
        let mut j = i + 1;
        while j < n && f(keys[j]) == k {
            j += 1;
        }
        let p = (j - i) as f64 / nf;
        h -= p * p.log2();
        runs += 1;
        i = j;
    }
    (h, runs)
}

/// One sort, two entropies: returns (H(prefix), H(prefix, last), distinct prefix values), where the
/// prefix is every column but the last. Sorted keys keep each prefix value's run contiguous.
pub fn ent_nested(cols: &[&[i64]]) -> (f64, f64, usize) {
    let n = cols[0].len();
    if n == 0 {
        return (0.0, 0.0, 0);
    }
    let last = cols[cols.len() - 1];
    let ml = last.iter().copied().max().unwrap_or(0) as u64 + 1;
    let mut keys: Vec<u64> = cols[0].iter().map(|&x| x as u64).collect();
    for c in &cols[1..cols.len() - 1] {
        let m = c.iter().copied().max().unwrap_or(0) as u64 + 1;
        for (k, &x) in keys.iter_mut().zip(c.iter()) {
            *k = *k * m + x as u64;
        }
    }
    for (k, &x) in keys.iter_mut().zip(last.iter()) {
        *k = *k * ml + x as u64;
    }
    keys.sort_unstable();
    let (hp, dp) = plogp_runs(&keys, |k| k / ml);
    let (ha, _) = plogp_runs(&keys, |k| k);
    (hp, ha, dp)
}

/// I(a; b).
pub fn mi(a: &[i64], b: &[i64]) -> f64 {
    ent(&[a]) + ent(&[b]) - ent(&[a, b])
}

/// I(yn; s | yp).
pub fn cmi(s: &[i64], yp: &[i64], yn: &[i64]) -> f64 {
    ent(&[s, yp]) + ent(&[yp, yn]) - ent(&[yp]) - ent(&[s, yp, yn])
}

/// Conditional MI with the parts that depend only on the target precomputed, for screening many sources.
pub struct CmiScreen<'a> {
    pub yp: &'a [i64],
    pub yn: &'a [i64],
    h_yp_yn: f64,
    h_yp: f64,
}

impl<'a> CmiScreen<'a> {
    pub fn new(yp: &'a [i64], yn: &'a [i64]) -> Self {
        CmiScreen {
            yp,
            yn,
            h_yp_yn: ent(&[yp, yn]),
            h_yp: ent(&[yp]),
        }
    }

    pub fn cmi(&self, s: &[i64]) -> f64 {
        let (h_s_yp, h_s_yp_yn, _) = ent_nested(&[s, self.yp, self.yn]);
        h_s_yp + self.h_yp_yn - self.h_yp - h_s_yp_yn
    }

    /// cmi(s) minus cmi of s circularly shifted by `sh` (the shift null that cancels estimator bias).
    pub fn shift_screen(&self, s: &[i64], sh: usize) -> f64 {
        self.cmi(s) - self.cmi(&roll(s, sh))
    }
}

/// numpy.roll: element i moves to (i + sh) mod n.
pub fn roll(s: &[i64], sh: usize) -> Vec<i64> {
    let n = s.len();
    if n == 0 {
        return Vec::new();
    }
    let sh = sh % n;
    let mut out = Vec::with_capacity(n);
    out.extend_from_slice(&s[n - sh..]);
    out.extend_from_slice(&s[..n - sh]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_values() {
        let a: Vec<i64> = (0..1000).map(|i| i % 4).collect();
        assert!((ent(&[&a]) - 2.0).abs() < 1e-9);
        assert!((mi(&a, &a) - 2.0).abs() < 1e-9);
        let b: Vec<i64> = (0..1000).map(|i| (i / 4) % 2).collect();
        assert!(mi(&a, &b).abs() < 1e-9);
        // yn = s xor yp: s tells nothing alone, everything given yp
        let s: Vec<i64> = (0..4000).map(|i| (i * 7 / 3) % 2).collect();
        let yp: Vec<i64> = (0..4000).map(|i| (i * 5 / 11) % 2).collect();
        let yn: Vec<i64> = s.iter().zip(&yp).map(|(x, y)| x ^ y).collect();
        assert!(cmi(&s, &yp, &yn) > 0.9);
        let sc = CmiScreen::new(&yp, &yn);
        assert!((sc.cmi(&s) - cmi(&s, &yp, &yn)).abs() < 1e-9);
        let (h1, h2, d) = ent_nested(&[&s, &yp]);
        assert!((h1 - ent(&[&s])).abs() < 1e-9 && (h2 - ent(&[&s, &yp])).abs() < 1e-9 && d == 2);
        assert_eq!(roll(&[1, 2, 3, 4], 1), vec![4, 1, 2, 3]);
    }
}
