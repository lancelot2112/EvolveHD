//! A stack of levels, each a [`Brain`] over its own symbol stream.
//!
//! Level 0 reads bytes. When a level has kept a chunk node, every completed chunk becomes a symbol of
//! the next level's stream (a vocabulary of the first `vocab` chunks seen twice; the rest share one
//! "other" symbol), and a new level is spawned to predict that stream: its own target, surprise,
//! screens, trials and growth. Nothing in it is specific to its level. Each level's predicted next
//! symbol is offered to the level below as an ordinary candidate node ("upper").
use crate::brain::{Brain, Cfg};
use std::collections::HashMap;

/// Maps completed chunks of one level to symbols of the next.
pub struct VocabMap {
    pub vocab: usize,
    /// 0: the first `vocab` chunks seen twice, keyed by (cell, generation), kept for good (experiments 17–19).
    /// > 0: keyed by chunk text, with counts that halve every `halflife` chunks; a text seen twice takes a
    /// free slot, or the slot of the weakest symbol if its count is more than twice that symbol's.
    pub halflife: usize,
    counts: HashMap<Vec<u8>, f32>,
    by_text: HashMap<Vec<u8>, u32>,
    weakest: Option<(f32, u32)>,
    n: u64,
    /// Slots given a new text so far.
    pub swaps: u64,
    /// Never give a slot a new text once all are taken (text-keyed, first come).
    pub fixed: bool,
    ids: HashMap<(i32, u32), u32>,
    seen: HashMap<(i32, u32), u32>,
    /// The text of each symbol (as of when it was assigned), and of the "other" symbol.
    pub texts: Vec<Vec<u8>>,
    /// Completed-chunk texts and how often each occurred (for reports).
    pub chunk_counts: HashMap<Vec<u8>, u32>,
    last_cut: usize,
}

impl VocabMap {
    fn new(vocab: usize, halflife: usize) -> Self {
        VocabMap {
            vocab,
            halflife,
            counts: HashMap::new(),
            by_text: HashMap::new(),
            weakest: None,
            n: 0,
            swaps: 0,
            fixed: false,
            ids: HashMap::new(),
            seen: HashMap::new(),
            texts: vec![],
            chunk_counts: HashMap::new(),
            last_cut: 0,
        }
    }

    fn with(vocab: usize, cfg: &Cfg) -> Self {
        let mut m = Self::new(vocab, cfg.vocab_halflife);
        m.fixed = cfg.vocab_fixed;
        m
    }

    /// The symbol for a completed chunk, and the slot that was just given a new text (if any).
    fn lookup(&mut self, key: (i32, u32), text: &[u8]) -> (u32, Option<u32>) {
        if self.halflife == 0 {
            return (self.id(key, text), None);
        }
        self.n += 1;
        if self.n % self.halflife as u64 == 0 {
            let by_text = &self.by_text;
            self.counts.retain(|k, c| {
                *c *= 0.5;
                *c >= 0.5 || by_text.contains_key(k)
            });
            self.weakest = None;
        }
        let c = {
            let e = self.counts.entry(text.to_vec()).or_insert(0.0);
            *e += 1.0;
            *e
        };
        if let Some(&i) = self.by_text.get(text) {
            return (i, None);
        }
        if c < 2.0 {
            return (self.vocab as u32, None);
        }
        if self.texts.len() < self.vocab {
            let i = self.texts.len() as u32;
            self.by_text.insert(text.to_vec(), i);
            self.texts.push(text.to_vec());
            return (i, None);
        }
        if self.fixed {
            return (self.vocab as u32, None);
        }
        if self.weakest.is_none() || self.n % 256 == 0 {
            let mut w = (f32::INFINITY, 0u32);
            for (i, t) in self.texts.iter().enumerate() {
                let ci = *self.counts.get(t).unwrap_or(&0.0);
                if ci < w.0 {
                    w = (ci, i as u32);
                }
            }
            self.weakest = Some(w);
        }
        let (wc, wi) = self.weakest.unwrap();
        if c > 2.0 * wc.max(0.5) {
            let old = std::mem::replace(&mut self.texts[wi as usize], text.to_vec());
            self.by_text.remove(&old);
            self.by_text.insert(text.to_vec(), wi);
            self.weakest = None;
            self.swaps += 1;
            return (wi, Some(wi));
        }
        (self.vocab as u32, None)
    }

    /// The vocabulary as text lines: symbol, current count, text.
    pub fn dump(&self) -> String {
        let mut o = String::new();
        for (i, t) in self.texts.iter().enumerate() {
            let c = self.counts.get(t).copied().unwrap_or(f32::NAN);
            o += &format!(
                "{i}\t{c:.1}\t{}\n",
                String::from_utf8_lossy(t).replace('\n', "\\n")
            );
        }
        o
    }

    /// The symbol for a completed chunk: its own id once seen twice and while there is room, else "other".
    fn id(&mut self, key: (i32, u32), text: &[u8]) -> u32 {
        if let Some(&i) = self.ids.get(&key) {
            return i;
        }
        let n = self.seen.entry(key).or_insert(0);
        *n += 1;
        if *n >= 2 && self.texts.len() < self.vocab {
            let i = self.texts.len() as u32;
            self.ids.insert(key, i);
            self.texts.push(text.to_vec());
            return i;
        }
        self.vocab as u32
    }
}

pub struct Stack {
    pub levels: Vec<Brain>,
    pub maps: Vec<VocabMap>,
    pub max_levels: usize,
    pub vocab: usize,
    cfg: Cfg,
    /// Settings for levels above 0 (default: the same as level 0).
    pub upper_cfg: Option<Cfg>,
    /// (level, byte tick, what happened)
    pub log: Vec<(usize, usize, String)>,
    /// Texts of level 0's symbols when it reads a symbol stream instead of bytes.
    pub base_texts: Option<Vec<Vec<u8>>>,
    /// For fused levels (index l+1): the last unit text that matched each input cell (for reports).
    pub cell_texts: Vec<HashMap<u32, Vec<u8>>>,
}

impl Stack {
    pub fn new(data: Vec<u8>, cfg: Cfg, max_levels: usize, vocab: usize) -> Self {
        let b = Brain::new(data, cfg.clone());
        Stack {
            levels: vec![b],
            maps: vec![VocabMap::with(vocab, &cfg)],
            max_levels,
            vocab,
            cfg,
            upper_cfg: None,
            log: vec![],
            base_texts: None,
            cell_texts: vec![HashMap::new()],
        }
    }

    /// A stack whose level 0 reads a symbol stream (e.g. whole words), with each symbol's text.
    pub fn from_syms(
        syms: Vec<u32>,
        texts: Vec<Vec<u8>>,
        cfg: Cfg,
        max_levels: usize,
        vocab: usize,
    ) -> Self {
        let b = Brain::from_syms(syms, texts.len() + 1, cfg.clone());
        Stack {
            levels: vec![b],
            maps: vec![VocabMap::with(vocab, &cfg)],
            max_levels,
            vocab,
            cfg,
            upper_cfg: None,
            log: vec![],
            base_texts: Some(texts),
            cell_texts: vec![HashMap::new()],
        }
    }

    /// Text of input symbol `s` of level `l`.
    fn sym_text(&self, l: usize, s: u32) -> Vec<u8> {
        if l == 0 {
            return match &self.base_texts {
                Some(t) => t
                    .get(s as usize)
                    .cloned()
                    .unwrap_or_else(|| b"\xb7 ".to_vec()),
                None => vec![s as u8],
            };
        }
        if let Some(t) = self.cell_texts.get(l).and_then(|m| m.get(&s)) {
            return t.clone();
        }
        let m = &self.maps[l - 1];
        m.texts
            .get(s as usize)
            .cloned()
            .unwrap_or_else(|| b"\xb7".to_vec())
    }

    /// The fused code of level l's unit over ticks [from, t): each part's input-cell code rotated by its
    /// role (0 = the unit's last part), ORed; with relational roles, also the part before the unit and the
    /// states of the level's kept holds. No thinning: the level above's matcher pool sparsifies.
    fn fuse_unit(&self, l: usize, from: usize, t: usize) -> Vec<u16> {
        let b = &self.levels[l];
        let d = crate::bits::D;
        let rot = |r: usize| (977 * (r + 1)) % d;
        let mut out: Vec<u16> = vec![];
        let mut put = |code: &crate::bits::Code, r: usize| {
            for &x in code.iter() {
                out.push(((x as usize + rot(r)) % d) as u16);
            }
        };
        let r_max = self.cfg.fuse_parts;
        for (r, tt) in (from..t).rev().take(r_max).enumerate() {
            if let Some(c) = b.input_code(tt) {
                put(&c, r);
            }
        }
        if self.cfg.fuse_rel {
            if from >= 1 {
                if let Some(c) = b.input_code(from - 1) {
                    put(&c, r_max);
                }
            }
            for (i, c) in b.hold_codes().iter().enumerate() {
                put(c, r_max + 1 + i);
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    pub fn run(&mut self, progress: usize) {
        let n = self.levels[0].tt;
        let t0 = std::time::Instant::now();
        for i in 0..n {
            self.levels[0].tick();
            self.after_tick(0, i);
            if progress > 0 && (i + 1) % progress == 0 {
                let mut line = format!("t={} secs={:.0}", i + 1, t0.elapsed().as_secs_f64());
                for (l, b) in self.levels.iter().enumerate() {
                    let lo = b.t.saturating_sub(progress / 4usize.pow(l as u32)).max(0);
                    let (rmb, rows, hmb, pmb, tmb) = b.mem_mb();
                    line += &format!(
                        " | L{l}: t={} bits={:.3} kept={} mem: rows {rows} {rmb:.0}MB hist {hmb:.0} pools {pmb:.0} ticks {tmb:.0}",
                        b.t,
                        if b.t > lo { b.bpb(lo, b.t) } else { 0.0 },
                        b.kept.len()
                    );
                }
                eprintln!("{line}");
            }
        }
    }

    fn after_tick(&mut self, l: usize, byte_t: usize) {
        let t = self.levels[l].t - 1;
        let done = self.levels[l].completed_chunk(t);
        let Some((cell, gen, cut)) = done else { return };
        let t = cut;
        // the text of the completed chunk: this level's inputs since the last cut
        let from = self.maps[l].last_cut;
        let syms: Vec<u32> = self.levels[l].syms[from..t].to_vec();
        let mut text = vec![];
        for s in syms {
            text.extend(self.sym_text(l, s));
        }
        self.maps[l].last_cut = t;
        if text.len() <= 200 {
            *self.maps[l].chunk_counts.entry(text.clone()).or_insert(0) += 1;
        }
        let fused = if self.cfg.fuse {
            Some(self.fuse_unit(l, from, t))
        } else {
            None
        };
        let (sym, renamed) = if fused.is_some() {
            (0, None)
        } else {
            self.maps[l].lookup((cell, gen), &text)
        };
        if self.levels.len() > l + 1 {
            if let Some(r) = renamed {
                self.levels[l + 1].reassign_symbol(r);
            }
            match fused.clone() {
                Some(f) => {
                    self.levels[l + 1].push_code(f);
                    let lt = self.levels[l + 1].t - 1;
                    let c = self.levels[l + 1].syms[lt];
                    self.cell_texts[l + 1].insert(c, text.clone());
                }
                None => self.levels[l + 1].push(sym),
            }
            self.after_tick(l + 1, byte_t);
            let u = self.levels[l + 1].predict_cell();
            self.levels[l].upper_state = u;
            self.levels[l].add_upper();
        } else if self.levels.len() < self.max_levels && self.levels[l].chunk_kept() {
            let mut cfg = self.upper_cfg.clone().unwrap_or_else(|| self.cfg.clone());
            cfg.seed = self.cfg.seed + 1000 * (l as u64 + 1);
            cfg.oracle = 0;
            let fusing = fused.is_some();
            if fusing {
                cfg.real = true; // a fused level matches its inputs with a real pool
            }
            let v = if fusing {
                self.vocab + 64
            } else {
                self.vocab + 1
            };
            self.levels.push(Brain::new_symbols(v, cfg));
            self.maps.push(VocabMap::with(self.vocab, &self.cfg));
            self.cell_texts.push(HashMap::new());
            self.log.push((
                l + 1,
                byte_t,
                format!("level {} spawned over level {l}'s chunks", l + 1),
            ));
            match fused {
                Some(f) => {
                    self.levels[l + 1].push_code(f);
                    let lt = self.levels[l + 1].t - 1;
                    let c = self.levels[l + 1].syms[lt];
                    self.cell_texts[l + 1].insert(c, text);
                }
                None => self.levels[l + 1].push(sym),
            }
        }
    }
}
