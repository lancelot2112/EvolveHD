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
