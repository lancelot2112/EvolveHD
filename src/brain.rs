//! The grown grammar on a byte stream. A port of `reference/python/text_sim.py` (`TextBrain`),
//! with the same tick order, controller schedule and defaults.
//!
//! One target: the byte node, a matcher pool over fixed random 32-of-2048 byte codes (optionally
//! corrupted by bit dropout and spurious bits). Every derived node is one of:
//! - delay: a view of the byte node d ticks back;
//! - hold: X latched whenever the byte node shows value v (an event gate);
//! - bind: real = a matcher pool over the shift-bind of its parts' winner codes; symbolic = an exact table;
//! - distill: a kept bind edge's source cells, clustered by their predicted-successor profiles.
//!
//! Prime edges are writer masks ([`crate::edge`]). The readout is a log-linear mix with learned gains.
//! Trials are shadow trials scored by paired log-loss. Every bits-per-byte figure is prequential.
use crate::bits::{self, Bits, Code, D};
use crate::edge::{Edge, TargetIndex};
use crate::info::{
    ent, ent_nested, heldout_gain, heldout_gain_mix, par_map, roll, CmiScreen, MixBase,
};
use crate::pool::{rotate, Pool};
use crate::rng::Rng;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

#[derive(Clone, Debug)]
pub struct Cfg {
    pub seed: u64,
    pub real: bool,
    pub grow: bool,
    pub oracle: usize,
    pub noise_drop: f64,
    pub noise_spur: usize,
    pub merge: u32,
    pub burn: usize,
    pub evalw: usize,
    pub acc_gain: f64,
    pub acc_z: f64,
    pub prune_gain: f64,
    pub prune_every: usize,
    pub screen_thr: f64,
    pub syn_thr: f64,
    pub tmp_thr: f64,
    pub w: usize,
    pub max_trials: usize,
    pub max_derived: usize,
    pub distill_gain: f64,
    pub eps: f64,
    /// --floor: the code length reported for each byte comes from a learned floor over the level's prediction:
    /// a switching mixture (fixed share) over a few fixed weights of a uniform guess, (1 - l) p + l / vocab, so a
    /// stretch the level can't predict costs about log2(vocab) bits instead of the eps floor's 17. The level's own
    /// learning (surprise, screens, chunk starts) still reads its raw bits.
    pub floor: bool,
    /// --prune-guard: a kept edge is not judged over a window where the level codes worse than a uniform guess
    /// (raw bits per tick above log2(vocab)): nothing predicts there, so the window says nothing about the edge.
    /// Its window starts again instead.
    pub prune_guard: bool,
    /// --prune-harmful: a kept edge is pruned only when it does harm over its window (mean gain below zero, and
    /// clearly: z below -1), not merely when it does too little. In mixed material an edge may only help in some
    /// kinds of bytes; it then does little on average without doing harm.
    pub prune_harmful: bool,
    /// --map-cap N: a cell's byte counts (the map from the cell that matched to the byte it stood for) are halved
    /// together once they pass N, so a stretch of other material can't fix the map for good (0: never).
    pub map_cap: u32,
    /// --map-two N: a fast map (halved past N) beside the slow one, mixed (see `cellfast`).
    pub map_two: u32,
    /// --copy-act: a seek-and-copy act beside the level. At each byte it points back to the last place the current
    /// context (the last 5 bytes, then checked backwards up to 32) occurred, and offers the byte that followed
    /// there. It reads on while it is right, and past a wrong byte too (up to 8 in a row) with its length reset;
    /// a seek replaces the pointer when it finds a longer match (while the current one is under 16). Its offer is
    /// trusted by how often offers of the same match length and recent record (right in 0-1, 2-4, 5-6 or 7-8 of
    /// its last 8 offers) were right before (counted, prequential). The code is a switching mixture over
    /// the level (after its floor), the act and a uniform guess, one per trust context (match length and recent
    /// record; implies the floor). Env COPY_PLAIN: the first version (no reading on, trust by length only);
    /// COPY_CAP: halve the trust counts past this many offers (default 4096).
    pub copy_act: bool,
    pub bind_cap: usize,
    pub hebb: f64,
    /// Writer masks drop their lowest-numbered eligible bits, as the Python reference did.
    pub drop_lowest: bool,
    /// Offer the word moves: surprise-reset chunks and a previous-chunk hold.
    pub words: bool,
    pub chunk_cap: usize,
    /// If > 0, the word moves use running count screens averaged over this many recent ticks,
    /// instead of a held-out split of the stored window.
    pub run_window: usize,
    /// Offer the chunk-class Compress move when a previous-chunk edge fails.
    pub classes: bool,
    /// Offer bind(previous chunk, chunk) when the previous chunk informs only beyond the chunk.
    pub conjoin_words: bool,
    /// Longest trial, as a multiple of the normal one, for a source with more than 4,096 states whose
    /// gain is positive but below the bar (1 = off).
    pub long_trials: usize,
    /// Sources with more states than this get trial extensions.
    pub long_trial_states: usize,
    /// Let real bind and chunk pools double when most evictions remove recently used cells.
    pub grow_pools: bool,
    /// Largest size a grown pool may reach.
    pub max_pool: usize,
    /// Total cells all grown pools may hold together.
    pub pool_budget: usize,
    /// An eviction of a cell used within this many ticks counts as pressure.
    pub young_age: u64,
    /// New writer-mask rows copy a per-edge template (much cheaper than drawing fresh masks).
    pub row_template: bool,
    /// Writer-mask rows start empty instead of full of random bits.
    pub sparse_rows: bool,
    /// Writer-mask rows age (all counts halve) every this many writes; 0 = never.
    pub age_every: u32,
    /// Start with an order-0 edge from a constant source (the symbol frequencies, pooled over all contexts).
    pub bias_edge: bool,
    /// Scale each row's evidence by its Witten-Bell confidence n / (n + T).
    pub confidence: bool,
    /// Writer-mask bits per level (Python: 640) and most bits written per update (Python: 8).
    pub mask_bits: usize,
    pub write_bits: usize,
    /// Bits of each target code the readout uses (all 32 by default; fewer = sparser readout codes).
    pub code_bits: usize,
    /// Writer-mask levels are sampled per code bit instead of per write.
    pub bitwise_levels: bool,
    /// Running and held-out screens score a learned mixture with the baseline instead of joint backoff.
    pub screen_mix: bool,
    /// Screen threshold (bits) when screens score mixtures: they don't overstate, so it can be low.
    pub mix_thr: f64,
    /// An extended trial is judged on its gain since the last extension, not since it began, so a source
    /// that learns slowly is not held back by its early ticks.
    pub fresh_extensions: bool,
    /// In a stack, the next level's vocabulary is keyed by chunk text and adapts, its counts halving every
    /// this many chunks (0 = the first chunks seen twice, kept for good).
    pub vocab_halflife: usize,
    /// Recency moves: for each W, a candidate "recent(W)" whose edge has one small writer-mask row that
    /// holds about the last W target codes, so it scores what was seen recently (a cache, in bits).
    pub recency: Vec<usize>,
    /// Install the recency edges as kept from the start (for measuring them), instead of trialling them.
    pub recency_kept: bool,
    /// Story state: an edge whose rows are keyed by the `story` most salient recent content symbols
    /// (salience decays with half-life `story_half` content symbols; symbols below `story_skip` and the
    /// last symbol, "other", are not content). 0 = off. Installed as kept, for measuring it.
    pub story: usize,
    pub story_half: f64,
    pub story_skip: usize,
    /// Learned content: select story keys by burstiness, salience over the salience a key would have at its
    /// long-run rate, keeping those above 1 (so frequent words such as "." or "the" fall out without a
    /// hand-set cutoff), and weight rows by that ratio.
    pub story_burst: bool,
    /// Story rows age (halve) every this many writes (0 = never), and write one row per update.
    pub story_age: u32,
    pub story_write1: bool,
    /// Kept multi-row edges whose rows are chosen from outside each tick (`Brain::set_ext`), e.g. the bits of a
    /// meaning story code or a working-memory code; never pruned, their gains logged. Written one row per tick.
    pub ext_edges: Vec<String>,
    /// External edges write every row of their set each tick instead of one drawn by weight.
    pub ext_write_all: bool,
    /// Error-modulated writes: 0 = off (counting writes), 1 = gated by the level's miss (1 - p(y))^alpha,
    /// 2 = gated by each edge's own miss; floor `err_floor`; `err_forget` 0/1/2 (see `Edge::forget`).
    pub err_write: u8,
    pub err_alpha: f64,
    pub err_floor: f64,
    pub err_forget: u8,
    /// The bias edge's own mask bits and aging (0 = the same as every edge), to set its memory apart.
    pub bias_mask_bits: usize,
    pub bias_age: Option<u32>,
    /// In a stack, the units handed up are cut where surprise exceeds this many sds (0 = the chunk node's
    /// own cut): larger values hand up longer units than the chunk node uses as context.
    pub hand_up_z: f64,
    /// In a stack, cut handed-up units at local peaks of the prediction's entropy (branching entropy) that
    /// stand this many sds above its running mean, instead of at surprising inputs.
    pub hand_up_entropy: Option<f64>,
    /// With `vocab_halflife` > 0: never reassign a slot once all are taken.
    pub vocab_fixed: bool,
    /// In a stack, hand up each unit as a fusion of its parts (each part's input-cell code rotated by its
    /// role, roles counted from the unit's end, ORed) into the level above's matcher pool, instead of a slot.
    pub fuse: bool,
    /// Most parts fused per unit (roles 0..fuse_parts-1 from the end).
    pub fuse_parts: usize,
    /// Also fuse relational roles: the part before the unit, and the states of the level's kept holds.
    pub fuse_rel: bool,
    /// Mask bits of a fused level's input cells, and the share of them an input must overlap to match a
    /// cell instead of recruiting one (the similarity radius).
    pub fuse_k: u32,
    pub fuse_bar: f64,
    /// Force real (matcher) input pools even when --symbolic is set (fused levels need them).
    pub force_real: bool,
}

impl Default for Cfg {
    fn default() -> Self {
        Cfg {
            seed: 0,
            real: true,
            grow: true,
            oracle: 0,
            noise_drop: 0.0,
            noise_spur: 0,
            merge: 0,
            burn: 4000,
            evalw: 8000,
            acc_gain: 0.01,
            acc_z: 2.5,
            prune_gain: 0.003,
            prune_every: 10000,
            screen_thr: 0.02,
            syn_thr: 0.08,
            tmp_thr: 0.02,
            w: 30000,
            max_trials: 3,
            max_derived: 24,
            distill_gain: 0.05,
            eps: 0.002,
            floor: false,
            prune_guard: false,
            prune_harmful: false,
            map_cap: 0,
            map_two: 0,
            copy_act: false,
            bind_cap: 1024,
            hebb: 1.0,
            drop_lowest: true,
            words: false,
            chunk_cap: 4096,
            run_window: 0,
            classes: false,
            conjoin_words: false,
            long_trials: 1,
            long_trial_states: 4096,
            grow_pools: false,
            max_pool: 1 << 17,
            pool_budget: 1 << 20,
            young_age: 50_000,
            row_template: false,
            sparse_rows: false,
            age_every: 0,
            bias_edge: false,
            confidence: false,
            mask_bits: 640,
            write_bits: 8,
            code_bits: bits::CK,
            bitwise_levels: false,
            screen_mix: false,
            mix_thr: 0.005,
            fresh_extensions: false,
            vocab_halflife: 0,
            recency: vec![],
            recency_kept: false,
            story: 0,
            story_half: 200.0,
            story_skip: 100,
            story_burst: false,
            story_age: 0,
            story_write1: false,
            ext_edges: vec![],
            ext_write_all: false,
            err_write: 0,
            err_alpha: 1.0,
            err_floor: 0.05,
            err_forget: 0,
            bias_mask_bits: 0,
            bias_age: None,
            hand_up_z: 0.0,
            hand_up_entropy: None,
            vocab_fixed: false,
            fuse: false,
            fuse_parts: 8,
            fuse_rel: false,
            fuse_k: 24,
            fuse_bar: 10.0 / 24.0,
            force_real: false,
        }
    }
}

type PairKey = (i32, i32, u32, u32);

pub struct BindReal {
    pub pool: Pool,
    cache: HashMap<PairKey, usize>,
    code_key: Vec<Option<PairKey>>,
}

impl BindReal {
    fn new(cap: usize, hebb: f64) -> Self {
        let mut pool = Pool::new(cap, 48, 40);
        pool.hebb = hebb;
        BindReal {
            pool,
            cache: HashMap::new(),
            code_key: vec![None; cap],
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn step(
        &mut self,
        sx: i32,
        sy: i32,
        gx: u32,
        gy: u32,
        cx: &Code,
        cy: &Code,
        t: usize,
        rng: &mut Rng,
    ) -> i32 {
        if sx < 0 || sy < 0 {
            return -1;
        }
        let key = (sx, sy, gx, gy);
        if let Some(&c) = self.cache.get(&key) {
            if self.pool.alive[c] && self.code_key[c] == Some(key) {
                // the cell still holds this exact pair: a full match, as match_input would find
                self.pool.touch(c, t as u64);
                return c as i32;
            }
        }
        let mut inp: Bits = bits::from_positions(cx);
        for &b in rotate(cy, 977).iter() {
            bits::set(&mut inp, b as usize);
        }
        let idx = bits::positions(&inp);
        let m = self.pool.match_input(&idx, &inp, t as u64, rng);
        if m.recruited || bits::and_count(&self.pool.masks[m.cell], &inp) >= self.pool.k {
            if let Some(old) = self.code_key[m.cell] {
                self.cache.remove(&old);
            }
            self.cache.insert(key, m.cell);
            self.code_key[m.cell] = Some(key);
        }
        m.cell as i32
    }
}

pub struct BindSym {
    map: HashMap<(i32, i32), usize>,
    pub codes: Vec<Code>,
    rng: Rng,
}

impl BindSym {
    fn id(&mut self, a: i32, b: i32) -> i32 {
        if a < 0 || b < 0 {
            return -1;
        }
        if let Some(&c) = self.map.get(&(a, b)) {
            return c as i32;
        }
        let c = self.map.len();
        self.map.insert((a, b), c);
        self.codes.push(bits::rand_code(&mut self.rng));
        c as i32
    }
}

pub struct Distill {
    pub edge: usize,
    pub k: usize,
    pub codes: Vec<Code>,
    assign: HashMap<u32, usize>,
    cent: Option<Vec<Vec<f64>>>,
}

/// A recurrent bind reset at surprising bytes: its state is the run of bytes since the last cut.
pub struct ChunkReal {
    pub pool: Pool,
    start: Code,
    cache: HashMap<(i32, u32, i32, u32), usize>,
    code_key: Vec<Option<(i32, u32, i32, u32)>>,
}

impl ChunkReal {
    #[allow(clippy::too_many_arguments)]
    fn step(
        &mut self,
        prev: i32,
        b: i32,
        bgen: u32,
        bcode: &Code,
        reset: bool,
        t: usize,
        rng: &mut Rng,
    ) -> i32 {
        if b < 0 {
            return -1;
        }
        let (key, pc) = if reset || prev < 0 {
            ((-1, 0, b, bgen), self.start)
        } else {
            (
                (prev, self.pool.gen[prev as usize], b, bgen),
                self.pool.codes[prev as usize],
            )
        };
        if let Some(&c) = self.cache.get(&key) {
            if self.pool.alive[c] && self.code_key[c] == Some(key) {
                self.pool.touch(c, t as u64);
                return c as i32;
            }
        }
        let mut inp: Bits = bits::from_positions(bcode);
        for &x in rotate(&pc, 333).iter() {
            bits::set(&mut inp, x as usize);
        }
        let idx = bits::positions(&inp);
        let m = self.pool.match_input(&idx, &inp, t as u64, rng);
        if m.recruited || bits::and_count(&self.pool.masks[m.cell], &inp) >= self.pool.k {
            if let Some(old) = self.code_key[m.cell] {
                self.cache.remove(&old);
            }
            self.cache.insert(key, m.cell);
            self.code_key[m.cell] = Some(key);
        }
        m.cell as i32
    }
}

/// Exact chunk: a trie over (previous chunk or start, byte).
pub struct ChunkSym {
    map: HashMap<(i32, i32), usize>,
    pub codes: Vec<Code>,
    rng: Rng,
}

impl ChunkSym {
    fn step(&mut self, prev: i32, b: i32, reset: bool) -> i32 {
        if b < 0 {
            return -1;
        }
        let k = (if reset || prev < 0 { -1 } else { prev }, b);
        if let Some(&c) = self.map.get(&k) {
            return c as i32;
        }
        let c = self.map.len();
        self.map.insert(k, c);
        self.codes.push(bits::rand_code(&mut self.rng));
        c as i32
    }
}

/// Compress for chunks: completed chunks grouped by what follows them (the next chunk's first two bytes).
/// State = the class of the chunk completed at the last cut; class k means rare or not yet fitted.
pub struct ChunkClass {
    pub k: usize,
    pub codes: Vec<Code>,
    /// Per completed chunk value: counts of next first byte (0..256) and second byte (256..512).
    succ: HashMap<i32, HashMap<u16, u32>>,
    seen: HashMap<i32, u32>,
    assign: HashMap<i32, usize>,
    cent: Option<Vec<Vec<f64>>>,
    /// The completed chunk still waiting for its second following byte.
    pending2: i32,
    last: i32,
    /// The text of each completed chunk value, as last seen (for reports).
    texts: HashMap<i32, Vec<u8>>,
    last_cut: usize,
}

impl ChunkClass {
    fn profile(&self, c: i32) -> Option<Vec<f64>> {
        let m = self.succ.get(&c)?;
        let mut v = vec![0.0; 512];
        for (&i, &n) in m {
            v[i as usize] = n as f64;
        }
        let nrm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        if nrm == 0.0 {
            return None;
        }
        v.iter_mut().for_each(|x| *x /= nrm);
        Some(v)
    }

    fn nearest(cent: &[Vec<f64>], v: &[f64]) -> usize {
        let mut best = (f64::NEG_INFINITY, 0);
        for (j, c) in cent.iter().enumerate() {
            let x: f64 = c.iter().zip(v).map(|(a, b)| a * b).sum();
            if x > best.0 {
                best = (x, j);
            }
        }
        best.1
    }

    /// k-means over the most frequent completed chunks (warm-started, so class ids stay stable).
    fn refit(&mut self, rng: &mut Rng) {
        let mut vals: Vec<(u32, i32)> = self
            .seen
            .iter()
            .filter(|(_, &n)| n >= 5)
            .map(|(&c, &n)| (n, c))
            .collect();
        vals.sort_unstable_by(|a, b| b.cmp(a));
        vals.truncate(6000);
        let mut cells = vec![];
        let mut p = vec![];
        for &(_, c) in &vals {
            if let Some(v) = self.profile(c) {
                cells.push(c);
                p.push(v);
            }
        }
        if p.len() < self.k {
            return;
        }
        if let Some(mut cent) = self.cent.take() {
            // stable ids: members keep their class; centroids follow their members; new chunks join the nearest
            for (j, c) in cent.iter_mut().enumerate() {
                let mut m = vec![0.0; 512];
                let mut n = 0;
                for (cell, v) in cells.iter().zip(&p) {
                    if self.assign.get(cell) == Some(&j) {
                        n += 1;
                        m.iter_mut().zip(v).for_each(|(x, y)| *x += y);
                    }
                }
                if n > 0 {
                    let nrm = m.iter().map(|x| x * x).sum::<f64>().sqrt() + 1e-12;
                    *c = m.into_iter().map(|x| x / nrm).collect();
                }
            }
            for (cell, v) in cells.iter().zip(&p) {
                if !self.assign.contains_key(cell) {
                    self.assign.insert(*cell, Self::nearest(&cent, v));
                }
            }
            self.cent = Some(cent);
            return;
        }
        let mut cent: Vec<Vec<f64>> = rng
            .choose_distinct(p.len(), self.k)
            .into_iter()
            .map(|i| p[i].clone())
            .collect();
        for _ in 0..8 {
            let a: Vec<usize> = p.iter().map(|v| Self::nearest(&cent, v)).collect();
            for (j, c) in cent.iter_mut().enumerate() {
                let mut m = vec![0.0; 512];
                let mut n = 0;
                for (v, &aj) in p.iter().zip(&a) {
                    if aj == j {
                        n += 1;
                        m.iter_mut().zip(v).for_each(|(x, y)| *x += y);
                    }
                }
                if n > 0 {
                    let nrm = m.iter().map(|x| x * x).sum::<f64>().sqrt() + 1e-12;
                    *c = m.into_iter().map(|x| x / nrm).collect();
                }
            }
        }
        self.assign = cells
            .iter()
            .zip(&p)
            .map(|(&c, v)| (c, Self::nearest(&cent, v)))
            .collect();
        self.cent = Some(cent);
    }

    fn class(&self, c: i32) -> i32 {
        if c < 0 {
            return -1;
        }
        *self.assign.get(&c).unwrap_or(&self.k) as i32
    }

    /// Tick t: byte y; `done` = the chunk completed at a cut at t, if any.
    fn observe(&mut self, y: i32, done: Option<i32>) {
        if self.pending2 >= 0 && y >= 0 {
            *self
                .succ
                .entry(self.pending2)
                .or_default()
                .entry(256 + (y as u16).min(255))
                .or_insert(0) += 1;
        }
        self.pending2 = -1;
        if let Some(c) = done {
            if c >= 0 && y >= 0 {
                *self
                    .succ
                    .entry(c)
                    .or_default()
                    .entry((y as u16).min(255))
                    .or_insert(0) += 1;
                *self.seen.entry(c).or_insert(0) += 1;
                self.pending2 = c;
            }
            self.last = c;
        }
    }
}

/// Top-down input from the level above: its predicted next symbol, as a state of this level.
pub struct UpperNode {
    pub codes: Vec<Code>,
    rng: Rng,
}

pub enum Kind {
    Byte,
    /// A source that is always in state 0: its one row learns the overall symbol frequencies (order 0).
    Const(Code),
    Upper(Box<UpperNode>),
    /// parts[0] = the chunk.
    PrevClass(Box<ChunkClass>, f64),
    ChunkReal(Box<ChunkReal>, f64),
    ChunkSym(Box<ChunkSym>, f64),
    /// The chunk completed at the last cut (parts[0] = the chunk).
    PrevWord(f64),
    Delay {
        d: usize,
    },
    Hold {
        v: i32,
    },
    BindReal(Box<BindReal>),
    BindSym(Box<BindSym>),
    Distill(Box<Distill>),
    /// The position sense: where the current byte sits in its file (the reader is told where files start, as a
    /// reader of files knows when one is opened). m = 0: the offset from the file's start (0..31 exactly, then
    /// one state per power of two); m > 0: the offset modulo m (alignment). Codes are drawn per state on first use.
    Pos(Box<UpperNode>, usize),
    Taken,
}

impl Kind {
    pub fn label(&self) -> &'static str {
        match self {
            Kind::Byte => "byte",
            Kind::Delay { .. } => "delay",
            Kind::Hold { .. } => "hold",
            Kind::BindReal(_) | Kind::BindSym(_) => "bind",
            Kind::Distill(_) => "distill",
            Kind::ChunkReal(..) | Kind::ChunkSym(..) => "chunk",
            Kind::PrevWord(_) => "hold",
            Kind::PrevClass(..) => "distill",
            Kind::Upper(_) => "upper",
            Kind::Pos(..) => "pos",
            Kind::Const(_) => "const",
            Kind::Taken => "?",
        }
    }
}

/// Ticks of history each node keeps. Screens, replays and delays look back at most this far.
pub const RING: usize = 1 << 20;

/// A node's state history as a ring buffer: `h[t]` is the state at tick t for the last RING ticks.
#[derive(Clone)]
pub struct Hist {
    buf: Vec<i32>,
}

impl Hist {
    pub fn new() -> Self {
        Hist {
            buf: vec![-1; RING],
        }
    }

    /// An empty history for a removed node (its frozen state is kept on the node).
    pub fn freed() -> Self {
        Hist { buf: Vec::new() }
    }

    /// States for ticks lo..=hi.
    pub fn range(&self, lo: usize, hi: usize) -> Vec<i32> {
        assert!(hi + 1 - lo <= RING, "history window longer than the ring");
        (lo..=hi).map(|t| self.buf[t & (RING - 1)]).collect()
    }
}

impl Default for Hist {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Index<usize> for Hist {
    type Output = i32;
    fn index(&self, t: usize) -> &i32 {
        &self.buf[t & (RING - 1)]
    }
}

impl std::ops::IndexMut<usize> for Hist {
    fn index_mut(&mut self, t: usize) -> &mut i32 {
        &mut self.buf[t & (RING - 1)]
    }
}

pub struct Node {
    pub name: String,
    pub kind: Kind,
    pub parts: Vec<usize>,
    pub born: usize,
    pub extra: String,
    pub h: Hist,
    pub state: i32,
    pub removed: bool,
}

pub struct ByteNode {
    pub pool: Pool,
    pub bytecode: Vec<Code>,
    pub sym: HashMap<u32, usize>,
}

/// One event in the growth log: [t, kind, a, b?, value?].
pub struct Event {
    pub t: usize,
    pub kind: String,
    pub a: String,
    pub b: Option<String>,
    pub v: Option<f64>,
}

pub struct Brain {
    /// The byte text (level 0 only; empty above), for reports and generation.
    pub data: Vec<u8>,
    /// The input stream: bytes at level 0, symbol ids above. `vocab` symbols in all.
    pub syms: Vec<u32>,
    pub vocab: usize,
    /// Level 0 of text: the input symbols are bytes (a symbol stream may also have 256 symbols).
    pub bytes: bool,
    /// Top-down input: the level above's predicted next symbol, as its input cell (-1 if none).
    pub upper_state: i32,
    /// The position sense: start offsets of the files in the stream (empty: no position nodes).
    pub file_starts: Vec<usize>,
    pub upper: Option<usize>,
    /// After cleanup removes the upper node, it is offered again only from this tick on.
    upper_retry: usize,
    /// Recency candidates: (window W, live node, offered again from this tick on).
    recency: Vec<(usize, Option<usize>, usize)>,
    /// Story edge (slab index), salience per symbol (scaled by `story_g`), and the scale.
    story_edge: Option<usize>,
    story_sal: std::collections::BTreeMap<u32, f64>, // ordered, so runs are reproducible
    story_g: f64,
    story_cnt: HashMap<u32, f64, crate::info::FxBuild>,
    story_tot: f64,
    /// Story keys per tick (e.g. full word ids, so words outside the vocabulary are keys too); content keys
    /// are those >= `story_skip`. Without them the key is the symbol, and "other" is the last symbol.
    pub story_keys: Option<Vec<u32>>,
    /// Slab indices of the external multi-row edges, in `Cfg::ext_edges` order.
    pub ext: Vec<usize>,
    /// Copy channels (experiment 31): sets of (symbol, score) supplied from outside each tick (e.g. a
    /// hippocampal store's recall), added to the logits with a gain that depends on the previous input cell
    /// (base gain + a learned term per previous cell: a thalamus-like, context-dependent gain).
    copy_sets: Vec<Vec<(u32, f64)>>,
    copy_b0: Vec<f64>,
    copy_w: Vec<HashMap<i32, f64>>,
    copy_v: Vec<f64>,
    copy_pos: Vec<Vec<(usize, f64)>>,
    copy_prev: i32,
    pub copy_lr: f64,
    /// Copy channels use one gain for every context (no context-dependent term).
    pub copy_nocontext: bool,
    /// A context key for the copy gains supplied from outside (instead of the previous input cell).
    pub copy_ctx_ext: Option<i32>,
    pub t: usize,
    pub tt: usize,
    pub cfg: Cfg,
    rng: Rng,
    pub byte: ByteNode,
    pub nodes: Vec<Node>,
    by_name: HashMap<String, usize>,
    pub derived: Vec<usize>,
    slab: Vec<Edge>,
    /// Kept edges (slab ids); the first is the byte node's own edge.
    pub kept: Vec<usize>,
    pub trials: Vec<usize>,
    trial_t0: HashMap<usize, usize>,
    /// When an extended trial's current judging window began (fresh extensions).
    judge_t0: HashMap<usize, usize>,
    backoff: HashMap<String, (usize, usize)>,
    priority: HashSet<String>,
    scr: HashMap<String, f64>,
    kept_since: HashMap<String, usize>,
    pub gain_at_keep: HashMap<String, f64>,
    pub events: Vec<Event>,
    tried: HashSet<String>,
    pub bits: Vec<f64>,
    /// The part of `bits` paid for mapping the winner cell back to a byte.
    pub bits_map: Vec<f64>,
    /// --floor: the code length after the learned floor (what `bpb` reports when the floor is on).
    pub coded: Vec<f64>,
    floor_w: Vec<f64>,
    /// --copy-act state: last position after each 5-byte context (hash table), the pointer, the match length,
    /// per length bucket (right, offered) counts and the mixture weights; counts of offers and hits.
    copy_tab: Vec<u32>,
    copy_ptr: Option<usize>,
    copy_len: usize,
    copy_cal: Vec<(u32, u32)>,
    copy_mw: Vec<[f64; 3]>,
    pub copy_stats: [u64; 3],
    copy_cap: u32,
    copy_hist: u32,
    copy_ctx: bool,
    copy_keep: bool,
    copy_miss: u32,
    /// --prune-guard: prune judgments skipped over noise windows.
    pub guarded: usize,
    pub novel: Vec<bool>,
    pub hit: Vec<bool>,
    cellbyte: Vec<Vec<u32>>,
    cellbyte_n: Vec<u32>,
    /// --map-two N: a second, fast map per cell (counts halved past N) beside the slow one, and a switching
    /// mixture over the two (fixed share), so a change of material is followed without forgetting what holds.
    cellfast: Vec<Vec<u32>>,
    cellfast_n: Vec<u32>,
    map_w: [f64; 2],
    tix: TargetIndex,
    alive_idx: Vec<usize>,
    pos: Vec<i64>,
    alive_dirty: bool,
    pub merged: usize,
    /// Seconds spent in: [tick body, temporal, synergy, screen, other controller work].
    /// Seconds spent per stage: 1 temporal, 2 synergy, 3 screen, 4 check, 5 scoring edges, 6 trial and
    /// kept-edge gain accounting, 7 row writes, 8 derived-node updates, 9 running screens, 10 input matching,
    /// 11 the whole tick.
    pub prof: [f64; 12],
    /// Running mean and sd of the byte's bits, as seen before tick t (the surprise statistic).
    pub mu: Vec<f64>,
    pub sd: Vec<f64>,
    /// Entropy (bits) of the level's prediction at each tick, before the input is seen, and its running
    /// mean and variance (for cutting units at uncertainty peaks).
    pub ent: Vec<f32>,
    ent_m: f64,
    ent_v: f64,
    sm: f64,
    sv: f64,
    pub chunk: Option<usize>,
    pub prev: Option<usize>,
    pub pclass: Option<usize>,
    /// Extra top-level entries for the summary JSON (key, raw JSON).
    pub extra_json: Vec<(String, String)>,
    /// Running count screens for the word moves (when cfg.run_window > 0).
    pub run_scr: Vec<RunScreen>,
    /// No learning or growth (generation mode).
    pub frozen: bool,
    /// A fused input to observe on the next tick instead of a symbol's code.
    pending: Option<Vec<u16>>,
}

/// Only the log-probability of `yi`, computed exactly as `log_softmax_at` does, without the probability vector.
fn log_softmax_lp(logits: &[f64], yi: usize) -> f64 {
    let m = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let s: f64 = logits.iter().map(|&x| (x - m).exp()).sum();
    logits[yi] - m - s.ln()
}

fn log_softmax_at(logits: &[f64], yi: usize) -> (Vec<f64>, f64) {
    let m = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let z: Vec<f64> = logits.iter().map(|&x| (x - m).exp()).collect();
    let s: f64 = z.iter().sum();
    let p = z.iter().map(|&x| x / s).collect();
    (p, logits[yi] - m - s.ln())
}

fn json_str(s: &str) -> String {
    let mut o = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn json_f(x: f64) -> String {
    if x.is_finite() {
        format!("{x}")
    } else {
        "null".into()
    }
}

impl Brain {
    pub fn new(data: Vec<u8>, cfg: Cfg) -> Self {
        let syms = data.iter().map(|&b| b as u32).collect();
        Self::build(data, syms, 256, true, cfg)
    }

    /// A brain over a stream of symbols 0..vocab that will be fed one at a time (see `push`).
    /// A brain over a whole symbol stream given up front (like `new` for bytes).
    pub fn from_syms(syms: Vec<u32>, vocab: usize, cfg: Cfg) -> Self {
        Self::build(vec![], syms, vocab, false, cfg)
    }

    pub fn new_symbols(vocab: usize, cfg: Cfg) -> Self {
        Self::build(vec![], vec![], vocab, false, cfg)
    }

    fn build(data: Vec<u8>, syms: Vec<u32>, vocab: usize, bytes: bool, cfg: Cfg) -> Self {
        let tt = syms.len();
        let mut rng = Rng::new(cfg.seed);
        let mut pool = match (cfg.real, bytes) {
            (true, true) => Pool::new(192, 24, 10),
            (true, false) => {
                let k = cfg.fuse_k;
                let bar = (cfg.fuse_bar * k as f64).round() as u32;
                Pool::new(vocab + 64, k, bar.clamp(1, k))
            }
            (false, _) => Pool::new(vocab, 0, 0),
        };
        pool.hebb = cfg.hebb;
        let bytecode = (0..vocab).map(|_| bits::rand_code(&mut rng)).collect();
        let byte = ByteNode {
            pool,
            bytecode,
            sym: HashMap::new(),
        };
        let cap = byte.pool.cap;
        let n0 = Node {
            name: "byte".into(),
            kind: Kind::Byte,
            parts: vec![],
            born: 0,
            extra: String::new(),
            h: Hist::new(),
            state: -1,
            removed: false,
        };
        let mut b = Brain {
            data,
            syms,
            vocab,
            bytes,
            upper_state: -1,
            file_starts: vec![],
            upper: None,
            upper_retry: 0,
            recency: cfg.recency.iter().map(|&w| (w, None, 0)).collect(),
            story_edge: None,
            story_sal: std::collections::BTreeMap::new(),
            story_g: 1.0,
            story_cnt: HashMap::default(),
            story_tot: 0.0,
            story_keys: None,
            ext: vec![],
            copy_sets: vec![],
            copy_b0: vec![],
            copy_w: vec![],
            copy_v: vec![],
            copy_pos: vec![],
            copy_prev: -1,
            copy_lr: 0.01,
            copy_nocontext: false,
            copy_ctx_ext: None,
            t: 0,
            tt,
            cfg: cfg.clone(),
            rng,
            byte,
            nodes: vec![n0],
            by_name: HashMap::from([("byte".to_string(), 0)]),
            derived: vec![],
            slab: vec![Edge::new(0, 1.0)],
            kept: vec![0],
            trials: vec![],
            trial_t0: HashMap::new(),
            judge_t0: HashMap::new(),
            backoff: HashMap::new(),
            priority: HashSet::new(),
            scr: HashMap::new(),
            kept_since: HashMap::new(),
            gain_at_keep: HashMap::new(),
            events: vec![],
            tried: HashSet::new(),
            bits: vec![0.0; tt],
            bits_map: vec![0.0; tt],
            coded: vec![0.0; tt],
            floor_w: vec![],
            guarded: 0,
            copy_tab: vec![],
            copy_ptr: None,
            copy_len: 0,
            copy_cal: vec![],
            copy_mw: vec![],
            copy_stats: [0; 3],
            copy_cap: std::env::var("COPY_CAP").ok().and_then(|x| x.parse().ok()).unwrap_or(4096),
            copy_hist: 0,
            // the act's rule: read on past a wrong byte, and trust by length and recent record (COPY_PLAIN: neither,
            // the first version)
            copy_ctx: std::env::var("COPY_PLAIN").is_err(),
            copy_keep: std::env::var("COPY_PLAIN").is_err(),
            copy_miss: 0,
            novel: vec![false; tt],
            hit: vec![false; tt],
            cellbyte: vec![vec![0; vocab]; cap],
            cellbyte_n: vec![0; cap],
            cellfast: if cfg.map_two > 0 { vec![vec![0; vocab]; cap] } else { vec![] },
            cellfast_n: if cfg.map_two > 0 { vec![0; cap] } else { vec![] },
            map_w: [0.5, 0.5],
            tix: TargetIndex::new(),
            alive_idx: vec![],
            pos: vec![-1; cap],
            alive_dirty: true,
            merged: 0,
            prof: [0.0; 12],
            mu: vec![0.0; tt],
            ent: vec![0.0; tt],
            ent_m: 0.0,
            ent_v: 1.0,
            sd: vec![1.0; tt],
            sm: 4.0,
            sv: 1.0,
            chunk: None,
            prev: None,
            pclass: None,
            extra_json: vec![],
            run_scr: vec![],
            frozen: false,
            pending: None,
        };
        if cfg.bias_edge {
            let code = bits::rand_code(&mut b.rng);
            let nd = b.new_node("bias".into(), Kind::Const(code), vec![], String::new());
            let i = b.push_node(nd);
            let mut e = b.mk_edge(i, 1.0);
            if cfg.bias_mask_bits > 0 {
                e.kw = cfg.bias_mask_bits;
            }
            if let Some(a) = cfg.bias_age {
                e.age_every = a;
            }
            b.slab.push(e);
            b.kept.push(b.slab.len() - 1);
            b.kept_since.insert("bias".into(), 0);
        }
        if cfg.story > 0 {
            let code = bits::rand_code(&mut b.rng);
            let nd = b.new_node("story".into(), Kind::Const(code), vec![], String::new());
            let i = b.push_node(nd);
            let mut e = b.mk_edge(i, 0.0);
            e.multi = true;
            e.age_every = cfg.story_age;
            e.write_one = cfg.story_write1;
            b.slab.push(e);
            b.kept.push(b.slab.len() - 1);
            b.kept_since.insert("story".into(), 0);
            b.story_edge = Some(b.slab.len() - 1);
        }
        for name in cfg.ext_edges.clone() {
            let code = bits::rand_code(&mut b.rng);
            let nd = b.new_node(name.clone(), Kind::Const(code), vec![], String::new());
            let i = b.push_node(nd);
            let mut e = b.mk_edge(i, 0.0);
            e.multi = true;
            e.age_every = cfg.story_age;
            e.write_one = !cfg.ext_write_all;
            b.slab.push(e);
            b.kept.push(b.slab.len() - 1);
            b.kept_since.insert(name, 0);
            b.ext.push(b.slab.len() - 1);
        }
        if cfg.recency_kept {
            for r in 0..b.recency.len() {
                let w = b.recency[r].0;
                let code = bits::rand_code(&mut b.rng);
                let nd = b.new_node(
                    format!("recent({w})"),
                    Kind::Const(code),
                    vec![],
                    String::new(),
                );
                let i = b.push_node(nd);
                let mut e = b.mk_edge(i, 1.0);
                e.kw = (w * cfg.code_bits.clamp(1, bits::CK)).min(D);
                e.sparse = true;
                e.age_every = 0;
                b.slab.push(e);
                b.kept.push(b.slab.len() - 1);
                b.kept_since.insert(format!("recent({w})"), 0);
                b.recency[r].1 = Some(i);
                b.recency[r].2 = usize::MAX;
            }
        }
        b.slab[0].drop_lowest = cfg.drop_lowest;
        b.slab[0].template = cfg.row_template;
        b.slab[0].sparse = cfg.sparse_rows;
        b.slab[0].age_every = cfg.age_every;
        b.slab[0].confidence = cfg.confidence;
        b.slab[0].kw = cfg.mask_bits;
        b.slab[0].swaps = cfg.write_bits;
        b.slab[0].bitwise = cfg.bitwise_levels;
        b.tix.rk = cfg.code_bits.clamp(1, bits::CK);
        if cfg.oracle > 0 {
            b.oracle(cfg.oracle);
        }
        b
    }

    fn mk_edge(&self, src: usize, beta: f64) -> Edge {
        let mut e = Edge::new(src, beta);
        e.drop_lowest = self.cfg.drop_lowest;
        e.template = self.cfg.row_template;
        e.sparse = self.cfg.sparse_rows;
        e.age_every = self.cfg.age_every;
        e.confidence = self.cfg.confidence;
        e.kw = self.cfg.mask_bits;
        e.swaps = self.cfg.write_bits;
        e.bitwise = self.cfg.bitwise_levels;
        e
    }

    // ---------------------------------------------------------------- node helpers
    fn code_of(&self, n: usize, c: i32) -> Code {
        let c = c as usize;
        match &self.nodes[n].kind {
            Kind::Byte => self.byte.pool.codes[c],
            Kind::Delay { .. } | Kind::Hold { .. } => {
                self.code_of(self.nodes[n].parts[0], c as i32)
            }
            Kind::BindReal(b) => b.pool.codes[c],
            Kind::BindSym(b) => b.codes[c],
            Kind::Distill(d) => d.codes[c],
            Kind::ChunkReal(k, _) => k.pool.codes[c],
            Kind::ChunkSym(k, _) => k.codes[c],
            Kind::PrevWord(_) => self.code_of(self.nodes[n].parts[0], c as i32),
            Kind::PrevClass(k, _) => k.codes[c],
            Kind::Upper(u) => u.codes[c],
            Kind::Pos(u, _) => u.codes[c],
            Kind::Const(code) => *code,
            Kind::Taken => unreachable!(),
        }
    }

    fn gen_of(&self, n: usize, c: i32) -> u32 {
        if c < 0 {
            return 0;
        }
        let c = c as usize;
        match &self.nodes[n].kind {
            Kind::Byte => self.byte.pool.gen[c],
            Kind::Delay { .. } | Kind::Hold { .. } => self.gen_of(self.nodes[n].parts[0], c as i32),
            Kind::BindReal(b) => b.pool.gen[c],
            Kind::BindSym(_)
            | Kind::Distill(_)
            | Kind::ChunkSym(..)
            | Kind::PrevClass(..)
            | Kind::Upper(_)
            | Kind::Pos(..)
            | Kind::Const(_) => 1,
            Kind::ChunkReal(k, _) => k.pool.gen[c],
            Kind::PrevWord(_) => self.gen_of(self.nodes[n].parts[0], c as i32),
            Kind::Taken => unreachable!(),
        }
    }

    fn live(&self, name: &str) -> Option<usize> {
        self.by_name.get(name).copied()
    }

    fn order(&self) -> Vec<usize> {
        let mut v = vec![0];
        v.extend(self.derived.iter().copied());
        v
    }

    fn push_node(&mut self, nd: Node) -> usize {
        let i = self.nodes.len();
        self.by_name.insert(nd.name.clone(), i);
        self.nodes.push(nd);
        self.derived.push(i);
        i
    }

    fn add(&mut self, nd: Node, why: String) -> usize {
        let kind = nd.kind.label().to_string();
        let name = nd.name.clone();
        let i = self.push_node(nd);
        self.events.push(Event {
            t: self.t,
            kind,
            a: name.clone(),
            b: Some(why),
            v: None,
        });
        self.priority.insert(name);
        i
    }

    fn new_node(&self, name: String, kind: Kind, parts: Vec<usize>, extra: String) -> Node {
        Node {
            name,
            kind,
            parts,
            born: self.t,
            extra,
            h: Hist::new(),
            state: -1,
            removed: false,
        }
    }

    fn make_delay(&self, d: usize, t: usize) -> Node {
        let mut nd = self.new_node(
            format!("byte@{d}"),
            Kind::Delay { d },
            vec![0],
            d.to_string(),
        );
        nd.born = t;
        if t + 2 > d {
            let n = t + 2 - d;
            let lo = (d - 1).max((t + 1).saturating_sub(RING - 16));
            for tt in lo..d - 1 + n {
                nd.h[tt] = self.nodes[0].h[tt + 1 - d];
            }
        }
        nd.state = nd.h[t];
        nd
    }

    fn make_hold(&self, x: usize, v: i32, t: usize) -> Node {
        let lab = self.label(v);
        let mut nd = self.new_node(
            format!("hold({}|{})", self.nodes[x].name, lab),
            Kind::Hold { v },
            vec![x, 0],
            lab,
        );
        let mut cur = -1;
        for tt in (t + 1).saturating_sub(RING)..=t {
            if self.nodes[0].h[tt] == v {
                cur = self.nodes[x].h[tt];
            }
            nd.h[tt] = cur;
        }
        nd.state = nd.h[t];
        nd
    }

    fn make_bind(&mut self, x: usize, y: usize, replay: usize) -> Node {
        let t = self.t;
        let name = format!("({}*{})", self.nodes[x].name, self.nodes[y].name);
        if self.cfg.real {
            let distinct: HashSet<(i32, i32)> = ((t + 1).saturating_sub(replay.max(1))..=t)
                .map(|tt| (self.nodes[x].h[tt], self.nodes[y].h[tt]))
                .filter(|&(a, b)| a >= 0 && b >= 0)
                .collect();
            let cap = self.sized_cap(self.cfg.bind_cap, distinct.len());
            let mut b = BindReal::new(cap, self.cfg.hebb);
            let mut h = Hist::new();
            let lo = (t + 1).saturating_sub(replay);
            if replay > 0 {
                for tt in lo..=t {
                    let (sx, sy) = (self.nodes[x].h[tt], self.nodes[y].h[tt]);
                    if sx < 0 || sy < 0 {
                        continue;
                    }
                    let (gx, gy) = (self.gen_of(x, sx), self.gen_of(y, sy));
                    let (cx, cy) = (self.code_of(x, sx), self.code_of(y, sy));
                    h[tt] = b.step(sx, sy, gx, gy, &cx, &cy, tt, &mut self.rng);
                }
            }
            let mut nd =
                self.new_node(name, Kind::BindReal(Box::new(b)), vec![x, y], String::new());
            nd.state = h[t];
            nd.h = h;
            nd
        } else {
            let seed = self.rng.next_u64();
            let mut b = BindSym {
                map: HashMap::new(),
                codes: vec![],
                rng: Rng::new(seed),
            };
            let mut h = Hist::new();
            let lo = (t + 1).saturating_sub(40000);
            if replay > 0 {
                for tt in lo..=t {
                    h[tt] = b.id(self.nodes[x].h[tt], self.nodes[y].h[tt]);
                }
            }
            let mut nd = self.new_node(name, Kind::BindSym(Box::new(b)), vec![x, y], String::new());
            nd.state = h[t];
            nd.h = h;
            nd
        }
    }

    fn bind(&mut self, x: usize, y: usize, why: String) {
        let (a, b) = (&self.nodes[x].name, &self.nodes[y].name);
        if self.live(&format!("({a}*{b})")).is_some() || self.live(&format!("({b}*{a})")).is_some()
        {
            return;
        }
        let nd = self.make_bind(x, y, 20000);
        self.add(nd, why);
    }

    fn oracle(&mut self, order: usize) {
        let mut taps = HashMap::new();
        for k in 2..=order {
            let nd = self.make_delay(k, 0);
            taps.insert(k, self.push_node(nd));
        }
        let mut prev = 0;
        for k in 2..=order {
            let nd = self.make_bind(prev, taps[&k], 0);
            let name = nd.name.clone();
            let i = self.push_node(nd);
            self.slab.push(self.mk_edge(i, 0.0));
            self.kept.push(self.slab.len() - 1);
            self.kept_since.insert(name, 0);
            prev = i;
        }
    }

    fn label(&self, cell: i32) -> String {
        if cell < 0 || self.cellbyte_n[cell as usize] == 0 {
            return format!("#{cell}");
        }
        let cb = &self.cellbyte[cell as usize];
        let mut b = 0;
        for i in 0..self.vocab {
            if cb[i] > cb[b] {
                b = i;
            }
        }
        if !self.bytes {
            return format!("#s{b}");
        }
        match b as u8 {
            b' ' => "' '".into(),
            b'\n' => "'\\n'".into(),
            b'\'' => "\"'\"".into(),
            c if (0x21..0x7f).contains(&c) => format!("'{}'", c as char),
            c => format!("'\\x{c:02x}'"),
        }
    }

    // ---------------------------------------------------------------- distill
    fn profile(&mut self, eid: usize, c: u32) -> Option<Vec<f64>> {
        let src = self.slab[eid].src;
        let g = self.gen_of(src, c as i32);
        match self.slab[eid].rows.get(&c) {
            Some(r) if r.gen == g => {}
            _ => return None,
        }
        let r = self.slab[eid].row(
            c,
            g,
            &self.tix,
            &self.byte.pool.codes,
            &self.byte.pool.alive,
            &mut self.rng,
        );
        let mut v: Vec<f64> = r.counts.iter().map(|&x| x as f64).collect();
        for (j, a) in self.byte.pool.alive.iter().enumerate() {
            if !a {
                v[j] = 0.0;
            }
        }
        let n = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        if n > 0.0 {
            v.iter_mut().for_each(|x| *x /= n);
            Some(v)
        } else {
            None
        }
    }

    fn refit(&mut self, d: &mut Distill) {
        let mut cells: Vec<u32> = self.slab[d.edge].rows.keys().copied().collect();
        cells.sort_unstable();
        let mut keep_cells = vec![];
        let mut p = vec![];
        for &c in &cells {
            if let Some(v) = self.profile(d.edge, c) {
                keep_cells.push(c);
                p.push(v);
            }
        }
        if p.len() < d.k {
            d.assign = keep_cells
                .iter()
                .enumerate()
                .map(|(i, &c)| (c, i % d.k))
                .collect();
            return;
        }
        let dim = p[0].len();
        let mut cent = match d.cent.take() {
            Some(c) if c[0].len() == dim => c,
            _ => self
                .rng
                .choose_distinct(p.len(), d.k)
                .into_iter()
                .map(|i| p[i].clone())
                .collect(),
        };
        let nearest = |v: &Vec<f64>, cent: &Vec<Vec<f64>>| -> usize {
            let mut best = (f64::NEG_INFINITY, 0);
            for (j, c) in cent.iter().enumerate() {
                let s: f64 = v.iter().zip(c).map(|(a, b)| a * b).sum();
                if s > best.0 {
                    best = (s, j);
                }
            }
            best.1
        };
        for _ in 0..8 {
            let a: Vec<usize> = p.iter().map(|v| nearest(v, &cent)).collect();
            for (j, c) in cent.iter_mut().enumerate() {
                let mut m = vec![0.0; dim];
                let mut n = 0;
                for (v, &aj) in p.iter().zip(&a) {
                    if aj == j {
                        n += 1;
                        m.iter_mut().zip(v).for_each(|(x, y)| *x += y);
                    }
                }
                if n > 0 {
                    let norm = m.iter().map(|x| x * x).sum::<f64>().sqrt() + 1e-12;
                    *c = m.into_iter().map(|x| x / norm).collect();
                }
            }
        }
        d.assign = keep_cells
            .iter()
            .zip(&p)
            .map(|(&c, v)| (c, nearest(v, &cent)))
            .collect();
        d.cent = Some(cent);
    }

    fn cluster(&mut self, d: &mut Distill, s: i32) -> i32 {
        if s < 0 {
            return -1;
        }
        if let Some(&a) = d.assign.get(&(s as u32)) {
            return a as i32;
        }
        let v = match self.profile(d.edge, s as u32) {
            Some(v) => v,
            None => return -1,
        };
        let cent = match &d.cent {
            Some(c) => c,
            None => return -1,
        };
        let mut best = (f64::NEG_INFINITY, 0);
        for (j, c) in cent.iter().enumerate() {
            let x: f64 = v.iter().zip(c).map(|(a, b)| a * b).sum();
            if x > best.0 {
                best = (x, j);
            }
        }
        d.assign.insert(s as u32, best.1);
        best.1 as i32
    }

    fn make_distill(&mut self, eid: usize, bind_name: &str) -> Node {
        let t = self.t;
        let k = 32;
        let codes = (0..k).map(|_| bits::rand_code(&mut self.rng)).collect();
        let mut d = Distill {
            edge: eid,
            k,
            codes,
            assign: HashMap::new(),
            cent: None,
        };
        self.refit(&mut d);
        let src = self.slab[eid].src;
        let mut h = Hist::new();
        let lo = (t + 1).saturating_sub(40000);
        let mut memo: HashMap<i32, i32> = HashMap::new();
        for tt in lo..=t {
            let s = self.nodes[src].h[tt];
            if s < 0 {
                continue;
            }
            let c = match memo.get(&s) {
                Some(&c) => c,
                None => {
                    let c = self.cluster(&mut d, s);
                    memo.insert(s, c);
                    c
                }
            };
            h[tt] = c;
        }
        let mut nd = self.new_node(
            format!("Z[{bind_name}]"),
            Kind::Distill(Box::new(d)),
            vec![src],
            k.to_string(),
        );
        nd.state = h[t];
        nd.h = h;
        nd
    }

    // ---------------------------------------------------------------- one tick
    fn refresh_alive(&mut self) {
        self.tix
            .refresh(&self.byte.pool.codes, &self.byte.pool.alive);
        self.alive_idx = (0..self.byte.pool.cap)
            .filter(|&c| self.byte.pool.alive[c])
            .collect();
        self.pos = vec![-1; self.byte.pool.cap];
        for (i, &c) in self.alive_idx.iter().enumerate() {
            self.pos[c] = i as i64;
        }
        self.alive_dirty = false;
    }

    fn observe(&mut self, byte: u32, t: usize) -> (i32, bool) {
        let r0 = self.byte.pool.recruits;
        let s = if let Some(fused) = self.pending.take() {
            let inp = bits::from_positions(&fused);
            let idx = bits::positions(&inp);
            self.byte
                .pool
                .match_input(&idx, &inp, t as u64, &mut self.rng)
                .cell as i32
        } else if self.cfg.real {
            let code = self.byte.bytecode[byte as usize];
            let mut inp: Bits = bits::ZERO;
            for &b in code.iter() {
                if self.cfg.noise_drop <= 0.0 || self.rng.f64() >= self.cfg.noise_drop {
                    bits::set(&mut inp, b as usize);
                }
            }
            for _ in 0..self.cfg.noise_spur {
                let b = self.rng.below(D);
                bits::set(&mut inp, b);
            }
            let idx = bits::positions(&inp);
            self.byte
                .pool
                .match_input(&idx, &inp, t as u64, &mut self.rng)
                .cell as i32
        } else {
            match self.byte.sym.get(&byte) {
                Some(&c) => c as i32,
                None => {
                    let c = self.byte.sym.len();
                    self.byte.sym.insert(byte, c);
                    self.byte.pool.new_code(c, &mut self.rng);
                    self.byte.pool.recruits += 1;
                    c as i32
                }
            }
        };
        self.nodes[0].state = s;
        self.nodes[0].h[t] = s;
        (s, self.byte.pool.recruits > r0)
    }

    fn score_edge(&mut self, eid: usize) -> bool {
        let src = self.slab[eid].src;
        let st = self.nodes[src].state;
        let g = self.gen_of(src, st);
        self.slab[eid].score(
            st,
            g,
            &self.alive_idx,
            &self.tix,
            &self.byte.pool.codes,
            &self.byte.pool.alive,
            &mut self.rng,
        );
        self.slab[eid].sc.is_some()
    }

    pub fn tick(&mut self) {
        let t = self.t;
        self.mu[t] = self.sm;
        self.sd[t] = self.sv.sqrt();
        let eps = self.cfg.eps;
        let mut byte = self.syms[t];
        let fused = self.pending.is_some();
        if self.alive_dirty {
            self.refresh_alive();
        }
        let tick0 = std::time::Instant::now();
        let na = self.alive_idx.len();
        let mut logits = vec![0.0; na];
        let kept = self.kept.clone();
        let trials = self.trials.clone();
        for &e in &kept {
            if self.score_edge(e) {
                self.slab[e].add_contribution(&mut logits, false);
            }
        }
        if let Some(k) = self.copy_ctx_ext {
            self.copy_prev = k;
        }
        for j in 0..self.copy_sets.len() {
            let gain =
                self.copy_b0[j] + self.copy_w[j].get(&self.copy_prev).copied().unwrap_or(0.0);
            let mut v = std::mem::take(&mut self.copy_pos[j]);
            v.clear();
            for &(sym, sc) in &self.copy_sets[j] {
                if let Some(&c) = self.byte.sym.get(&sym) {
                    if c < self.pos.len() && self.pos[c] >= 0 {
                        let pos = self.pos[c] as usize;
                        logits[pos] += gain * sc;
                        v.push((pos, sc));
                    }
                }
            }
            self.copy_pos[j] = v;
        }
        let mut tsc = vec![];
        for &e in &trials {
            if self.score_edge(e) {
                tsc.push(e);
            }
        }
        self.prof[5] += tick0.elapsed().as_secs_f64();
        if na > 0 {
            let m = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let z: f64 = logits.iter().map(|&x| (x - m).exp()).sum();
            let h: f64 = logits
                .iter()
                .map(|&x| {
                    let p = (x - m).exp() / z;
                    if p > 0.0 {
                        -p * p.log2()
                    } else {
                        0.0
                    }
                })
                .sum();
            self.ent[t] = h as f32;
            let d = h - self.ent_m;
            self.ent_m += 0.001 * d;
            self.ent_v = 0.999 * self.ent_v + 0.001 * d * d;
        }
        let c_obs = std::time::Instant::now();
        let (y, novel) = self.observe(byte, t);
        self.prof[10] += c_obs.elapsed().as_secs_f64();
        if fused {
            // a fused input has no symbol of its own: record the cell it matched as its symbol, and map
            // every cell to one dummy symbol (so the cell-to-symbol cost is about zero)
            self.syms[t] = y.max(0) as u32;
            byte = 0;
        }
        let yi: i64 = if novel { -1 } else { self.pos[y as usize] };
        let mut p = vec![];
        let q;
        if yi >= 0 && na > 0 {
            let (pp, lp) = log_softmax_at(&logits, yi as usize);
            p = pp;
            q = (1.0 - eps) * lp.exp() + eps / self.vocab as f64;
            let am = logits
                .iter()
                .enumerate()
                .fold(
                    (f64::NEG_INFINITY, 0),
                    |b, (i, &x)| if x > b.0 { (x, i) } else { b },
                )
                .1;
            self.hit[t] = am == yi as usize;
        } else {
            q = eps / self.vocab as f64;
        }
        let yu = y as usize;
        let mut pb = (self.cellbyte[yu][byte as usize] as f64 + 0.02)
            / (self.cellbyte_n[yu] as f64 + 0.02 * self.vocab as f64);
        if self.cfg.map_two > 0 {
            let pf = (self.cellfast[yu][byte as usize] as f64 + 0.02) / (self.cellfast_n[yu] as f64 + 0.02 * self.vocab as f64);
            let p = self.map_w[0] * pb + self.map_w[1] * pf;
            let (a, b) = (self.map_w[0] * pb / p, self.map_w[1] * pf / p);
            const SH: f64 = 1.0 / 1000.0;
            self.map_w = [(1.0 - SH) * a + SH * b, (1.0 - SH) * b + SH * a];
            pb = p;
            self.cellfast[yu][byte as usize] += 1;
            self.cellfast_n[yu] += 1;
            if self.cellfast_n[yu] > self.cfg.map_two {
                let row = &mut self.cellfast[yu];
                let mut n = 0;
                for x in row.iter_mut() {
                    *x >>= 1;
                    n += *x;
                }
                self.cellfast_n[yu] = n;
            }
        }
        self.cellbyte[yu][byte as usize] += 1;
        self.cellbyte_n[yu] += 1;
        if self.cfg.map_cap > 0 && self.cellbyte_n[yu] > self.cfg.map_cap {
            // --map-cap: halve the cell's byte counts together, so its map follows what the cell means now
            let row = &mut self.cellbyte[yu];
            let mut n = 0;
            for x in row.iter_mut() {
                *x >>= 1;
                n += *x;
            }
            self.cellbyte_n[yu] = n;
        }
        self.bits[t] = -q.log2() - pb.log2();
        self.bits_map[t] = -pb.log2();
        if self.cfg.copy_act && self.bytes {
            // the level's code after its floor is the first expert beside the act
            let fl = self.floor_code(self.bits[t]);
            self.coded[t] = self.copy_code(t, fl);
        } else if self.cfg.floor {
            self.coded[t] = self.floor_code(self.bits[t]);
        }
        self.novel[t] = novel;
        if yi >= 0 && na > 0 && !self.frozen {
            let yi = yi as usize;
            let c_tr = std::time::Instant::now();
            let base_loss = -q.log2();
            let mut lw: Vec<f64> = Vec::with_capacity(logits.len());
            for &e in &tsc {
                lw.clear();
                lw.extend_from_slice(&logits);
                self.slab[e].add_contribution(&mut lw, false);
                let (pw, lpw) = log_softmax_at(&lw, yi);
                let qw = (1.0 - eps) * lpw.exp() + eps / self.vocab as f64;
                let d = base_loss + qw.log2();
                if t - self.trial_t0[&e] >= self.cfg.burn {
                    let ed = &mut self.slab[e];
                    ed.sum += d;
                    ed.sq += d * d;
                    ed.n += 1;
                }
                self.slab[e].learn_beta(&pw, yi, 0.003);
            }
            for &e in &kept[1..] {
                if self.slab[e].sc.is_none() {
                    continue;
                }
                lw.clear();
                lw.extend_from_slice(&logits);
                self.slab[e].add_contribution(&mut lw, true);
                let lpo = log_softmax_lp(&lw, yi);
                let qo = (1.0 - eps) * lpo.exp() + eps / self.vocab as f64;
                let d = -qo.log2() - base_loss;
                let ed = &mut self.slab[e];
                ed.sum += d;
                ed.sq += d * d;
                ed.n += 1;
            }
            for j in 0..self.copy_pos.len() {
                // the gain's gradient: the channel's score for the true word minus its expected score
                let sy: f64 = self.copy_pos[j]
                    .iter()
                    .filter(|x| x.0 == yi)
                    .map(|x| x.1)
                    .sum();
                let e: f64 = self.copy_pos[j].iter().map(|x| p[x.0] * x.1).sum();
                let g = sy - e;
                if g == 0.0 {
                    continue;
                }
                self.copy_v[j] = 0.999 * self.copy_v[j] + 0.001 * g * g;
                let step = self.copy_lr * g / (self.copy_v[j].sqrt() + 1e-4);
                self.copy_b0[j] += step;
                if !self.copy_nocontext {
                    *self.copy_w[j].entry(self.copy_prev).or_insert(0.0) += step;
                }
            }
            for &e in &kept {
                self.slab[e].learn_beta(&p, yi, 0.003);
            }
            self.prof[6] += c_tr.elapsed().as_secs_f64();
            let c_wr = std::time::Instant::now();
            let ycode = self.byte.pool.codes[yu];
            let tgt = bits::from_positions(&ycode[..self.tix.rk]);
            if self.cfg.err_write > 0 {
                let (a, fl) = (self.cfg.err_alpha, self.cfg.err_floor);
                let gate = |py: f64| (1.0 - py).max(0.0).powf(a).max(fl);
                let argmax_wrong = |q: &[f64]| {
                    let mut jb = usize::MAX;
                    for (j, &v) in q.iter().enumerate() {
                        if j != yi && (jb == usize::MAX || v > q[jb]) {
                            jb = j;
                        }
                    }
                    jb
                };
                let jw = argmax_wrong(&p);
                for &e in kept.iter().chain(trials.iter()) {
                    let (g, wr) = if self.cfg.err_write == 1 {
                        (
                            gate(p[yi]),
                            (jw != usize::MAX).then(|| (self.alive_idx[jw], p[jw])),
                        )
                    } else {
                        // the edge's own prediction: softmax of its contribution alone
                        match self.slab[e].contribution() {
                            Some(c) => {
                                let mx = c.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                                let z: f64 = c.iter().map(|x| (x - mx).exp()).sum();
                                let q: Vec<f64> = c.iter().map(|x| (x - mx).exp() / z).collect();
                                let j = argmax_wrong(&q);
                                (
                                    gate(q[yi]),
                                    (j != usize::MAX).then(|| (self.alive_idx[j], q[j])),
                                )
                            }
                            None => (1.0, None),
                        }
                    };
                    let ed = &mut self.slab[e];
                    ed.err_g = g;
                    ed.err_wrong = wr;
                    ed.forget = self.cfg.err_forget;
                }
            }
            for &e in kept.iter().chain(trials.iter()) {
                let src = self.slab[e].src;
                let g = self.gen_of(src, self.slab[e].cur);
                self.slab[e].learn(
                    g,
                    &ycode,
                    &tgt,
                    &self.tix,
                    &self.byte.pool.codes,
                    &self.byte.pool.alive,
                    &mut self.rng,
                );
            }
            self.prof[7] += c_wr.elapsed().as_secs_f64();
        }
        if novel {
            self.alive_dirty = true;
        }
        if self.story_edge.is_some() {
            self.story_update(byte as usize, t);
        }
        if self.cfg.real && self.cfg.merge > 0 && t % 1000 == 999 && !self.frozen {
            let n = self.byte.pool.consolidate(self.cfg.merge);
            if n > 0 {
                self.merged += n;
                self.alive_dirty = true;
            }
        }
        let c_up = std::time::Instant::now();
        for i in self.derived.clone() {
            self.update_node(i, t);
        }
        self.prof[8] += c_up.elapsed().as_secs_f64();
        if self.cfg.words && self.cfg.run_window > 0 && !self.frozen {
            if self.chunk.is_none() && self.run_scr.is_empty() {
                for z in [0.5, 1.0, 1.5] {
                    self.run_scr.push(RunScreen::make(
                        self.cfg.screen_mix,
                        Cand::Chunk {
                            z,
                            prev: -1,
                            map: HashMap::new(),
                        },
                        t,
                        self.cfg.run_window,
                    ));
                }
            }
            let c_fs = std::time::Instant::now();
            self.feed_screens(t);
            self.prof[9] += c_fs.elapsed().as_secs_f64();
        }
        if self.cfg.grow && t % 1000 == 999 && !self.frozen {
            let c0 = std::time::Instant::now();
            self.check();
            self.prof[4] += c0.elapsed().as_secs_f64();
        }
        let a = 1.0 / 2000.0;
        let d = self.bits[t] - self.sm;
        self.sm += a * d;
        self.sv = (1.0 - a) * self.sv + a * d * d;
        self.prof[11] += tick0.elapsed().as_secs_f64();
        self.copy_prev = self.nodes[0].state;
        self.t += 1;
    }

    /// The system's distribution over the next byte, from the current node states.
    pub fn predict(&mut self) -> Vec<f64> {
        if self.alive_dirty {
            self.refresh_alive();
        }
        let na = self.alive_idx.len();
        let mut logits = vec![0.0; na];
        for e in self.kept.clone() {
            if self.score_edge(e) {
                let c = self.slab[e].contribution().unwrap();
                logits.iter_mut().zip(&c).for_each(|(l, s)| *l += s);
            }
        }
        let v = self.vocab;
        let mut out = vec![0.0; v];
        if na == 0 {
            return vec![1.0 / v as f64; v];
        }
        let m = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let z: f64 = logits.iter().map(|l| (l - m).exp()).sum();
        for (j, &c) in self.alive_idx.iter().enumerate() {
            let cb = &self.cellbyte[c];
            let b = (0..v).max_by_key(|&i| cb[i]).unwrap();
            out[b] += (logits[j] - m).exp() / z;
        }
        out
    }

    /// Room for n more bytes in the stream.
    fn extend(&mut self, n: usize) {
        self.tt += n;
        if self.bytes {
            self.data.resize(self.tt, 0);
        }
        self.syms.resize(self.tt, 0);
        self.bits.resize(self.tt, 0.0);
        self.bits_map.resize(self.tt, 0.0);
        self.coded.resize(self.tt, 0.0);
        self.novel.resize(self.tt, false);
        self.hit.resize(self.tt, false);
        self.mu.resize(self.tt, 0.0);
        self.sd.resize(self.tt, 1.0);
        self.ent.resize(self.tt, 0.0);
    }

    /// Freeze learning, read `prompt`, then sample `n` bytes, feeding each back in as input.
    /// `temp` < 1 sharpens the distribution. Returns the generated bytes.
    pub fn generate(&mut self, prompt: &[u8], n: usize, temp: f64, rng: &mut Rng) -> Vec<u8> {
        self.frozen = true;
        self.extend(prompt.len() + n);
        for &b in prompt {
            let t = self.t;
            self.data[t] = b;
            self.syms[t] = b as u32;
            self.tick();
        }
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let p = self.predict();
            let w: Vec<f64> = p.iter().map(|x| x.powf(1.0 / temp)).collect();
            let z: f64 = w.iter().sum();
            let mut u = rng.f64() * z;
            let mut b = 0;
            for (i, x) in w.iter().enumerate() {
                if u < *x {
                    b = i;
                    break;
                }
                u -= x;
            }
            let t = self.t;
            self.data[t] = b as u8;
            self.syms[t] = b as u32;
            self.tick();
            out.push(b as u8);
        }
        out
    }

    /// Feed one more input symbol and run a tick.
    /// Feed a fused input (set-bit positions) as the next tick's input.
    pub fn push_code(&mut self, fused: Vec<u16>) {
        self.extend(1);
        self.pending = Some(fused);
        let t = self.t;
        self.syms[t] = 0;
        self.tick();
    }

    /// The input cell's code at tick t (-1 cell: None).
    pub fn input_code(&self, t: usize) -> Option<Code> {
        let c = self.nodes[0].h[t];
        if c < 0 {
            None
        } else {
            Some(self.byte.pool.codes[c as usize])
        }
    }

    /// Codes of the level's kept hold nodes' current states (for relational fusion roles).
    pub fn hold_codes(&self) -> Vec<Code> {
        let mut out = vec![];
        for &e in self.kept.iter().skip(1) {
            let n = self.slab[e].src;
            if matches!(self.nodes[n].kind, Kind::Hold { .. }) && self.nodes[n].state >= 0 {
                out.push(self.code_of(n, self.nodes[n].state));
            }
        }
        out
    }

    pub fn push(&mut self, sym: u32) {
        self.extend(1);
        let t = self.t;
        self.syms[t] = sym;
        self.tick();
    }

    /// A screen threshold: the given one, or `mix_thr` when screens score mixtures.
    fn thr(&self, normal: f64) -> f64 {
        if self.cfg.screen_mix {
            self.cfg.mix_thr
        } else {
            normal
        }
    }

    /// Symbol `sym` now stands for something new: give it a fresh code, so readers treat it as a new input.
    pub fn reassign_symbol(&mut self, sym: u32) {
        if self.cfg.real {
            self.byte.bytecode[sym as usize] = bits::rand_code(&mut self.rng);
        } else if let Some(&c) = self.byte.sym.get(&sym) {
            self.byte.pool.new_code(c, &mut self.rng);
        }
        self.alive_dirty = true;
    }

    /// The input cell this level expects next (argmax of the kept edges' mix), or -1.
    pub fn predict_cell(&mut self) -> i32 {
        if self.alive_dirty {
            self.refresh_alive();
        }
        let na = self.alive_idx.len();
        if na == 0 {
            return -1;
        }
        let mut logits = vec![0.0; na];
        for e in self.kept.clone() {
            if self.score_edge(e) {
                let c = self.slab[e].contribution().unwrap();
                logits.iter_mut().zip(&c).for_each(|(l, s)| *l += s);
            }
        }
        let j = (0..na)
            .max_by(|&a, &b| logits[a].partial_cmp(&logits[b]).unwrap().then(b.cmp(&a)))
            .unwrap();
        self.alive_idx[j] as i32
    }

    /// Whether this level's chunk node exists and its edge is kept.
    pub fn chunk_kept(&self) -> bool {
        match self.chunk {
            Some(c) => self.kept.iter().any(|&e| self.slab[e].src == c),
            None => false,
        }
    }

    /// If the chunk was cut at tick t: the chunk completed just before (its cell and generation).
    /// A unit to hand up that ends just before tick `cut`: (chunk cell, generation, cut).
    pub fn completed_chunk(&self, t: usize) -> Option<(i32, u32, usize)> {
        let c = self.chunk?;
        if let Some(z) = self.cfg.hand_up_entropy {
            // a local peak of predictive entropy at t-1: many continuations were possible there
            if t >= 2 {
                let (h0, h1, h2) = (self.ent[t - 2], self.ent[t - 1], self.ent[t]);
                if h1 > h0 && h1 >= h2 && (h1 as f64) > self.ent_m + z * self.ent_v.sqrt() {
                    let cell = self.nodes[c].h[t - 2];
                    if cell >= 0 {
                        return Some((cell, self.gen_of(c, cell), t - 1));
                    }
                }
            }
            return None;
        }
        let z = if self.cfg.hand_up_z > 0.0 {
            self.cfg.hand_up_z
        } else {
            self.chunk_z(c)
        };
        if t >= 1 && self.boundary(t, z) {
            let cell = self.nodes[c].h[t - 1];
            if cell >= 0 {
                return Some((cell, self.gen_of(c, cell), t));
            }
        }
        None
    }

    /// The position sense: the files' start offsets (sorted). Adds the position nodes (offset from the start, and
    /// offset modulo 2, 4, 8 and 16) as candidates like any other derived node.
    pub fn set_file_starts(&mut self, starts: Vec<usize>) {
        self.file_starts = starts;
        for m in [0usize, 2, 4, 8, 16] {
            let seed = self.rng.next_u64();
            let name = if m == 0 { "pos".to_string() } else { format!("pos%{m}") };
            let nd = self.new_node(name, Kind::Pos(Box::new(UpperNode { codes: vec![], rng: Rng::new(seed) }), m), vec![], String::new());
            self.add(nd, "the position sense".into());
        }
    }
    /// The state of position node i for the byte that will arrive after tick t (the prediction for t + 1), or -1
    /// before the first file starts.
    fn pos_state(&self, i: usize, t: usize) -> i32 {
        let m = match &self.nodes[i].kind {
            Kind::Pos(_, m) => *m,
            _ => unreachable!(),
        };
        let next = t + 1;
        let k = self.file_starts.partition_point(|&x| x <= next);
        if k == 0 {
            return -1;
        }
        let off = next - self.file_starts[k - 1];
        if m > 0 {
            (off % m) as i32
        } else if off < 32 {
            off as i32
        } else {
            (32 + (usize::BITS - off.leading_zeros()) as usize - 6) as i32
        }
    }

    /// Add the top-down node: the level above's predicted next symbol.
    pub fn add_upper(&mut self) {
        if self.upper.is_some() || self.t < self.upper_retry {
            return;
        }
        let seed = self.rng.next_u64();
        let nd = self.new_node(
            "upper".into(),
            Kind::Upper(Box::new(UpperNode {
                codes: vec![],
                rng: Rng::new(seed),
            })),
            vec![],
            String::new(),
        );
        let i = self.add(nd, "a level above exists".into());
        self.upper = Some(i);
    }

    /// A surprising byte at t starts a new chunk.
    pub fn boundary(&self, t: usize, z: f64) -> bool {
        self.bits[t] > self.mu[t] + z * self.sd[t]
    }

    fn update_node(&mut self, i: usize, t: usize) {
        let s = match &self.nodes[i].kind {
            Kind::Delay { d } => {
                if t + 1 >= *d {
                    self.nodes[self.nodes[i].parts[0]].h[t + 1 - d]
                } else {
                    -1
                }
            }
            Kind::Hold { v } => {
                let (x, y) = (self.nodes[i].parts[0], self.nodes[i].parts[1]);
                if self.nodes[y].state == *v {
                    self.nodes[x].state
                } else {
                    self.nodes[i].state
                }
            }
            Kind::BindReal(_) => {
                let (x, y) = (self.nodes[i].parts[0], self.nodes[i].parts[1]);
                let (sx, sy) = (self.nodes[x].state, self.nodes[y].state);
                if sx < 0 || sy < 0 {
                    -1
                } else {
                    let (gx, gy) = (self.gen_of(x, sx), self.gen_of(y, sy));
                    let (cx, cy) = (self.code_of(x, sx), self.code_of(y, sy));
                    let rng = &mut self.rng;
                    match &mut self.nodes[i].kind {
                        Kind::BindReal(b) => b.step(sx, sy, gx, gy, &cx, &cy, t, rng),
                        _ => unreachable!(),
                    }
                }
            }
            Kind::BindSym(_) => {
                let (x, y) = (self.nodes[i].parts[0], self.nodes[i].parts[1]);
                let (sx, sy) = (self.nodes[x].state, self.nodes[y].state);
                match &mut self.nodes[i].kind {
                    Kind::BindSym(b) => b.id(sx, sy),
                    _ => unreachable!(),
                }
            }
            Kind::Distill(_) => {
                let k = std::mem::replace(&mut self.nodes[i].kind, Kind::Taken);
                let mut d = match k {
                    Kind::Distill(d) => d,
                    _ => unreachable!(),
                };
                let src = self.slab[d.edge].src;
                let s = self.cluster(&mut d, self.nodes[src].state);
                self.nodes[i].kind = Kind::Distill(d);
                s
            }
            Kind::ChunkReal(_, z) => {
                let z = *z;
                let b = self.nodes[0].state;
                let (bg, bc) = if b >= 0 {
                    (self.gen_of(0, b), self.code_of(0, b))
                } else {
                    (0, [0; bits::CK])
                };
                let reset = self.boundary(t, z);
                let prev = self.nodes[i].state;
                let rng = &mut self.rng;
                match &mut self.nodes[i].kind {
                    Kind::ChunkReal(k, _) => k.step(prev, b, bg, &bc, reset, t, rng),
                    _ => unreachable!(),
                }
            }
            Kind::ChunkSym(_, z) => {
                let reset = self.boundary(t, *z);
                let (prev, b) = (self.nodes[i].state, self.nodes[0].state);
                match &mut self.nodes[i].kind {
                    Kind::ChunkSym(k, _) => k.step(prev, b, reset),
                    _ => unreachable!(),
                }
            }
            Kind::PrevWord(z) => {
                let c = self.nodes[i].parts[0];
                if t >= 1 && self.boundary(t, *z) && !self.nodes[c].removed {
                    self.nodes[self.nodes[i].parts[0]].h[t - 1]
                } else {
                    self.nodes[i].state
                }
            }
            Kind::PrevClass(_, z) => {
                let c = self.nodes[i].parts[0];
                let done = if t >= 1 && self.boundary(t, *z) && !self.nodes[c].removed {
                    Some(self.nodes[c].h[t - 1])
                } else {
                    None
                };
                let y = self.nodes[0].state;
                match &mut self.nodes[i].kind {
                    Kind::PrevClass(k, _) => {
                        if let Some(dc) = done {
                            if t > k.last_cut && t - k.last_cut <= 40 && t <= self.data.len() {
                                k.texts.insert(dc, self.data[k.last_cut..t].to_vec());
                            }
                            k.last_cut = t;
                        }
                        k.observe(y, done);
                        k.class(k.last)
                    }
                    _ => unreachable!(),
                }
            }
            Kind::Upper(_) => {
                let s = self.upper_state;
                if let Kind::Upper(u) = &mut self.nodes[i].kind {
                    while s >= 0 && u.codes.len() <= s as usize {
                        let c = bits::rand_code(&mut u.rng);
                        u.codes.push(c);
                    }
                }
                s
            }
            Kind::Pos(..) => {
                let s = self.pos_state(i, t);
                if let Kind::Pos(u, _) = &mut self.nodes[i].kind {
                    while s >= 0 && u.codes.len() <= s as usize {
                        let c = bits::rand_code(&mut u.rng);
                        u.codes.push(c);
                    }
                }
                s
            }
            Kind::Const(_) => 0,
            Kind::Byte | Kind::Taken => return,
        };
        self.nodes[i].state = s;
        self.nodes[i].h[t] = s;
    }

    // ---------------------------------------------------------------- growth controller
    fn series(&self, n: usize, w: usize) -> Vec<i64> {
        let t = self.t;
        self.nodes[n]
            .h
            .range(t + 1 - w, t)
            .iter()
            .map(|&x| x as i64 + 1)
            .collect()
    }

    /// Source node of edge e.
    pub fn slab_src(&self, e: usize) -> usize {
        self.slab[e].src
    }

    /// Starting size for a new real pool: the default, or, when pools may grow, the next power of two
    /// above 1.5 times the distinct inputs seen in its replay window (within the maximum and the budget).
    fn sized_cap(&self, default: usize, distinct: usize) -> usize {
        if !self.cfg.grow_pools {
            return default;
        }
        let total: usize = self
            .derived
            .iter()
            .map(|&n| match &self.nodes[n].kind {
                Kind::BindReal(b) => b.pool.cap,
                Kind::ChunkReal(k, _) => k.pool.cap,
                _ => 0,
            })
            .sum();
        let room = self.cfg.pool_budget.saturating_sub(total);
        (distinct * 3 / 2)
            .next_power_of_two()
            .clamp(default, self.cfg.max_pool)
            .min(room.max(default))
    }

    /// Double every real pool whose evictions mostly remove recently used cells.
    fn grow_pools(&mut self) {
        let t = self.t;
        let total: usize = self
            .derived
            .iter()
            .map(|&n| match &self.nodes[n].kind {
                Kind::BindReal(b) => b.pool.cap,
                Kind::ChunkReal(k, _) => k.pool.cap,
                _ => 0,
            })
            .sum();
        let mut room = self.cfg.pool_budget.saturating_sub(total);
        for n in self.derived.clone() {
            let (max_pool, ya) = (self.cfg.max_pool, self.cfg.young_age);
            let pool = match &mut self.nodes[n].kind {
                Kind::BindReal(b) => &mut b.pool,
                Kind::ChunkReal(k, _) => &mut k.pool,
                _ => continue,
            };
            pool.young_age = ya;
            let (ev, young) = pool.take_pressure();
            let cap = pool.cap;
            if ev >= 100 && young * 2 > ev && cap < max_pool && cap <= room {
                let new = (cap * 2).min(max_pool);
                pool.grow(new);
                match &mut self.nodes[n].kind {
                    Kind::BindReal(b) => b.code_key.resize(new, None),
                    Kind::ChunkReal(k, _) => k.code_key.resize(new, None),
                    _ => {}
                }
                room -= new - cap;
                let name = self.nodes[n].name.clone();
                self.events.push(Event {
                    t,
                    kind: "grow".into(),
                    a: name,
                    b: Some(format!("{cap}->{new}, {young}/{ev} young evictions")),
                    v: None,
                });
            }
        }
    }

    /// Free a discarded edge's writer-mask rows, unless a live distill still reads them.
    fn release_edge(&mut self, e: usize) {
        self.trial_t0.remove(&e);
        let read = self
            .derived
            .iter()
            .any(|&n| matches!(&self.nodes[n].kind, Kind::Distill(d) if d.edge == e));
        if !read {
            self.slab[e].rows = HashMap::default();
            self.slab[e].sc = None;
        }
    }

    /// Number of distinct states a node has used so far.
    fn cardinality(&self, n: usize) -> usize {
        match &self.nodes[n].kind {
            Kind::BindReal(b) => b.pool.cap,
            Kind::BindSym(b) => b.map.len(),
            Kind::ChunkReal(k, _) => k.pool.cap,
            Kind::ChunkSym(k, _) => k.map.len(),
            Kind::PrevWord(_) => self.cardinality(self.nodes[n].parts[0]),
            Kind::PrevClass(k, _) => k.k + 1,
            Kind::Distill(d) => d.k,
            _ => self.byte.pool.cap,
        }
    }

    /// Trial length multiplier: sources with very many states get longer trials, so their rows can learn.
    fn trial_mult(&self, e: usize) -> usize {
        if self.cfg.long_trials > 1
            && self.cardinality(self.slab[e].src) > self.cfg.long_trial_states
        {
            self.cfg.long_trials
        } else {
            1
        }
    }

    fn name_of_edge(&self, e: usize) -> String {
        self.nodes[self.slab[e].src].name.clone()
    }

    fn check(&mut self) {
        let t = self.t;
        let c = self.cfg.clone();
        // trial decisions
        for e in self.trials.clone() {
            let age = t - self.trial_t0[&e];
            if let Some(&j0) = self.judge_t0.get(&e) {
                if t - j0 < c.evalw {
                    continue; // the fresh window after an extension is not full yet
                }
            }
            if age >= c.burn + c.evalw && self.slab[e].n > 100 {
                let (m, z) = self.slab[e].stats();
                // a many-state source still learning (positive, below the bar): extend its trial
                let passes = m >= c.acc_gain && z >= c.acc_z;
                if !passes && m > 0.0 && age < (c.burn + c.evalw) * self.trial_mult(e) {
                    if c.fresh_extensions {
                        self.slab[e].reset_stats();
                        self.judge_t0.insert(e, t);
                    }
                    continue;
                }
                self.judge_t0.remove(&e);
                self.trials.retain(|&x| x != e);
                let k = self.name_of_edge(e);
                if m >= c.acc_gain && z >= c.acc_z {
                    self.slab[e].reset_stats();
                    self.kept.push(e);
                    self.kept_since.insert(k.clone(), t);
                    self.gain_at_keep.insert(k.clone(), m);
                    self.events.push(Event {
                        t,
                        kind: "keep".into(),
                        a: "byte".into(),
                        b: Some(k.clone()),
                        v: Some(m),
                    });
                    let nd = self.slab[e].src;
                    let is_bind =
                        matches!(self.nodes[nd].kind, Kind::BindReal(_) | Kind::BindSym(_));
                    if is_bind
                        && m >= c.distill_gain
                        && self.live(&format!("Z[{k}]")).is_none()
                        && self.derived.len() < c.max_derived
                    {
                        let dn = self.make_distill(e, &k);
                        self.add(dn, k.clone());
                    }
                } else {
                    let bo = self.backoff.get(&k).map(|b| b.1).unwrap_or(10000);
                    self.backoff.insert(k.clone(), (t + bo, bo * 2));
                    self.events.push(Event {
                        t,
                        kind: "reject".into(),
                        a: "byte".into(),
                        b: Some(k),
                        v: Some(m),
                    });
                    self.release_edge(e);
                }
            }
        }
        // prune kept edges
        let noise_from = |me: &Self, t0: usize| -> bool {
            // the window [t0, t] codes worse than uniform (raw bits)
            let lo = t0.min(t);
            t > lo && me.bits[lo..=t].iter().sum::<f64>() / (t + 1 - lo) as f64 > (me.vocab as f64).log2()
        };
        for e in self.kept[1..].to_vec() {
            let k = self.name_of_edge(e);
            let since = self.kept_since.get(&k).copied().unwrap_or(0);
            if t - since >= c.prune_every
                && self.slab[e].n > 100
            {
                let (m, z) = self.slab[e].stats();
                if c.prune_guard && noise_from(self, since) {
                    self.guarded += 1;
                    self.slab[e].reset_stats();
                    self.kept_since.insert(k, t);
                    continue;
                }
                if Some(e) == self.story_edge || self.ext.contains(&e) {
                    // measured, not pruned: log its gain over the window
                    self.events.push(Event {
                        t,
                        kind: if Some(e) == self.story_edge {
                            "story".into()
                        } else {
                            k.clone()
                        },
                        a: "byte".into(),
                        b: None,
                        v: Some(m),
                    });
                } else if if c.prune_harmful { m < 0.0 && z < -1.0 } else { m < c.prune_gain || z < 1.0 } {
                    self.kept.retain(|&x| x != e);
                    self.backoff.insert(k.clone(), (t + 10000, 10000));
                    self.events.push(Event {
                        t,
                        kind: "prune".into(),
                        a: "byte".into(),
                        b: Some(k.clone()),
                        v: Some(m),
                    });
                    self.release_edge(e);
                }
                self.slab[e].reset_stats();
                self.kept_since.insert(k, t);
            }
        }
        if t % 10000 == 9999 && c.grow_pools {
            self.grow_pools();
        }
        if t % 10000 == 9999 {
            for i in self.derived.clone() {
                if let Kind::PrevClass(k, _) = &mut self.nodes[i].kind {
                    k.refit(&mut self.rng);
                }
                if matches!(self.nodes[i].kind, Kind::Distill(_)) {
                    let k = std::mem::replace(&mut self.nodes[i].kind, Kind::Taken);
                    if let Kind::Distill(mut d) = k {
                        self.refit(&mut d);
                        self.nodes[i].kind = Kind::Distill(d);
                    }
                }
            }
        }
        let w = (t + 1).min(c.w);
        if w < 5000 {
            return;
        }
        if t % 10000 == 9999 && self.derived.len() < c.max_derived {
            let c0 = std::time::Instant::now();
            self.temporal(w);
            self.prof[1] += c0.elapsed().as_secs_f64();
        }
        if t % 10000 == 4999 && self.derived.len() < c.max_derived {
            let c0 = std::time::Instant::now();
            self.synergy(w);
            self.prof[2] += c0.elapsed().as_secs_f64();
        }
        if t % 5000 == 4999 {
            let c0 = std::time::Instant::now();
            self.screen(w);
            self.prof[3] += c0.elapsed().as_secs_f64();
        }
        if t % 10000 == 9999 {
            self.gc();
        }
        if !self.recency.is_empty() {
            self.add_recency();
        }
        self.fill();
    }

    fn screen(&mut self, w: usize) {
        let y = self.series(0, w);
        let (yp, yn) = (&y[..w - 1], &y[1..]);
        let sc = CmiScreen::new(yp, yn);
        let mut out = vec![];
        // the series and shift draws in order, then every candidate scored in parallel against one baseline
        let jobs: Vec<(usize, Vec<i64>, usize)> = self
            .derived
            .clone()
            .into_iter()
            .map(|n| (n, self.series(n, w), self.rng.range(500, w - 500)))
            .collect();
        let vals: Vec<f64> = if self.cfg.screen_mix {
            let base = MixBase::new(yp, yn);
            par_map(&jobs, |(_, s, _)| base.gain(&s[..w - 1]))
        } else {
            jobs.iter()
                .map(|(_, s, sh)| sc.shift_screen(&s[..w - 1], *sh))
                .collect()
        };
        for ((n, _, _), v) in jobs.iter().zip(vals) {
            out.push((self.nodes[*n].name.clone(), v));
        }
        self.scr.extend(out);
    }

    fn fill(&mut self) {
        let t = self.t;
        let free = self.cfg.max_trials.saturating_sub(self.trials.len());
        if free == 0 {
            return;
        }
        let kept_names: HashSet<String> = self.kept.iter().map(|&e| self.name_of_edge(e)).collect();
        let trial_names: HashSet<String> =
            self.trials.iter().map(|&e| self.name_of_edge(e)).collect();
        let mut cands = vec![];
        for (k, &v) in &self.scr {
            if kept_names.contains(k) || trial_names.contains(k) || self.live(k).is_none() {
                continue;
            }
            if let Some(&(until, _)) = self.backoff.get(k) {
                if until > t {
                    continue;
                }
            }
            let v = if self.priority.contains(k) {
                v + 10.0
            } else {
                v
            };
            if v > self.thr(self.cfg.screen_thr) {
                cands.push((v, k.clone()));
            }
        }
        cands.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then_with(|| b.1.cmp(&a.1)));
        for (_, k) in cands.into_iter().take(free) {
            let n = self.live(&k).unwrap();
            let mut e = self.mk_edge(n, 0.0);
            if let Some(w) = self.recency_window(n) {
                // one row that holds about the last w codes: it fills, then drops old bits at random
                e.kw = (w * self.tix.rk).min(D);
                e.sparse = true;
                e.age_every = 0;
            }
            self.slab.push(e);
            let e = self.slab.len() - 1;
            self.trials.push(e);
            self.trial_t0.insert(e, t);
            self.priority.remove(&k);
            self.tried.insert(k.clone());
            self.events.push(Event {
                t,
                kind: "trial".into(),
                a: "byte".into(),
                b: Some(k),
                v: None,
            });
        }
    }

    fn temporal(&mut self, w: usize) {
        self.temporal_inner(w);
        if self.cfg.words && self.derived.len() < self.cfg.max_derived {
            self.word_moves(w);
        }
    }

    fn temporal_inner(&mut self, w: usize) {
        let t = self.t;
        let w = w.min(t + 1 - 16);
        let y = self.series(0, w);
        let (yp, yn) = (&y[..w - 1], &y[1..]);
        let sc = CmiScreen::new(yp, yn);
        let thr = self.thr(self.cfg.tmp_thr);
        // (value, Some(d)) for a delay, (value, None, x, v) for a hold
        let mut best: Option<(f64, Option<usize>, usize, i32)> = None;
        let base = if self.cfg.screen_mix {
            Some(MixBase::new(yp, yn))
        } else {
            None
        };
        let mut djobs: Vec<(usize, Vec<i64>, usize)> = vec![];
        for d in [2usize, 3, 4, 5, 6, 8, 12, 16] {
            if self.live(&format!("byte@{d}")).is_some() {
                continue;
            }
            let h = &self.nodes[0].h;
            let s: Vec<i64> = h
                .range(t + 1 - w - (d - 1), t - (d - 1))
                .iter()
                .map(|&x| x as i64 + 1)
                .collect();
            let sh = self.rng.range(500, w - 500);
            djobs.push((d, s, sh));
        }
        let dvals: Vec<f64> = match &base {
            Some(b) => par_map(&djobs, |(_, s, _)| b.gain(&s[..w - 1])),
            None => djobs
                .iter()
                .map(|(_, s, sh)| sc.shift_screen(&s[..w - 1], *sh))
                .collect(),
        };
        for ((d, _, _), v) in djobs.iter().zip(dvals) {
            if v > thr && best.map_or(true, |b| v > b.0) {
                best = Some((v, Some(*d), 0, 0));
            }
        }
        // event-gated holds: latch the byte or a tap whenever the byte node shows a moderately frequent value
        let mut cnt: HashMap<i64, usize> = HashMap::new();
        for &x in &y {
            if x > 0 {
                *cnt.entry(x - 1).or_insert(0) += 1;
            }
        }
        let mut evs: Vec<i32> = cnt
            .iter()
            .filter(|(_, &n)| {
                let f = n as f64 / w as f64;
                (0.005..=0.2).contains(&f)
            })
            .map(|(&v, _)| v as i32)
            .collect();
        evs.sort_unstable();
        let mut xs = vec![0];
        xs.extend(
            self.derived
                .iter()
                .copied()
                .filter(|&i| matches!(self.nodes[i].kind, Kind::Delay { .. })),
        );
        // each source's history once; the hold series are built and scored in parallel, the shift draws in order
        let mut xhs: HashMap<usize, Vec<i32>> = HashMap::new();
        let mut hjobs: Vec<(usize, i32, usize)> = vec![];
        for &x in &xs {
            for &v in &evs {
                let lab = self.label(v);
                if self
                    .live(&format!("hold({}|{})", self.nodes[x].name, lab))
                    .is_some()
                {
                    continue;
                }
                xhs.entry(x)
                    .or_insert_with(|| self.nodes[x].h.range(t + 1 - w, t));
                let sh = self.rng.range(500, w - 500);
                hjobs.push((x, v, sh));
            }
        }
        let hold_series = |x: usize, v: i32| -> Vec<i64> {
            let xh = &xhs[&x];
            let mut hs = Vec::with_capacity(w);
            let mut cur: i64 = 0;
            let mut seen = false;
            for j in 0..w {
                if y[j] - 1 == v as i64 {
                    cur = xh[j] as i64 + 1;
                    seen = true;
                }
                hs.push(if seen { cur } else { 0 });
            }
            hs
        };
        let hvals: Vec<f64> = match &base {
            Some(b) => par_map(&hjobs, |&(x, v, _)| b.gain(&hold_series(x, v)[..w - 1])),
            None => hjobs
                .iter()
                .map(|&(x, v, sh)| sc.shift_screen(&hold_series(x, v)[..w - 1], sh))
                .collect(),
        };
        for (&(x, v, _), val) in hjobs.iter().zip(hvals) {
            if val > thr && best.map_or(true, |b| val > b.0) {
                best = Some((val, None, x, v));
            }
        }
        if let Some((val, d, x, v)) = best {
            let nd = match d {
                Some(d) => self.make_delay(d, t),
                None => self.make_hold(x, v, t),
            };
            self.add(nd, format!("screen byte {val:.2}"));
        }
    }

    /// Structural containment through binds and distills, by name (as in Python): a node rebuilt after
    /// cleanup gets a new index but is still the same structure.
    fn contains(&self, n: usize, other: usize) -> bool {
        if n == other || self.nodes[n].name == self.nodes[other].name {
            return true;
        }
        match self.nodes[n].kind {
            Kind::BindReal(_) | Kind::BindSym(_) | Kind::Distill(_) => {
                self.nodes[n].parts.iter().any(|&p| self.contains(p, other))
            }
            _ => false,
        }
    }

    fn synergy(&mut self, w: usize) {
        let y = self.series(0, w);
        let yn = &y[1..];
        let order = self.order();
        let ser: HashMap<usize, Vec<i64>> = order
            .iter()
            .map(|&n| (n, self.series(n, w)[..w - 1].to_vec()))
            .collect();
        let sh = self.rng.range(500, w - 500);
        let h_yn = ent(&[yn]);
        let mi_yn = |s: &[i64]| -> (f64, usize) {
            let (hs, hj, d) = ent_nested(&[s, yn]);
            (hs + h_yn - hj, d)
        };
        // (the single-source shift screen is only needed without mixture screens)
        let single: HashMap<usize, f64> = if self.cfg.screen_mix {
            HashMap::new()
        } else {
            ser.iter()
                .map(|(&n, s)| (n, mi_yn(s).0 - mi_yn(&roll(s, sh)).0))
                .collect()
        };
        let mut mix_jobs: Vec<(usize, usize)> = vec![];
        let mut anchors = vec![0];
        for &e in self.kept[1..].iter().chain(self.trials.iter()) {
            let n = self.slab[e].src;
            if !anchors.contains(&n) && !self.nodes[n].removed {
                anchors.push(n);
            }
        }
        let mut best: Option<(f64, usize, usize)> = None;
        for &a in &anchors {
            if !ser.contains_key(&a) {
                continue;
            }
            for &b in &order {
                if b == a {
                    continue;
                }
                let (na, nb) = (&self.nodes[a].name, &self.nodes[b].name);
                if self.live(&format!("({na}*{nb})")).is_some()
                    || self.live(&format!("({nb}*{na})")).is_some()
                {
                    continue;
                }
                if self.contains(b, a) || self.contains(a, b) {
                    continue;
                }
                if matches!(self.nodes[a].kind, Kind::Const(_))
                    || matches!(self.nodes[b].kind, Kind::Const(_))
                {
                    continue; // a constant adds nothing to a bind
                }
                let (sa, sb) = (&ser[&a], &ser[&b]);
                let m = sb.iter().copied().max().unwrap_or(0) + 1;
                let pair: Vec<i64> = sa.iter().zip(sb).map(|(x, z)| x * m + z).collect();
                if self.cfg.screen_mix {
                    // what b adds beyond the anchor a, as a held-out mixture gain (scored below, in parallel)
                    mix_jobs.push((a, b));
                    continue;
                }
                let (m0, nd) = mi_yn(&pair);
                if nd > w / 4
                    || (self.cfg.real
                        && nd
                            > 2 * (if self.cfg.grow_pools {
                                self.cfg.max_pool
                            } else {
                                self.cfg.bind_cap
                            }))
                {
                    continue; // not enough data per context, or far beyond capacity
                }
                let j = m0 - mi_yn(&roll(&pair, sh)).0;
                let g = j - single[&a].max(single[&b]).max(0.0);
                if g > self.cfg.syn_thr && best.map_or(true, |bb| g > bb.0) {
                    best = Some((g, a, b));
                }
            }
        }
        if !mix_jobs.is_empty() {
            let mut bases: HashMap<usize, MixBase> = HashMap::new();
            for &(a, _) in &mix_jobs {
                bases.entry(a).or_insert_with(|| MixBase::new(&ser[&a], yn));
            }
            let gs = par_map(&mix_jobs, |&(a, b)| bases[&a].gain(&ser[&b]));
            for (&(a, b), g) in mix_jobs.iter().zip(gs) {
                if g > self.cfg.mix_thr && best.map_or(true, |bb| g > bb.0) {
                    best = Some((g, a, b));
                }
            }
        }
        if let Some((g, a, b)) = best {
            self.bind(a, b, format!("synergy byte {g:.2}"));
        }
    }

    fn gc(&mut self) {
        self.gc_inner();
        if self.chunk.is_some_and(|c| self.nodes[c].removed) {
            self.chunk = None;
        }
        if self.prev.is_some_and(|c| self.nodes[c].removed) {
            self.prev = None;
        }
        if self.pclass.is_some_and(|c| self.nodes[c].removed) {
            self.pclass = None;
        }
        if self.upper.is_some_and(|c| self.nodes[c].removed) {
            self.upper = None;
            self.upper_retry = self.t + 100_000;
        }
        let t = self.t;
        for r in 0..self.recency.len() {
            if let Some(c) = self.recency[r].1 {
                if self.nodes[c].removed {
                    self.recency[r].1 = None;
                    self.recency[r].2 = t + 100_000;
                }
            }
        }
    }

    /// Offer the recency candidates that are not live (after burn-in, and again after cleanup removed them).
    fn add_recency(&mut self) {
        for r in 0..self.recency.len() {
            let (w, live, retry) = self.recency[r];
            if live.is_some() || self.t < retry.max(self.cfg.evalw) {
                continue;
            }
            let code = bits::rand_code(&mut self.rng);
            let nd = self.new_node(
                format!("recent({w})"),
                Kind::Const(code),
                vec![],
                String::new(),
            );
            let i = self.add(nd, format!("recency over the last {w} symbols"));
            self.recency[r].1 = Some(i);
        }
    }

    /// Add a content symbol to the story state and reselect the edge's rows: the `story` most salient
    /// symbols, weighted by salience.
    fn story_update(&mut self, sym: usize, t: usize) {
        let key = match &self.story_keys {
            Some(k) => k[t],
            None => {
                if sym + 1 >= self.vocab {
                    return;
                }
                sym as u32
            }
        };
        if (key as usize) < self.cfg.story_skip {
            return;
        }
        self.story_g *= 2f64.powf(1.0 / self.cfg.story_half);
        if self.story_g > 1e100 {
            self.story_sal.values_mut().for_each(|v| *v /= 1e100);
            self.story_g /= 1e100;
        }
        *self.story_sal.entry(key).or_insert(0.0) += self.story_g;
        let g = self.story_g;
        if t % 256 == 0 {
            // forget keys that have faded below a thousandth of a fresh one
            self.story_sal.retain(|_, v| *v >= g * 1e-3);
        }
        let mut top: Vec<(u32, f64)> = if self.cfg.story_burst {
            // expected salience of a key at its long-run rate r: r * sum_k 2^(-k/half) = r / (1 - 2^(-1/half))
            *self.story_cnt.entry(key).or_insert(0.0) += 1.0;
            self.story_tot += 1.0;
            let h = 1.0 / (1.0 - 2f64.powf(-1.0 / self.cfg.story_half));
            let tot = self.story_tot;
            let cnt = &self.story_cnt;
            self.story_sal
                .iter()
                .map(|(&i, &v)| {
                    (
                        i,
                        (v / g) / (cnt.get(&i).copied().unwrap_or(1.0) / tot * h) * g,
                    )
                })
                .filter(|&(_, r)| r > g)
                .collect()
        } else {
            self.story_sal.iter().map(|(&i, &v)| (i, v)).collect()
        };
        let k = self.cfg.story.min(top.len());
        if k < top.len() {
            top.select_nth_unstable_by(k, |a, b| b.1.partial_cmp(&a.1).unwrap());
            top.truncate(k);
        }
        let e = self.story_edge.unwrap();
        self.slab[e].set = top.iter().map(|&(i, v)| (i, (v / g) as f32)).collect();
    }

    /// Set copy channel j's (symbol, score) set for this tick, creating the channel if needed.
    pub fn set_copy(&mut self, j: usize, set: Vec<(u32, f64)>) {
        while self.copy_sets.len() <= j {
            self.copy_sets.push(vec![]);
            self.copy_b0.push(0.0);
            self.copy_w.push(HashMap::new());
            self.copy_v.push(0.01);
            self.copy_pos.push(vec![]);
        }
        self.copy_sets[j] = set;
    }

    /// The copy channels' gains: (base, per previous cell), for reports.
    pub fn copy_gains(&self) -> Vec<(f64, Vec<(i32, f64)>)> {
        (0..self.copy_b0.len())
            .map(|j| {
                let mut v: Vec<(i32, f64)> = self.copy_w[j].iter().map(|(&k, &x)| (k, x)).collect();
                v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
                (self.copy_b0[j], v)
            })
            .collect()
    }

    /// Set the rows external edge j reads and writes this tick (row key, weight).
    pub fn set_ext(&mut self, j: usize, set: Vec<(u32, f32)>) {
        let e = self.ext[j];
        self.slab[e].set = set;
    }

    /// Give symbol `sym`'s cell a fresh code and generation (a slot handed to a new word), updating the
    /// target index for that one cell. Does nothing if the symbol has no cell yet.
    pub fn renew_sym(&mut self, sym: u32) {
        if let Some(&c) = self.byte.sym.get(&sym) {
            let old = self.byte.pool.codes[c];
            self.byte.pool.new_code(c, &mut self.rng);
            let new = self.byte.pool.codes[c];
            if self.alive_dirty {
                self.refresh_alive();
            } else {
                self.tix.change_cell(c, &old, &new);
            }
        }
    }

    /// The recency window of node n, if it is a recency candidate.
    fn recency_window(&self, n: usize) -> Option<usize> {
        self.recency.iter().find(|r| r.1 == Some(n)).map(|r| r.0)
    }

    fn gc_inner(&mut self) {
        let t = self.t;
        let mut used: HashSet<usize> = self
            .kept
            .iter()
            .chain(self.trials.iter())
            .map(|&e| self.slab[e].src)
            .collect();
        let mut changed = true;
        while changed {
            changed = false;
            for &n in &self.derived {
                if used.contains(&n) {
                    for &p in &self.nodes[n].parts {
                        if used.insert(p) {
                            changed = true;
                        }
                    }
                }
            }
        }
        for n in self.derived.clone() {
            let nd = &self.nodes[n];
            // the position nodes are senses, not learned structure: never collected
            if !used.contains(&n) && self.tried.contains(&nd.name) && t - nd.born > 15000 && !matches!(nd.kind, Kind::Pos(..)) {
                let (name, kind) = (nd.name.clone(), nd.kind.label().to_string());
                self.derived.retain(|&x| x != n);
                self.by_name.remove(&name);
                self.nodes[n].removed = true;
                self.scr.remove(&name);
                self.backoff.remove(&name);
                // free memory the node no longer needs, but keep its frozen state
                self.nodes[n].h = Hist::freed();
                self.free_kind(n);
                self.events.push(Event {
                    t,
                    kind: "gc".into(),
                    a: name,
                    b: Some(kind),
                    v: None,
                });
            }
        }
    }

    /// Drop the parts of a removed node that only matching needs: masks, index and caches. Its codes and
    /// generations stay, since live nodes may still read its frozen state.
    fn free_kind(&mut self, n: usize) {
        match &mut self.nodes[n].kind {
            Kind::BindReal(b) => {
                b.pool.freeze();
                b.cache = HashMap::new();
            }
            Kind::ChunkReal(k, _) => {
                k.pool.freeze();
                k.cache = HashMap::new();
            }
            Kind::BindSym(b) => b.map = HashMap::new(),
            Kind::ChunkSym(k, _) => k.map = HashMap::new(),
            _ => {}
        }
    }

    // ---------------------------------------------------------------- run and report
    pub fn run(&mut self) {
        let every: usize = std::env::var("EVOLVEHD_PROGRESS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let t0 = std::time::Instant::now();
        while self.t < self.tt {
            self.tick();
            if every > 0 && self.t % every == 0 {
                let cells: usize = self
                    .derived
                    .iter()
                    .map(|&n| match &self.nodes[n].kind {
                        Kind::BindReal(b) => b.pool.cap,
                        Kind::ChunkReal(k, _) => k.pool.cap,
                        _ => 0,
                    })
                    .sum();
                eprintln!(
                    "t={} bpb(last {every})={:.3} kept={} cells={} secs={:.0}",
                    self.t,
                    self.bpb(self.t - every, self.t),
                    self.kept.len(),
                    cells,
                    t0.elapsed().as_secs_f64()
                );
            }
        }
    }

    /// Approximate memory in MB: (writer-mask rows, their count, node histories, matcher pools, per-tick arrays).
    pub fn mem_mb(&self) -> (f64, usize, f64, f64, f64) {
        let mb = 1.0 / (1024.0 * 1024.0);
        let mut rows = 0usize;
        let mut row_b = 0usize;
        for e in &self.slab {
            for r in e.rows.values() {
                rows += 1;
                row_b += r.masks.len() * std::mem::size_of::<Bits>()
                    + r.counts.len()
                        * std::mem::size_of_val(&r.counts.first().copied().unwrap_or_default())
                    + 96;
            }
        }
        let hist = self.nodes.iter().filter(|n| !n.removed).count() * RING * 4;
        let pool_b = |p: &Pool| p.cap * (std::mem::size_of::<Bits>() + 64 + 40);
        let mut pools = pool_b(&self.byte.pool);
        for n in &self.nodes {
            if n.removed {
                pools += match &n.kind {
                    Kind::BindReal(b) => b.pool.cap * 72,
                    Kind::ChunkReal(k, _) => k.pool.cap * 72,
                    Kind::BindSym(b) => b.codes.len() * 64,
                    Kind::ChunkSym(k, _) => k.codes.len() * 64,
                    _ => 0,
                };
                continue;
            }
            match &n.kind {
                Kind::BindReal(b) => pools += pool_b(&b.pool) + b.cache.len() * 48,
                Kind::ChunkReal(k, _) => pools += pool_b(&k.pool) + k.cache.len() * 56,
                Kind::BindSym(b) => pools += b.codes.len() * 64 + b.map.len() * 40,
                Kind::ChunkSym(k, _) => pools += k.codes.len() * 64 + k.map.len() * 40,
                _ => {}
            }
        }
        let tick = self.tt * (8 * 4 + 2 + 4 + 1);
        (
            row_b as f64 * mb,
            rows,
            hist as f64 * mb,
            pools as f64 * mb,
            tick as f64 * mb,
        )
    }

    /// Mean bits per byte over [lo, hi).
    pub fn bpb(&self, lo: usize, hi: usize) -> f64 {
        let hi = hi.min(self.t);
        self.code_bits()[lo..hi].iter().sum::<f64>() / (hi - lo) as f64
    }
    /// The code length per tick as reported: after the learned floor with --floor, else the raw bits.
    pub fn code_bits(&self) -> &[f64] {
        if self.cfg.floor || self.cfg.copy_act { &self.coded } else { &self.bits }
    }
    /// --copy-act: code byte t (raw level cost `raw` bits) through the mixture of the level, the copy act and a
    /// uniform guess, then move the act on (read on if right, else seek) and record the context.
    fn copy_code(&mut self, t: usize, raw: f64) -> f64 {
        const K: usize = 5;
        const TB: usize = 22;
        const NB: usize = 18 * 4; // buckets: 0 = no offer, 1..=17 by match length; times 4 by the act's recent record
        const SH: f64 = 1.0 / 1000.0;
        if self.copy_tab.is_empty() {
            self.copy_tab = vec![u32::MAX; 1 << TB];
            self.copy_cal = vec![(0, 0); NB];
            self.copy_mw = vec![[1.0 / 3.0; 3]; NB];
        }
        let y = self.data[t];
        let offer = self.copy_ptr.filter(|&p| p < t).map(|p| self.data[p]);
        let bk = if offer.is_some() { 1 + self.copy_len.min(32) / 2 } else { 0 };
        // the act's recent record: how many of its last 8 offers were right (0-1, 2-4, 5-6, 7-8)
        let rc = match self.copy_hist.count_ones() { 0..=1 => 0, 2..=4 => 1, 5..=6 => 2, _ => 3 };
        let bk = bk.min(17) * 4 + if self.copy_ctx { rc } else { 0 };
        let u = 1.0 / 256.0;
        let pl = (-raw).exp2();
        let pc = match offer {
            Some(o) => {
                let (r, n) = self.copy_cal[bk];
                let c = (r as f64 + 0.5) / (n as f64 + 1.0);
                if o == y { c } else { (1.0 - c) / 255.0 }
            }
            None => u,
        };
        let pk = [pl, pc, u];
        let w = self.copy_mw[bk];
        let p: f64 = w.iter().zip(&pk).map(|(a, b)| a * b).sum();
        let mut nw = [0.0; 3];
        for i in 0..3 {
            nw[i] = w[i] * pk[i] / p;
        }
        for i in 0..3 {
            nw[i] = (1.0 - SH) * nw[i] + SH * (1.0 - nw[i]) / 2.0;
        }
        self.copy_mw[bk] = nw;
        // credit the act, then read on or seek
        if let Some(o) = offer {
            self.copy_stats[0] += 1;
            let e = &mut self.copy_cal[bk];
            e.1 += 1;
            if o == y {
                e.0 += 1;
                self.copy_stats[1] += 1;
            }
            if e.1 > self.copy_cap {
                e.0 >>= 1;
                e.1 >>= 1;
            }
            self.copy_hist = (self.copy_hist << 1 | (o == y) as u8 as u32) & 0xFF;
            if o == y {
                self.copy_ptr = self.copy_ptr.map(|p| p + 1);
                self.copy_len += 1;
            } else if self.copy_keep {
                // keep reading on past a wrong byte (a changed address, a different value), with no trust from
                // length; a better match found by the seek below replaces it
                self.copy_ptr = self.copy_ptr.map(|p| p + 1);
                self.copy_len = 0;
                self.copy_miss += 1;
                if self.copy_miss > 8 {
                    self.copy_ptr = None;
                    self.copy_miss = 0;
                }
            } else {
                self.copy_ptr = None;
                self.copy_len = 0;
            }
            if o == y {
                self.copy_miss = 0;
            }
        }
        if t + 1 >= K {
            let mut h: u64 = 0;
            for &b in &self.data[t + 1 - K..=t] {
                h = (h ^ b as u64).wrapping_mul(0x100000001B3);
            }
            let slot = (h >> (64 - TB)) as usize;
            if self.copy_ptr.is_none() || (self.copy_keep && self.copy_len < 16) {
                let q = self.copy_tab[slot];
                if q != u32::MAX && (q as usize) <= t {
                    // check the match backwards: how many bytes before q equal the bytes up to t
                    let q = q as usize;
                    let mut l = 0;
                    while l < 32 && l < q && self.data[q - 1 - l] == self.data[t - l] {
                        l += 1;
                    }
                    if l >= K && (self.copy_ptr.is_none() || l > self.copy_len) {
                        self.copy_ptr = Some(q);
                        self.copy_miss = 0;
                        self.copy_len = l;
                        self.copy_stats[2] += 1;
                    }
                }
            }
            self.copy_tab[slot] = (t + 1) as u32;
        }
        -p.log2()
    }
    /// --floor: code a tick whose raw cost is `raw` bits through the switching mixture over uniform weights,
    /// then update the mixture (prequential: the weights used come from the ticks before).
    fn floor_code(&mut self, raw: f64) -> f64 {
        const L: [f64; 9] = [1.0 / 4096.0, 1.0 / 512.0, 1.0 / 128.0, 1.0 / 32.0, 1.0 / 8.0, 0.25, 0.5, 0.75, 0.9375];
        const SHARE: f64 = 1.0 / 1000.0;
        if self.floor_w.is_empty() {
            self.floor_w = vec![1.0 / L.len() as f64; L.len()];
        }
        let pm = (-raw).exp2();
        let u = 1.0 / self.vocab as f64;
        let pk: Vec<f64> = L.iter().map(|&l| (1.0 - l) * pm + l * u).collect();
        let p: f64 = self.floor_w.iter().zip(&pk).map(|(w, q)| w * q).sum();
        let k = L.len() as f64;
        for (w, q) in self.floor_w.iter_mut().zip(&pk) {
            *w *= q / p;
        }
        let tot: f64 = self.floor_w.iter().sum();
        for w in self.floor_w.iter_mut() {
            let x = *w / tot;
            *w = (1.0 - SHARE) * x + SHARE * (1.0 - x) / (k - 1.0);
        }
        -p.log2()
    }

    pub fn summary_json(&self, blocks: usize, secs: f64) -> String {
        let tt = self.t;
        let mut o = String::from("{");
        let bl: Vec<String> = (0..tt)
            .step_by(blocks)
            .map(|i| json_f(self.bpb(i, i + blocks)))
            .collect();
        let _ = write!(o, "\"bpb\":[{}],", bl.join(","));
        let ac: Vec<String> = (0..tt)
            .step_by(blocks)
            .map(|i| {
                let hi = (i + blocks).min(tt);
                json_f(self.hit[i..hi].iter().filter(|&&x| x).count() as f64 / (hi - i) as f64)
            })
            .collect();
        let _ = write!(o, "\"acc\":[{}],", ac.join(","));
        let bm: Vec<String> = (0..tt)
            .step_by(blocks)
            .map(|i| {
                let hi = (i + blocks).min(tt);
                json_f(self.bits_map[i..hi].iter().sum::<f64>() / (hi - i) as f64)
            })
            .collect();
        let _ = write!(o, "\"bpb_map\":[{}],", bm.join(","));
        let nv: Vec<String> = (0..tt)
            .step_by(blocks)
            .map(|i| {
                self.novel[i..(i + blocks).min(tt)]
                    .iter()
                    .filter(|&&x| x)
                    .count()
                    .to_string()
            })
            .collect();
        let _ = write!(o, "\"novel\":[{}],", nv.join(","));
        let ev: Vec<String> = self
            .events
            .iter()
            .map(|e| {
                let mut s = format!("[{},{},{}", e.t, json_str(&e.kind), json_str(&e.a));
                if let Some(b) = &e.b {
                    let _ = write!(s, ",{}", json_str(b));
                }
                if let Some(v) = e.v {
                    let _ = write!(s, ",{}", json_f((v * 1e4).round() / 1e4));
                }
                s.push(']');
                s
            })
            .collect();
        let _ = write!(o, "\"events\":[{}],", ev.join(","));
        let nd: Vec<String> = self
            .derived
            .iter()
            .map(|&n| {
                let x = &self.nodes[n];
                let parts: Vec<String> = x
                    .parts
                    .iter()
                    .map(|&p| json_str(&self.nodes[p].name))
                    .collect();
                let mut s = format!(
                    "{{\"name\":{},\"kind\":{},\"parts\":[{}],\"born\":{},\"extra\":{}",
                    json_str(&x.name),
                    json_str(x.kind.label()),
                    parts.join(","),
                    x.born,
                    json_str(&x.extra)
                );
                match &x.kind {
                    Kind::BindReal(b) => {
                        let _ = write!(
                            s,
                            ",\"cells\":{},\"evictions\":{}",
                            b.pool.n_alive(),
                            b.pool.evictions
                        );
                    }
                    Kind::BindSym(b) => {
                        let _ = write!(s, ",\"cells\":{},\"evictions\":0", b.map.len());
                    }
                    Kind::ChunkReal(k, _) => {
                        let _ = write!(
                            s,
                            ",\"cells\":{},\"evictions\":{}",
                            k.pool.n_alive(),
                            k.pool.evictions
                        );
                    }
                    Kind::ChunkSym(k, _) => {
                        let _ = write!(s, ",\"cells\":{},\"evictions\":0", k.map.len());
                    }
                    _ => {}
                }
                s.push('}');
                s
            })
            .collect();
        let _ = write!(o, "\"nodes\":[{}],", nd.join(","));
        let kp: Vec<String> = self
            .kept
            .iter()
            .map(|&e| {
                let k = self.name_of_edge(e);
                format!(
                    "{}:{{\"beta\":{},\"gain_at_keep\":{}}}",
                    json_str(&k),
                    json_f((self.slab[e].beta * 1e3).round() / 1e3),
                    json_f((self.gain_at_keep.get(&k).copied().unwrap_or(0.0) * 1e4).round() / 1e4)
                )
            })
            .collect();
        let _ = write!(o, "\"kept\":{{{}}},", kp.join(","));
        // byte-pool purity over the last 50k ticks
        let lo = tt.saturating_sub(50000);
        let cells: Vec<i64> = if tt > lo {
            self.nodes[0]
                .h
                .range(lo, tt - 1)
                .iter()
                .map(|&x| x as i64 + 1)
                .collect()
        } else {
            vec![]
        };
        let by: Vec<i64> = self.syms[lo..tt].iter().map(|&x| x as i64).collect();
        let (hc, hb, hj) = (ent(&[&cells]), ent(&[&by]), ent(&[&cells, &by]));
        let _ = write!(
            o,
            "\"info\":{{\"recruits\":{},\"byte_cells\":{},\"merged\":{},\"nmi_cell_byte\":{},\"H_cell_given_byte\":{}}},",
            self.byte.pool.recruits,
            self.byte.pool.n_alive(),
            self.merged,
            json_f((hc + hb - hj) / hb.max(1e-9)),
            json_f(hj - hb)
        );
        for (k, v) in &self.extra_json {
            let _ = write!(o, "{}:{},", json_str(k), v);
        }
        let _ = write!(o, "\"ticks\":{},\"secs\":{}}}", tt, json_f(secs));
        o
    }
}

/// Witten-Bell counts for one context: total and per next value.
#[derive(Default)]
struct WbCtx {
    n: u32,
    d: HashMap<i32, u32>,
}

impl WbCtx {
    fn p(&self, y: i32, lower: f64) -> f64 {
        let t = self.d.len() as f64;
        (*self.d.get(&y).unwrap_or(&0) as f64 + t * lower) / (self.n as f64 + t)
    }
    fn add(&mut self, y: i32) {
        self.n += 1;
        *self.d.entry(y).or_insert(0) += 1;
    }
    /// Halve every count; false if nothing is left.
    fn halve(&mut self) -> bool {
        self.d.retain(|_, c| {
            *c /= 2;
            *c > 0
        });
        self.n = self.d.values().sum();
        self.n > 0
    }
}

/// What a running screen measures.
pub enum Cand {
    /// A virtual exact chunk at this z (trie over (previous id or start, byte)).
    Chunk {
        z: f64,
        prev: i32,
        map: HashMap<(i32, i32), i32>,
    },
    /// The chunk node's state latched at its last boundary.
    Prev { chunk: usize, z: f64, last: i32 },
    /// Same context, but measured beyond the current chunk instead of the current byte (log only).
    PrevGivenChunk { chunk: usize, z: f64, last: i32 },
    /// A node's state, measured beyond the byte (base None) or beyond another node's state.
    Node {
        node: usize,
        base: Option<usize>,
        label: &'static str,
    },
}

/// A prequential count screen: Witten-Bell p(y' | s, y) against p(y' | y), scored on each tick before
/// learning from it, from the moment the candidate is proposed. Its value is the mean gain over the
/// most recent `window` ticks, so it can use far more data than a stored window.
pub struct RunScreen {
    pub cand: Cand,
    pub born: usize,
    window: usize,
    base: HashMap<i32, WbCtx>,
    ctx: HashMap<(i32, i32), WbCtx>,
    uni: Vec<f64>,
    uni_n: f64,
    pending: Option<(i32, i32)>,
    fed: u64,
    blocks: std::collections::VecDeque<(f64, usize)>,
    /// Score the gain of a mixture (1 - lam) p(y'|b) + lam p(y'|s,b), with lam learned online, instead of
    /// backing off from the joint context alone (which charges heavily for contexts seen only a few times).
    mix: bool,
    lam: f64,
}

impl RunScreen {
    pub fn new(cand: Cand, born: usize, window: usize) -> Self {
        RunScreen {
            cand,
            born,
            window,
            base: HashMap::new(),
            ctx: HashMap::new(),
            uni: vec![1.0; 257],
            uni_n: 257.0,
            pending: None,
            fed: 0,
            blocks: Default::default(),
            mix: false,
            lam: 0.1,
        }
    }

    pub fn make(mix: bool, cand: Cand, born: usize, window: usize) -> Self {
        let mut r = Self::new(cand, born, window);
        r.mix = mix;
        r
    }

    /// Feed tick t: y = byte state at t (the outcome for the pending pair), s = the context now,
    /// b = the baseline context now (usually y itself).
    fn feed(&mut self, y: i32, s: i32, b: i32) {
        if let Some((sp, yp)) = self.pending.take() {
            if y >= 0 {
                let yi = y as usize;
                if yi >= self.uni.len() {
                    self.uni_n += (yi + 1 - self.uni.len()) as f64;
                    self.uni.resize(yi + 1, 1.0);
                }
                let mut pb = self.uni[yi] / self.uni_n;
                if let Some(c) = self.base.get(&yp) {
                    pb = c.p(y, pb);
                }
                let mut ps = pb;
                if let Some(c) = self.ctx.get(&(sp, yp)) {
                    ps = c.p(y, pb);
                }
                let g = if self.mix {
                    // online EM step on the mixture weight, after scoring with the weight as it was
                    let m = (1.0 - self.lam) * pb + self.lam * ps;
                    let post = self.lam * ps / m;
                    self.lam = (self.lam + 0.001 * (post - self.lam)).clamp(1e-4, 1.0 - 1e-4);
                    m.log2() - pb.log2()
                } else {
                    ps.log2() - pb.log2()
                };
                match self.blocks.back_mut() {
                    Some(b) if b.1 < 1000 => {
                        b.0 += g;
                        b.1 += 1;
                    }
                    _ => self.blocks.push_back((g, 1)),
                }
                while self.blocks.len() * 1000 > self.window + 1000 {
                    self.blocks.pop_front();
                }
                self.base.entry(yp).or_default().add(y);
                self.ctx.entry((sp, yp)).or_default().add(y);
                self.uni[yi] += 1.0;
                self.uni_n += 1.0;
                self.fed += 1;
                if self.fed % 1_000_000 == 0 {
                    // exponential forgetting keeps the tables to recently seen contexts
                    self.base.retain(|_, c| c.halve());
                    self.ctx.retain(|_, c| c.halve());
                }
            }
        }
        self.pending = Some((s, b));
    }

    /// Mean gain in bits per tick over the recent window, and the ticks it covers.
    pub fn value(&self) -> (f64, usize) {
        let (g, n) = self
            .blocks
            .iter()
            .fold((0.0, 0), |a, b| (a.0 + b.0, a.1 + b.1));
        (if n > 0 { g / n as f64 } else { 0.0 }, n)
    }
}

// -------------------------------------------------------------------- word moves
impl Brain {
    /// Advance the running screens by one tick (called after the nodes update).
    fn feed_screens(&mut self, t: usize) {
        if self.run_scr.is_empty() {
            return;
        }
        let y = self.nodes[0].state;
        let mut rs = std::mem::take(&mut self.run_scr);
        for r in rs.iter_mut() {
            let s = match &mut r.cand {
                Cand::Chunk { z, prev, map } => {
                    if y < 0 {
                        *prev = -1;
                        -1
                    } else {
                        let k = (
                            if self.boundary(t, *z) || *prev < 0 {
                                -1
                            } else {
                                *prev
                            },
                            y,
                        );
                        let n = map.len() as i32;
                        *prev = *map.entry(k).or_insert(n);
                        *prev
                    }
                }
                Cand::Prev { chunk, z, last } | Cand::PrevGivenChunk { chunk, z, last } => {
                    if t >= 1 && self.boundary(t, *z) && !self.nodes[*chunk].removed {
                        *last = self.nodes[*chunk].h[t - 1];
                    }
                    *last
                }
                Cand::Node { node, .. } => self.nodes[*node].state,
            };
            let b = match &r.cand {
                Cand::PrevGivenChunk { chunk, .. } => self.nodes[*chunk].state,
                Cand::Node { base: Some(n), .. } => self.nodes[*n].state,
                _ => y,
            };
            r.feed(y, s, b);
        }
        self.run_scr = rs;
    }

    /// The chunk ids a trie would give over the last w ticks (a virtual screen, nothing built).
    fn virtual_chunk(&self, z: f64, w: usize) -> Vec<i64> {
        let t = self.t;
        let mut ids = vec![-1i64; w];
        let mut m: HashMap<(i64, i64), i64> = HashMap::new();
        let mut prev: i64 = -1;
        for (i, tt) in (t + 1 - w..=t).enumerate() {
            let b = self.nodes[0].h[tt] as i64;
            if b < 0 {
                prev = -1;
                continue;
            }
            let k = (
                if self.boundary(tt, z) || prev < 0 {
                    -1
                } else {
                    prev
                },
                b,
            );
            let n = m.len() as i64;
            prev = *m.entry(k).or_insert(n);
            ids[i] = prev;
        }
        ids
    }

    fn make_chunk(&mut self, z: f64) -> Node {
        let t = self.t;
        let name = format!("chunk(z={z})");
        let mut h = Hist::new();
        let kind = if self.cfg.real {
            let w = (t + 1).min(20000);
            let distinct: HashSet<i64> = self
                .virtual_chunk(z, w)
                .into_iter()
                .filter(|&v| v >= 0)
                .collect();
            let cap = self.sized_cap(self.cfg.chunk_cap, distinct.len());
            let mut pool = Pool::new(cap, 48, 40);
            pool.hebb = self.cfg.hebb;
            let start = bits::rand_code(&mut self.rng);
            let mut k = ChunkReal {
                pool,
                start,
                cache: HashMap::new(),
                code_key: vec![None; cap],
            };
            let mut prev = -1;
            for tt in (t + 1).saturating_sub(20000)..=t {
                let b = self.nodes[0].h[tt];
                let (bg, bc) = if b >= 0 {
                    (self.gen_of(0, b), self.code_of(0, b))
                } else {
                    (0, [0; bits::CK])
                };
                prev = k.step(prev, b, bg, &bc, self.boundary(tt, z), tt, &mut self.rng);
                h[tt] = prev;
            }
            Kind::ChunkReal(Box::new(k), z)
        } else {
            let seed = self.rng.next_u64();
            let mut k = ChunkSym {
                map: HashMap::new(),
                codes: vec![],
                rng: Rng::new(seed),
            };
            let mut prev = -1;
            for tt in (t + 1).saturating_sub(40000)..=t {
                prev = k.step(prev, self.nodes[0].h[tt], self.boundary(tt, z));
                h[tt] = prev;
            }
            Kind::ChunkSym(Box::new(k), z)
        };
        let mut nd = self.new_node(name, kind, vec![0], format!("{z}"));
        nd.state = h[t];
        nd.h = h;
        nd
    }

    fn make_prevword(&mut self, c: usize, z: f64) -> Node {
        let t = self.t;
        let mut h = Hist::new();
        let mut st = -1;
        for tt in (t + 1).saturating_sub(40000).max(1)..=t {
            if self.boundary(tt, z) {
                st = self.nodes[c].h[tt - 1];
            }
            h[tt] = st;
        }
        let mut nd = self.new_node(
            format!("prevword[{}]", self.nodes[c].name),
            Kind::PrevWord(z),
            vec![c],
            format!("{z}"),
        );
        nd.state = st;
        nd.h = h;
        nd
    }

    fn make_pclass(&mut self, c: usize, z: f64) -> Node {
        let t = self.t;
        let k = 64;
        let codes = (0..=k).map(|_| bits::rand_code(&mut self.rng)).collect();
        let mut cc = ChunkClass {
            k,
            codes,
            succ: HashMap::new(),
            seen: HashMap::new(),
            assign: HashMap::new(),
            cent: None,
            pending2: -1,
            last: -1,
            texts: HashMap::new(),
            last_cut: 0,
        };
        let lo = (t + 1).saturating_sub(200_000).max(1);
        for tt in lo..=t {
            let done = if self.boundary(tt, z) {
                Some(self.nodes[c].h[tt - 1])
            } else {
                None
            };
            if let Some(dc) = done {
                if tt > cc.last_cut && tt - cc.last_cut <= 40 && tt <= self.data.len() {
                    cc.texts.insert(dc, self.data[cc.last_cut..tt].to_vec());
                }
                cc.last_cut = tt;
            }
            cc.observe(self.nodes[0].h[tt], done);
        }
        cc.refit(&mut self.rng);
        let mut h = Hist::new();
        let mut last = -1;
        for tt in (t + 1).saturating_sub(40000).max(1)..=t {
            if self.boundary(tt, z) {
                last = self.nodes[c].h[tt - 1];
            }
            h[tt] = cc.class(last);
        }
        cc.last = last;
        let mut nd = self.new_node(
            format!("class[{}]", self.nodes[c].name),
            Kind::PrevClass(Box::new(cc), z),
            vec![c],
            format!("{k}"),
        );
        nd.state = h[t];
        nd.h = h;
        nd
    }

    fn chunk_z(&self, c: usize) -> f64 {
        match &self.nodes[c].kind {
            Kind::ChunkReal(_, z) | Kind::ChunkSym(_, z) => *z,
            _ => unreachable!(),
        }
    }

    fn word_moves(&mut self, w: usize) {
        if self.cfg.run_window > 0 {
            return self.word_moves_running();
        }
        let t = self.t;
        let y = self.series(0, w);
        let (yp, yn) = (&y[..w - 1], &y[1..]);
        let thr = self.thr(self.cfg.tmp_thr);
        match self.chunk {
            None => {
                let mut best: Option<(f64, f64)> = None;
                for z in [0.5, 1.0, 1.5] {
                    let s: Vec<i64> = self.virtual_chunk(z, w)[..w - 1]
                        .iter()
                        .map(|x| x + 1)
                        .collect();
                    let v = if self.cfg.screen_mix {
                        heldout_gain_mix(&s, yp, yn)
                    } else {
                        heldout_gain(&s, yp, yn)
                    };
                    self.events.push(Event {
                        t,
                        kind: "screen-chunk".into(),
                        a: format!("z={z}"),
                        b: None,
                        v: Some(v),
                    });
                    if v > thr && best.map_or(true, |b| v > b.0) {
                        best = Some((v, z));
                    }
                }
                if let Some((v, z)) = best {
                    let nd = self.make_chunk(z);
                    let i = self.add(nd, format!("screen byte {v:.2}"));
                    self.chunk = Some(i);
                }
            }
            Some(c) if self.prev.is_none() => {
                let z = self.chunk_z(c);
                let ch = self.nodes[c].h.range(t + 1 - w, t);
                let mut last: i64 = -1;
                let mut s = Vec::with_capacity(w);
                for (j, tt) in (t + 1 - w..=t).enumerate() {
                    if self.boundary(tt, z) {
                        last = j as i64;
                    }
                    s.push(if last >= 1 {
                        ch[last as usize - 1] as i64 + 1
                    } else {
                        0
                    });
                }
                let v = if self.cfg.screen_mix {
                    heldout_gain_mix(&s[..w - 1], yp, yn)
                } else {
                    heldout_gain(&s[..w - 1], yp, yn)
                };
                let name = self.nodes[c].name.clone();
                self.events.push(Event {
                    t,
                    kind: "screen-prevword".into(),
                    a: name,
                    b: None,
                    v: Some(v),
                });
                if v > thr {
                    let nd = self.make_prevword(c, z);
                    let i = self.add(nd, format!("screen byte {v:.2}"));
                    self.prev = Some(i);
                }
            }
            _ => {}
        }
    }

    fn word_moves_running(&mut self) {
        let t = self.t;
        let thr = self.thr(self.cfg.tmp_thr);
        match self.chunk {
            None => {
                if self
                    .run_scr
                    .iter()
                    .any(|r| matches!(r.cand, Cand::Prev { .. }))
                {
                    self.run_scr.clear(); // the chunk was cleaned up: screen chunks again from now
                    return;
                }
                let mut best: Option<(f64, f64)> = None;
                for r in &self.run_scr {
                    if let Cand::Chunk { z, .. } = r.cand {
                        let (v, n) = r.value();
                        self.events.push(Event {
                            t,
                            kind: "screen-chunk".into(),
                            a: format!("z={z}"),
                            b: Some(format!("n={n}")),
                            v: Some(v),
                        });
                        if n >= 5000 && v > thr && best.map_or(true, |b| v > b.0) {
                            best = Some((v, z));
                        }
                    }
                }
                if let Some((v, z)) = best {
                    let nd = self.make_chunk(z);
                    let i = self.add(nd, format!("screen byte {v:.2}"));
                    self.chunk = Some(i);
                    self.run_scr = vec![
                        RunScreen::make(
                            self.cfg.screen_mix,
                            Cand::Prev {
                                chunk: i,
                                z,
                                last: -1,
                            },
                            t,
                            self.cfg.run_window,
                        ),
                        RunScreen::make(
                            self.cfg.screen_mix,
                            Cand::PrevGivenChunk {
                                chunk: i,
                                z,
                                last: -1,
                            },
                            t,
                            self.cfg.run_window,
                        ),
                    ];
                }
            }
            Some(c) => {
                let name = self.nodes[c].name.clone();
                let (v, n) = match self.run_scr.first() {
                    Some(r) => r.value(),
                    None => return,
                };
                self.events.push(Event {
                    t,
                    kind: "screen-prevword".into(),
                    a: name.clone(),
                    b: Some(format!("n={n}")),
                    v: Some(v),
                });
                let mut v2 = f64::NEG_INFINITY;
                if let Some(r) = self.run_scr.get(1) {
                    let n2;
                    (v2, n2) = r.value();
                    self.events.push(Event {
                        t,
                        kind: "screen-prev-given-chunk".into(),
                        a: name,
                        b: Some(format!("n={n2}")),
                        v: Some(v2),
                    });
                }
                for r in self.run_scr.iter().skip(2) {
                    if let Cand::Node { node, label, .. } = r.cand {
                        let (v3, n3) = r.value();
                        self.events.push(Event {
                            t,
                            kind: label.into(),
                            a: self.nodes[node].name.clone(),
                            b: Some(format!("n={n3}")),
                            v: Some(v3),
                        });
                    }
                }
                // Conjoin with the level above: its predicted symbol, measured beyond the chunk.
                if let Some(u) = self.upper {
                    if !self.run_scr.iter().any(|r| {
                        matches!(
                            r.cand,
                            Cand::Node {
                                label: "screen-upper-given-chunk",
                                ..
                            }
                        )
                    }) {
                        self.run_scr.push(RunScreen::make(
                            self.cfg.screen_mix,
                            Cand::Node {
                                node: u,
                                base: Some(c),
                                label: "screen-upper-given-chunk",
                            },
                            t,
                            self.cfg.run_window,
                        ));
                    }
                    let uv = self
                        .run_scr
                        .iter()
                        .find(|r| {
                            matches!(
                                r.cand,
                                Cand::Node {
                                    label: "screen-upper-given-chunk",
                                    ..
                                }
                            )
                        })
                        .map(|r| r.value());
                    if let Some((v3, n3)) = uv {
                        let (a, b) = (self.nodes[u].name.clone(), self.nodes[c].name.clone());
                        if self.cfg.conjoin_words
                            && v3 > thr
                            && n3 >= 20000
                            && self.derived.len() < self.cfg.max_derived
                            && self.live(&format!("({a}*{b})")).is_none()
                            && self.live(&format!("({b}*{a})")).is_none()
                        {
                            self.bind(u, c, format!("conjoin: upper given chunk {v3:.2}"));
                        }
                    }
                }
                // Compress: the previous chunk carries information, but its edge failed to learn it
                // (too many states for one edge): group chunks into classes by what follows them.
                let pname = format!("prevword[{}]", self.nodes[c].name);
                let failed = self
                    .events
                    .iter()
                    .any(|e| e.kind == "reject" && e.b.as_deref() == Some(pname.as_str()));
                let kept_prev = self
                    .kept
                    .iter()
                    .any(|&e| self.nodes[self.slab[e].src].name == pname);
                // the information is either additive (edge tried and failed) or only in conjunction with the chunk
                let pressure = (failed && v > thr) || (v2 > thr && n >= 20000);
                if self.cfg.classes
                    && self.pclass.is_none()
                    && !kept_prev
                    && pressure
                    && self.derived.len() < self.cfg.max_derived
                {
                    let z = self.chunk_z(c);
                    let nd = self.make_pclass(c, z);
                    let i = self.add(nd, format!("prev screens {v:.2} / given chunk {v2:.2}"));
                    self.pclass = Some(i);
                    self.run_scr.truncate(2);
                    self.run_scr.push(RunScreen::make(
                        self.cfg.screen_mix,
                        Cand::Node {
                            node: i,
                            base: None,
                            label: "screen-class",
                        },
                        t,
                        self.cfg.run_window,
                    ));
                    self.run_scr.push(RunScreen::make(
                        self.cfg.screen_mix,
                        Cand::Node {
                            node: i,
                            base: Some(c),
                            label: "screen-class-given-chunk",
                        },
                        t,
                        self.cfg.run_window,
                    ));
                }
                // Conjoin: the previous chunk informs only together with the current chunk -> bind the two.
                if self.cfg.conjoin_words
                    && v2 > thr
                    && n >= 20000
                    && self.derived.len() + 2 <= self.cfg.max_derived
                {
                    let pw = match self.prev {
                        Some(p) => Some(p),
                        None if self.pclass.is_none() => {
                            let z = self.chunk_z(c);
                            let nd = self.make_prevword(c, z);
                            let i = self.add(nd, format!("conjoin part, given chunk {v2:.2}"));
                            self.prev = Some(i);
                            Some(i)
                        }
                        None => None,
                    };
                    if let Some(pw) = pw {
                        let (a, b) = (self.nodes[pw].name.clone(), self.nodes[c].name.clone());
                        if self.live(&format!("({a}*{b})")).is_none()
                            && self.live(&format!("({b}*{a})")).is_none()
                        {
                            let before = self.derived.len();
                            self.bind(pw, c, format!("conjoin: prev given chunk {v2:.2}"));
                            if self.derived.len() > before {
                                // the conjoin's part must not be cleaned up while the bind is untried
                                self.tried.remove(&a);
                            }
                        }
                    }
                }
                if self.prev.is_none() && self.pclass.is_none() && n >= 5000 && v > thr {
                    let z = self.chunk_z(c);
                    let nd = self.make_prevword(c, z);
                    let i = self.add(nd, format!("screen byte {v:.2}"));
                    self.prev = Some(i);
                }
            }
        }
    }

    /// For the chunk-class node: each class's most frequent member chunks, as text.
    pub fn class_report(&self) -> Option<String> {
        let i = self.pclass?;
        let k = match &self.nodes[i].kind {
            Kind::PrevClass(k, _) => k,
            _ => return None,
        };
        let mut by: Vec<Vec<(u32, i32)>> = vec![vec![]; k.k];
        for (&c, &a) in &k.assign {
            by[a].push((*k.seen.get(&c).unwrap_or(&0), c));
        }
        let mut out = vec![];
        for (j, mut m) in by.into_iter().enumerate() {
            m.sort_unstable_by(|a, b| b.cmp(a));
            let tot: u32 = m.iter().map(|x| x.0).sum();
            let ex: Vec<String> = m
                .iter()
                .take(12)
                .filter_map(|&(_, c)| k.texts.get(&c))
                .map(|b| json_str(&String::from_utf8_lossy(b)))
                .collect();
            out.push(format!(
                "{{\"class\":{j},\"members\":{},\"uses\":{tot},\"top\":[{}]}}",
                m.len(),
                ex.join(",")
            ));
        }
        Some(format!("[{}]", out.join(",")))
    }

    /// Boundary alignment with real word starts over [lo, hi), and how many chunks are one whole word.
    pub fn word_report(&self, lo: usize, hi: usize) -> Option<String> {
        if !self.bytes {
            return None;
        }
        let c = self.chunk?;
        let z = self.chunk_z(c);
        let hi = hi.min(self.t);
        let alpha = |b: u8| {
            (b as char).is_ascii_alphabetic() || b == b'\'' || (b >= 0xC0 && b != 0xD7 && b != 0xF7)
        };
        let isa =
            |b: u8| (b as char).is_ascii_alphabetic() || (b >= 0xC0 && b != 0xD7 && b != 0xF7);
        let d = &self.data;
        let (mut nb, mut ws, mut both) = (0usize, 0usize, 0usize);
        let mut starts = vec![];
        for t in lo..hi {
            let bnd = self.boundary(t, z);
            let wstart = alpha(d[t]) && !alpha(d[t - 1]);
            nb += bnd as usize;
            ws += wstart as usize;
            both += (bnd && wstart) as usize;
            if bnd {
                starts.push(t);
            }
        }
        let (mut whole, mut segs) = (0usize, 0usize);
        let mut ex = vec![];
        for p in starts.windows(2) {
            let (a, b) = (p[0], p[1]);
            let seg = &d[a..b];
            segs += 1;
            let mut e = seg.len();
            while e > 0 && b" \n,.;:!?\"-".contains(&seg[e - 1]) {
                e -= 1;
            }
            let core = &seg[..e];
            let starts_word = a == 0 || !alpha(d[a - 1]);
            let ends_word = seg.len() > core.len() || b >= d.len() || !isa(d[b]);
            if core.len() >= 2 && core.iter().all(|&x| isa(x)) && starts_word && ends_word {
                whole += 1;
            }
            if ex.len() < 40 {
                ex.push(json_str(&String::from_utf8_lossy(seg)));
            }
        }
        let mean_len = if starts.len() > 1 {
            (starts[starts.len() - 1] - starts[0]) as f64 / (starts.len() - 1) as f64
        } else {
            0.0
        };
        Some(format!(
            "{{\"z\":{z},\"boundaries\":{nb},\"word_starts\":{ws},\"precision\":{},\"recall\":{},\"segments\":{segs},\"whole_word_segments\":{},\"mean_seg_len\":{},\"examples\":[{}]}}",
            json_f(both as f64 / nb.max(1) as f64),
            json_f(both as f64 / ws.max(1) as f64),
            json_f(whole as f64 / segs.max(1) as f64),
            json_f(mean_len),
            ex.join(",")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grows_on_a_periodic_stream() {
        // "abcab" repeated with noise-free structure: needs more than the byte itself to predict
        let data: Vec<u8> = b"the cat sat on the mat. "
            .iter()
            .cycle()
            .take(30000)
            .copied()
            .collect();
        for real in [false, true] {
            let mut b = Brain::new(
                data.clone(),
                Cfg {
                    real,
                    ..Cfg::default()
                },
            );
            b.run();
            let late = b.bpb(25000, 30000);
            assert!(late < 0.5, "real={real} late bpb {late}");
            assert!(b.kept.len() > 1, "real={real}: nothing kept");
        }
    }
}
