//! Acts and senses beside the byte brain's level 0 (experiment 62): the stride act (value operators: the byte k
//! back, and the byte k back plus its last change) and the regime sense (which kind of material the recent bytes
//! are, by learned clusters of their byte counts).

/// The stride act. For each stride k it keeps two predictors of the next byte: the byte k back ("repeat": records,
/// pixel rows, fixed fields) and the byte k back plus the change between it and the byte 2k back ("step":
/// counters, ramps, the low byte of an increasing integer). Each predictor has a recent score per lane (the
/// byte's offset modulo 8, from the file's start when known), so different bytes of a record can follow
/// different predictors. The act offers the byte of the best-scoring predictor in this lane; it is trusted by
/// how often offers with that score level and kind were right before (counted, prequential).
pub struct StrideAct {
    pub strides: Vec<usize>,
    /// --stride-wide: number predictors too, (stride k, width w, little endian?): the w-byte number k back plus
    /// its change from the number 2k back, as a whole number with carries; the byte offered is the current byte's
    /// place in that number (the byte's lane modulo w gives the place).
    nums: Vec<(usize, usize, bool)>,
    score: Vec<[u16; 8]>,
    cal: Vec<(u32, u32)>,
    pub mw: Vec<[f64; 3]>,
    pub stats: [u64; 3],
    last: Option<(usize, u8, usize)>,
    lane: usize,
}

pub const STRIDES: [usize; 21] = [1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 128, 192, 256, 384, 512, 600, 768, 1024];

impl StrideAct {
    pub fn new(contexts: usize, wide: bool) -> Self {
        let s = STRIDES.to_vec();
        let mut nums = vec![];
        if wide {
            for w in [2usize, 4, 8] {
                for &k in s.iter().filter(|&&k| k >= w && k % w == 0) {
                    nums.push((k, w, true));
                    nums.push((k, w, false));
                }
            }
        }
        StrideAct { score: vec![[0; 8]; 2 * s.len() + nums.len()], cal: vec![(0, 0); contexts], mw: vec![[1.0 / 3.0; 3]; contexts], stats: [0; 3], last: None, strides: s, nums, lane: 0 }
    }
    /// Trust contexts per regime: no offer, then 8 score levels for each of repeat, step and number.
    pub const CTX: usize = 25;
    fn kind(&self, p: usize) -> usize {
        if p < 2 * self.strides.len() { p % 2 } else { 2 }
    }
    fn pred(&self, p: usize, data: &[u8], t: usize) -> Option<u8> {
        if p >= 2 * self.strides.len() {
            let (k, w, le) = self.nums[p - 2 * self.strides.len()];
            let j = self.lane % w; // the byte's place in its number (from the start of the file)
            let s0 = t.checked_sub(j)?;
            if s0 < 2 * k {
                return None;
            }
            let num = |a: usize| -> u64 {
                let b = &data[a..a + w];
                if le { b.iter().rev().fold(0u64, |v, &x| v << 8 | x as u64) } else { b.iter().fold(0u64, |v, &x| v << 8 | x as u64) }
            };
            let (v1, v2) = (num(s0 - k), num(s0 - 2 * k));
            let v = v1.wrapping_add(v1.wrapping_sub(v2));
            let place = if le { j } else { w - 1 - j };
            return Some((v >> (8 * place)) as u8);
        }
        let k = self.strides[p / 2];
        if p % 2 == 0 {
            (t >= k).then(|| data[t - k])
        } else {
            (t >= 2 * k).then(|| data[t - k].wrapping_add(data[t - k].wrapping_sub(data[t - 2 * k])))
        }
    }
    /// The act's offer for byte t (from bytes before t) and its trust context (0 = no offer).
    pub fn offer(&mut self, data: &[u8], t: usize, lane: usize) -> (Option<u8>, usize) {
        self.lane = lane;
        let mut best: Option<(u16, usize)> = None;
        for p in 0..self.score.len() {
            let s = self.score[p][lane];
            if self.pred(p, data, t).is_some() && best.map_or(true, |b| s > b.0) {
                best = Some((s, p));
            }
        }
        match best {
            Some((s, p)) if s >= 32 => {
                let o = self.pred(p, data, t).unwrap();
                let c = 1 + self.kind(p) * 8 + (s as usize >> 5).min(7);
                self.last = Some((p, o, c));
                (Some(o), c)
            }
            _ => {
                self.last = None;
                (None, 0)
            }
        }
    }
    /// The trust in an offer from context c: how often such offers were right.
    pub fn trust(&self, c: usize) -> f64 {
        let (r, n) = self.cal[c];
        (r as f64 + 0.5) / (n as f64 + 1.0)
    }
    /// After byte t (value y): credit the offer, and score every predictor in this lane.
    pub fn learn(&mut self, data: &[u8], t: usize, lane: usize, y: u8, ctx: usize) {
        self.lane = lane;
        if let Some((_, o, _)) = self.last {
            self.stats[0] += 1;
            let e = &mut self.cal[ctx];
            e.1 += 1;
            if o == y {
                e.0 += 1;
                self.stats[1] += 1;
            }
            if e.1 > 4096 {
                e.0 >>= 1;
                e.1 >>= 1;
            }
        }
        for p in 0..self.score.len() {
            if let Some(o) = self.pred(p, data, t) {
                let s = &mut self.score[p][lane];
                *s = *s - (*s >> 3) + if o == y { 32 } else { 0 };
            }
        }
    }
}

/// The regime sense: the byte counts of the last W bytes, matched every 16 bytes to learned centroids (cosine
/// similarity). A window less than 0.6 similar to every centroid starts a new one (up to K); otherwise the best
/// centroid moves 1/50 of the way toward it. (The 0.6 is a setting: --regime-thr.) The regime is the best centroid's index.
pub struct Regime {
    w: usize,
    k: usize,
    counts: Vec<u32>,
    cents: Vec<Vec<f32>>,
    pub id: i32,
    pub sizes: Vec<u64>,
    thr: f32,
}

impl Regime {
    pub fn new(w: usize, k: usize, thr: f32) -> Self {
        Regime { w, k, counts: vec![0; 256], cents: vec![], id: -1, sizes: vec![], thr }
    }
    /// Byte t has arrived (data[..=t] known).
    pub fn step(&mut self, data: &[u8], t: usize) {
        self.counts[data[t] as usize] += 1;
        if t >= self.w {
            self.counts[data[t - self.w] as usize] -= 1;
        }
        if t % 16 != 15 || t + 1 < self.w {
            return;
        }
        let nrm: f32 = self.counts.iter().map(|&c| (c * c) as f32).sum::<f32>().sqrt().max(1.0);
        let h: Vec<f32> = self.counts.iter().map(|&c| c as f32 / nrm).collect();
        let mut best = (-1.0f32, usize::MAX);
        for (i, c) in self.cents.iter().enumerate() {
            let s: f32 = c.iter().zip(&h).map(|(a, b)| a * b).sum();
            if s > best.0 {
                best = (s, i);
            }
        }
        if best.1 == usize::MAX || (best.0 < self.thr && self.cents.len() < self.k) {
            self.cents.push(h);
            self.sizes.push(0);
            best.1 = self.cents.len() - 1;
        } else {
            let c = &mut self.cents[best.1];
            for (a, b) in c.iter_mut().zip(&h) {
                *a += (b - *a) / 50.0;
            }
            let n: f32 = c.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
            c.iter_mut().for_each(|x| *x /= n);
        }
        self.sizes[best.1] += 1;
        self.id = best.1 as i32;
    }
}

/// The description expert (experiment 60's prototype, moved into the stack). The segment before each file (from
/// the previous file start to this one) is read as that file's description: its tokens are runs of non-space bytes
/// with a trailing "." "," or ":" dropped (at most 64). While the file is read, a belief over which token the
/// current byte comes from is kept (an alignment, HMM style):
///   start  a learned distribution over token positions for the file's first byte;
///   move   jumps of -16..+8 tokens from one byte to the next, learned;
///   emit   each token's learned byte distribution;
///   background  a learned byte distribution for bytes the description doesn't explain, with a learned share per
///          file offset (0..7, 8+).
/// Before each byte it predicts from the belief and the background; after it, Bayes updates the belief and the
/// counts take the posterior as soft counts weighted by the description's share in explaining the byte (the jump
/// counts are hard: one count for the jump between the likeliest positions, when the description explains the byte
/// more than the background does).
pub struct DescExpert {
    vocab: std::collections::HashMap<Vec<u8>, usize>,
    emit: Vec<[f64; 256]>,
    jumps: [f64; 25],
    start: [f64; 64],
    bg: [f64; 256],
    lam: [(f64, f64); 9],
    toks: Vec<usize>,
    belief: Vec<f64>,
    prev_arg: usize,
    seg: usize,
    pub off: usize,
    pub conf: f64,
    pub mw: Vec<[f64; 3]>,
}

impl DescExpert {
    pub fn new() -> Self {
        let mut jumps = [1.0; 25];
        jumps[17] += 4.0; // a mild prior for reading on by one token (index 16 is a jump of 0)
        DescExpert { vocab: Default::default(), emit: vec![], jumps, start: [1.0; 64], bg: [1.0; 256], lam: [(1.0, 1.0); 9], toks: vec![], belief: vec![], prev_arg: 0, seg: usize::MAX, off: 0, conf: 0.0, mw: vec![[1.0 / 3.0; 3]; 9 * 4] }
    }
    /// A new file starts at byte a; the bytes from p (the previous file's start) to a are its description.
    fn begin(&mut self, data: &[u8], p: usize, a: usize) {
        self.toks.clear();
        for w in data[p..a].split(|&b| b == b' ' || b == b'\n' || b == b'\t' || b == b'\r') {
            let mut w = w;
            while let Some((&l, rest)) = w.split_last() {
                if l == b'.' || l == b',' || l == b':' { w = rest } else { break }
            }
            if w.is_empty() || w.len() > 32 {
                continue;
            }
            let n = self.vocab.len();
            let id = *self.vocab.entry(w.to_vec()).or_insert(n);
            if id == self.emit.len() {
                self.emit.push([0.05; 256]);
            }
            self.toks.push(id);
            if self.toks.len() == 64 {
                break;
            }
        }
        let m = self.toks.len();
        let s: f64 = self.start[..m].iter().sum();
        self.belief = self.start[..m].iter().map(|x| x / s.max(1e-12)).collect();
        self.off = 0;
    }
    /// The probability of byte y at t (and learn from it). seg: the start of t's file; prev: the start of the file
    /// before it (None: no description).
    pub fn code(&mut self, data: &[u8], t: usize, seg: usize, prev: Option<usize>) -> f64 {
        self.code_w(data, t, seg, prev, 1.0)
    }
    /// As `code`, with every learning step (emissions, start, jumps, background, shares) scaled by `lr` in [0, 1].
    pub fn code_w(&mut self, data: &[u8], t: usize, seg: usize, prev: Option<usize>, lr: f64) -> f64 {
        let u = 1.0 / 256.0;
        if seg != self.seg {
            self.seg = seg;
            match prev {
                Some(p) => self.begin(data, p, seg),
                None => self.toks.clear(),
            }
        } else {
            self.off += 1;
        }
        let m = self.toks.len();
        if m == 0 {
            self.conf = 0.0;
            return u;
        }
        let k = self.off;
        if k > 0 {
            let jt: f64 = self.jumps.iter().sum();
            let mut nb = vec![0.0; m];
            for (di, &pj) in self.jumps.iter().enumerate() {
                let d = di as isize - 16;
                for i in 0..m {
                    let j = i as isize + d;
                    if j >= 0 && (j as usize) < m {
                        nb[j as usize] += pj / jt * self.belief[i];
                    }
                }
            }
            let s: f64 = nb.iter().sum();
            self.belief = if s > 0.0 { nb.iter().map(|x| x / s).collect() } else { vec![1.0 / m as f64; m] };
        }
        let y = data[t] as usize;
        let rows: Vec<f64> = self.toks.iter().map(|&w| self.emit[w].iter().sum()).collect();
        let mut pd = [0.0f64; 256];
        for (i, &w) in self.toks.iter().enumerate() {
            let b = self.belief[i] / rows[i];
            if b > 1e-9 {
                for (x, e) in pd.iter_mut().zip(self.emit[w].iter()) {
                    *x += b * e;
                }
            }
        }
        let lb = k.min(8);
        let (ld, lg) = self.lam[lb];
        let lam = lg / (ld + lg);
        let bgs: f64 = self.bg.iter().sum();
        let pb = (1.0 - lam) * pd[y] + lam * self.bg[y] / bgs;
        self.conf = pd.iter().cloned().fold(0.0, f64::max) * (1.0 - lam);
        let r = (1.0 - lam) * pd[y] / pb;
        self.lam[lb].0 += lr * r;
        self.lam[lb].1 += lr * (1.0 - r);
        self.bg[y] += lr * (1.0 - r);
        let mut post: Vec<f64> = self.toks.iter().enumerate().map(|(i, &w)| self.belief[i] * self.emit[w][y] / rows[i]).collect();
        let ps: f64 = post.iter().sum();
        post.iter_mut().for_each(|x| *x /= ps.max(1e-300));
        for (i, &w) in self.toks.iter().enumerate() {
            if post[i] * r * lr > 1e-3 {
                self.emit[w][y] += post[i] * r * lr;
            }
        }
        let am = (0..m).max_by(|&a, &b| post[a].partial_cmp(&post[b]).unwrap()).unwrap();
        if k == 0 {
            for i in 0..m {
                self.start[i] += post[i] * r * lr;
            }
        } else if r > 0.5 {
            let d = am as isize - self.prev_arg as isize;
            if (-16..=8).contains(&d) {
                self.jumps[(d + 16) as usize] += r * lr;
            }
        }
        self.prev_arg = am;
        self.belief = post.iter().zip(&self.belief).map(|(p, b)| r * p + (1.0 - r) * b).collect();
        pb
    }
}

/// The role gate for the description expert (--desc-role): a Go/NoGo learner, like the word reader's act gate,
/// that decides from a segment's words whether the segment describes the one after it. Its features are the
/// segment's tokens (hashed, present or not), a bias and the segment's length (in powers of two); its weights are
/// small integers, changed by one on a mistake (perceptron). Its teacher is the level above: over the first 16
/// bytes of the next segment, the expert's shadow gain (the cost of the code before the expert stage minus the cost
/// with it, whether or not it was used). A segment whose next 16 bytes gained more than 1 bit in all is a
/// description. Ties go to Go, so at first everything is read as a description (as before) and the gate learns to
/// say NoGo where that doesn't pay.
pub struct RoleGate {
    w: Vec<i32>,
    feats: Vec<usize>,
    go: bool,
    acc: f64,
    n: usize,
    pub stats: [u64; 4],
}

impl RoleGate {
    const NF: usize = 4096;
    pub fn new() -> Self {
        RoleGate { w: vec![0; Self::NF], feats: vec![], go: true, acc: 0.0, n: 0, stats: [0; 4] }
    }
    fn h(x: &[u8], salt: u64) -> usize {
        let mut v: u64 = 0xcbf29ce484222325 ^ salt;
        for &b in x {
            v = (v ^ b as u64).wrapping_mul(0x100000001b3);
        }
        (v % Self::NF as u64) as usize
    }
    /// A new segment starts; `desc` is the segment before it (None: none). Returns Go (read it as a description).
    pub fn begin(&mut self, desc: Option<&[u8]>) -> bool {
        self.feats.clear();
        self.acc = 0.0;
        self.n = 0;
        let Some(d) = desc else {
            self.go = false;
            return false;
        };
        self.feats.push(Self::h(b"bias", 1));
        self.feats.push(Self::h(&[(usize::BITS - d.len().leading_zeros()) as u8], 2));
        for w in d.split(|&b| b == b' ' || b == b'\n' || b == b'\t' || b == b'\r') {
            if !w.is_empty() && w.len() <= 32 {
                let f = Self::h(w, 3);
                if !self.feats.contains(&f) {
                    self.feats.push(f);
                }
            }
            if self.feats.len() >= 66 {
                break;
            }
        }
        self.go = self.feats.iter().map(|&f| self.w[f]).sum::<i32>() >= 0;
        self.stats[if self.go { 0 } else { 1 }] += 1;
        self.go
    }
    pub fn go(&self) -> bool {
        self.go
    }
    /// The expert's shadow gain on one byte of the segment (bits); after 16 bytes, the gate is taught.
    pub fn observe(&mut self, gain: f64) {
        if self.feats.is_empty() || self.n >= 16 {
            return;
        }
        self.acc += gain;
        self.n += 1;
        if self.n == 16 {
            let label = self.acc > 1.0;
            if std::env::var("ROLE_DUMP").is_ok() {
                eprintln!("ROLE go={} gain={:.2} label={}", self.go as u8, self.acc, label as u8);
            }
            if label {
                self.stats[2] += 1;
            }
            if label != self.go {
                self.stats[3] += 1;
                for &f in &self.feats {
                    self.w[f] += if label { 1 } else { -1 };
                }
            }
        }
    }
}
