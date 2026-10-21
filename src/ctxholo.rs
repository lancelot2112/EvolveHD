//! A sparse, contextified holographic trace (experiment 31e), behind the bitwise store's `--hp-ctxholo`.
//!
//! Context. A context code of `k` slots, each an element offset and a 4-phase offset. It drifts: every word a
//! few slots (`drift`) are redrawn, and at each learned boundary more (`bdrift`). Two moments' contexts share
//! the slots not redrawn in between, so their overlap falls off with the time between them.
//!
//! Contextified key. A word as a key in the current context is `k` active elements, slot i at element
//! (hash(word, i) + offset_i) mod d, with phase (hash phase(word, i) + phase_i) mod 4. So the same word in another
//! story (another context) lands on other elements, and a word's bindings from long ago no longer line up with
//! its key now: recency comes from the drift, not from fading.
//!
//! Binding key -> value writes, at the key's active elements only, the phasor of (key phase + value phase) mod 4
//! (the value's phase at element e is fixed, hash(value, e)): +-m on the real or imaginary part, clipped to
//! `levels` states per part. Reading a key rotates its elements back by its phases and scores each candidate
//! value by correlation there (a choice of re, im, -re, -im per element: add and subtract only). The baseline
//! is the same correlation over a background sum of the values written at each element. Integer throughout.
pub struct CtxHolo {
    pub d: usize,
    pub k: usize,
    lo: i16,
    hi: i16,
    tr: [Vec<i16>; 4],
    bg: [Vec<i32>; 4],
    ctx_off: Vec<u32>,
    ctx_ph: Vec<u8>,
    drift: usize,
    bdrift: usize,
    tick: u64,
    /// Each value's phase at every element (computed once from the hash; nw x d phases).
    vtab: Vec<u8>,
}

/// An unbound read: the key's active elements and the trace there rotated back by the key's phases.
pub struct Z {
    pub idx: Vec<u32>,
    pub re: Vec<i32>,
    pub im: Vec<i32>,
    pub dir: usize,
}

fn hash(a: u64, b: u64) -> u64 {
    crate::rng::hash2(a, b)
}

#[inline]
fn pick(re: i32, im: i32, p: u8) -> i32 {
    match p & 3 {
        0 => re,
        1 => im,
        2 => -re,
        _ => -im,
    }
}

impl CtxHolo {
    /// `levels` per part: an odd count is symmetric (3: -1..1, 5: -2..2), an even one runs -L/2..L/2-1.
    /// `ctx` false: no context (keys are the same in every story, for a slow memory across stories).
    pub fn new(d: usize, k: usize, levels: usize, drift: usize, bdrift: usize, nw: usize, ctx: bool) -> Self {
        let vtab = (0..nw as u64)
            .flat_map(|v| (0..d as u64).map(move |e| (hash(v + 0x7777, e) & 3) as u8))
            .collect();
        CtxHolo {
            vtab,
            d,
            k,
            lo: -((levels / 2) as i16),
            hi: if levels % 2 == 1 { (levels / 2) as i16 } else { (levels / 2) as i16 - 1 },
            tr: [vec![0; d], vec![0; d], vec![0; d], vec![0; d]],
            bg: [vec![0; d], vec![0; d], vec![0; d], vec![0; d]],
            ctx_off: (0..k).map(|i| if ctx { (hash(91, i as u64) % d as u64) as u32 } else { 0 }).collect(),
            ctx_ph: (0..k).map(|i| if ctx { (hash(92, i as u64) & 3) as u8 } else { 0 }).collect(),
            drift: if ctx { drift } else { 0 },
            bdrift: if ctx { bdrift } else { 0 },
            tick: 0,
        }
    }

    /// Move the context on by one word (more at a boundary).
    pub fn step(&mut self, boundary: bool) {
        self.tick += 1;
        let n = self.drift + if boundary { self.bdrift } else { 0 };
        for j in 0..n {
            let h = hash(self.tick * 7919 + j as u64, 93);
            let i = (h % self.k as u64) as usize;
            self.ctx_off[i] = (hash(h, 94) % self.d as u64) as u32;
            self.ctx_ph[i] = (hash(h, 95) & 3) as u8;
        }
    }

    /// The key's active elements and phases in the current context.
    fn key(&self, w: u32) -> (Vec<u32>, Vec<u8>) {
        let mut idx = Vec::with_capacity(self.k);
        let mut ph = Vec::with_capacity(self.k);
        for i in 0..self.k {
            let h = hash(w as u64 + 1, i as u64);
            idx.push(((h % self.d as u64) as u32 + self.ctx_off[i]) % self.d as u32);
            ph.push(((h >> 40) as u8 + self.ctx_ph[i]) & 3);
        }
        (idx, ph)
    }

    #[inline]
    fn vph(&self, val: u32, e: u32) -> u8 {
        self.vtab[val as usize * self.d + e as usize]
    }

    /// Bind key -> value (magnitude m) in direction dir, in the current context.
    pub fn write(&mut self, dir: usize, key: u32, val: u32, m: i16) {
        let (idx, ph) = self.key(key);
        let (lo, hi) = (self.lo, self.hi);
        for (&e, &kp) in idx.iter().zip(&ph) {
            let vp = self.vph(val, e);
            let e = e as usize;
            let (re, im) = (2 * dir, 2 * dir + 1);
            match (kp + vp) & 3 {
                0 => self.tr[re][e] = (self.tr[re][e] + m).clamp(lo, hi),
                1 => self.tr[im][e] = (self.tr[im][e] + m).clamp(lo, hi),
                2 => self.tr[re][e] = (self.tr[re][e] - m).clamp(lo, hi),
                _ => self.tr[im][e] = (self.tr[im][e] - m).clamp(lo, hi),
            }
            match vp {
                0 => self.bg[re][e] += 1,
                1 => self.bg[im][e] += 1,
                2 => self.bg[re][e] -= 1,
                _ => self.bg[im][e] -= 1,
            }
        }
    }

    /// Read a key in the current context.
    pub fn read(&self, dir: usize, key: u32) -> Z {
        let (idx, ph) = self.key(key);
        let (tre, tim) = (&self.tr[2 * dir], &self.tr[2 * dir + 1]);
        let mut re = Vec::with_capacity(self.k);
        let mut im = Vec::with_capacity(self.k);
        for (&e, &kp) in idx.iter().zip(&ph) {
            let (a, b) = (tre[e as usize] as i32, tim[e as usize] as i32);
            let (x, y) = match kp & 3 {
                0 => (a, b),
                1 => (b, -a),
                2 => (-a, -b),
                _ => (-b, a),
            };
            re.push(x);
            im.push(y);
        }
        Z { idx, re, im, dir }
    }

    /// (score, baseline) of a value against a read.
    pub fn score(&self, z: &Z, val: u32) -> (i64, i64) {
        let (bre, bim) = (&self.bg[2 * z.dir], &self.bg[2 * z.dir + 1]);
        let mut s = 0i64;
        let mut b = 0i64;
        for (i, &e) in z.idx.iter().enumerate() {
            let p = self.vph(val, e);
            s += pick(z.re[i], z.im[i], p) as i64;
            b += pick(bre[e as usize], bim[e as usize], p) as i64;
        }
        (s, b)
    }
}
