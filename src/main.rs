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

/// The situation model's settings from --sit-* flags (see src/situation.rs).
fn sit_cfg(args: &[String]) -> evolvehd::situation::SitCfg {
    let g = |k: &str, d: &str| arg(args, k).unwrap_or(d).to_string();
    evolvehd::situation::SitCfg {
        d: g("--sit-d", "65536").parse().unwrap(),
        k: g("--sit-k", "64").parse().unwrap(),
        mag: g("--sit-mag", "2").parse().unwrap(),
        cue: g("--sit-cue", "4").parse().unwrap(),
        win: g("--sit-win", "8").parse().unwrap(),
        ntop: g("--sit-ntop", "40").parse().unwrap(),
        pool: g("--sit-pool", "0.6").parse().unwrap(),
        recluster: g("--sit-recluster", "2000").parse().unwrap(),
        mdim: g("--sit-mdim", "2048").parse().unwrap(),
        mwin: g("--sit-mwin", "2").parse().unwrap(),
        bdrift: g("--sit-bdrift", "1").parse().unwrap(),
        clock: g("--sit-clock", "0").parse().unwrap(),
        sense: flag(args, "--sit-sense"),
        sense_mix: g("--sit-sense-mix", "1").parse().unwrap(),
        nsenses: g("--sit-nsenses", "3").parse().unwrap(),
        sense_new: g("--sit-sense-new", "0.3").parse().unwrap(),
        kinds: flag(args, "--sit-kinds"),
        mood: flag(args, "--sit-mood"),
        recency: flag(args, "--sit-recency"),
        bounce: flag(args, "--sit-bounce"),
        supersede: flag(args, "--sit-supersede"),
        compose: g("--sit-compose", "0").parse().unwrap(),
        precision: flag(args, "--sit-precision"),
        entity_cues: flag(args, "--sit-entity-cues"),
        live: flag(args, "--sit-live"),
        nroutes: g("--sit-nroutes", "0").parse().unwrap(),
        query: flag(args, "--sit-query"),
        qform14: flag(args, "--sit-qform14"),
        kindfirst: flag(args, "--sit-kindfirst"),
        replay: g("--sit-replay", "0").parse().unwrap(),
        fpool: flag(args, "--sit-fpool"),
        lookback: g("--sit-lookback", "0").parse().unwrap(),
        merge: flag(args, "--sit-merge"),
        sq_max: g("--sit-sq", "0").parse().unwrap(),
        sq_mode: g("--sit-sq-mode", "0").parse().unwrap(),
        sq_credit: g("--sit-sq-credit", "0").parse().unwrap(),
        sq_rank: g("--sit-sq-rank", "0").parse().unwrap(),
        sq_kinds: flag(args, "--sit-sq-kinds"),
        sq_price: g("--sit-sq-price", "0").parse().unwrap(),
        sq_open: g("--sit-sq-open", "0").parse().unwrap(),
        cred: g("--sit-cred", "2").parse().unwrap(),
        seed: g("--sit-seed", "0").parse().unwrap(),
    }
}

/// The bitwise hippocampal store's settings from --hp-* flags (see src/hippo.rs).
fn hippo_cfg(args: &[String]) -> evolvehd::hippo::HippoCfg {
    let g = |k: &str, d: &str| arg(args, k).unwrap_or(d).to_string();
    evolvehd::hippo::HippoCfg {
        s: g("--hp-s", "16").parse().unwrap(),
        l: g("--hp-l", "256").parse().unwrap(),
        win: g("--hp-win", "8").parse().unwrap(),
        planes: g("--hp-planes", "8").parse().unwrap(),
        epoch: g("--hp-epoch", "32").parse().unwrap(),
        cue: g("--hp-cue", "4").parse().unwrap(),
        k: g("--hp-k", "8").parse().unwrap(),
        k2: g("--hp-k2", "2").parse().unwrap(),
        khit: g("--hp-khit", "0").parse().unwrap(),
        ctx: g("--hp-ctx", "0,2,3")
            .split(',')
            .map(|x| x.parse().unwrap())
            .collect(),
        seg_r: g("--hp-seg-r", "16").parse().unwrap(),
        ctx_bind: flag(args, "--hp-ctx-bind"),
        lift: !flag(args, "--hp-no-lift"),
        steep: flag(args, "--hp-steep"),
        offer: g("--hp-offer", "1").parse().unwrap(),
        offer_all: flag(args, "--hp-offer-all"),
        front_rank: flag(args, "--hp-front-rank"),
        att_rate: flag(args, "--hp-att-rate"),
        cancel: flag(args, "--hp-cancel"),
        excess: flag(args, "--hp-excess"),
        phase: g("--hp-phase", "0").parse().unwrap(),
        typed: flag(args, "--hp-typed"),
        holo_d: g("--hp-holo", "0").parse().unwrap(),
        holo_levels: g("--hp-holo-levels", "16").parse().unwrap(),
        holo_decay: g("--hp-holo-decay", "0").parse().unwrap(),
        holo_mag: g("--hp-holo-mag", "1").parse().unwrap(),
        holo_sup: g("--hp-holo-sup", "0").parse().unwrap(),
        ch_d: g("--hp-ctxholo", "0").parse().unwrap(),
        ch_k: g("--hp-ch-k", "1024").parse().unwrap(),
        ch_drift: g("--hp-ch-drift", "4").parse().unwrap(),
        ch_bdrift: g("--hp-ch-bdrift", "32").parse().unwrap(),
        ch_levels: g("--hp-ch-levels", "0").parse().unwrap(),
        ch2_d: g("--hp-ch2", "0").parse().unwrap(),
        ch2_k: g("--hp-ch2-k", "1024").parse().unwrap(),
        ch2_levels: g("--hp-ch2-levels", "16").parse().unwrap(),
        ch2_every: g("--hp-ch2-every", "1").parse().unwrap(),
        hop2_ph: g("--hp-hop2-ph", "")
            .split(',')
            .filter(|x| !x.is_empty())
            .map(|x| x.parse().unwrap())
            .collect(),
    }
}

/// The heteroassociative store's settings from --as-* flags (see src/assoc.rs).
fn assoc_cfg(args: &[String]) -> evolvehd::assoc::AssocCfg {
    let g = |k: &str, d: &str| arg(args, k).unwrap_or(d).to_string();
    evolvehd::assoc::AssocCfg {
        s: g("--as-s", "32").parse().unwrap(),
        l: g("--as-l", "64").parse().unwrap(),
        win: g("--as-win", "8").parse().unwrap(),
        fade: g("--as-fade", "0.8").parse().unwrap(),
        half: g("--as-half", "20").parse().unwrap(),
        cue: g("--as-cue", "6").parse().unwrap(),
        ctx: g("--as-ctx", "4").parse().unwrap(),
        k: g("--as-k", "8").parse().unwrap(),
        hops: g("--as-hops", "2").parse().unwrap(),
        glr: g("--as-glr", "0.01").parse().unwrap(),
        sharpen: g("--as-sharpen", "2").parse().unwrap(),
        cue_pow: g("--as-cue-pow", "2").parse().unwrap(),
        exclude: !flag(args, "--as-keep-cue"),
        roles: flag(args, "--as-roles"),
        cue_gate: flag(args, "--as-cue-gate"),
        cue_lr: g("--as-cue-lr", "0.1").parse().unwrap(),
    }
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
    if let Some(x) = arg(args, "--err-write") {
        c.err_write = match x {
            "global" | "1" => 1,
            "local" | "2" => 2,
            _ => 0,
        };
    }
    if let Some(x) = arg(args, "--err-alpha") {
        c.err_alpha = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--err-floor") {
        c.err_floor = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--err-forget") {
        c.err_forget = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--story") {
        c.story = x.parse().unwrap();
    }
    if let Some(x) = arg(args, "--story-half") {
        c.story_half = x.parse().unwrap();
    }
    c.story_write1 = flag(args, "--story-write1");
    c.story_burst = flag(args, "--story-burst");
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
        Some("meaning") => {
            // the meaning-code prototype (scripts/meaning_codes.py) in Rust: reads full word ids and the
            // prequential unigram probability of each word (--up, f64 LE), writes each readout's probability of
            // the exact word per tick (OUT.q_bits / .q_mean / .q_wm, f64 LE) for scripts/score_meaning.py
            use evolvehd::meaning::{BitRows, Meaning, MeaningCfg, Snapshot};
            let rd = |p: &str| std::fs::read(p).unwrap();
            let ids: Vec<u32> = rd(arg(&args, "--data").expect("--data"))
                .chunks(4)
                .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            let up: Vec<f64> = rd(arg(&args, "--up").expect("--up"))
                .chunks(8)
                .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
                .collect();
            let out = arg(&args, "--out").expect("--out").to_string();
            let g = |k: &str, d: &str| arg(&args, k).unwrap_or(d).to_string();
            let mut cfg = MeaningCfg::default();
            cfg.d = g("--D", "2048").parse().unwrap();
            cfg.k_word = g("--K", "64").parse().unwrap();
            cfg.k_story = g("--k", "64").parse().unwrap();
            cfg.win = g("--win", "16").parse().unwrap();
            cfg.fast = g("--fast", "0").parse().unwrap();
            cfg.fast_n = g("--fast-n", "8").parse().unwrap();
            cfg.wm_half = g("--wm-half", "0").parse().unwrap();
            cfg.wm_k = g("--wm-k", "64").parse().unwrap();
            cfg.wm_rot = g("--wm-rot", "1") != "0";
            cfg.identity = flag(&args, "--identity");
            let snap_every: usize = g("--snap", "20000").parse().unwrap();
            let beta = 256.0;
            let n = ids.len();
            let nw = *ids.iter().max().unwrap() as usize + 1;
            let wm_on = cfg.wm_half > 0.0;
            let d = cfg.d;
            let mut mn = Meaning::new(cfg, nw);
            let small = flag(&args, "--rows16");
            let mut rows = BitRows::new(d, nw, small);
            let mut wrows = if wm_on {
                Some(BitRows::new(d, nw, small))
            } else {
                None
            };
            let mut snap: Option<Snapshot> = None;
            let (mut qb, mut qm, mut qw) = (vec![0.0f64; n], vec![0.0f64; n], vec![0.0f64; n]);
            let t0 = Instant::now();
            for i in 0..n {
                let w = ids[i];
                if let Some(wr) = wrows.as_mut() {
                    qw[i] = wr.prob(&mn.wm_code, w, up[i], beta);
                    let c = mn.wm_code.clone();
                    wr.learn(&c, w);
                }
                qb[i] = rows.prob(&mn.code, w, up[i], beta);
                if let Some(s) = &snap {
                    qm[i] = s.prob(w, &mn.incode, mn.code.len());
                }
                let c = mn.code.clone();
                rows.learn(&c, w);
                mn.observe(w);
                if i % snap_every == snap_every - 1 {
                    snap = Some(Snapshot::take(&mut mn));
                }
                if i % 250000 == 0 {
                    eprintln!("t={i} secs={:.0}", t0.elapsed().as_secs_f64());
                }
            }
            let wr = |name: &str, v: &[f64]| {
                let b: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
                std::fs::write(format!("{out}.{name}"), b).unwrap();
            };
            wr("q_bits", &qb);
            wr("q_mean", &qm);
            if wm_on {
                wr("q_wm", &qw);
            }
            println!(
                "{n} words, {nw} distinct, {:.0} s; increments refused at the 16-bit cap: {} (code rows), {} (working memory)",
                t0.elapsed().as_secs_f64(),
                rows.saturated,
                wrows.as_ref().map(|r| r.saturated).unwrap_or(0)
            );
        }
        Some("assoc") => {
            // the heteroassociative store alone: for every word, the rank of the word among each hop's
            // candidates (255 = not among them), as u8 per hop, to OUT.rank; and each hop's k candidates (u32,
            // u32::MAX for none) to OUT.top
            let raw = std::fs::read(arg(&args, "--data").expect("--data")).unwrap();
            let ids: Vec<u32> = raw
                .chunks(4)
                .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            let nw = *ids.iter().max().unwrap() as usize + 1;
            enum St {
                A(evolvehd::assoc::Assoc),
                H(evolvehd::hippo::Hippo),
                S(evolvehd::situation::Situation),
            }
            impl St {
                fn recall(&mut self) -> Vec<Vec<(u32, f64)>> {
                    match self {
                        St::A(a) => a.recall(),
                        St::H(h) => h.recall(),
                        St::S(x) => vec![x.recall()],
                    }
                }
                fn observe(&mut self, w: u32) {
                    match self {
                        St::A(a) => a.observe(w),
                        St::H(h) => h.observe(w),
                        St::S(x) => x.observe(w),
                    }
                }
            }
            let (mut a, hops, kk) = if flag(&args, "--sit") {
                (St::S(evolvehd::situation::Situation::new(sit_cfg(&args), nw)), 1, 8)
            } else if flag(&args, "--hippo") {
                let c = hippo_cfg(&args);
                let k = c.k;
                (St::H(evolvehd::hippo::Hippo::new(c, nw)), 2, k)
            } else {
                let c = assoc_cfg(&args);
                let (h, k) = (c.hops, c.k);
                (St::A(evolvehd::assoc::Assoc::new(c, nw)), h, k)
            };
            let t0 = Instant::now();
            let mut out: Vec<u8> = Vec::with_capacity(ids.len() * hops);
            let mut top: Vec<u32> = Vec::with_capacity(ids.len() * hops);
            let mut chans: Vec<u32> = vec![];
            for &w in &ids {
                let sets = a.recall();
                for h in 0..hops {
                    let r = sets
                        .get(h)
                        .and_then(|s| s.iter().position(|x| x.0 == w))
                        .map(|p| p as u8)
                        .unwrap_or(255);
                    out.push(r);
                    for i in 0..kk {
                        top.push(
                            sets.get(h)
                                .and_then(|s| s.get(i))
                                .map(|x| x.0)
                                .unwrap_or(u32::MAX),
                        );
                    }
                }
                if let St::H(h) = &a {
                    for ch in 0..h.nch_pub {
                        for i in 0..kk {
                            chans.push(
                                h.chan_lists
                                    .get(ch)
                                    .and_then(|l| l.get(i))
                                    .copied()
                                    .unwrap_or(u32::MAX),
                            );
                        }
                    }
                }
                a.observe(w);
            }
            eprintln!("{} words, {:.1} s", ids.len(), t0.elapsed().as_secs_f64());
            if let St::S(x) = &mut a {
                let voc: Vec<String> = arg(&args, "--vocab").map(|p| std::fs::read_to_string(p).unwrap().lines().map(String::from).collect()).unwrap_or_default();
                let id = |s: &str| voc.iter().position(|w| w == s).map(|i| i as u32);
                if let (Some(to), Some(went), Some(kit), Some(gave), Some(apple), Some(mary)) = (id("to"), id("went"), id("kitchen"), id("gave"), id("apple"), id("mary")) {
                    let (ab, ap, bp) = x.sense_overlap(to, &[went, kit], &[gave, apple, mary]);
                    eprintln!("sense of 'to': moving vs giving overlap {ab:.2}; moving vs plain {ap:.2}; giving vs plain {bp:.2}; senses learned for 'to': {}", x.n_senses(to));
                }
            }
            if let St::S(x) = &a {
                let voc: Vec<String> = arg(&args, "--vocab").map(|p| std::fs::read_to_string(p).unwrap().lines().map(String::from).collect()).unwrap_or_default();
                let name = |w: u32| voc.get(w as usize).cloned().unwrap_or(w.to_string());
                eprintln!("learned boundaries: {} | paths {} | relation classes {}", x.boundaries().iter().map(|&w| name(w)).collect::<Vec<_>>().join(" "), x.n_paths(), x.n_classes());
                eprintln!("question boundaries: {} | clauses committed {}, dropped {}", x.question_boundaries().iter().map(|&w| name(w)).collect::<Vec<_>>().join(" "), x.committed, x.dropped);
                if x.cfg.sq_max > 0 {
                    let words = x.words_seen().max(1);
                    eprintln!("open queries: opened {}, closed {}, mean open {:.2}, peak {}, upkeep {:.3} reads+writes per word", x.sq_opened, x.sq_closed, x.sq_open_sum as f64 / words as f64, x.sq_peak, x.sq_work as f64 / words as f64);
                    for o in x.open_queries() {
                        let cl = |c: u32| if c == u32::MAX - 1 { "any".to_string() } else { c.to_string() };
                        eprintln!("  open query {}: ({}, kind {}) then ({}, kind {}), usefulness {}, credited {}, upkeep {}, opened at word {}", o.id, cl(o.p1.0), name(o.p1.1), cl(o.p2.0), name(o.p2.1), o.util, o.credits, o.work, o.opened_at);
                    }
                    for (p, n) in x.sq_top_cands(12) {
                        let cl = |c: u32| if c == u32::MAX - 1 { "any".to_string() } else { c.to_string() };
                        eprintln!("  candidate ({}, kind {}) then ({}, kind {}): held {} answers since last opened", cl(p.0 .0), name(p.0 .1), cl(p.1 .0), name(p.1 .1), n);
                    }
                }
                if x.cfg.lookback > 0 {
                    eprintln!("lookback: answered from the window {} times; after a question boundary {} of {}", x.used_window, x.used_window_q, x.questions_seen);
                }
                if x.cfg.kinds {
                    eprintln!("kinds: {}", x.kind_groups().iter().map(|g| g.iter().map(|&w| name(w)).collect::<Vec<_>>().join(" ")).collect::<Vec<_>>().join(" | "));
                }
                for cl in x.classes(8) {
                    let ps: Vec<String> = cl.iter().map(|p| format!("{} {}", if p[0] == 0 { "f" } else { "b" }, p[1..].iter().rev().map(|&w| name(w)).collect::<Vec<_>>().join(" "))).collect();
                    eprintln!("  class: {}", ps.join(" | "));
                }
            }
            if !chans.is_empty() {
                std::fs::write(
                    format!("{}.chan", arg(&args, "--out").unwrap()),
                    chans
                        .iter()
                        .flat_map(|x| x.to_le_bytes())
                        .collect::<Vec<u8>>(),
                )
                .unwrap();
            }
            if let St::H(h) = &a {
                let voc: Vec<String> = arg(&args, "--vocab")
                    .map(|p| {
                        std::fs::read_to_string(p)
                            .unwrap()
                            .lines()
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                let b: Vec<String> = h
                    .boundaries()
                    .iter()
                    .map(|&w| voc.get(w as usize).cloned().unwrap_or(w.to_string()))
                    .collect();
                eprintln!(
                    "learned boundaries: {} | attention updates {} | superseded {}",
                    b.join(" "),
                    h.att_updates,
                    h.superseded
                );
                if flag(&args, "--seg-stats") {
                    for (w, n, hh, bd) in h.seg_stats() {
                        let r = |x: u64| x as f64 / n as f64;
                        eprintln!(
                            "  {:<10} n {n:5} hit rate before {:.2} after {:.2} pair {:.2}{}",
                            voc.get(w as usize).cloned().unwrap_or_default(),
                            r(hh[0]),
                            r(hh[1]),
                            r(hh[2]),
                            if bd { "  boundary" } else { "" }
                        );
                    }
                }
            }
            let o = arg(&args, "--out").expect("--out");
            std::fs::write(format!("{o}.rank"), out).unwrap();
            std::fs::write(
                format!("{o}.top"),
                top.iter()
                    .flat_map(|x| x.to_le_bytes())
                    .collect::<Vec<u8>>(),
            )
            .unwrap();
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
            let mut q_uni = vec![0.0f64; n];
            let mut q_keep = vec![0.0f64; n];
            let mut oc: HashMap<u32, u64> = HashMap::new();
            let mut on = 0u64;
            for t in 0..n {
                let w = ids[t];
                if w < base {
                    mapped[t] = w;
                    continue;
                }
                let decayed =
                    |v: f64, tl: usize| v * 2f64.powf(-((t - tl) as f64) / slot_half.max(1.0));
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
                        q_uni[t] = q;
                        if keep_half > 0.0 {
                            let qc = if keep_tot > 0.0 {
                                keep.get(&w).copied().unwrap_or(0.0) / keep_tot
                            } else {
                                0.0
                            };
                            q_keep[t] = qc;
                            let best = (0..8)
                                .min_by(|&a, &b| lam_loss[a].partial_cmp(&lam_loss[b]).unwrap())
                                .unwrap();
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
                                        let va = if owner[a].is_some() {
                                            decayed(use_v[a], last[a])
                                        } else {
                                            -1.0
                                        };
                                        let vb = if owner[b].is_some() {
                                            decayed(use_v[b], last[b])
                                        } else {
                                            -1.0
                                        };
                                        va.partial_cmp(&vb).unwrap()
                                    })
                                    .unwrap()
                            } else {
                                (0..ns)
                                    .min_by_key(|&s| (owner[s].is_some(), last[s]))
                                    .unwrap()
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
            // meaning codes in the word level (experiment 29): --mcode-edge and --wm-edge add kept edges keyed by
            // the bits of the meaning story code and of the working memory; --mean-spell lets the lit meaning name
            // "other" words, mixed into their spelling with the unigram counts and the kept-word memory
            use evolvehd::meaning::{Meaning, MeaningCfg, Snapshot};
            let mcode_edge = flag(&args, "--mcode-edge");
            let wm_edge = flag(&args, "--wm-edge");
            let mean_spell = flag(&args, "--mean-spell");
            let mut bcfg = cfg_from(&args);
            bcfg.ext_write_all = flag(&args, "--ext-write-all");
            if mcode_edge {
                bcfg.ext_edges.push("mcode".into());
            }
            if wm_edge {
                bcfg.ext_edges.push("wm".into());
            }
            let mut mn = if mcode_edge || wm_edge || mean_spell {
                let g = |k: &str, d: &str| arg(&args, k).unwrap_or(d).to_string();
                let mut mc = MeaningCfg::default();
                mc.d = g("--m-D", "2048").parse().unwrap();
                mc.k_word = g("--m-K", "64").parse().unwrap();
                mc.k_story = g("--m-k", "64").parse().unwrap();
                mc.fast = g("--m-fast", "0").parse().unwrap();
                mc.fast_n = g("--m-fast-n", "8").parse().unwrap();
                mc.wm_half = if wm_edge {
                    g("--m-wm-half", "4").parse().unwrap()
                } else {
                    0.0
                };
                mc.identity = flag(&args, "--m-identity");
                Some(Meaning::new(mc, *ids.iter().max().unwrap() as usize + 1))
            } else {
                None
            };
            let mlk = [0.0, 0.05, 0.1, 0.2, 0.3, 0.45, 0.6, 0.75];
            let mlm = [0.0, 0.02, 0.05, 0.1, 0.2, 0.3, 0.45];
            let mut mloss = vec![0.0f64; mlk.len() * mlm.len()];
            let mut b = Brain::new_symbols(vocab, bcfg);
            b.story_keys = Some(ids.clone());
            // the hippocampal store (experiment 31): --epi, --epi-skip, --epi-bounds ids, --epi-half, --epi-k, --epi-hops
            let mut epi = if flag(&args, "--epi") {
                use evolvehd::episodic::{EpiCfg, Episodic};
                let g = |k: &str, d: &str| arg(&args, k).unwrap_or(d).to_string();
                Some(
                    Episodic::new(EpiCfg {
                        skip: g("--epi-skip", "100").parse().unwrap(),
                        bounds: g("--epi-bounds", "")
                            .split(',')
                            .filter(|x| !x.is_empty())
                            .map(|x| x.parse().unwrap())
                            .collect(),
                        half: g("--epi-half", "200").parse().unwrap(),
                        k: g("--epi-k", "8").parse().unwrap(),
                        hops: g("--epi-hops", "2").parse().unwrap(),
                        roles: flag(&args, "--epi-roles") || flag(&args, "--epi-both"),
                        both: flag(&args, "--epi-both"),
                        ctx_words: g("--epi-ctx", "0").parse().unwrap(),
                        mclass: flag(&args, "--epi-mclass"),
                        mdim: g("--epi-mdim", "14336").parse().unwrap(),
                        mbar: g("--epi-mbar", "0.5").parse().unwrap(),
                        learned: flag(&args, "--epi-learned"),
                        win: g("--epi-win", "8").parse().unwrap(),
                        cue: g("--epi-cue", "6").parse().unwrap(),
                        role_cap: g("--epi-role-cap", "32").parse().unwrap(),
                        gate_cap: g("--epi-gate-cap", "128").parse().unwrap(),
                    })
                    .with_meaning(*ids.iter().max().unwrap() as usize + 1),
                )
            } else {
                None
            };
            // the heteroassociative store (experiment 31b): --assoc, --as-* (see src/assoc.rs)
            let mut hip = if flag(&args, "--hippo") {
                Some(evolvehd::hippo::Hippo::new(
                    hippo_cfg(&args),
                    *ids.iter().max().unwrap() as usize + 1,
                ))
            } else {
                None
            };
            let mut asc = if flag(&args, "--assoc") {
                Some(evolvehd::assoc::Assoc::new(
                    assoc_cfg(&args),
                    *ids.iter().max().unwrap() as usize + 1,
                ))
            } else {
                None
            };
            b.copy_nocontext = flag(&args, "--copy-nocontext");
            if let Some(x) = arg(&args, "--copy-lr") {
                b.copy_lr = x.parse().unwrap();
            }
            let t0 = Instant::now();
            let prog: usize = std::env::var("EVOLVEHD_PROGRESS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            // the meaning state depends only on the word stream, so it runs on its own thread, a step ahead,
            // handing each tick's story code, working-memory code and naming probability to the word level
            let has_m = mn.is_some();
            let (tx, rx) = std::sync::mpsc::sync_channel::<(Vec<u32>, Vec<u32>, f64)>(4096);
            std::thread::scope(|sc| {
                if let Some(mut m) = mn.take() {
                    let (ids_r, mapped_r) = (&ids, &mapped);
                    sc.spawn(move || {
                        let mut msnap: Option<Snapshot> = None;
                        for t in 0..n {
                            let qm = if mean_spell && mapped_r[t] == other {
                                msnap
                                    .as_ref()
                                    .map(|s| s.prob_oov(ids_r[t], &m.code, &m.incode))
                                    .unwrap_or(0.0)
                            } else {
                                0.0
                            };
                            if tx.send((m.code.clone(), m.wm_code.clone(), qm)).is_err() {
                                return;
                            }
                            m.observe(ids_r[t]);
                            if mean_spell && t % 20000 == 19999 {
                                msnap = Some(Snapshot::take_oov(&mut m, base));
                            }
                        }
                    });
                } else {
                    drop(tx);
                }
                for t in 0..n {
                    let item = if has_m {
                        Some(rx.recv().unwrap())
                    } else {
                        None
                    };
                    if let Some((code, wmc, _)) = item.as_ref() {
                        let mut j = 0;
                        if mcode_edge {
                            b.set_ext(j, code.iter().map(|&x| (x, 1.0)).collect());
                            j += 1;
                        }
                        if wm_edge {
                            b.set_ext(j, wmc.iter().map(|&x| (x, 1.0)).collect());
                        }
                    }
                    if let Some(ep) = epi.as_mut() {
                        let sets = if ep.cfg.learned {
                            ep.recall_learned()
                        } else if ep.cfg.both {
                            ep.recall_both()
                        } else if ep.cfg.roles {
                            ep.recall_roles()
                        } else {
                            ep.recall()
                        };
                        for (j, set) in sets.into_iter().enumerate() {
                            b.set_copy(j, set);
                        }
                        b.copy_ctx_ext = ep.ctx_key();
                    }
                    if let Some(a) = asc.as_mut() {
                        for (j, set) in a.recall().into_iter().enumerate() {
                            b.set_copy(j, set);
                        }
                    }
                    if let Some(h) = hip.as_mut() {
                        for (j, set) in h.recall().into_iter().enumerate() {
                            b.set_copy(j, set);
                        }
                    }
                    b.push(mapped[t]);
                    if let Some(ep) = epi.as_mut() {
                        ep.observe(ids[t]);
                    }
                    if let Some(a) = asc.as_mut() {
                        a.observe(ids[t]);
                    }
                    if let Some(h) = hip.as_mut() {
                        h.observe(ids[t]);
                    }
                    if let Some((_, _, qm)) = item {
                        if mean_spell && mapped[t] == other {
                            let (q, qc) = (q_uni[t], q_keep[t]);
                            let mut best = 0;
                            for (i, l) in mloss.iter().enumerate() {
                                if *l < mloss[best] {
                                    best = i;
                                }
                            }
                            let p_at = |i: usize| {
                                let (lk, lm) = (mlk[i / mlm.len()], mlm[i % mlm.len()]);
                                if lk + lm >= 0.95 {
                                    f64::NAN
                                } else {
                                    (1.0 - lk - lm) * q + lk * qc + lm * qm
                                }
                            };
                            spell[t] = -p_at(best).log2();
                            for i in 0..mloss.len() {
                                let p = p_at(i);
                                mloss[i] += if p.is_nan() { f64::INFINITY } else { -p.log2() };
                            }
                        }
                    }
                    if let Some(sym) = renew[t] {
                        b.renew_sym(sym);
                    }
                    if prog > 0 && (t + 1) % prog == 0 {
                        eprintln!(
                            "t={} secs={:.0} bits={:.3}",
                            t + 1,
                            t0.elapsed().as_secs_f64(),
                            b.bpb(t + 1 - prog, t + 1)
                        );
                    }
                }
            });
            let vocab_txt: Vec<String> = arg(&args, "--epi-vocab")
                .map(|p| {
                    std::fs::read_to_string(p)
                        .unwrap()
                        .lines()
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();
            if let Some(ep) = epi.as_ref() {
                let (r, g, c) = ep.stats();
                eprintln!("store: {r} role classes, {g} gate classes, {c} recall channels");
            }
            let mut gl: Vec<(f64, String)> = vec![];
            for (j, (b0, w)) in b.copy_gains().into_iter().enumerate() {
                let top: Vec<String> = w
                    .iter()
                    .take(3)
                    .map(|(c, x)| format!("cell {c}: {x:+.2}"))
                    .collect();
                let lab = match epi.as_ref() {
                    Some(ep)
                        if (ep.cfg.roles || ep.cfg.learned)
                            && j < ep.chan_names.len()
                                + if ep.cfg.both || ep.cfg.learned {
                                    ep.cfg.hops
                                } else {
                                    0
                                } =>
                    {
                        ep.chan_label(j, &vocab_txt)
                    }
                    _ => format!("channel {j}"),
                };
                let best = w.first().map(|x| x.1).unwrap_or(0.0);
                gl.push((
                    best,
                    format!("{lab}: base {b0:+.2}; context {}", top.join(", ")),
                ));
            }
            gl.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            for (_, l) in gl.iter().take(12) {
                eprintln!("copy {l}");
            }
            if let Some(p) = arg(&args, "--dump-bits") {
                // per word: the bits paid (f64 LE) and whether the top prediction was right (u8)
                let raw: Vec<u8> = b.bits[..n].iter().flat_map(|x| x.to_le_bytes()).collect();
                std::fs::write(format!("{p}.bits"), raw).unwrap();
                let hits: Vec<u8> = b.hit[..n].iter().map(|&h| h as u8).collect();
                std::fs::write(format!("{p}.hits"), hits).unwrap();
            }
            {
                let p = &b.prof;
                let names = [
                    "",
                    "temporal",
                    "synergy",
                    "screen",
                    "check",
                    "scoring",
                    "gains",
                    "writes",
                    "nodes",
                    "run-screens",
                    "matching",
                    "tick",
                ];
                let parts: Vec<String> = (1..12)
                    .map(|i| format!("{} {:.1}", names[i], p[i]))
                    .collect();
                eprintln!("profile (s): {}", parts.join(", "));
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
