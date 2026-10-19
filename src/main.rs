//! Command line for the text experiments.
//!
//! ```text
//! evolvehd text  --data corpus/bookA.bin,corpus/bookB.bin [--seed 0] [--symbolic] [--oracle 4] [--no-grow]
//!                [--noise 0.25,4] [--merge 8] [--hebb 0.02] [--drop-random] [--words] [--run-window 200000] [--classes] [--conjoin-words] [--long-trials 5] [--long-trial-states 4096]
//!                [--grow-pools] [--readout2 | --readout3 | --row-template --confidence --mask-bits 1024 --write-bits 16] [--sparse-rows] [--age-every 128] [--bias-edge] [--code-bits 8] [--bitwise-levels]
//!                [--max-pool 131072] [--generate "prompt one|prompt two" --gen-len 300 --temp 0.7 --gen-out file] [--pool-budget 1048576] [--young-age 50000] [--chunk-cap 4096]
//!                [--out run.json]
//! evolvehd ngram --data corpus/bookA.bin,corpus/bookB.bin [--order 4]
//! evolvehd table [--corpus corpus] [--out-dir runs]
//! evolvehd stack --data ... [text flags] [--levels 3] [--vocab 512] [--out-dir runs/stack] [--upper "flags for levels above 0"]   # the standard text table, jobs run in parallel
//! ```
use evolvehd::baseline::{witten_bell_bits, witten_bell_bits_syms, WbModel};
use evolvehd::brain::{Brain, Cfg};
use evolvehd::rng::Rng;
use evolvehd::stack::Stack;
use std::collections::HashMap;
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
    c.bias_edge = flag(args, "--bias-edge");
    c.bitwise_levels = flag(args, "--bitwise-levels");
    c.screen_mix = flag(args, "--screen-mix");
    c.fresh_extensions = flag(args, "--fresh-extensions");
    if let Some(x) = arg(args, "--mix-thr") {
        c.mix_thr = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--window") {
        c.w = x.parse().unwrap();
    }
    c.recency_kept = flag(args, "--recency-kept");
    if let Some(x) = arg(args, "--story") {
        c.story = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--story-half") {
        c.story_half = x.parse().unwrap();
    }
    c.story_write1 = flag(args, "--story-write1");
    if let Some(x) = arg(args, "--story-age") {
        c.story_age = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--story-skip") {
        c.story_skip = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--bias-mask-bits") {
        c.bias_mask_bits = x.parse().unwrap();
    }
    c.vocab_fixed = flag(args, "--vocab-fixed");
    c.fuse = flag(args, "--fuse");
    c.fuse_rel = flag(args, "--fuse-rel");
    if flag(args, "--real") {
        c.force_real = true;
        c.real = true;
    }
    if let Some(x) = arg(args, "--fuse-parts") {
        c.fuse_parts = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--fuse-k") {
        c.fuse_k = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--fuse-bar") {
        c.fuse_bar = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--hand-up-entropy") {
        c.hand_up_entropy = Some(x.parse().unwrap());
    }
    if let Some(x) = arg(args, "--hand-up-z") {
        c.hand_up_z = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--bias-age") {
        c.bias_age = Some(x.parse().unwrap());
    }
    if let Some(x) = arg(args, "--recency") {
        c.recency = x.split(',').map(|v| v.parse().unwrap()).collect();
    }
    if let Some(x) = arg(args, "--vocab-halflife") {
        c.vocab_halflife = x.parse().unwrap();
    }
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
    if flag(args, "--readout3") {
        // readout2's masks, full writes, rows that start empty, and levels sampled per code bit
        c.mask_bits = 1024;
        c.write_bits = 32;
        c.sparse_rows = true;
        c.bitwise_levels = true;
        c.confidence = true;
    }
    if let Some(x) = arg(args, "--mask-bits") {
        c.mask_bits = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--write-bits") {
        c.write_bits = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--code-bits") {
        c.code_bits = x.parse().unwrap();
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
            let levels: usize = arg(&args, "--levels").unwrap_or("3").parse().unwrap();
            let vocab: usize = arg(&args, "--vocab").unwrap_or("512").parse().unwrap();
            let dir = arg(&args, "--out-dir").unwrap_or("runs/stack").to_string();
            std::fs::create_dir_all(&dir).unwrap();
            let progress: usize = std::env::var("EVOLVEHD_PROGRESS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let t0 = Instant::now();
            let mut st = if let Some(sp) = arg(&args, "--syms") {
                // level 0 reads a symbol stream (u32 LE); --syms-vocab gives each symbol's text, one per line
                let raw = std::fs::read(sp).unwrap();
                let syms: Vec<u32> = raw
                    .chunks(4)
                    .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect();
                let texts: Vec<Vec<u8>> =
                    std::fs::read_to_string(arg(&args, "--syms-vocab").expect("--syms-vocab"))
                        .unwrap()
                        .lines()
                        .map(|l| l.replace("\\n", "\n").into_bytes())
                        .collect();
                Stack::from_syms(syms, texts, cfg_from(&args), levels, vocab)
            } else {
                Stack::new(
                    load(arg(&args, "--data").expect("--data")),
                    cfg_from(&args),
                    levels,
                    vocab,
                )
            };
            if let Some(u) = arg(&args, "--upper") {
                // the upper flags come first, so they win over the shared ones (the first occurrence counts)
                let mut ua: Vec<String> = args[..2].to_vec();
                ua.extend(u.split_whitespace().map(String::from));
                ua.extend(args[2..].iter().cloned());
                st.upper_cfg = Some(cfg_from(&ua));
            }
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
                if l + 1 < st.levels.len() {
                    // the vocabulary this level hands up
                    std::fs::write(format!("{dir}/level{l}.vocab.txt"), st.maps[l].dump()).unwrap();
                    // the units this level handed up, commonest first: count, text
                    let mut u: Vec<(&Vec<u8>, &u32)> = st.maps[l].chunk_counts.iter().collect();
                    u.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
                    let mut o = String::new();
                    for (t, c) in u.iter() {
                        o += &format!("{c}\t{}\n", String::from_utf8_lossy(t).replace('\n', "\\n"));
                    }
                    std::fs::write(format!("{dir}/level{l}.units.txt"), o).unwrap();
                }
                let n = b.t;
                let half = n / 2;
                let own = if n > half { b.bpb(half, n) } else { 0.0 };
                rep += &format!("\nLevel {l}: {n} ticks, vocab {}; bits per symbol over the second half {own:.3}\n", b.vocab);
                if l == 0 && b.bytes {
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
                if l + 1 < st.levels.len() && st.maps[l].halflife > 0 {
                    rep += &format!(
                        "  vocabulary handed up: {} slots given a new text\n",
                        st.maps[l].swaps
                    );
                }
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
        Some("wordslots") => {
            // a word-level brain over full word ids (u32 LE, frequency ranks): the `--base` commonest words
            // have their own symbols, `--slots` more are handed to recent words outside them (least recently
            // used slot first), the rest are "other". Scored on exact word identity: an "other" word also
            // pays its spelling, from unigram counts over the words outside the base.
            let raw = std::fs::read(arg(&args, "--data").expect("--data")).unwrap();
            let ids: Vec<u32> = raw
                .chunks(4)
                .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            let base: u32 = arg(&args, "--base").unwrap_or("512").parse().unwrap();
            let ns: usize = arg(&args, "--slots").unwrap_or("0").parse().unwrap();
            let other = base;
            let vocab = base as usize + 1 + ns;
            // --keep-half H: the story state also names "other" words: their spelling mixes unigram counts
            // with a decaying memory of recent words outside the base (half-life H words), the mixing weight
            // chosen online (the best of a grid so far). --slot-half H: evict the slot with the lowest
            // decaying use count instead of the least recently used.
            let keep_half: f64 = arg(&args, "--keep-half").unwrap_or("0").parse().unwrap();
            let slot_half: f64 = arg(&args, "--slot-half").unwrap_or("0").parse().unwrap();
            let lams = [0.0, 0.05, 0.1, 0.2, 0.3, 0.45, 0.6, 0.75];
            let mut lam_loss = [0.0f64; 8];
            let mut keep: HashMap<u32, f64> = HashMap::new();
            let mut keep_tot = 0.0f64;
            let mut kg = 1.0f64;
            let mut use_v: Vec<f64> = vec![0.0; ns];
            let n = ids.len();
            // slots: owner word per slot, last use; word -> slot
            let mut owner: Vec<Option<u32>> = vec![None; ns];
            let mut last: Vec<usize> = vec![0; ns];
            let mut slot_of: HashMap<u32, usize> = HashMap::new();
            let mut mapped = vec![0u32; n];
            let mut renew: Vec<Option<u32>> = vec![None; n];
            let mut spell = vec![0.0f64; n];
            let mut spell0 = vec![0.0f64; n];
            let mut oc: HashMap<u32, u64> = HashMap::new();
            let mut on = 0u64;
            for t in 0..n {
                let w = ids[t];
                if w < base {
                    mapped[t] = w;
                    continue;
                }
                let decayed = |v: f64, tl: usize| v * 2f64.powf(-((t - tl) as f64) / slot_half.max(1.0));
                match slot_of.get(&w) {
                    Some(&s) => {
                        mapped[t] = base + 1 + s as u32;
                        use_v[s] = decayed(use_v[s], last[s]) + 1.0;
                        last[s] = t;
                    }
                    None => {
                        mapped[t] = other;
                        let tq = oc.len() as f64;
                        let q = if on > 0 {
                            (*oc.get(&w).unwrap_or(&0) as f64 + tq * 1e-5) / (on as f64 + tq)
                        } else {
                            1e-5
                        };
                        spell[t] = -q.log2();
                        spell0[t] = spell[t];
                        if keep_half > 0.0 {
                            let qc = if keep_tot > 0.0 { keep.get(&w).copied().unwrap_or(0.0) / keep_tot } else { 0.0 };
                            let best = (0..8).min_by(|&a, &b| lam_loss[a].partial_cmp(&lam_loss[b]).unwrap()).unwrap();
                            for k in 0..8 {
                                let p = (1.0 - lams[k]) * q + lams[k] * qc;
                                lam_loss[k] -= p.log2();
                            }
                            spell[t] = -((1.0 - lams[best]) * q + lams[best] * qc).log2();
                        }
                        if ns > 0 {
                            let s = if slot_half > 0.0 {
                                (0..ns)
                                    .min_by(|&a, &b| {
                                        let va = if owner[a].is_some() { decayed(use_v[a], last[a]) } else { -1.0 };
                                        let vb = if owner[b].is_some() { decayed(use_v[b], last[b]) } else { -1.0 };
                                        va.partial_cmp(&vb).unwrap()
                                    })
                                    .unwrap()
                            } else {
                                (0..ns).min_by_key(|&s| (owner[s].is_some(), last[s])).unwrap()
                            };
                            use_v[s] = 1.0;
                            if let Some(o) = owner[s] {
                                slot_of.remove(&o);
                            }
                            owner[s] = Some(w);
                            slot_of.insert(w, s);
                            last[s] = t;
                            renew[t] = Some(base + 1 + s as u32);
                        }
                    }
                }
                *oc.entry(w).or_insert(0) += 1;
                on += 1;
                if keep_half > 0.0 {
                    // remember w (all words outside the base; the cache is consulted only for "other" ticks)
                    kg *= 2f64.powf(1.0 / keep_half);
                    *keep.entry(w).or_insert(0.0) += kg;
                    keep_tot += kg;
                    if kg > 1e100 {
                        keep.values_mut().for_each(|v| *v /= 1e100);
                        keep_tot /= 1e100;
                        kg /= 1e100;
                    }
                    if t % 1024 == 0 {
                        keep.retain(|_, v| *v >= kg * 1e-4);
                        keep_tot = keep.values().sum();
                    }
                }
            }
            let mut b = Brain::new_symbols(vocab, cfg_from(&args));
            b.story_keys = Some(ids.clone());
            let t0 = Instant::now();
            let prog: usize = std::env::var("EVOLVEHD_PROGRESS").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
            for t in 0..n {
                b.push(mapped[t]);
                if let Some(sym) = renew[t] {
                    b.renew_sym(sym);
                }
                if prog > 0 && (t + 1) % prog == 0 {
                    eprintln!("t={} secs={:.0} bits={:.3}", t + 1, t0.elapsed().as_secs_f64(), b.bpb(t + 1 - prog, t + 1));
                }
            }
            let h = n / 2;
            let sp = spell[h..].iter().sum::<f64>() / (n - h) as f64;
            let sp0 = spell0[h..].iter().sum::<f64>() / (n - h) as f64;
            let own = b.bpb(h, n);
            let oth = mapped[h..].iter().filter(|&&m| m == other).count() as f64 / (n - h) as f64;
            let slot = mapped[h..].iter().filter(|&&m| m > other).count() as f64 / (n - h) as f64;
            let mut line = format!(
                "{n} words, base {base}, slots {ns}, {:.0} s: other {oth:.3}, in slots {slot:.3} | second half: symbols {own:.3} + spelling {sp:.3} = exact words {:.3} bits",
                t0.elapsed().as_secs_f64(),
                own + sp
            );
            for o in 1..=2 {
                let w = witten_bell_bits_syms(&mapped, o, vocab);
                let m = w[h..n].iter().sum::<f64>() / (n - h) as f64;
                line += &format!(" | WB{o} {:.3} (exact {:.3})", m, m + sp);
            }
            let kept: Vec<String> = b
                .kept
                .iter()
                .skip(1)
                .map(|&e| b.nodes[b.slab_src(e)].name.clone())
                .collect();
            println!("{line}\n  spelling without the kept-word memory: {sp0:.3} (exact words {:.3})\n  kept: {}", own + sp0, kept.join(" "));
            if let Some(p) = arg(&args, "--out") {
                std::fs::write(p, b.summary_json(1000, 0.0)).unwrap();
            }
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
