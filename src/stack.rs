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
    ids: HashMap<(i32, u32), u32>,
    seen: HashMap<(i32, u32), u32>,
    /// The text of each symbol (as of when it was assigned), and of the "other" symbol.
    pub texts: Vec<Vec<u8>>,
    /// Completed-chunk texts and how often each occurred (for reports).
    pub chunk_counts: HashMap<Vec<u8>, u32>,
    last_cut: usize,
}

impl VocabMap {
    fn new(vocab: usize) -> Self {
        VocabMap {
            vocab,
            ids: HashMap::new(),
            seen: HashMap::new(),
            texts: vec![],
            chunk_counts: HashMap::new(),
            last_cut: 0,
        }
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
}

impl Stack {
    pub fn new(data: Vec<u8>, cfg: Cfg, max_levels: usize, vocab: usize) -> Self {
        let b = Brain::new(data, cfg.clone());
        Stack {
            levels: vec![b],
            maps: vec![VocabMap::new(vocab)],
            max_levels,
            vocab,
            cfg,
            upper_cfg: None,
            log: vec![],
        }
    }

    /// Text of input symbol `s` of level `l`.
    fn sym_text(&self, l: usize, s: u32) -> Vec<u8> {
        if l == 0 {
            return vec![s as u8];
        }
        let m = &self.maps[l - 1];
        m.texts
            .get(s as usize)
            .cloned()
            .unwrap_or_else(|| b"\xb7".to_vec())
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
        let Some((cell, gen)) = done else { return };
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
        let sym = self.maps[l].id((cell, gen), &text);
        if self.levels.len() > l + 1 {
            self.levels[l + 1].push(sym);
            self.after_tick(l + 1, byte_t);
            let u = self.levels[l + 1].predict_cell();
            self.levels[l].upper_state = u;
            self.levels[l].add_upper();
        } else if self.levels.len() < self.max_levels && self.levels[l].chunk_kept() {
            let mut cfg = self.upper_cfg.clone().unwrap_or_else(|| self.cfg.clone());
            cfg.seed = self.cfg.seed + 1000 * (l as u64 + 1);
            cfg.oracle = 0;
            self.levels.push(Brain::new_symbols(self.vocab + 1, cfg));
            self.maps.push(VocabMap::new(self.vocab));
            self.log.push((
                l + 1,
                byte_t,
                format!("level {} spawned over level {l}'s chunks", l + 1),
            ));
            self.levels[l + 1].push(sym);
        }
    }
}
