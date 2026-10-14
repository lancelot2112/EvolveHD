//! Command line for the text experiments.
//!
//! ```text
//! evolvehd text  --data corpus/bookA.bin,corpus/bookB.bin [--seed 0] [--symbolic] [--oracle 4] [--no-grow]
//!                [--noise 0.25,4] [--merge 8] [--hebb 0.02] [--drop-random] [--words] [--run-window 200000] [--classes] [--conjoin-words] [--long-trials 5] [--long-trial-states 4096]
//!                [--grow-pools] [--readout2 | --row-template --confidence --mask-bits 1024 --write-bits 16] [--sparse-rows] [--age-every 128]
//!                [--max-pool 131072] [--generate "prompt one|prompt two" --gen-len 300 --temp 0.7 --gen-out file] [--pool-budget 1048576] [--young-age 50000] [--chunk-cap 4096]
//!                [--out run.json]
//! evolvehd ngram --data corpus/bookA.bin,corpus/bookB.bin [--order 4]
//! evolvehd table [--corpus corpus] [--out-dir runs]
//! evolvehd stack --data ... [text flags] [--levels 3] [--vocab 512] [--out-dir runs/stack]   # the standard text table, jobs run in parallel
//! ```
use evolvehd::baseline::{witten_bell_bits, witten_bell_bits_syms, WbModel};
use evolvehd::brain::{Brain, Cfg};
use evolvehd::rng::Rng;
use evolvehd::stack::Stack;
use std::time::Instant;

fn arg_owned(k: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    arg(&args, k).map(String::from)
}

fn arg<'a>(args: &'a [String], k: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == k)
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
}

fn flag(args: &[String], k: &str) -> bool {
    args.iter().any(|a| a == k)
}

fn load(spec: &str) -> Vec<u8> {
    let mut v = vec![];
    for p in spec.split(',') {
        v.extend(std::fs::read(p).unwrap_or_else(|e| panic!("reading {p}: {e}")));
    }
    v
}

fn cfg_from(args: &[String]) -> Cfg {
    let mut c = Cfg::default();
    if let Some(s) = arg(args, "--seed") {
        c.seed = s.parse().unwrap();
    }
    c.real = !flag(args, "--symbolic");
    if let Some(o) = arg(args, "--oracle") {
        c.oracle = o.parse().unwrap();
        c.grow = false;
    }
    if flag(args, "--no-grow") {
        c.grow = false;
    }
    if let Some(n) = arg(args, "--noise") {
        let (a, b) = n.split_once(',').unwrap();
        c.noise_drop = a.parse().unwrap();
        c.noise_spur = b.parse().unwrap();
    }
    if let Some(m) = arg(args, "--merge") {
        c.merge = m.parse().unwrap();
    }
    c.drop_lowest = !flag(args, "--drop-random");
    c.words = flag(args, "--words");
    c.classes = flag(args, "--classes");
    c.conjoin_words = flag(args, "--conjoin-words");
    c.grow_pools = flag(args, "--grow-pools");
    c.row_template = flag(args, "--row-template");
    c.sparse_rows = flag(args, "--sparse-rows");
    if let Some(x) = arg(args, "--age-every") {
        c.age_every = x.parse().unwrap();
    }
    c.confidence = flag(args, "--confidence");
    if flag(args, "--readout2") {
        // wider writer masks, full-width writes, template rows, learned confidence gain
        c.mask_bits = 1024;
        c.write_bits = 16;
        c.row_template = true;
        c.confidence = true;
    }
    if let Some(x) = arg(args, "--mask-bits") {
        c.mask_bits = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--write-bits") {
        c.write_bits = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--max-pool") {
        c.max_pool = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--pool-budget") {
        c.pool_budget = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--young-age") {
        c.young_age = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--long-trial-states") {
        c.long_trial_states = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--long-trials") {
        c.long_trials = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--run-window") {
        c.run_window = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--chunk-cap") {
        c.chunk_cap = x.parse().unwrap();
    }
    if let Some(h) = arg(args, "--hebb") {
        c.hebb = h.parse().unwrap();
    }
    c
}

/// The bits-per-byte columns used in the wiki tables, for a stream of book A (300k) then book B.
fn columns(b: &Brain) -> String {
    let n = b.t;
    if n >= 400_000 {
        format!(
            "A last100k {:.3} | B 0-50k {:.3} | B 50-100k {:.3} | A whole {:.3}",
            b.bpb(200_000, 300_000),
            b.bpb(300_000, 350_000),
            b.bpb(350_000, 400_000),
            b.bpb(0, 300_000)
        )
    } else {
        format!(
            "whole {:.3} | first 50k {:.3} | last 50k {:.3}",
            b.bpb(0, n),
            b.bpb(0, 50_000.min(n)),
            b.bpb(n.saturating_sub(50_000), n)
        )
    }
}

fn run_one(name: &str, data: Vec<u8>, cfg: Cfg, out: Option<String>) -> String {
    let t0 = Instant::now();
    let mut b = Brain::new(data, cfg);
    b.run();
    let secs = t0.elapsed().as_secs_f64();
    let mut words = String::new();
    if b.t >= 400_000 {
        for (k, lo, hi) in [("words_A", 200_000, 300_000), ("words_B", 300_000, 400_000)] {
            if let Some(r) = b.word_report(lo, hi) {
                b.extra_json.push((k.to_string(), r));
            }
        }
    } else if let Some(r) = b.word_report(b.t / 2, b.t) {
        b.extra_json.push(("words_last_half".to_string(), r));
    }
    if let Some(r) = b.class_report() {
        b.extra_json.push(("classes".to_string(), r));
    }
    for (k, r) in &b.extra_json {
        if k == "classes" {
            continue;
        }
        let grab = |f: &str| {
            r.split(&format!("\"{f}\":"))
                .nth(1)
                .and_then(|x| x.split([',', '}']).next())
                .unwrap_or("?")
                .to_string()
        };
        words.push_str(&format!(
            "  [{k}: precision {} recall {} whole {}]",
            grab("precision"),
            grab("recall"),
            grab("whole_word_segments")
        ));
    }
    if std::env::var("EVOLVEHD_PROF").is_ok() {
        let p = b.prof;
        eprintln!("prof: total {secs:.1}s, controller {:.1}s (temporal {:.1}, synergy {:.1}, screen {:.1})", p[4], p[1], p[2], p[3]);
    }
    if let Some(p) = out {
        std::fs::write(&p, b.summary_json(1000, secs)).unwrap();
    }
    let cols = columns(&b);
    if let Some(prompts) = arg_owned("--generate") {
        let n: usize = arg_owned("--gen-len")
            .map(|x| x.parse().unwrap())
            .unwrap_or(300);
        let temp: f64 = arg_owned("--temp")
            .map(|x| x.parse().unwrap())
            .unwrap_or(0.7);
        let train = b.data[..b.t].to_vec();
        let wb = WbModel::train(&train, 4);
        let mut out = String::new();
        for (i, pr) in prompts.split('|').enumerate() {
            let mut r1 = Rng::new(1000 + i as u64);
            let mut r2 = Rng::new(1000 + i as u64);
            let g = b.generate(pr.as_bytes(), n, temp, &mut r1);
            let w = wb.generate(pr.as_bytes(), n, temp, &mut r2);
            out.push_str(&format!(
                "=== prompt: {pr:?}\n--- grown\n{}{}\n--- witten-bell order 4\n{}{}\n\n",
                pr,
                String::from_utf8_lossy(&g),
                pr,
                String::from_utf8_lossy(&w)
            ));
        }
        if let Some(p) = arg_owned("--gen-out") {
            std::fs::write(p, &out).unwrap();
        }
        eprintln!("{out}");
    }
    let kept: Vec<String> = b
        .kept
        .iter()
        .skip(1)
        .map(|&e| b.nodes[b.slab_src(e)].name.clone())
        .collect();
    format!(
        "{name:<44} {secs:7.1}s  {}  kept: {}{words}",
        cols,
        kept.join(" ")
    )
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("text") => {
            let data = load(arg(&args, "--data").expect("--data"));
            let line = run_one(
                "text",
                data,
                cfg_from(&args),
                arg(&args, "--out").map(String::from),
            );
            println!("{line}");
        }
        Some("stack") => {
            let data = load(arg(&args, "--data").expect("--data"));
            let levels: usize = arg(&args, "--levels").unwrap_or("3").parse().unwrap();
            let vocab: usize = arg(&args, "--vocab").unwrap_or("512").parse().unwrap();
            let dir = arg(&args, "--out-dir").unwrap_or("runs/stack").to_string();
            std::fs::create_dir_all(&dir).unwrap();
            let progress: usize = std::env::var("EVOLVEHD_PROGRESS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let t0 = Instant::now();
            let mut st = Stack::new(data, cfg_from(&args), levels, vocab);
            st.run(progress);
            let secs = t0.elapsed().as_secs_f64();
            let mut rep = String::new();
            rep += &format!("levels: {} in {secs:.0} s\n", st.levels.len());
            for (l, msg) in st.log.iter().map(|(l, t, m)| (l, format!("byte {t}: {m}"))) {
                rep += &format!("  L{l} {msg}\n");
            }
            for (l, b) in st.levels.iter().enumerate() {
                std::fs::write(format!("{dir}/level{l}.json"), b.summary_json(1000, secs)).unwrap();
                let raw: Vec<u8> = b.syms[..b.t].iter().flat_map(|x| x.to_le_bytes()).collect();
                std::fs::write(format!("{dir}/level{l}.syms"), raw).unwrap();
                let n = b.t;
                let half = n / 2;
                let own = if n > half { b.bpb(half, n) } else { 0.0 };
                rep += &format!("\nLevel {l}: {n} ticks, vocab {}; bits per symbol over the second half {own:.3}\n", b.vocab);
                if l == 0 {
                    rep += &format!("  {}\n", columns(b));
                } else {
                    for o in 0..=3 {
                        let w = witten_bell_bits_syms(&b.syms[..n], o, b.vocab);
                        let m = w[half..n].iter().sum::<f64>() / (n - half).max(1) as f64;
                        rep += &format!("  Witten-Bell order {o} on the same symbols: {m:.3}\n");
                    }
                }
                let kept: Vec<String> = b
                    .kept
                    .iter()
                    .skip(1)
                    .map(|&e| b.nodes[b.slab_src(e)].name.clone())
                    .collect();
                rep += &format!("  kept: {}\n", kept.join(" "));
                if l < st.maps.len() {
                    let mut top: Vec<(&Vec<u8>, &u32)> = st.maps[l].chunk_counts.iter().collect();
                    top.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
                    let ex: Vec<String> = top
                        .iter()
                        .filter(|(t, _)| t.len() >= 2)
                        .take(30)
                        .map(|(t, c)| format!("{:?}x{c}", String::from_utf8_lossy(t)))
                        .collect();
                    rep += &format!("  most frequent chunks: {}\n", ex.join(" "));
                    let mut long: Vec<(&Vec<u8>, &u32)> = st.maps[l]
                        .chunk_counts
                        .iter()
                        .filter(|(_, &c)| c >= 3)
                        .collect();
                    long.sort_by(|a, b| b.0.len().cmp(&a.0.len()).then(b.1.cmp(a.1)));
                    let lx: Vec<String> = long
                        .iter()
                        .take(20)
                        .map(|(t, c)| format!("{:?}x{c}", String::from_utf8_lossy(t)))
                        .collect();
                    rep += &format!("  longest chunks seen 3+ times: {}\n", lx.join(" "));
                }
            }
            std::fs::write(format!("{dir}/report.txt"), &rep).unwrap();
            println!("{rep}");
        }
        Some("syms") => {
            // a single brain over a saved symbol stream (u32 little-endian), for diagnosing upper levels
            let raw = std::fs::read(arg(&args, "--data").expect("--data")).unwrap();
            let syms: Vec<u32> = raw
                .chunks(4)
                .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            let vocab: usize = arg(&args, "--vocab")
                .map(|x| x.parse().unwrap())
                .unwrap_or(syms.iter().copied().max().unwrap_or(0) as usize + 1);
            let mut b = Brain::new_symbols(vocab, cfg_from(&args));
            let t0 = Instant::now();
            for &x in &syms {
                b.push(x);
            }
            let n = b.t;
            let h = n / 2;
            let own = b.bpb(h, n);
            let mut line = format!(
                "{n} symbols, vocab {vocab}, {:.1} s: second half {own:.3} bits per symbol",
                t0.elapsed().as_secs_f64()
            );
            for o in 0..=2 {
                let w = witten_bell_bits_syms(&syms, o, vocab);
                line += &format!(
                    " | WB{o} {:.3}",
                    w[h..n].iter().sum::<f64>() / (n - h) as f64
                );
            }
            let kept: Vec<String> = b
                .kept
                .iter()
                .skip(1)
                .map(|&e| b.nodes[b.slab_src(e)].name.clone())
                .collect();
            println!("{line}  kept: {}", kept.join(" "));
            if let Some(p) = arg(&args, "--out") {
                std::fs::write(p, b.summary_json(1000, 0.0)).unwrap();
            }
        }
        Some("ngram") => {
            let data = load(arg(&args, "--data").expect("--data"));
            let order: usize = arg(&args, "--order").unwrap_or("4").parse().unwrap();
            let bits = witten_bell_bits(&data, order);
            if let Some(p) = arg(&args, "--out") {
                let bl: Vec<String> = bits
                    .chunks(1000)
                    .map(|c| format!("{}", c.iter().sum::<f64>() / c.len() as f64))
                    .collect();
                std::fs::write(
                    p,
                    format!("{{\"order\":{order},\"bpb\":[{}]}}", bl.join(",")),
                )
                .unwrap();
            }
            let m = |lo: usize, hi: usize| {
                bits[lo..hi.min(bits.len())].iter().sum::<f64>() / (hi.min(bits.len()) - lo) as f64
            };
            if bits.len() >= 400_000 {
                println!("order {order}: A last100k {:.3} | B 0-50k {:.3} | B 50-100k {:.3} | A whole {:.3}", m(200_000, 300_000), m(300_000, 350_000), m(350_000, 400_000), m(0, 300_000));
            } else {
                println!("order {order}: whole {:.3}", m(0, bits.len()));
            }
        }
        Some("table") => {
            let dir = arg(&args, "--corpus").unwrap_or("corpus").to_string();
            let out_dir = arg(&args, "--out-dir").unwrap_or("runs").to_string();
            std::fs::create_dir_all(&out_dir).unwrap();
            let a = std::fs::read(format!("{dir}/bookA.bin")).unwrap();
            let bb = std::fs::read(format!("{dir}/bookB.bin")).unwrap();
            let ab: Vec<u8> = a.iter().chain(bb.iter()).copied().collect();
            let d = Cfg::default();
            let jobs: Vec<(&str, Cfg, bool)> = vec![
                ("grown, real pools, seed 0", d.clone(), true),
                (
                    "grown, real pools, seed 1",
                    Cfg {
                        seed: 1,
                        ..d.clone()
                    },
                    true,
                ),
                (
                    "grown, exact nodes",
                    Cfg {
                        real: false,
                        ..d.clone()
                    },
                    true,
                ),
                (
                    "hand-wired order 4, exact nodes",
                    Cfg {
                        real: false,
                        grow: false,
                        oracle: 4,
                        ..d.clone()
                    },
                    true,
                ),
                (
                    "hand-wired order 4, real pools",
                    Cfg {
                        grow: false,
                        oracle: 4,
                        ..d.clone()
                    },
                    true,
                ),
                (
                    "grown, real pools, noisy, merge",
                    Cfg {
                        noise_drop: 0.25,
                        noise_spur: 4,
                        merge: 8,
                        hebb: 0.02,
                        ..d.clone()
                    },
                    true,
                ),
                ("grown, real pools, fresh on book B", d.clone(), false),
            ];
            let handles: Vec<_> = jobs
                .into_iter()
                .enumerate()
                .map(|(i, (name, cfg, both))| {
                    let data = if both { ab.clone() } else { bb.clone() };
                    let out = format!("{out_dir}/{i:02}.json");
                    let name = name.to_string();
                    std::thread::spawn(move || run_one(&name, data, cfg, Some(out)))
                })
                .collect();
            for h in handles {
                println!("{}", h.join().unwrap());
            }
        }
        _ => {
            eprintln!("usage: evolvehd text|ngram|table ... (see src/main.rs)");
            std::process::exit(2);
        }
    }
}
