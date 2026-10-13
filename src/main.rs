//! Command line for the text experiments.
//!
//! ```text
//! evolvehd text  --data corpus/bookA.bin,corpus/bookB.bin [--seed 0] [--symbolic] [--oracle 4] [--no-grow]
//!                [--noise 0.25,4] [--merge 8] [--hebb 0.02] [--out run.json]
//! evolvehd ngram --data corpus/bookA.bin,corpus/bookB.bin [--order 4]
//! evolvehd table [--corpus corpus] [--out-dir runs]   # the standard text table, jobs run in parallel
//! ```
use evolvehd::baseline::witten_bell_bits;
use evolvehd::brain::{Brain, Cfg};
use std::time::Instant;

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
    if std::env::var("EVOLVEHD_PROF").is_ok() {
        let p = b.prof;
        eprintln!("prof: total {secs:.1}s, controller {:.1}s (temporal {:.1}, synergy {:.1}, screen {:.1})", p[4], p[1], p[2], p[3]);
    }
    if let Some(p) = out {
        std::fs::write(&p, b.summary_json(1000, secs)).unwrap();
    }
    let kept: Vec<String> = b
        .kept
        .iter()
        .skip(1)
        .map(|&e| b.nodes[b.slab_src(e)].name.clone())
        .collect();
    format!(
        "{name:<44} {secs:7.1}s  {}  kept: {}",
        columns(&b),
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
        Some("ngram") => {
            let data = load(arg(&args, "--data").expect("--data"));
            let order: usize = arg(&args, "--order").unwrap_or("4").parse().unwrap();
            let bits = witten_bell_bits(&data, order);
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
