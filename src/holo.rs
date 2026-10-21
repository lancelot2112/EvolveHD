//! A quantized complex holographic trace (experiment 31d), the memory behind the bitwise store's `--hp-holo`.
//!
//! Every word has two fixed 4-phase codes, one as a key and one as a value (each of `d` elements one of 1, i,
//! -1, -i, i.e. a phase index 0..3, from a hash; 2 bits per element; two because binding by adding phases
//! commutes), and every role phase a 4-phase code too. A binding key -> value in role r
//! adds the phasor of phase (k + v + r) mod 4 into the trace: +-1 on the real or the imaginary part of each
//! element, clipped to `levels` states per part (saturation is the forgetting: new writes push old ones out).
//! One trace per direction. Reading a key rotates the trace back by the key's phase, element by element (a swap
//! and sign change: add and subtract only); a value's score is the correlation of the result with its code in
//! the role asked for, again a choice among re, im, -re, -im per element. The baseline (how much a value is
//! written at all, in that role) is the same correlation against a background sum of everything written.
//! All integer.
pub struct Holo {
    pub d: usize,
    lo: i16,
    hi: i16,
    /// re, im per direction
    tr: [Vec<i16>; 4],
    /// background sums of written values (re, im per direction), not clipped
    bg: [Vec<i32>; 4],
    codes: Vec<Vec<u8>>,
    /// Key codes, separate from value codes: binding adds phases, which commutes, so with one code per word
    /// key -> value and value -> key would be the same trace entry (and the directions would collapse).
    kcodes: Vec<Vec<u8>>,
    roles: Vec<Vec<u8>>,
    /// Per (value, role): the value's phasor in that role as +-1/0 real and imaginary parts, so a score is two
    /// integer dot products (vectorizes).
    cv: Vec<(Vec<i8>, Vec<i8>)>,
    nroles: usize,
}

fn hash(a: u64, b: u64) -> u64 {
    crate::rng::hash2(a, b)
}


impl Holo {
    pub fn new(d: usize, levels: usize, nw: usize, nroles: usize) -> Self {
        let code = |salt: u64, i: u64| -> Vec<u8> {
            (0..d).map(|e| (hash(salt * 1_000_003 + i, e as u64) & 3) as u8).collect()
        };
        let codes: Vec<Vec<u8>> = (0..nw as u64).map(|w| code(1, w)).collect();
        let kcodes: Vec<Vec<u8>> = (0..nw as u64).map(|w| code(3, w)).collect();
        let roles: Vec<Vec<u8>> = (0..nroles.max(1) as u64).map(|r| if r == 0 { vec![0; d] } else { code(2, r) }).collect();
        let mut cv = vec![];
        for c in &codes {
            for ro in &roles {
                let re: Vec<i8> = (0..d).map(|e| [1i8, 0, -1, 0][((c[e] + ro[e]) & 3) as usize]).collect();
                let im: Vec<i8> = (0..d).map(|e| [0i8, 1, 0, -1][((c[e] + ro[e]) & 3) as usize]).collect();
                cv.push((re, im));
            }
        }
        Holo {
            cv,
            nroles: nroles.max(1),
            d,
            lo: -((levels / 2) as i16),
            hi: (levels / 2) as i16 - 1,
            tr: [vec![0; d], vec![0; d], vec![0; d], vec![0; d]],
            bg: [vec![0; d], vec![0; d], vec![0; d], vec![0; d]],
            codes,
            kcodes,
            roles,
        }
    }

    /// Bind key -> value (in role r) into direction dir's trace, once.
    pub fn write(&mut self, dir: usize, key: u32, val: u32, r: usize) {
        self.write_m(dir, key, val, r, 1);
    }

    /// The value best recalled for a key in role r, among the candidates, with its score.
    pub fn best(&self, dir: usize, key: u32, r: usize, cands: &[u32]) -> Option<(u32, i64)> {
        let z = self.unbind(dir, key);
        cands.iter().map(|&w| (w, self.dot(&z.0, &z.1, w, r))).max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
    }

    /// Bind key -> value with magnitude m (m < 0 takes a binding out: the superseded fact cancels); only
    /// positive writes count in the background.
    pub fn write_m(&mut self, dir: usize, key: u32, val: u32, r: usize, m: i16) {
        let (k, v, ro) = (&self.kcodes[key as usize], &self.codes[val as usize], &self.roles[r]);
        let (lo, hi) = (self.lo, self.hi);
        let [t0, t1, t2, t3] = &mut self.tr;
        let (re, im) = if dir == 0 { (t0, t1) } else { (t2, t3) };
        let [b0, b1, b2, b3] = &mut self.bg;
        let (bre, bim) = if dir == 0 { (b0, b1) } else { (b2, b3) };
        for e in 0..self.d {
            let p = (k[e] + v[e] + ro[e]) & 3;
            match p {
                0 => re[e] = (re[e] + m).clamp(lo, hi),
                1 => im[e] = (im[e] + m).clamp(lo, hi),
                2 => re[e] = (re[e] - m).clamp(lo, hi),
                _ => im[e] = (im[e] - m).clamp(lo, hi),
            }
            if m < 0 {
                continue;
            }
            match (v[e] + ro[e]) & 3 {
                0 => bre[e] += 1,
                1 => bim[e] += 1,
                2 => bre[e] -= 1,
                _ => bim[e] -= 1,
            }
        }
    }

    /// Halve every element (a shift toward zero): fading, so recent bindings outweigh old ones.
    pub fn halve(&mut self) {
        for t in self.tr.iter_mut() {
            t.iter_mut().for_each(|x| *x /= 2);
        }
    }

    /// The trace rotated back by a key: (re, im), each element times i^-k.
    pub fn unbind(&self, dir: usize, key: u32) -> (Vec<i32>, Vec<i32>) {
        let k = &self.kcodes[key as usize];
        let (re, im) = (&self.tr[2 * dir], &self.tr[2 * dir + 1]);
        let mut zr = vec![0i32; self.d];
        let mut zi = vec![0i32; self.d];
        for e in 0..self.d {
            let (a, b) = (re[e] as i32, im[e] as i32);
            // times i^-k: k = 0: (a, b); 1: (b, -a); 2: (-a, -b); 3: (-b, a)
            let (x, y) = match k[e] & 3 {
                0 => (a, b),
                1 => (b, -a),
                2 => (-a, -b),
                _ => (-b, a),
            };
            zr[e] = x;
            zi[e] = y;
        }
        (zr, zi)
    }

    fn dot(&self, zr: &[i32], zi: &[i32], val: u32, r: usize) -> i64 {
        let (cr, ci) = &self.cv[val as usize * self.nroles + r];
        let a: i64 = zr.iter().zip(cr).map(|(&x, &c)| (x * c as i32) as i64).sum();
        let b: i64 = zi.iter().zip(ci).map(|(&x, &c)| (x * c as i32) as i64).sum();
        a + b
    }

    /// A value's score against an unbound trace, in role r (or summed over roles 0..n when r is None).
    pub fn score(&self, z: &(Vec<i32>, Vec<i32>), val: u32, r: Option<usize>, _n: usize) -> i64 {
        match r {
            Some(r) => self.dot(&z.0, &z.1, val, r),
            None => (0..self.nroles).map(|r| self.dot(&z.0, &z.1, val, r)).sum(),
        }
    }

    /// How much a value has been written at all in direction dir (role r, or all), from the background sums.
    pub fn base(&self, dir: usize, val: u32, r: Option<usize>, _n: usize) -> i64 {
        let (zr, zi) = (&self.bg[2 * dir], &self.bg[2 * dir + 1]);
        match r {
            Some(r) => self.dot(zr, zi, val, r),
            None => (0..self.nroles).map(|r| self.dot(zr, zi, val, r)).sum(),
        }
    }
}
