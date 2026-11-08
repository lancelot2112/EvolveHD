//! Diagnostic: the description expert alone on a stream with description segments, told which segments are
//! descriptions (as --desc-oracle). Prints its cost (bits per byte) on each file's first MAGIC bytes after a right (R)
//! or wrong (W) description, by quarter of the stream. Env SPELL_ONLY, SPELL_PRIOR, SPELL_NOV as in the stack.
//! usage: spelltest STREAM(.bin/.starts/.descs/.items.json prefix) [--spell]
use evolvehd::acts::{DescExpert, Spell};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let p = &a[1];
    let spell = a.iter().any(|x| x == "--spell");
    let data = std::fs::read(format!("{p}.bin")).unwrap();
    let rd = |e: &str| -> Vec<usize> { std::fs::read_to_string(format!("{p}.{e}")).unwrap().lines().map(|l| l.parse().unwrap()).collect() };
    let starts = rd("starts");
    let descs = rd("descs");
    let items = std::fs::read_to_string(format!("{p}.items.json")).unwrap();
    // cond and magic length per file start, from the items JSON (minimal parse)
    let mut info = std::collections::HashMap::new();
    for it in items.split("{\"format\"").skip(1) {
        let g = |k: &str| -> String { let i = it.find(&format!("\"{k}\": ")).unwrap() + k.len() + 4; it[i..].split(|c| c == ',' || c == '}').next().unwrap().trim().trim_matches('"').to_string() };
        let st: usize = g("start").parse().unwrap();
        let ml: usize = it[it.find("\"fields\": [[").unwrap() + 12..].split(',').nth(1).unwrap().trim().parse().unwrap();
        info.insert(st, (g("cond"), ml));
    }
    let mut de = DescExpert::new();
    if spell {
        de.spell = Some(Spell::new());
    }
    let n = starts.len();
    let mut acc = vec![[(0.0f64, 0usize); 2]; 4];
    for (i, &s) in starts.iter().enumerate() {
        if descs.binary_search(&s).is_ok() {
            continue;
        }
        let e = if i + 1 < n { starts[i + 1] } else { data.len() };
        let prev = if i > 0 && descs.binary_search(&starts[i - 1]).is_ok() { Some(starts[i - 1]) } else { None };
        let q = 4 * i / n;
        for t in s..e {
            let pr = de.code(&data, t, s, prev);
            if let Some((c, ml)) = info.get(&s) {
                if t - s < *ml && prev.is_some() {
                    let ci = if c == "R" { 0 } else { 1 };
                    acc[q][ci].0 += -pr.log2();
                    acc[q][ci].1 += 1;
                }
            }
        }
    }
    for q in 0..4 {
        let r = acc[q][0].0 / acc[q][0].1 as f64;
        let w = acc[q][1].0 / acc[q][1].1 as f64;
        print!("q{q} R {r:.2} W {w:.2} R-W {:+.2} | ", r - w);
    }
    println!();
    if let (Some(sp), Ok(k)) = (de.spell.as_ref(), std::env::var("KEYS")) {
        // KEYS: comma-separated pieces as text, e.g. "\\x1,x1a,1a" (1 = token start written as ^)
        let keys: Vec<[u8; 4]> = k.split(',').map(|t| { let b: Vec<u8> = t.bytes().map(|c| if c == b'^' { 1 } else { c }).collect(); let mut a = [0u8; 4]; a[0] = b.len() as u8; a[1..1 + b.len()].copy_from_slice(&b); a }).collect();
        sp.debug_dump(&keys);
    }
}
