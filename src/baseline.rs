//! Online (prequential) byte n-gram model with Witten-Bell interpolation: the yardstick for text runs.
use std::collections::HashMap;

/// Bits for each byte of `data` under an order-`order` Witten-Bell model, scored before learning.
pub fn witten_bell_bits(data: &[u8], order: usize) -> Vec<f64> {
    // one table per context length: context bytes packed into a u64 -> (total, distinct, counts)
    struct Ctx {
        n: u32,
        d: HashMap<u8, u32>,
    }
    let mut tabs: Vec<HashMap<u64, Ctx>> = (0..=order).map(|_| HashMap::new()).collect();
    let key = |i: usize, k: usize| -> u64 {
        let mut x: u64 = 0;
        for &b in &data[i - k..i] {
            x = (x << 8) | b as u64;
        }
        x
    };
    let mut out = Vec::with_capacity(data.len());
    for (i, &c) in data.iter().enumerate() {
        let mut p = 1.0 / 256.0;
        for k in 0..=order.min(i) {
            if let Some(cx) = tabs[k].get(&key(i, k)) {
                let t = cx.d.len() as f64;
                p = (*cx.d.get(&c).unwrap_or(&0) as f64 + t * p) / (cx.n as f64 + t);
            }
        }
        out.push(-p.log2());
        for k in 0..=order.min(i) {
            let cx = tabs[k].entry(key(i, k)).or_insert_with(|| Ctx {
                n: 0,
                d: HashMap::new(),
            });
            cx.n += 1;
            *cx.d.entry(c).or_insert(0) += 1;
        }
    }
    out
}

/// A trained Witten-Bell model that can give the next-byte distribution for any history, and sample.
pub struct WbModel {
    order: usize,
    tabs: Vec<HashMap<u64, (u32, HashMap<u8, u32>)>>,
}

impl WbModel {
    pub fn train(data: &[u8], order: usize) -> Self {
        let mut tabs: Vec<HashMap<u64, (u32, HashMap<u8, u32>)>> =
            (0..=order).map(|_| HashMap::new()).collect();
        for i in 0..data.len() {
            for k in 0..=order.min(i) {
                let e = tabs[k]
                    .entry(pack(&data[i - k..i]))
                    .or_insert_with(|| (0, HashMap::new()));
                e.0 += 1;
                *e.1.entry(data[i]).or_insert(0) += 1;
            }
        }
        WbModel { order, tabs }
    }

    /// p(next byte | the last `order` bytes of `hist`).
    pub fn dist(&self, hist: &[u8]) -> Vec<f64> {
        let mut p = vec![1.0 / 256.0; 256];
        for k in 0..=self.order.min(hist.len()) {
            if let Some((n, d)) = self.tabs[k].get(&pack(&hist[hist.len() - k..])) {
                let t = d.len() as f64;
                let mut q = vec![0.0; 256];
                for (c, x) in q.iter_mut().enumerate() {
                    *x = (*d.get(&(c as u8)).unwrap_or(&0) as f64 + t * p[c]) / (*n as f64 + t);
                }
                p = q;
            }
        }
        p
    }

    pub fn generate(
        &self,
        prompt: &[u8],
        n: usize,
        temp: f64,
        rng: &mut crate::rng::Rng,
    ) -> Vec<u8> {
        let mut h = prompt.to_vec();
        for _ in 0..n {
            let w: Vec<f64> = self.dist(&h).iter().map(|x| x.powf(1.0 / temp)).collect();
            let z: f64 = w.iter().sum();
            let mut u = rng.f64() * z;
            let mut b = 255u8;
            for (i, x) in w.iter().enumerate() {
                if u < *x {
                    b = i as u8;
                    break;
                }
                u -= x;
            }
            h.push(b);
        }
        h[prompt.len()..].to_vec()
    }
}

fn pack(ctx: &[u8]) -> u64 {
    ctx.iter().fold(0u64, |x, &b| (x << 8) | b as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learns_a_cycle() {
        let data: Vec<u8> = b"abcd".iter().cycle().take(4000).copied().collect();
        let b = witten_bell_bits(&data, 2);
        let tail: f64 = b[3000..].iter().sum::<f64>() / 1000.0;
        assert!(tail < 0.01, "{tail}");
    }
}
