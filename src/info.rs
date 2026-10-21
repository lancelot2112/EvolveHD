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

/// Bits per tick that context (s, yp) adds over yp alone for predicting yn, on held-out data:
/// counts from the first half, Witten-Bell interpolation, log-loss gain on the second half.
/// Unlike the shift-null screen, it stays honest for contexts with thousands of states.
pub fn heldout_gain(s: &[i64], yp: &[i64], yn: &[i64]) -> f64 {
    use std::collections::HashMap;
    let n = s.len();
    let h = n / 2;
    if h == 0 || n == h {
        return 0.0;
    }
    let v = yn.iter().copied().max().unwrap_or(0) as usize + 1;
    let mut p0 = vec![1.0f64; v];
    for &y in &yn[..h] {
        p0[y as usize] += 1.0;
    }
    let z: f64 = p0.iter().sum();
    p0.iter_mut().for_each(|x| *x /= z);
    struct C {
        n: u32,
        d: HashMap<i64, u32>,
    }
    let mut cb: HashMap<i64, C> = HashMap::new();
    let mut cs: HashMap<(i64, i64), C> = HashMap::new();
    for i in 0..h {
        let e = cb.entry(yp[i]).or_insert_with(|| C {
            n: 0,
            d: HashMap::new(),
        });
        e.n += 1;
        *e.d.entry(yn[i]).or_insert(0) += 1;
        let e = cs.entry((s[i], yp[i])).or_insert_with(|| C {
            n: 0,
            d: HashMap::new(),
        });
        e.n += 1;
        *e.d.entry(yn[i]).or_insert(0) += 1;
    }
    let mut tot = 0.0;
    for i in h..n {
        let y = yn[i];
        let mut pb = if (y as usize) < v {
            p0[y as usize]
        } else {
            1e-9
        };
        if let Some(c) = cb.get(&yp[i]) {
            let t = c.d.len() as f64;
            pb = (*c.d.get(&y).unwrap_or(&0) as f64 + t * pb) / (c.n as f64 + t);
        }
        let mut ps = pb;
        if let Some(c) = cs.get(&(s[i], yp[i])) {
            let t = c.d.len() as f64;
            ps = (*c.d.get(&y).unwrap_or(&0) as f64 + t * pb) / (c.n as f64 + t);
        }
        tot += ps.log2() - pb.log2();
    }
    tot / (n - h) as f64
}

/// Like [`heldout_gain`], but scores a mixture (1 - l) p(yn | yp) + l p(yn | s, yp), with l chosen from a
/// grid on the first fifth of the second half and the gain reported on the rest. A weight of 0 is among
/// the choices, so a context with no information scores about zero instead of being charged for sparsity.
pub fn heldout_gain_mix(s: &[i64], yp: &[i64], yn: &[i64]) -> f64 {
    MixBase::new(yp, yn).gain(s)
}

/// A fast, deterministic hasher (FxHash) for the screens' count tables: the tables are never iterated, so
/// the hasher changes only speed.
#[derive(Default, Clone, Copy)]
pub struct Fx(u64);
impl std::hash::Hasher for Fx {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u64(b as u64);
        }
    }
    fn write_u64(&mut self, x: u64) {
        self.0 = (self.0.rotate_left(5) ^ x).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    fn write_i64(&mut self, x: i64) {
        self.write_u64(x as u64);
    }
}
pub type FxBuild = std::hash::BuildHasherDefault<Fx>;
type FxMap<K, V> = std::collections::HashMap<K, V, FxBuild>;

#[derive(Default)]
struct Ctx {
    n: u32,
    d: FxMap<i64, u32>,
}
fn wb_of(c: Option<&Ctx>, y: i64, lower: f64) -> f64 {
    match c {
        Some(c) => {
            let t = c.d.len() as f64;
            (*c.d.get(&y).unwrap_or(&0) as f64 + t * lower) / (c.n as f64 + t)
        }
        None => lower,
    }
}

/// The baseline half of `heldout_gain_mix`, which depends only on the context and the target: built once and
/// shared by every candidate screened against them (the arithmetic is the same as computing it each time).
pub struct MixBase<'a> {
    yp: &'a [i64],
    yn: &'a [i64],
    n: usize,
    h: usize,
    /// The baseline probability of the target at each scored tick i in h..n.
    pb: Vec<f64>,
}

impl<'a> MixBase<'a> {
    pub fn new(yp: &'a [i64], yn: &'a [i64]) -> Self {
        let n = yp.len().min(yn.len());
        let h = n / 2;
        if h == 0 || n == h {
            return MixBase {
                yp,
                yn,
                n,
                h,
                pb: vec![],
            };
        }
        let v = yn.iter().copied().max().unwrap_or(0) as usize + 1;
        let mut p0 = vec![1.0f64; v];
        for &y in &yn[..h] {
            p0[y as usize] += 1.0;
        }
        let z: f64 = p0.iter().sum();
        p0.iter_mut().for_each(|x| *x /= z);
        let mut cb: FxMap<i64, Ctx> = FxMap::default();
        for i in 0..h {
            let e = cb.entry(yp[i]).or_default();
            e.n += 1;
            *e.d.entry(yn[i]).or_insert(0) += 1;
        }
        let pb = (h..n)
            .map(|i| {
                let y = yn[i];
                let base = if (y as usize) < v {
                    p0[y as usize]
                } else {
                    1e-9
                };
                wb_of(cb.get(&yp[i]), y, base)
            })
            .collect();
        MixBase { yp, yn, n, h, pb }
    }

    /// How much candidate context s adds beyond the baseline: the held-out gain of the best mixture weight.
    pub fn gain(&self, s: &[i64]) -> f64 {
        let (n, h) = (self.n.min(s.len()), self.h);
        if h == 0 || n == h || self.pb.is_empty() {
            return 0.0;
        }
        let (yp, yn) = (self.yp, self.yn);
        let mut cs: FxMap<(i64, i64), Ctx> = FxMap::default();
        for i in 0..h {
            let e = cs.entry((s[i], yp[i])).or_default();
            e.n += 1;
            *e.d.entry(yn[i]).or_insert(0) += 1;
        }
        const LAMS: [f64; 9] = [0.0, 0.02, 0.05, 0.1, 0.2, 0.35, 0.5, 0.7, 0.9];
        let cal = h + (n - h) / 5;
        let mut acc = [[0.0f64; 9]; 2];
        for i in h..n {
            let y = yn[i];
            let pb = self.pb[i - h];
            let ps = wb_of(cs.get(&(s[i], yp[i])), y, pb);
            let k = if i < cal { 0 } else { 1 };
            for (j, &l) in LAMS.iter().enumerate() {
                acc[k][j] += ((1.0 - l) * pb + l * ps).log2() - pb.log2();
            }
        }
        let best = (0..LAMS.len())
            .max_by(|&a, &b| acc[0][a].partial_cmp(&acc[0][b]).unwrap())
            .unwrap();
        if n > cal {
            acc[1][best] / (n - cal) as f64
        } else {
            0.0
        }
    }
}

/// Map f over items on all cores (EVOLVEHD_THREADS to override), results in item order.
pub fn par_map<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let nt = crate::meaning::threads().min(items.len().max(1));
    if nt <= 1 || items.len() < 2 {
        return items.iter().map(&f).collect();
    }
    let chunk = items.len().div_ceil(nt);
    let f = &f;
    std::thread::scope(|sc| {
        let hs: Vec<_> = items
            .chunks(chunk)
            .map(|c| sc.spawn(move || c.iter().map(f).collect::<Vec<R>>()))
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    })
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
        // a context that fixes the next value gains about a bit over yp alone; noise gains nothing
        let ctx: Vec<i64> = (0..8000).map(|i| (i * 13 / 7) % 2).collect();
        let yp0: Vec<i64> = vec![0; 8000];
        let yn0 = ctx.clone();
        assert!(heldout_gain(&ctx, &yp0, &yn0) > 0.9);
        let noise: Vec<i64> = (0..8000).map(|i| (i * 7919 % 31) as i64).collect();
        assert!(heldout_gain(&noise, &yp0, &yn0) < 0.05);
        assert!(heldout_gain_mix(&ctx, &yp0, &yn0) > 0.8);
        assert!(heldout_gain_mix(&noise, &yp0, &yn0).abs() < 0.05);
    }
}
