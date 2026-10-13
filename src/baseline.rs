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
