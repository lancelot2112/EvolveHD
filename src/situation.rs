//! A learned situation model on ternary memory (experiment 34), the Rust port of `scripts/situation_slots.py`, with
//! relation classes pooled by meaning codes. Nothing about the words is given; inside, only bits and small
//! integers.
//!
//! 1. Clauses from learned boundaries: for each word u, the words before and after it at a stochastically kept
//!    earlier occurrence; hit counts are collision rates, and u is a boundary when lo^3 hi^2 > n^2 pair^3.
//! 2. Facts: each word w is bound to every earlier word s of its clause (back to and including the first
//!    boundary), both ways. The relation is the path of words between them, with its direction.
//! 3. Pooling: a path's code is the OR of its words' meaning codes (`Meaning`, a narrow-window instance, learned
//!    online), each rotated by its place in the path. Paths join the relation class whose prototype they overlap
//!    most (popcount, at least `pool` of the code), else start a class; prototypes are their members' most
//!    frequent bits. So "went to the" and "walked to the" become one relation once "went" and "walked" mean alike.
//!    Paths are re-assigned every `recluster` words as the meanings mature.
//! 4. Fast memory, ternary: a fact (s, class, w) is written at K active elements chosen from s *and* the class
//!    (one slot per entity and relation), phase key + value per element, adding +-mag to the real or imaginary
//!    part, clipped to -1, 0, +1 (mag 1: conflicting writes cancel to 0; mag 2: the newest sign wins). A slowly
//!    drifting context offsets the elements (a slot per learned boundary is redrawn).
//! 5. Slow memory: counts per (context, cue position) over classes (hop 1) and class pairs (hop 2). When the next
//!    word w arrives, each cue word's slots for the most frequent classes are read; a class whose slot ranks w
//!    first earns a count for that context. Hop 2 goes through the best value of each of the 10 most frequent
//!    classes. Prediction reads the slots of the context's best-counted classes.
//!
//! Experiment 36 (the expert forum's changes), each behind a switch, all integer:
//! - `kinds`: learned value kinds. Each word has 8-bit counters over its frames (the word before, the two words
//!   before, the word after), halved together when one saturates. (4-bit counters split the people into two
//!   kinds: a frame is a noisy observation, and a short window follows which verb a person used last.) Words are leader-clustered by the Jaccard
//!   overlap of their frame codes (counters at a quarter of their maximum or more), at least 2/5 by
//!   cross-multiplication, every `recluster` words. A kind keeps its name (a word id) across reclusterings.
//!   Slots are keyed on (entity, class, kind of the value), and C1/C2 count (class, kind). A slot of kind k is read
//!   against the words of kind k only (a typed clean-up), so an empty or noisy slot cannot return a word of
//!   another kind.
//! - `mood`: questions do not write. A clause's bindings are buffered until the boundary that closes it. Each word
//!   has a 4-bit counter of "after this boundary, the store's top proposal was the next word": up by 1 when it
//!   was, down by 1 on half the misses (a stochastic decrement from a hash bit, so it climbs above a third). A
//!   clause closed by a boundary whose counter is 8 or more is a cue, and its buffer is dropped.
//! - `recency`: among one cue word's proposals of one kind, those with at least half the best count go newest first
//!   (the slot's last write time, compared within the entity).
//! - `bounce`: reasoning as a loop of reads instead of the C1/C2 tables. At each step (at most 3) it either stops
//!   or reads one of the current entity's live slots and moves to the value read.
//!   - The slot is chosen by success counts per (context, cue position, step).
//!   - The stop is a 4-bit up/down counter per (context, kind of the current entity, step), stepped up where the
//!     answer was reached and down at an intermediate.
//! - `supersede`: when x's slot of kind k is written under one class, its other live slots of kind k in the same
//!   direction get the reserved NONE value (never proposed), so an entity keeps one live value per (direction,
//!   kind).
//! - `compose`: data bouncing at write time. When a fact (x, c, v) is committed and ((c, k), (c2, k2)) is among
//!   the `compose` most counted two-hop pairs, v's (c2, k2) slot is read then and written to x's composite slot,
//!   which C1 can learn like any class: "emma dropped the cup" also stores where emma is at that moment.
//! - `precision`: rank routes by Go minus NoGo, hits minus misses (a precision ranking, (hits + 1) / (hits + misses
//!   + 2), let rare lucky routes beat well-tested ones: where-person fell from 91% to 61-73%). A miss is
//!   counted for the routes behind the answer actually given (the top proposal) when the next word was something
//!   else: Go and NoGo, the forum's three-factor rule, credited like Select (only the taken action is seen). Ranking by hits alone lets a route through a function word win, because it lands on the
//!   most recent place by chance a third of the time and is never penalized.
//! - `entity_cues`: only words of the larger learned kinds (at least 3 members) are read from as cue entities. A
//!   function word's slots hold whatever was written next to it last ("the" -> the latest place), a recency
//!   buffer that wins by luck a third of the time.
//! - `live`: learn over the cue entity's live slots (two hops: the intermediate's too) instead of the most
//!   frequent classes. The two-hop search over the 10 most frequent classes never saw "the apple was got by",
//!   whose class is rarer.
//! - `query`: the general query learner, in place of the C1/C2 tables, the context hash, the cue positions, the
//!   entity-cue rule and the recency rule. A bitwise W_QK:
//!   - the query code is the last 4 words' frame codes, each in its position's block, plus identity bits, plus 4
//!     bits hashed from the words 1 and 3 back together (a conjunction: the question form, apart from its entity);
//!   - a route's key is a set of features: cue position, the cue word's kind, per hop the class, the kind and the
//!     slot's recency rank within its entity, and the number of hops;
//!   - W holds signed 8-bit weights per (query bit, feature);
//!   - every one- and two-hop route from the cue words' 8 newest live slots is scored by the sum of its weights
//!     under the query, and the best is read;
//!   - learning is mistake-driven and three-factor (a perceptron): when the top route was wrong, the best-scored
//!     route that held the next word is stepped up and the top route down. (Stepping up every route that held it
//!     instead fell to 52%: the frequent routes' weights swamp the rest.)
//! - `replay`: the query learner keeps its mistakes (the query, the routes as they were, the answer) in a ring of
//!   512 and re-learns this many per word, so the rarer two-hop questions train it as often as the common ones.
//! - `fpool`: relations pooled by the words' frame codes (the 8-bit frame counters of `kinds`, all integer)
//!   instead of the meaning module's codes (float inside); the meaning module is then not run. Only distinctive
//!   frame features count: those in at most an eighth of the words' codes (recounted at each recluster), so
//!   "went" and "got" (both after a name) differ by what follows them.
//! - `lookback` N: a window of the last N words (the transformer's context, made explicit). The same query
//!   learner scores routes over it: from a cue word's latest occurrences, the word at an offset, and again from
//!   there (two hops). An arbiter keeps, per (question form: the words 1, 3 and 4 back; both sources' margin buckets),
//!   how often the slots' answer and the window's answer were right where the two disagreed. It answers from the window where the window has been right more often
//!   (cross-multiplied, prior 1 in 2). So the window is read for its answer only where the slots are unsure.
//!   Both sources learn at every word.
//! - `sq_max`: open queries, at most this many at once. An open query is a two-hop pattern ((class, kind), then
//!   (class, kind)) kept answered for every entity while reading, in a derived slot of its own. A committed fact
//!   that matches its first hop moves the entity's pointer and refreshes its slot. One that matches its second
//!   hop refreshes every entity pointing to the fact's subject: "fred went to the kitchen" moves every object fred
//!   holds. The query learner reads derived slots like any other.
//!   - **Which open:** the two-hop patterns that held answers to questions are counted. An answer is the word
//!     after a boundary the mood counter has learned closes a question. The most counted (at least 16) opens
//!     when there is room.
//!   - **Which stay:** each open query starts with a balance of 512. It loses 64 when an answer given through its
//!     slot was wrong. It gains 32 when its slot held an answer
//!     and pays 1 per read or write of upkeep. After 2,000 words, one whose balance falls below 0 closes. So the number open tunes itself under
//!     the cap.
//! - `seed`: salts the codes, keys and context.
use crate::meaning::{Meaning, MeaningCfg};
use std::collections::HashMap;

fn hash(a: u64, b: u64) -> u64 {
    crate::rng::hash2(a, b)
}

#[derive(Clone, Debug)]
pub struct SitCfg {
    pub d: usize,
    pub k: usize,
    pub mag: i8,
    pub cue: usize,
    pub win: usize,
    pub ntop: usize,
    pub pool: f64,
    pub recluster: usize,
    pub mdim: usize,
    pub mwin: usize,
    pub bdrift: usize,
    /// Per-slot clocks: slot i's phase steps on every `clock` * 2^(i mod 7) words (0 = off), so a write fades
    /// gradually as the clocks of more of its slots tick; proposals from reliable routes are then ranked by
    /// strength, the freshest first.
    pub clock: u64,
    /// Attend each path word's meaning to its clause before pooling (its sense in context).
    pub sense: bool,
    pub sense_mix: f32,
    /// Learned senses per word (at most), and the match below which a context starts a new one.
    pub nsenses: usize,
    pub sense_new: f64,
    pub kinds: bool,
    pub mood: bool,
    pub recency: bool,
    pub bounce: bool,
    pub supersede: bool,
    /// Write-time composition: the number of most-counted two-hop pairs composed when a fact is committed (0 = off).
    pub compose: usize,
    /// Rank a route by its precision (hits over hits plus misses when it was proposed), not by its hit count.
    pub precision: bool,
    /// Only words of the larger kinds (at least 3 members: people, places, objects) act as cue entities; function
    /// words, each its own kind, are not read from.
    pub entity_cues: bool,
    /// Learn over the cue entity's live slots (and, for two hops, the intermediate's) instead of the most frequent
    /// classes: every relation an entity actually has is searched, however rare its class.
    pub live: bool,
    /// Routes proposed per cue word (most counted live first), and the credibility share: a route competes on
    /// recency if its count times `cred` reaches the best count.
    pub nroutes: usize,
    /// The general query learner (see the module docs) instead of the C1/C2 tables.
    pub query: bool,
    /// The query weights stored sparse at 8 bits instead of a dense 16-bit table. 1: an open-addressing table of
    /// 32-bit keys and 8-bit saturating weights (grown at 3/4 full). 2: per feature, a list of (query bit as 16 bits,
    /// weight as 8 bits), 3 bytes per nonzero weight; a weight that returns to 0 is dropped.
    pub qsparse: u32,
    /// The query code also carries 4 bits hashed from the words 1 and 4 back together ("?" ... "has" against "?" ...
    /// "is": the two question forms share the words 1 and 3 back).
    pub qform14: bool,
    /// An entity's read list takes each kind's newest value before any kind's second newest (the list is short: 8
    /// slots; without this a clause's function words push an older place out of it).
    pub kindfirst: bool,
    /// Query learner: mistakes kept in a ring of 512 and re-learned, this many per word (0 = off).
    pub replay: usize,
    /// Pool relations by the words' frame codes (8-bit frame counters, integer) instead of the meaning module.
    pub fpool: bool,
    /// Lookback: a window of the last `lookback` words, searched by the same query learner when the slots are unsure
    /// (0 = off).
    pub lookback: usize,
    /// Lookback merged: the window's routes compete with the slots' routes in one pool under the same learner
    /// (no separate arbiter).
    pub merge: bool,
    /// Open queries: the most open at once (0 = off). Each is a two-hop pattern ((class, kind) then (class, kind))
    /// kept answered for every entity as facts arrive, into a derived slot of its own; the system opens and closes
    /// them by their usefulness, so the number open tunes itself under this cap.
    pub sq_max: usize,
    /// How open queries are chosen. 0: top-down only (the patterns that held most question answers). 1: the input
    /// proposes (every committed fact proposes the two-hop chains starting from it, counted) and the higher level
    /// ranks (the proposals' route features scored by the learned query weights under the question codes seen
    /// lately). 2: as 1, and the chain must also have held at least 4 answers.
    pub sq_mode: u32,
    /// Open queries' credit. 0: credit (+32) whenever a query's slot held the answer. 1: credit only for what the
    /// query contributed: +32 when the answer given came through its slot and was right, +16 when the answer given
    /// was wrong and its slot held the right one; nothing when another route answered right anyway.
    pub sq_credit: u32,
    /// Which open queries' derived slots an entity's reader sees (it sees at most 4): 0 the newest written, 1 the
    /// queries with the highest balance, 2 the queries with the most credits.
    pub sq_rank: u32,
    /// Open queries by kinds: also count the chains in which both hops were the entity's newest value of their kind,
    /// as a pattern over the kinds alone (any relation, then any relation): "the newest person, then that person's
    /// newest place". The counts of all the relations that reach a person pool into one.
    pub sq_kinds: bool,
    /// The price of staying open. 0: a query closes when its balance is below 0 after its trial. 1: relative: when its
    /// net payoff per word since opening is below the average of the open queries past their trial (cross-multiplied).
    /// 2: either.
    pub sq_price: u32,
    /// Which answers count toward opening a pattern. 0: every answer it held. 1: only the answers it held when the
    /// answer given was wrong (it would have fixed a mistake): patterns that hold what is already answered right add
    /// nothing.
    pub sq_open: u32,
    pub cred: u32,
    pub seed: u64,
}

/// Sparse 8-bit weights: an open-addressing hash table (linear probing, power-of-two capacity, grown at half full)
/// of 32-bit keys and saturating 8-bit values. A key never updated reads 0.
pub struct SparseW {
    keys: Vec<u32>,
    vals: Vec<i8>,
    pub len: usize,
}
impl SparseW {
    const EMPTY: u32 = u32::MAX;
    pub fn new() -> Self {
        SparseW { keys: vec![Self::EMPTY; 1024], vals: vec![0; 1024], len: 0 }
    }
    fn slot(&self, k: u32) -> usize {
        // multiplicative hash (Fibonacci), then linear probing
        let m = self.keys.len() - 1;
        let mut i = ((k as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 32) as usize & m;
        while self.keys[i] != Self::EMPTY && self.keys[i] != k {
            i = (i + 1) & m;
        }
        i
    }
    pub fn get(&self, k: u32) -> i8 {
        let i = self.slot(k);
        if self.keys[i] == k { self.vals[i] } else { 0 }
    }
    pub fn add(&mut self, k: u32, d: i8) {
        if 4 * (self.len + 1) > 3 * self.keys.len() {
            let (ok, ov) = (std::mem::take(&mut self.keys), std::mem::take(&mut self.vals));
            self.keys = vec![Self::EMPTY; ok.len() * 2];
            self.vals = vec![0; ok.len() * 2];
            for (k, v) in ok.into_iter().zip(ov) {
                if k != Self::EMPTY {
                    let i = self.slot(k);
                    self.keys[i] = k;
                    self.vals[i] = v;
                }
            }
        }
        let i = self.slot(k);
        if self.keys[i] != k {
            self.keys[i] = k;
            self.len += 1;
        }
        self.vals[i] = self.vals[i].saturating_add(d);
    }
    /// Bytes held (keys and values, at the table's capacity).
    pub fn bytes(&self) -> usize {
        self.keys.len() * 5
    }
    /// (entries nonzero, entries at the 8-bit limit)
    pub fn census(&self) -> (usize, usize) {
        let mut nz = 0;
        let mut sat = 0;
        for (k, v) in self.keys.iter().zip(&self.vals) {
            if *k != Self::EMPTY && *v != 0 {
                nz += 1;
                if *v == i8::MAX || *v == i8::MIN {
                    sat += 1;
                }
            }
        }
        (nz, sat)
    }
}

/// Sparse 8-bit weights by feature: for each feature, the query bits with a nonzero weight (16 bits) and their
/// weights (8 bits, saturating). A feature's score under a query is the sum of its weights, each times the number of
/// times its bit is in the query.
pub struct RowW {
    bits: Vec<Vec<u16>>,
    vals: Vec<Vec<i8>>,
}
impl RowW {
    pub fn new(nf: usize) -> Self {
        RowW { bits: vec![vec![]; nf], vals: vec![vec![]; nf] }
    }
    pub fn score(&self, fi: u32, qcount: &[u8]) -> i64 {
        let (b, v) = (&self.bits[fi as usize], &self.vals[fi as usize]);
        let mut s = 0i64;
        for (&bb, &w) in b.iter().zip(v) {
            s += w as i64 * qcount[bb as usize] as i64;
        }
        s
    }
    pub fn add(&mut self, fi: u32, b: u16, d: i8) {
        let (bs, vs) = (&mut self.bits[fi as usize], &mut self.vals[fi as usize]);
        match bs.iter().position(|&x| x == b) {
            Some(i) => {
                vs[i] = vs[i].saturating_add(d);
                if vs[i] == 0 {
                    bs.swap_remove(i);
                    vs.swap_remove(i);
                }
            }
            None => {
                bs.push(b);
                vs.push(d);
            }
        }
    }
    /// Bytes held: 3 per weight at the lists' capacity, plus the lists' headers.
    pub fn bytes(&self) -> usize {
        self.bits.iter().map(|b| b.capacity() * 2).sum::<usize>()
            + self.vals.iter().map(|v| v.capacity()).sum::<usize>()
            + self.bits.len() * 48
    }
    pub fn census(&self) -> (usize, usize) {
        let nz = self.vals.iter().map(|v| v.len()).sum();
        let sat = self.vals.iter().flatten().filter(|&&w| w == i8::MAX || w == i8::MIN).count();
        (nz, sat)
    }
}

/// A learned sense of a word: a prototype of the contexts it is used in (counts over the identity bits of the
/// clause words around it).
struct Sense {
    counts: Vec<u32>,
    n: u32,
}

/// A pooled relation class: a prototype (bit counts) and its name (its first path).
struct Proto {
    counts: Vec<u32>,
    bits: Vec<u32>,
    dir: u8,
    n: u32,
}

pub struct Situation {
    pub cfg: SitCfg,
    nw: usize,
    re: Vec<i8>,
    im: Vec<i8>,
    bg: Vec<u64>,
    ctx_off: Vec<u32>,
    ctx_ph: Vec<u8>,
    // segmentation
    seg_draw: HashMap<u32, (u32, u32)>,
    seg_hits: HashMap<u32, [u64; 3]>,
    seg_n: HashMap<u32, u64>,
    boundary: Vec<bool>,
    // paths and classes
    mn: Meaning,
    path_count: HashMap<Vec<u32>, u64>, // dir-tagged path -> uses
    path_class: HashMap<Vec<u32>, u32>,
    protos: Vec<Proto>,
    class_count: Vec<u64>,
    senses: HashMap<u32, Vec<Sense>>,
    // slow memory
    c1: HashMap<(u64, usize), HashMap<(u32, u32), u32>>,
    c2: HashMap<(u64, usize), HashMap<((u32, u32), (u32, u32)), u32>>,
    hist: Vec<u32>,
    clause: Vec<u32>,
    seen: Vec<bool>,
    words: Vec<u32>,
    t: u64,
    pub last: Vec<(u32, u32)>, // the last prediction's proposals: (weight, value)
    learning: bool,
    // experiment 36
    pool_pm: u64,                                   // the pooling threshold in thousandths (integer compare)
    frame: Vec<Vec<u16>>,                           // 8-bit frame counters per word
    wcount: Vec<u64>,
    kind: Vec<u32>,                                 // each word's kind (a word id); kn = none yet
    kn: u32,
    ksize: HashMap<u32, u32>,                      // members per kind
    fdf: Vec<u32>,                                  // per frame feature: how many words' frame codes hold it
    fseen: u32,
    qw: Vec<i16>,                                   // the query learner: QD x NF signed 16-bit weights (dense)
    qs: SparseW,                                    // ... or the same weights sparse, 8-bit (--sit-qsparse 1)
    qr: RowW,                                       // ... or per feature, (bit, weight) lists (--sit-qsparse 2)
    qbits: Vec<u32>,                                // the query code of the current prediction
    qcands: Vec<(Vec<u32>, u32, i64)>,             // its routes: (features, value, score), best first
    ring: Vec<(Vec<u32>, Vec<(Vec<u32>, u32)>, u32)>, // replayed mistakes: (query, routes (features, value), answer)
    ring_at: usize,
    win: Vec<u32>,                                  // the lookback window: the last `lookback` words
    wcands: Vec<(Vec<u32>, u32, i64)>,             // the window's routes for the current prediction
    arb: HashMap<(u64, u32), [u16; 4]>,             // per (question form, slot margin bucket): slot right, slot n, window right, window n
    arb_key: (u64, u32),
    slot_top: Option<u32>,
    pub used_window: u64,                           // predictions answered from the window
    pub used_window_q: u64,                         // ... of them right after a question boundary
    pub questions_seen: u64,
    qhops: Vec<Route>,                              // each route's hops, aligned with qcands
    qranks: Vec<(u32, u32)>,                        // each route's recency ranks within kind, aligned with qcands
    after_q: bool,                                  // the current prediction follows a learned question boundary
    sq: Vec<OpenQuery>,                             // the open queries
    sq_cand: HashMap<((u32, u32), (u32, u32)), u32>, // two-hop patterns that answered, counted
    sq_in: HashMap<((u32, u32), (u32, u32)), (u32, u32)>, // chains the input proposed: (count, the last subject)
    qring: Vec<Vec<u32>>,                           // the question codes seen lately (the higher level)
    qring_at: usize,
    sq_ptr: HashMap<(u32, u32), u32>,               // (query id, entity x) -> the intermediate m it points to
    sq_rev: HashMap<(u32, u32), Vec<u32>>,          // (query id, m) -> the entities pointing to m
    sq_next: u32,
    pub sq_work: u64,                               // maintenance reads and writes
    pub sq_open_sum: u64,                           // open queries summed over words (for the mean)
    pub sq_peak: usize,                             // the most open at once
    pub sq_opened: u64,
    pub sq_closed: u64,
    mood: Vec<u8>,
    buf: Vec<(u32, u32, u8, u32)>,                  // buffered bindings: (entity, class, direction, value)
    stamp: HashMap<(u32, u32, u32), u64>,           // last write time per slot (entity, class, kind)
    occ: HashMap<(u32, u32), Vec<u32>>,             // live kinds per (entity, class): the slots' valid bits
    dcl: HashMap<(u32, u8), Vec<u32>>,              // classes written per (entity, direction)
    hop: HashMap<(u64, usize, u8), HashMap<(u32, u32), u32>>,
    g2: HashMap<((u32, u32), (u32, u32)), u32>,       // two-hop pairs counted over all contexts
    comp: HashMap<((u32, u32), (u32, u32)), u32>,     // composite classes, numbered from COMP
    miss: HashMap<(u64, usize, Route), u32>,          // NoGo: times a route was proposed and wrong
    routes: Vec<(u64, usize, Route, u32)>,            // the last prediction's routes: (context, cue, route, value)
    halt: HashMap<(u64, u32, u8), u8>,
    pub committed: u64,
    pub dropped: u64,
}

const DRE: [i8; 4] = [1, 0, -1, 0];
const FB: usize = 512;
const QD: usize = 4 * FB + 768; // query bits: 4 positions x frame code, 4 positions x 64 identity bits, 2 x 256 context bits
const NF: usize = 4096;         // route features (hashed)
/// A route: one slot (class, kind), or two (the second with class != u32::MAX).
type Route = (u32, u32, u32, u32);
const COMP: u32 = 1 << 24; // composite class ids start here, apart from the pooled classes
const DER: u32 = 1 << 25; // open queries' derived slots: class DER + query id
const ANY: u32 = u32::MAX - 1; // an open query's hop of any relation: the entity's newest value of that kind

/// An open query: a two-hop pattern kept answered for every entity, with its usefulness.
#[derive(Clone, Debug)]
pub struct OpenQuery {
    pub id: u32,
    pub p1: (u32, u32),
    pub p2: (u32, u32),
    pub util: i32,
    pub credits: u64,
    pub work: u64,
    pub opened_at: u64,
}
const DIM: [i8; 4] = [0, 1, 0, -1];

impl Situation {
    pub fn new(cfg: SitCfg, nw: usize) -> Self {
        let mut mc = MeaningCfg::default();
        mc.d = cfg.mdim;
        mc.win = cfg.mwin;
        mc.skip = 0;
        mc.seed = 21;
        let mut mn = Meaning::new(mc, nw);
        mn.sense_mix = cfg.sense_mix;
        let sd = cfg.seed.wrapping_mul(0x9E37_79B9);
        Situation {
            nw,
            re: vec![0; cfg.d],
            im: vec![0; cfg.d],
            bg: vec![0; nw + 1],
            ctx_off: (0..cfg.k).map(|i| (hash(91 + sd, i as u64) % cfg.d as u64) as u32).collect(),
            ctx_ph: (0..cfg.k).map(|i| (hash(92 + sd, i as u64) & 3) as u8).collect(),
            seg_draw: HashMap::new(),
            seg_hits: HashMap::new(),
            seg_n: HashMap::new(),
            boundary: vec![false; nw],
            mn,
            path_count: HashMap::new(),
            path_class: HashMap::new(),
            protos: vec![],
            class_count: vec![],
            senses: HashMap::new(),
            c1: HashMap::new(),
            c2: HashMap::new(),
            hist: vec![],
            clause: vec![],
            seen: vec![false; nw],
            words: vec![],
            t: 0,
            last: vec![],
            learning: true,
            pool_pm: (cfg.pool * 1000.0).round() as u64,
            frame: vec![vec![0; FB]; nw],
            wcount: vec![0; nw],
            kind: vec![nw as u32; nw + 1],
            kn: nw as u32,
            ksize: HashMap::new(),
            fdf: vec![0; FB],
            fseen: 0,
            qw: if cfg.query && cfg.qsparse == 0 { vec![0; QD * NF] } else { vec![] },
            qs: SparseW::new(),
            qr: RowW::new(if cfg.query && cfg.qsparse == 2 { NF } else { 0 }),
            qbits: vec![],
            qcands: vec![],
            ring: vec![],
            ring_at: 0,
            win: vec![],
            wcands: vec![],
            arb: HashMap::new(),
            arb_key: (0, 0),
            slot_top: None,
            used_window: 0,
            used_window_q: 0,
            questions_seen: 0,
            qhops: vec![],
            qranks: vec![],
            after_q: false,
            sq: vec![],
            sq_cand: HashMap::new(),
            sq_in: HashMap::new(),
            qring: vec![],
            qring_at: 0,
            sq_ptr: HashMap::new(),
            sq_rev: HashMap::new(),
            sq_next: 0,
            sq_work: 0,
            sq_open_sum: 0,
            sq_peak: 0,
            sq_opened: 0,
            sq_closed: 0,
            mood: vec![0; nw],
            buf: vec![],
            stamp: HashMap::new(),
            occ: HashMap::new(),
            dcl: HashMap::new(),
            hop: HashMap::new(),
            g2: HashMap::new(),
            comp: HashMap::new(),
            miss: HashMap::new(),
            routes: vec![],
            halt: HashMap::new(),
            committed: 0,
            dropped: 0,
            cfg,
        }
    }

    /// Diagnostics: the overlap (shared bits / k) of word w's sense in two contexts, and of its plain code with each.
    pub fn sense_overlap(&mut self, w: u32, c1: &[u32], c2: &[u32]) -> (f64, f64, f64) {
        let (a, b) = if self.cfg.nsenses > 0 {
            (self.sense_code(w, c1, false), self.sense_code(w, c2, false))
        } else {
            (self.mn.sense(w, c1), self.mn.sense(w, c2))
        };
        let p = self.mn.code_or_ident(w);
        let ov = |x: &[u32], y: &[u32]| x.iter().filter(|b| y.contains(b)).count() as f64 / x.len().max(1) as f64;
        (ov(&a, &b), ov(&a, &p), ov(&b, &p))
    }
    pub fn n_senses(&self, w: u32) -> usize {
        self.senses.get(&w).map(|v| v.len()).unwrap_or(0)
    }
    pub fn boundaries(&self) -> Vec<u32> {
        (0..self.nw as u32).filter(|&w| self.boundary[w as usize]).collect()
    }
    pub fn n_classes(&self) -> usize {
        self.protos.len()
    }
    pub fn n_paths(&self) -> usize {
        self.path_count.len()
    }
    /// The classes with the most uses, each with its most used paths (dir-tagged: first element 0 forward, 1 back).
    pub fn classes(&self, top: usize) -> Vec<Vec<Vec<u32>>> {
        let mut by: HashMap<u32, Vec<(u64, Vec<u32>)>> = HashMap::new();
        for (p, &c) in &self.path_count {
            if let Some(&k) = self.path_class.get(p) {
                by.entry(k).or_default().push((c, p.clone()));
            }
        }
        let mut v: Vec<(u64, Vec<Vec<u32>>)> = by
            .into_values()
            .map(|mut m| {
                m.sort_by(|a, b| b.0.cmp(&a.0));
                (m.iter().map(|x| x.0).sum(), m.into_iter().take(6).map(|x| x.1).collect())
            })
            .collect();
        v.sort_by(|a, b| b.0.cmp(&a.0));
        v.into_iter().filter(|x| x.1.len() > 1).take(top).map(|x| x.1).collect()
    }

    // ---- segmentation ----
    fn seg_update(&mut self, x: u32, u: u32, c: u32) {
        let d = self.seg_draw.get(&u).copied();
        let n = {
            let n = self.seg_n.entry(u).or_insert(0);
            *n += 1;
            *n
        };
        if let Some((dx, dc)) = d {
            let h = self.seg_hits.entry(u).or_insert([0; 3]);
            h[0] += (dx == x) as u64;
            h[1] += (dc == c) as u64;
            h[2] += (dx == x && dc == c) as u64;
        }
        if d.is_none() || hash(self.t, u as u64 + 5151) % 16 == 0 {
            self.seg_draw.insert(u, (x, c));
        }
        if n >= 64 && n % 32 == 0 {
            let [hp, hn, hj] = *self.seg_hits.get(&u).unwrap_or(&[0; 3]);
            let (lo, hi) = (hp.min(hn) as u128, hp.max(hn) as u128);
            let (n, hj) = (n as u128, hj as u128);
            let nb = 20 * hi <= 19 * n && lo * lo * lo * hi * hi > n * n * hj * hj * hj;
            if nb != self.boundary[u as usize] && std::env::var("SIT_DEBUG_BND").is_ok() {
                eprintln!("BND {} word {} -> {}", self.t, u, nb);
            }
            self.boundary[u as usize] = nb;
        }
        if n >= 4096 {
            self.seg_n.insert(u, n / 2);
            if let Some(h) = self.seg_hits.get_mut(&u) {
                h.iter_mut().for_each(|x| *x /= 2);
            }
        }
    }

    // ---- pooling ----
    /// A path's code: the OR of its words' meaning codes, each rotated by its place in the path.
    fn path_code(&mut self, path: &[u32]) -> Vec<u32> {
        let d = self.cfg.mdim;
        let mut out: Vec<u32> = vec![];
        // with senses, the key is [dir, s, w, path...]: each path word's code is its sense in the clause (the
        // context: the two ends and the other path words)
        // with senses the key is [dir, n, path..., the sense each path word took]
        let (words, sids): (Vec<u32>, Vec<u32>) = if self.cfg.sense {
            let n = path[1] as usize;
            (path[2..2 + n].to_vec(), path[2 + n..].to_vec())
        } else {
            (path[1..].to_vec(), vec![])
        };
        for (i, &w) in words.iter().enumerate() {
            let c = match sids.get(i) {
                Some(&k) if k != u32::MAX => self.sense_mask(w, k as usize),
                _ if self.cfg.fpool => self.frame_bits(w),
                _ => self.mn.code_or_ident(w),
            };
            out.extend(c.iter().map(|&b| ((b as usize + i * 7919) % d) as u32));
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// A word's sense in a context: a sparse subset of its own meaning code. The context (the identity bits of
    /// the clause words around it) picks the word's learned sense whose context prototype it matches best (mean
    /// count over the context's bits, relative to the sense's uses), or starts one (up to `nsenses`); when
    /// learning, that sense's prototype counts the context. The sense's code is the half of the word's meaning
    /// code most counted in its prototype.
    fn sense_code(&mut self, w: u32, ctx: &[u32], learn: bool) -> Vec<u32> {
        match self.sense_pick(w, ctx, learn) {
            Some(i) => self.sense_mask(w, i),
            None => self.mn.code_or_ident(w),
        }
    }

    /// Which of w's learned senses the context picks (learning: the sense counts the context, or a new one
    /// starts); None while there is no context or no sense yet.
    fn sense_pick(&mut self, w: u32, ctx: &[u32], learn: bool) -> Option<usize> {
        let mut cb: Vec<u32> = ctx.iter().flat_map(|&c| self.mn.ident(c).to_vec()).collect();
        cb.sort_unstable();
        cb.dedup();
        if cb.is_empty() {
            return None;
        }
        let d = self.cfg.mdim;
        let ss = self.senses.entry(w).or_default();
        let mut best = (0.0f64, usize::MAX);
        for (i, sn) in ss.iter().enumerate() {
            let m = cb.iter().map(|&b| sn.counts[b as usize] as f64).sum::<f64>() / (sn.n.max(1) as f64 * cb.len() as f64);
            if m > best.0 {
                best = (m, i);
            }
        }
        let i = if best.1 == usize::MAX || (best.0 < self.cfg.sense_new && ss.len() < self.cfg.nsenses) {
            if !learn {
                return if best.1 == usize::MAX { None } else { Some(best.1) };
            }
            ss.push(Sense { counts: vec![0; d], n: 0 });
            ss.len() - 1
        } else {
            best.1
        };
        if learn {
            let sn = &mut ss[i];
            sn.n += 1;
            for &b in &cb {
                sn.counts[b as usize] += 1;
            }
        }
        Some(i)
    }

    /// Sense i of w: the half of w's meaning code most counted in that sense's context prototype (the plain code
    /// while the sense is young).
    fn sense_mask(&mut self, w: u32, i: usize) -> Vec<u32> {
        let plain = self.mn.code_or_ident(w);
        let Some(sn) = self.senses.get(&w).and_then(|v| v.get(i)) else { return plain };
        if sn.n < 4 || plain.is_empty() {
            return plain;
        }
        let mut v: Vec<(u32, u32)> = plain.iter().map(|&b| (sn.counts[b as usize], b)).collect();
        v.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let keep = (plain.len() + 1) / 2;
        let mut out: Vec<u32> = v.into_iter().take(keep).filter(|x| x.0 > 0).map(|x| x.1).collect();
        if out.is_empty() {
            return plain;
        }
        out.sort_unstable();
        out
    }

    /// The relation class of a dir-tagged path (an empty path, adjacency, is its own class per direction).
    fn class_of(&mut self, path: &[u32]) -> u32 {
        if let Some(&c) = self.path_class.get(path) {
            return c;
        }
        let dir = path[0] as u8;
        let bare = if self.cfg.sense { 2 } else { 1 };
        let code = if path.len() <= bare { vec![] } else { self.path_code(path) };
        // the best overlap (shared bits over the larger size), as a fraction (shared, size), compared by
        // cross-multiplication: no floats, no division
        let mut best = (0u64, 1u64, usize::MAX);
        if !code.is_empty() && self.pool_pm > 0 {
            for (i, p) in self.protos.iter().enumerate() {
                if p.dir != dir || p.bits.is_empty() {
                    continue;
                }
                // overlap: shared bits over the larger of the two sizes (both sorted)
                let (mut a, mut b, mut sh) = (0, 0, 0);
                while a < code.len() && b < p.bits.len() {
                    if code[a] == p.bits[b] {
                        sh += 1;
                        a += 1;
                        b += 1;
                    } else if code[a] < p.bits[b] {
                        a += 1;
                    } else {
                        b += 1;
                    }
                }
                let (sh, den) = (sh as u64, code.len().max(p.bits.len()) as u64);
                if sh * 1000 >= self.pool_pm * den && (best.2 == usize::MAX || sh * best.1 > best.0 * den) {
                    best = (sh, den, i);
                }
            }
        }
        let c = if best.2 == usize::MAX {
            self.protos.push(Proto { counts: vec![0; self.cfg.mdim], bits: code.clone(), dir, n: 0 });
            self.class_count.push(0);
            self.protos.len() - 1
        } else {
            best.2
        };
        let p = &mut self.protos[c];
        p.n += 1;
        for &b in &code {
            p.counts[b as usize] += 1;
        }
        if p.n % 8 == 1 && !code.is_empty() {
            // the prototype: the code-sized set of its most counted bits
            let k = code.len();
            let mut idx: Vec<u32> = (0..p.counts.len() as u32).filter(|&i| p.counts[i as usize] > 0).collect();
            if idx.len() > k {
                idx.select_nth_unstable_by(k - 1, |&x, &y| p.counts[y as usize].cmp(&p.counts[x as usize]));
                idx.truncate(k);
            }
            idx.sort_unstable();
            p.bits = idx;
        }
        self.path_class.insert(path.to_vec(), c as u32);
        c as u32
    }

    /// Re-assign every path, most used first, as the meanings have matured; classes keep their numbers where
    /// their most used path lands again.
    fn recluster(&mut self) {
        if self.cfg.fpool {
            self.recount_fdf();
        }
        let mut paths: Vec<(u64, Vec<u32>)> = self.path_count.iter().map(|(p, &c)| (c, p.clone())).collect();
        paths.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        // each old class's most used path, to carry its counts over to wherever that path lands now
        let mut rep: HashMap<u32, Vec<u32>> = HashMap::new();
        for (_, p) in &paths {
            if let Some(&c) = self.path_class.get(p) {
                rep.entry(c).or_insert_with(|| p.clone());
            }
        }
        self.protos.clear();
        self.class_count.clear();
        self.path_class.clear();
        self.learning = false;
        for (c, p) in paths {
            let k = self.class_of(&p) as usize;
            self.class_count[k] += c;
        }
        self.learning = true;
        let raw: HashMap<u32, u32> = rep.into_iter().map(|(old, p)| (old, self.path_class[&p])).collect();
        // keep class numbers stable: a new class takes the number of the (lowest) old class whose most used path
        // landed in it, so slots written before the re-assignment stay addressed; the rest get unused numbers
        let nnew = self.protos.len();
        let mut fin: Vec<Option<u32>> = vec![None; nnew];
        let mut olds: Vec<(u32, u32)> = raw.iter().map(|(&o, &k)| (o, k)).collect();
        olds.sort_unstable();
        let mut used: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for &(o, k) in &olds {
            if fin[k as usize].is_none() && !used.contains(&o) {
                fin[k as usize] = Some(o);
                used.insert(o);
            }
        }
        let mut next = 0u32;
        for f in fin.iter_mut() {
            if f.is_none() {
                while used.contains(&next) {
                    next += 1;
                }
                *f = Some(next);
                used.insert(next);
            }
        }
        let fin: Vec<u32> = fin.into_iter().map(|x| x.unwrap()).collect();
        let size = fin.iter().copied().max().map_or(0, |m| m as usize + 1);
        let mut protos: Vec<Proto> = (0..size).map(|_| Proto { counts: vec![], bits: vec![], dir: 255, n: 0 }).collect();
        let mut counts = vec![0u64; size];
        for (k, p) in std::mem::take(&mut self.protos).into_iter().enumerate() {
            counts[fin[k] as usize] = self.class_count[k];
            protos[fin[k] as usize] = p;
        }
        self.protos = protos;
        self.class_count = counts;
        for v in self.path_class.values_mut() {
            *v = fin[*v as usize];
        }
        let map: HashMap<u32, u32> = raw.into_iter().map(|(o, k)| (o, fin[k as usize])).collect();
        let c1 = std::mem::take(&mut self.c1);
        for (k, m) in c1 {
            let e = self.c1.entry(k).or_default();
            for ((c, kd), n) in m {
                if let Some(&c2) = map.get(&c) {
                    *e.entry((c2, kd)).or_insert(0) += n;
                }
            }
        }
        let c2 = std::mem::take(&mut self.c2);
        for (k, m) in c2 {
            let e = self.c2.entry(k).or_default();
            for (((a, ka), (b, kb)), n) in m {
                if let (Some(&a2), Some(&b2)) = (map.get(&a), map.get(&b)) {
                    *e.entry(((a2, ka), (b2, kb))).or_insert(0) += n;
                }
            }
        }
        let miss = std::mem::take(&mut self.miss);
        for ((cx, j, (a, ka, b, kb)), n) in miss {
            let b2 = if b == u32::MAX { Some(b) } else { map.get(&b).copied() };
            if let (Some(&a2), Some(b2)) = (map.get(&a), b2) {
                *self.miss.entry((cx, j, (a2, ka, b2, kb))).or_insert(0) += n;
            }
        }
        let mut closing = vec![];
        for o in self.sq.iter_mut() {
            let mp = |c: u32| if c == ANY { Some(ANY) } else { map.get(&c).copied() };
            match (mp(o.p1.0), mp(o.p2.0)) {
                (Some(a), Some(b)) => {
                    o.p1.0 = a;
                    o.p2.0 = b;
                }
                _ => closing.push(o.id),
            }
        }
        for id in closing {
            self.sq_close(id);
        }
        self.sq_cand.retain(|p, _| p.0 .0 == ANY);
        self.sq_in.clear();
        let hop = std::mem::take(&mut self.hop);
        for (k, m) in hop {
            let e = self.hop.entry(k).or_default();
            for ((c, kd), n) in m {
                if let Some(&c2) = map.get(&c) {
                    *e.entry((c2, kd)).or_insert(0) += n;
                }
            }
        }
        let g2 = std::mem::take(&mut self.g2);
        for (((a, ka), (b, kb)), n) in g2 {
            if let (Some(&a2), Some(&b2)) = (map.get(&a), map.get(&b)) {
                *self.g2.entry(((a2, ka), (b2, kb))).or_insert(0) += n;
            }
        }
        let comp = std::mem::take(&mut self.comp);
        for (((a, ka), (b, kb)), id) in comp {
            if let (Some(&a2), Some(&b2)) = (map.get(&a), map.get(&b)) {
                self.comp.entry(((a2, ka), (b2, kb))).or_insert(id);
            }
        }
        // the valid bits and write times follow their classes (numbers mostly unchanged); classes merged into
        // another number lose their slots
        let occ = std::mem::take(&mut self.occ);
        for ((e, c), ks) in occ {
            if c >= COMP || map.get(&c) == Some(&c) {
                self.occ.insert((e, c), ks);
            }
        }
        let stamp = std::mem::take(&mut self.stamp);
        for ((e, c, k), t) in stamp {
            if c >= COMP || map.get(&c) == Some(&c) {
                self.stamp.insert((e, c, k), t);
            }
        }
        for cs in self.dcl.values_mut() {
            cs.retain(|c| *c >= COMP || map.get(c) == Some(c));
        }
        if self.cfg.kinds {
            self.recluster_kinds();
        }
    }

    // ---- kinds ----
    fn fb(a: u64, b: u64) -> usize {
        (hash(a, b) % FB as u64) as usize
    }
    fn frame_bump(&mut self, w: u32, f: usize) {
        let r = &mut self.frame[w as usize];
        r[f] += 1;
        if r[f] > 255 {
            r.iter_mut().for_each(|x| *x >>= 1); // 8-bit: halve together when one saturates
        }
    }
    /// A word's frame code as bit indices (an integer meaning code: its 8-bit frame counters at a quarter of their
    /// maximum or more); a few identity bits while it has no frames yet.
    fn frame_bits(&self, w: u32) -> Vec<u32> {
        let r = &self.frame[w as usize];
        let mx = *r.iter().max().unwrap_or(&0);
        if mx == 0 {
            return (0..4u64).map(|i| (hash(w as u64 + 77, i) % FB as u64) as u32).collect();
        }
        let th = (mx >> 2).max(1);
        // keep the distinctive features: those in at most an eighth of the words' codes (integer compare)
        (0..FB as u32).filter(|&b| r[b as usize] >= th && (self.fseen == 0 || self.fdf[b as usize] * 8 <= self.fseen)).collect()
    }
    /// Recount, per frame feature, how many words' frame codes hold it.
    fn recount_fdf(&mut self) {
        let mut df = vec![0u32; FB];
        let mut nseen = 0;
        for w in 0..self.nw as u32 {
            if self.wcount[w as usize] == 0 {
                continue;
            }
            nseen += 1;
            for (b, on) in self.frame_code(w).into_iter().enumerate() {
                if on {
                    df[b] += 1;
                }
            }
        }
        self.fdf = df;
        self.fseen = nseen;
    }
    fn frame_code(&self, w: u32) -> Vec<bool> {
        let r = &self.frame[w as usize];
        let th = (*r.iter().max().unwrap_or(&0) >> 2).max(1);
        r.iter().map(|&x| x >= th).collect()
    }
    /// Leader clustering of the words by their frame codes (Jaccard >= 2/5, cross-multiplied), most frequent first;
    /// each new cluster takes the old kind name most of its members had, so slots and counts keep their meaning.
    fn recluster_kinds(&mut self) {
        let mut order: Vec<u32> = (0..self.nw as u32).filter(|&w| self.wcount[w as usize] > 0).collect();
        order.sort_by(|a, b| self.wcount[*b as usize].cmp(&self.wcount[*a as usize]).then(a.cmp(b)));
        let mut leaders: Vec<(u32, Vec<bool>, Vec<u32>)> = vec![];
        for &w in &order {
            let c = self.frame_code(w);
            let mut placed = false;
            for l in leaders.iter_mut() {
                let (mut inter, mut uni) = (0u64, 0u64);
                for (x, y) in c.iter().zip(&l.1) {
                    inter += (*x && *y) as u64;
                    uni += (*x || *y) as u64;
                }
                if uni > 0 && 5 * inter >= 2 * uni {
                    l.2.push(w);
                    placed = true;
                    break;
                }
            }
            if !placed {
                leaders.push((w, c, vec![w]));
            }
        }
        leaders.sort_by(|a, b| b.2.len().cmp(&a.2.len()).then(a.0.cmp(&b.0)));
        let mut used: Vec<u32> = vec![];
        let mut newk = self.kind.clone();
        for (lw, _, ms) in &leaders {
            let mut votes: HashMap<u32, u32> = HashMap::new();
            for &w in ms {
                if self.kind[w as usize] != self.kn {
                    *votes.entry(self.kind[w as usize]).or_insert(0) += 1;
                }
            }
            let mut v: Vec<(u32, u32)> = votes.into_iter().collect();
            v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
            let name = v.iter().map(|x| x.0).find(|o| !used.contains(o)).unwrap_or_else(|| {
                if !used.contains(lw) { *lw } else { (0..self.nw as u32).find(|x| !used.contains(x)).unwrap() }
            });
            used.push(name);
            for &w in ms {
                newk[w as usize] = name;
            }
        }
        self.kind = newk;
        self.ksize.clear();
        for w in 0..self.nw {
            if self.wcount[w] > 0 {
                *self.ksize.entry(self.kind[w]).or_insert(0) += 1;
            }
        }
    }
    /// The classes of e's live slots (of kind kd, if given).
    fn live_classes(&self, e: u32, kd: Option<u32>) -> Vec<u32> {
        let mut v: Vec<u32> = vec![];
        for dir in 0..4u8 {
            if let Some(cs) = self.dcl.get(&(e, dir)) {
                for &c in cs {
                    if !v.contains(&c) && kd.map_or(true, |k| self.occ.get(&(e, c)).map_or(false, |ks| ks.contains(&k))) {
                        v.push(c);
                    }
                }
            }
        }
        v
    }
    /// Whether w may act as a cue entity (with --sit-entity-cues: a word of a kind with at least 3 members).
    fn is_entity(&self, w: u32) -> bool {
        !self.cfg.entity_cues || self.ksize.get(&self.kind[w as usize]).copied().unwrap_or(0) >= 3
    }
    fn kk(&self, v: u32) -> u32 {
        if self.cfg.kinds { self.kind[v as usize] } else { self.kn }
    }
    /// The kinds, as groups of words (diagnostics).
    pub fn kind_groups(&self) -> Vec<Vec<u32>> {
        let mut by: HashMap<u32, Vec<u32>> = HashMap::new();
        for w in 0..self.nw as u32 {
            if self.wcount[w as usize] > 0 {
                by.entry(self.kind[w as usize]).or_default().push(w);
            }
        }
        let mut v: Vec<Vec<u32>> = by.into_values().collect();
        v.sort_by(|a, b| b.len().cmp(&a.len()).then(a[0].cmp(&b[0])));
        v
    }
    /// The boundaries whose mood counter marks them as closing questions.
    pub fn question_boundaries(&self) -> Vec<u32> {
        (0..self.nw as u32).filter(|&w| self.boundary[w as usize] && self.mood[w as usize] >= 8).collect()
    }

    // ---- fast ternary memory ----
    /// The slot (entity e, class c, value kind kd): K elements and phases, offset by the context.
    fn key(&self, e: u32, c: u32, kd: u32) -> (Vec<u32>, Vec<u8>) {
        let (d, k) = (self.cfg.d as u64, self.cfg.k);
        let mut idx = Vec::with_capacity(k);
        let mut ph = Vec::with_capacity(k);
        let sc = if kd == self.kn { c as u64 } else { c as u64 * (self.nw as u64 + 1) + kd as u64 + (1 << 40) };
        let salt = self.cfg.seed.wrapping_mul(0x5851_F42D);
        for i in 0..k {
            let h = hash(e as u64 * 1_000_003 + sc + salt, i as u64);
            idx.push(((h % d) as u32 + self.ctx_off[i]) % d as u32);
            ph.push(((h >> 40) as u8 + self.ctx_ph[i]) & 3);
        }
        (idx, ph)
    }
    #[inline]
    fn vph(&self, v: u32, e: u32) -> u8 {
        (hash(v as u64 + 0x7777 + self.cfg.seed.wrapping_mul(0x1000_0001), e as u64) & 3) as u8
    }
    fn write(&mut self, e: u32, c: u32, kd: u32, v: u32) {
        let (idx, ph) = self.key(e, c, kd);
        let m = self.cfg.mag;
        for (&x, &kp) in idx.iter().zip(&ph) {
            let p = ((kp + self.vph(v, x)) & 3) as usize;
            let x = x as usize;
            self.re[x] = (self.re[x] + m * DRE[p]).clamp(-1, 1);
            self.im[x] = (self.im[x] + m * DIM[p]).clamp(-1, 1);
        }
        self.bg[v as usize] += 1;
    }
    /// The best value in e's slot for class c (by excess over its background, by subtraction), and its excess.
    fn read_best(&self, e: u32, c: u32, kd: u32, excl: &[u32]) -> Option<(u32, i128)> {
        let (idx, ph) = self.key(e, c, kd);
        let z: Vec<(i32, i32)> = idx
            .iter()
            .zip(&ph)
            .map(|(&x, &kp)| {
                let (a, b) = (self.re[x as usize] as i32, self.im[x as usize] as i32);
                match kp & 3 {
                    0 => (a, b),
                    1 => (b, -a),
                    2 => (-a, -b),
                    _ => (-b, a),
                }
            })
            .collect();
        // with kinds, a slot of kind kd is cleaned up against the words of that kind only
        let typed = self.cfg.kinds && kd != self.kn;
        let sc: Vec<(u32, i64)> = self
            .words
            .iter()
            .filter(|w| !excl.contains(w) && (!typed || self.kind[**w as usize] == kd))
            .map(|&v| {
                let s: i64 = idx
                    .iter()
                    .zip(&z)
                    .map(|(&x, &(zr, zi))| match self.vph(v, x) {
                        0 => zr,
                        1 => zi,
                        2 => -zr,
                        _ => -zi,
                    } as i64)
                    .sum();
                (v, s)
            })
            .filter(|x| x.1 > 0)
            .collect();
        let st: i128 = sc.iter().map(|x| x.1 as i128).sum();
        let bt: i128 = self.words.iter().map(|&w| self.bg[w as usize] as i128).sum::<i128>().max(1);
        sc.into_iter()
            .map(|(v, s)| (v, s as i128 * bt - self.bg[v as usize] as i128 * st))
            .filter(|x| x.1 > 0)
            .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
    }

    fn ctx_key(&self) -> u64 {
        let n = self.hist.len();
        let at = |j: usize| if j < n { self.hist[n - 1 - j] as u64 } else { u64::MAX };
        hash(hash(at(0), at(2)), at(3))
    }

    // ---- the general query learner ----
    /// The query code: the cue words' frame codes (8-bit frame counters at a quarter of their max), each in the
    /// block of its position, plus 4 identity bits per position.
    fn query_bits(&self, cue: &[u32]) -> Vec<u32> {
        let mut q = vec![];
        for (j, &u) in cue.iter().enumerate().take(4) {
            for (b, on) in self.frame_code(u).into_iter().enumerate() {
                if on {
                    q.push((j * FB + b) as u32);
                }
            }
            for i in 0..4u64 {
                q.push((4 * FB + j * 64 + (hash(u as u64 + 31, i) % 64) as usize) as u32);
            }
        }
        // conjunctive context bits: a hash of the words 1 and 3 back together, skipping the entity between them
        // (the question form)
        let w0 = cue.first().copied().unwrap_or(u32::MAX) as u64;
        let w2 = cue.get(2).copied().unwrap_or(u32::MAX) as u64;
        let cx = hash(hash(w0, 7), w2);
        for i in 0..4u64 {
            q.push((4 * FB + 256 + (hash(cx, i) % 256) as usize) as u32);
        }
        if self.cfg.qform14 {
            let w3 = cue.get(3).copied().unwrap_or(u32::MAX) as u64;
            let cy = hash(hash(w0, 11), w3);
            for i in 0..4u64 {
                q.push((4 * FB + 512 + (hash(cy, i) % 256) as usize) as u32);
            }
        }
        q
    }
    fn fid(tag: u64, a: u64, b: u64) -> u32 {
        (hash(hash(tag, a), b) % NF as u64) as u32
    }
    /// A route's features: the cue position, the cue word's kind, and per hop its class, its kind, its recency rank
    /// within the entity, plus the number of hops.
    fn route_feats(&self, j: usize, e: u32, hops: &[(u32, u32, u32)]) -> Vec<u32> {
        let mut f = vec![Self::fid(1, j as u64, 0), Self::fid(2, self.kk(e) as u64, 0), Self::fid(6, hops.len() as u64, 0)];
        for (s, &(c, k, r)) in hops.iter().enumerate() {
            f.push(Self::fid(3, s as u64, c as u64));
            f.push(Self::fid(4, s as u64, k as u64));
            f.push(Self::fid(5, s as u64, r.min(3) as u64));
            f.push(Self::fid(7, s as u64 * 1_000_003 + c as u64, k as u64));
        }
        if hops.len() == 2 {
            // conjunctions across the two hops: the pair of classes, the pair of kinds, the pair of ranks
            f.push(Self::fid(8, hops[0].0 as u64 * 1_000_003 + hops[1].0 as u64, hops[0].1 as u64 * 1009 + hops[1].1 as u64));
            f.push(Self::fid(9, hops[0].1 as u64, hops[1].1 as u64));
            f.push(Self::fid(10, hops[0].2.min(3) as u64, hops[1].2.min(3) as u64));
        }
        f
    }
    /// e's live slots, newest first (at most n): (class, kind, rank).
    fn newest_slots(&self, e: u32, n: usize) -> Vec<(u32, u32, u32)> {
        let mut v: Vec<(u64, u32, u32)> = vec![];
        for c in self.live_classes(e, None) {
            if let Some(ks) = self.occ.get(&(e, c)) {
                for &k in ks {
                    v.push((self.stamp_of(e, c, k), c, k));
                }
            }
        }
        v.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
        // the newest few of each kind, ranked within their kind (a slot's recency among the entity's values of the
        // same kind: the current holder is rank 0 even when the clause also wrote "up" or "." beside it)
        if self.cfg.sq_rank > 0 {
            // derived slots ranked by their query's balance or credits (stable: ties keep the newest first)
            let val = |c: u32| -> i64 {
                self.sq.iter().find(|o| o.id + DER == c).map_or(i64::MIN, |o| if self.cfg.sq_rank == 1 { o.util as i64 } else { o.credits as i64 })
            };
            let mut der: Vec<(u64, u32, u32)> = v.iter().filter(|x| x.1 >= DER).copied().collect();
            der.sort_by(|a, b| val(b.1).cmp(&val(a.1)));
            let mut it = der.into_iter();
            for x in v.iter_mut() {
                if x.1 >= DER {
                    *x = it.next().unwrap();
                }
            }
        }
        if self.cfg.kindfirst {
            // each kind's newest first, then the second newest of each (stable: newest first within a rank)
            let mut rk: HashMap<u32, u32> = HashMap::new();
            let mut tagged: Vec<(u32, (u64, u32, u32))> = v
                .iter()
                .map(|x| {
                    if x.1 >= DER {
                        return (0, *x);
                    }
                    let r = rk.entry(x.2).or_insert(0);
                    *r += 1;
                    (*r - 1, *x)
                })
                .collect();
            tagged.sort_by_key(|t| t.0);
            v = tagged.into_iter().map(|t| t.1).collect();
        }
        let mut seen_k: HashMap<u32, u32> = HashMap::new();
        let mut out = vec![];
        let mut derived = 0;
        for x in v {
            if x.1 >= DER {
                // open queries' derived slots have a budget of their own (they must not crowd out the facts)
                if derived < 4 {
                    out.push((x.1, x.2, 0));
                    derived += 1;
                }
                continue;
            }
            let r = seen_k.entry(x.2).or_insert(0);
            if *r < 2 && out.len() < n + derived {
                out.push((x.1, x.2, *r));
            }
            *r += 1;
        }
        out
    }
    /// Score features against the current query: the sum of W[q bit, feature].
    fn qscore(&self, q: &[u32], f: &[u32], cache: &mut HashMap<u32, i64>) -> i64 {
        // the query as counts per bit (a bit two hashes land on counts twice, as in the dense table)
        let mut qset = [0u8; QD];
        if self.cfg.qsparse == 2 {
            for &b in q {
                qset[b as usize] += 1;
            }
        }
        f.iter()
            .map(|&fi| {
                *cache.entry(fi).or_insert_with(|| {
                    if self.cfg.qsparse == 2 {
                        self.qr.score(fi, &qset)
                    } else if self.cfg.qsparse == 1 {
                        q.iter().map(|&b| self.qs.get(b * NF as u32 + fi) as i64).sum()
                    } else {
                        q.iter().map(|&b| self.qw[b as usize * NF + fi as usize] as i64).sum()
                    }
                })
            })
            .sum()
    }
    fn qupdate(&mut self, q: &[u32], f: &[u32], d: i16) {
        for &b in q {
            for &fi in f {
                if self.cfg.qsparse == 2 {
                    self.qr.add(fi, b as u16, d as i8);
                } else if self.cfg.qsparse == 1 {
                    self.qs.add(b * NF as u32 + fi, d as i8);
                } else {
                    let x = &mut self.qw[b as usize * NF + fi as usize];
                    *x = x.saturating_add(d);
                }
            }
        }
    }
    /// The query learner's candidates: every one- and two-hop route from the cue words' newest live slots, scored.
    fn query_recall(&mut self, cue: &[u32]) -> Vec<(u32, u32)> {
        let q = self.query_bits(cue);
        self.after_q = self.hist.last().map_or(false, |&u| self.boundary[u as usize] && self.mood[u as usize] >= 8);
        if self.cfg.sq_max > 0 && self.cfg.sq_mode > 0 && self.after_q {
            if self.qring.len() < 32 {
                self.qring.push(q.clone());
            } else {
                self.qring[self.qring_at] = q.clone();
            }
            self.qring_at = (self.qring_at + 1) % 32;
        }
        if let Ok(at) = std::env::var("SIT_SQDBG") {
            if at.split(',').any(|x| x.parse::<u64>().ok() == Some(self.t)) {
                for o in &self.sq {
                    for &e in cue.iter().take(3) {
                        let m = self.sq_ptr.get(&(o.id, e)).copied();
                        let d = self.read_best(e, DER + o.id, o.p2.1, &[]);
                        let ms = m.map(|m| format!("{:?}", self.newest_slots(m, 8))).unwrap_or_default();
                        eprintln!("SQDBG t {} q{} cue {} ptr {:?} derived {:?} | ptr's newest slots {}", self.t, o.id, e, m, d, ms);
                    }
                }
            }
        }
        let mut cache: HashMap<u32, i64> = HashMap::new();
        let mut cands: Vec<(Vec<u32>, u32, i64, u64, Route, (u32, u32))> = vec![];
        let dbg = std::env::var("SIT_QDBG").is_ok();
        let mut desc: Vec<String> = vec![];
        for (j, &e) in cue.iter().enumerate() {
            if !self.seen[e as usize] {
                continue;
            }
            for (c, k, r) in self.newest_slots(e, 8) {
                let Some((m, _)) = self.read_best(e, c, k, cue) else { continue };
                let f1 = self.route_feats(j, e, &[(c, k, r)]);
                let sc = self.qscore(&q, &f1, &mut cache);
                if dbg {
                    desc.push(format!("j{}:{}>c{}k{}r{}={}", j, e, c, k, r, m));
                }
                cands.push((f1, m, sc, self.stamp_of(e, c, k), (c, k, u32::MAX, u32::MAX), (r, u32::MAX)));
                for (c2, k2, r2) in self.newest_slots(m, 8) {
                    let mut ex = cue.to_vec();
                    ex.push(m);
                    let Some((v, _)) = self.read_best(m, c2, k2, &ex) else { continue };
                    let f2 = self.route_feats(j, e, &[(c, k, r), (c2, k2, r2)]);
                    let sc = self.qscore(&q, &f2, &mut cache);
                    if dbg {
                        desc.push(format!("j{}:{}>c{}k{}r{}={}>c{}k{}r{}={}", j, e, c, k, r, m, c2, k2, r2, v));
                    }
                    cands.push((f2, v, sc, self.stamp_of(e, c, k), (c, k, c2, k2), (r, r2)));
                }
            }
        }
        // best score first; ties: the newest first slot, then the shorter route (stable sort keeps one-hop first)
        if dbg {
            let mut ix: Vec<usize> = (0..cands.len()).collect();
            ix.sort_by(|&a, &b| cands[b].2.cmp(&cands[a].2).then(cands[b].3.cmp(&cands[a].3)));
            let parts: Vec<String> = ix.iter().take(10).map(|&i| format!("{}[{}]", desc[i], cands[i].2)).collect();
            eprintln!("QROUTES {} {}", self.t, parts.join(" "));
        }
        cands.sort_by(|a, b| b.2.cmp(&a.2).then(b.3.cmp(&a.3)));
        let lo = cands.iter().map(|x| x.2).min().unwrap_or(0);
        let props: Vec<(u32, u32)> = cands.iter().map(|x| ((x.2 - lo + 1).min(u32::MAX as i64) as u32, x.1)).collect();
        if std::env::var("SIT_QDBG").is_ok() && !cands.is_empty() {
            let vals: Vec<String> = cands.iter().take(12).map(|x| format!("{}:{}", x.1, x.2)).collect();
            eprintln!("QDBG {} n {} top {}", self.t, cands.len(), vals.join(" "));
        }
        self.qbits = q;
        self.qhops = cands.iter().map(|x| x.4).collect();
        self.qranks = cands.iter().map(|x| x.5).collect();
        self.qcands = cands.into_iter().map(|x| (x.0, x.1, x.2)).collect();
        props
    }
    /// Learn the query from word w: a mistake-driven (perceptron) three-factor update. When the top route was
    /// wrong, the best-scored route that held w is stepped up (Go) and the top route down (NoGo), for every
    /// (query bit, route feature) pair: pre-synaptic the query, post-synaptic the route, the modulator right or wrong.
    fn query_learn(&mut self, w: u32) {
        let cands = std::mem::take(&mut self.qcands);
        let q = std::mem::take(&mut self.qbits);
        let hops = std::mem::take(&mut self.qhops);
        let ranks = std::mem::take(&mut self.qranks);
        if self.cfg.sq_max > 0 && self.after_q {
            // which two-hop patterns answered (candidates to open), and which open queries' slots did (credit), at
            // the answers to questions (after a boundary the mood counter has learned closes a question)
            let mut credited: Vec<(u32, i32)> = vec![];
            let top_right = cands.first().map_or(false, |x| x.1 == w);
            for (i, (cd, h)) in cands.iter().zip(&hops).enumerate() {
                if cd.1 != w {
                    continue;
                }
                if h.2 != u32::MAX && h.0 < COMP && h.2 < COMP && (self.cfg.sq_open == 0 || !top_right) {
                    *self.sq_cand.entry(((h.0, h.1), (h.2, h.3))).or_insert(0) += 1;
                    if self.cfg.sq_kinds && ranks.get(i) == Some(&(0, 0)) {
                        *self.sq_cand.entry(((ANY, h.1), (ANY, h.3))).or_insert(0) += 1;
                    }
                }
                if h.0 >= DER && !credited.iter().any(|c| c.0 == h.0 - DER) {
                    let amt = if self.cfg.sq_credit == 0 {
                        32
                    } else if i == 0 {
                        32 // it gave the answer, and the answer was right
                    } else if !top_right {
                        16 // the answer given was wrong, and it held the right one
                    } else {
                        0 // another route answered right anyway
                    };
                    if amt > 0 {
                        credited.push((h.0 - DER, amt));
                    }
                }
            }
            for (id, amt) in credited {
                if let Some(o) = self.sq.iter_mut().find(|o| o.id == id) {
                    o.util = (o.util + amt).min(32768); // a credit of 32 is worth 32 reads and writes
                    o.credits += 1;
                }
            }
            // blame: the answer given came through an open query's slot and was wrong
            if let (Some(top), Some(h)) = (cands.first(), hops.first()) {
                if top.1 != w && h.0 >= DER {
                    if let Some(o) = self.sq.iter_mut().find(|o| o.id == h.0 - DER) {
                        o.util -= 64;
                    }
                }
            }
        }
        if let Some(top) = cands.first() {
            if top.1 != w {
                if let Some(good) = cands.iter().find(|x| x.1 == w) {
                    let (gf, tf) = (good.0.clone(), top.0.clone());
                    self.qupdate(&q, &gf, 1);
                    self.qupdate(&q, &tf, -1);
                    // keep the mistake for replay: the routes as they were, with their answer
                    if self.cfg.replay > 0 {
                        let ep = (q.clone(), cands.iter().map(|x| (x.0.clone(), x.1)).collect(), w);
                        if self.ring.len() < 512 {
                            self.ring.push(ep);
                        } else {
                            self.ring[self.ring_at] = ep;
                        }
                        self.ring_at = (self.ring_at + 1) % 512;
                    }
                }
            }
        }
        // replay: re-score a few kept mistakes under the current weights, and learn again from those still wrong
        for i in 0..self.cfg.replay {
            if self.ring.is_empty() {
                break;
            }
            let at = (hash(self.t, 4242 + i as u64) % self.ring.len() as u64) as usize;
            let (rq, routes, rw) = self.ring[at].clone();
            let mut cache: HashMap<u32, i64> = HashMap::new();
            let mut best: Option<(i64, usize)> = None;
            let mut good: Option<(i64, usize)> = None;
            for (r, (f, v)) in routes.iter().enumerate() {
                let sc = self.qscore(&rq, f, &mut cache);
                if best.map_or(true, |b| sc > b.0) {
                    best = Some((sc, r));
                }
                if *v == rw && good.map_or(true, |g| sc > g.0) {
                    good = Some((sc, r));
                }
            }
            if let (Some((_, b)), Some((_, g))) = (best, good) {
                if routes[b].1 != rw {
                    let (gf, bf) = (routes[g].0.clone(), routes[b].0.clone());
                    self.qupdate(&rq, &gf, 1);
                    self.qupdate(&rq, &bf, -1);
                }
            }
        }
    }

    /// The window's routes: from each cue word's latest occurrences (up to 3) in the lookback window, the word at
    /// an offset of -6..+6 (one hop), and from that word's latest 2 occurrences, the word at an offset again (two
    /// hops). Features: cue position, the cue word's kind, the occurrence rank, the offset, the value's kind, and for
    /// two hops the second occurrence rank, offset, the intermediate's kind and the pair of offsets.
    fn window_cands(&self, cue: &[u32], q: &[u32]) -> Vec<(Vec<u32>, u32, i64)> {
        let n = self.win.len();
        let occ = |x: u32, k: usize| -> Vec<usize> {
            let mut v = vec![];
            for i in (0..n).rev() {
                if self.win[i] == x {
                    v.push(i);
                    if v.len() >= k {
                        break;
                    }
                }
            }
            v
        };
        let mut cache: HashMap<u32, i64> = HashMap::new();
        let mut out: Vec<(Vec<u32>, u32, i64)> = vec![];
        for (j, &e) in cue.iter().enumerate() {
            let ke = self.kk(e) as u64;
            for (r, &i) in occ(e, 3).iter().enumerate() {
                for d in -6i64..=6 {
                    let p = i as i64 + d;
                    if d == 0 || p < 0 || p >= n as i64 {
                        continue;
                    }
                    let m = self.win[p as usize];
                    let km = self.kk(m) as u64;
                    let f1 = vec![
                        Self::fid(20, j as u64, 0), Self::fid(21, ke, 0), Self::fid(22, r as u64, 0),
                        Self::fid(23, (d + 6) as u64, 0), Self::fid(24, km, 0), Self::fid(29, (d + 6) as u64, km),
                    ];
                    let sc = self.qscore(q, &f1, &mut cache);
                    out.push((f1.clone(), m, sc));
                    if j != 1 && j != 0 {
                        continue; // two hops only from the nearest cue words (cost)
                    }
                    for (r2, &i2) in occ(m, 2).iter().enumerate() {
                        for d2 in -6i64..=6 {
                            let p2 = i2 as i64 + d2;
                            if d2 == 0 || p2 < 0 || p2 >= n as i64 {
                                continue;
                            }
                            let v = self.win[p2 as usize];
                            let kv = self.kk(v) as u64;
                            let mut f2 = f1.clone();
                            f2.pop();
                            f2.extend([
                                Self::fid(25, r2 as u64, 0), Self::fid(26, (d2 + 6) as u64, 0), Self::fid(27, kv, 0),
                                Self::fid(28, (d + 6) as u64, (d2 + 6) as u64), Self::fid(30, km, kv),
                            ]);
                            let sc = self.qscore(q, &f2, &mut cache);
                            out.push((f2, v, sc));
                        }
                    }
                }
            }
        }
        out.sort_by(|a, b| b.2.cmp(&a.2));
        out
    }
    /// The margin of a ranked candidate list: how far the top score is above the best score for another value,
    /// as a bucket (0 = no margin, else the bit length of the margin, up to 15).
    fn margin_bucket(c: &[(Vec<u32>, u32, i64)]) -> u32 {
        let Some(top) = c.first() else { return 0 };
        let other = c.iter().find(|x| x.1 != top.1).map(|x| x.2);
        match other {
            None => 15,
            Some(o) => {
                let m = (top.2 - o).max(0) as u64;
                (64 - m.leading_zeros()).min(15)
            }
        }
    }

    // ---- open queries ----
    /// Close the open queries that have stopped paying for their upkeep, and open the most counted candidate
    /// pattern if there is room: so the number open tunes itself.
    fn sq_manage(&mut self) {
        let t = self.t;
        // diagnostics: SIT_SQ_FORCE=a,b holds the kind-level chain (any, kind of a) then (any, kind of b) open from
        // word 2,048 on, and never closes it (a test of what the chain is worth once open)
        if let Ok(fw) = std::env::var("SIT_SQ_FORCE") {
            let w: Vec<u32> = fw.split(',').filter_map(|x| x.parse().ok()).collect();
            if w.len() == 2 && t == 2048 {
                let id = self.sq_next;
                self.sq_next += 1;
                let (ka, kb) = (self.kk(w[0]), self.kk(w[1]));
                self.sq.push(OpenQuery { id, p1: (ANY, ka), p2: (ANY, kb), util: 1 << 30, credits: 0, work: 0, opened_at: t });
                self.sq_opened += 1;
            }
        }
        // the open set's net payoff and age, over the queries past their trial (for the relative price)
        let (mut sn, mut sa) = (0i64, 0i64);
        for o in self.sq.iter().filter(|o| t - o.opened_at >= 2000) {
            sn += o.util as i64 - 512;
            sa += (t - o.opened_at) as i64;
        }
        let price = self.cfg.sq_price;
        let mut closing: Vec<u32> = self
            .sq
            .iter()
            .filter(|o| {
                let age = (t - o.opened_at) as i64;
                if age < 2000 {
                    return false;
                }
                let below_avg = (o.util as i64 - 512) * sa < sn * age; // its payoff per word below the set's
                match price {
                    0 => o.util < 0,
                    1 => below_avg,
                    _ => o.util < 0 || below_avg,
                }
            })
            .map(|o| o.id)
            .collect();
        closing.sort_unstable();
        for id in closing {
            self.sq_close(id);
        }
        if self.sq.len() < self.cfg.sq_max {
            let open: Vec<((u32, u32), (u32, u32))> = self.sq.iter().map(|o| (o.p1, o.p2)).collect();
            let best = if self.cfg.sq_mode == 0 {
                self.sq_cand.iter().filter(|(p, &n)| n >= 16 && !open.contains(p)).max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0))).map(|(p, _)| *p)
            } else {
                self.sq_pick(&open)
            };
            if let Some(p) = best {
                self.sq_cand.insert(p, 0);
                self.sq_in.remove(&p);
                let id = self.sq_next;
                self.sq_next += 1;
                self.sq.push(OpenQuery { id, p1: p.0, p2: p.1, util: 512, credits: 0, work: 0, opened_at: t });
                self.sq_opened += 1;
            }
        }
    }
    /// The input's proposals (counted at least 16 times; with mode 2, also at least 4 answers held) ranked by the
    /// higher level: the chain's route features summed under the learned query weights, over the question codes
    /// seen lately, at the best cue position. The best positive one, or none.
    fn sq_pick(&mut self, open: &[((u32, u32), (u32, u32))]) -> Option<((u32, u32), (u32, u32))> {
        if self.qring.is_empty() {
            return None;
        }
        if self.sq_in.len() > 65536 {
            // forget slowly: halve the counts and drop the ones that reach zero
            self.sq_in.retain(|_, v| {
                v.0 >>= 1;
                v.0 > 0
            });
        }
        let mut cands: Vec<(((u32, u32), (u32, u32)), u32)> = self
            .sq_in
            .iter()
            .filter(|(p, v)| v.0 >= 16 && !open.contains(p) && (self.cfg.sq_mode < 2 || self.sq_cand.get(p).copied().unwrap_or(0) >= 4))
            .map(|(p, v)| (*p, v.1))
            .collect();
        cands.sort_unstable();
        let mut best: Option<(i64, ((u32, u32), (u32, u32)))> = None;
        for (p, x) in cands {
            let hops = [(p.0 .0, p.0 .1, 0), (p.1 .0, p.1 .1, 0)];
            let mut top = i64::MIN;
            for j in 0..4 {
                let f = self.route_feats(j, x, &hops);
                let mut cache: HashMap<u32, i64> = HashMap::new();
                let mut s = 0i64;
                for q in &self.qring {
                    cache.clear();
                    s += self.qscore(q, &f, &mut cache);
                }
                top = top.max(s);
            }
            if top > 0 && best.map_or(true, |b| top > b.0) {
                best = Some((top, p));
            }
        }
        best.map(|b| b.1)
    }
    fn sq_close(&mut self, id: u32) {
        self.sq.retain(|o| o.id != id);
        self.sq_ptr.retain(|k, _| k.0 != id);
        self.sq_rev.retain(|k, _| k.0 != id);
        self.sq_closed += 1;
    }
    /// A committed fact (x, class c, kind kd) = v, checked against every open query: it may move x's pointer (the
    /// query's first hop) and refresh x's derived slot, or change an intermediate's second hop and refresh every
    /// entity pointing to it. A fact of the first hop's kind under another class detaches x.
    fn sq_on_fact(&mut self, x: u32, c: u32, kd: u32, v: u32) {
        let qs: Vec<(u32, (u32, u32), (u32, u32))> = self.sq.iter().map(|o| (o.id, o.p1, o.p2)).collect();
        let hit = |p: (u32, u32)| kd == p.1 && c < COMP && (p.0 == ANY || p.0 == c);
        for (id, p1, p2) in qs {
            let mut work = 0u64;
            if hit(p1) {
                if let Some(old) = self.sq_ptr.insert((id, x), v) {
                    if let Some(r) = self.sq_rev.get_mut(&(id, old)) {
                        r.retain(|&y| y != x);
                    }
                }
                self.sq_rev.entry((id, v)).or_default().push(x);
                work += 1;
                if let Some(z) = self.sq_hop2(v, p2, &[v, x]) {
                    self.sq_write(x, id, p2.1, z);
                    work += 2;
                }
            } else if kd == p1.1 && c < COMP && p1.0 != ANY {
                // detach: x took a value of that kind under another relation
                if let Some(old) = self.sq_ptr.remove(&(id, x)) {
                    if let Some(r) = self.sq_rev.get_mut(&(id, old)) {
                        r.retain(|&y| y != x);
                    }
                    work += 1;
                }
            }
            if hit(p2) {
                let xs: Vec<u32> = self.sq_rev.get(&(id, x)).cloned().unwrap_or_default();
                for xx in xs {
                    self.sq_write(xx, id, p2.1, v);
                    work += 1;
                }
            }
            if work > 0 {
                self.sq_work += work;
                if let Some(o) = self.sq.iter_mut().find(|o| o.id == id) {
                    o.work += work;
                    o.util -= work as i32; // upkeep: one unit per read or write
                }
            }
        }
    }
    /// An open query's second hop from the intermediate m: its slot (class, kind), or for a hop of any relation, m's
    /// newest value of that kind.
    fn sq_hop2(&self, m: u32, p2: (u32, u32), excl: &[u32]) -> Option<u32> {
        if p2.0 == ANY {
            // m's newest value of that kind, over all its live slots (not only the newest few: "daniel picked up the
            // key ." writes four values into daniel's slots and would push his place out of a short list)
            let mut best: Option<(u64, u32)> = None;
            for c in self.live_classes(m, None) {
                if c >= COMP {
                    continue;
                }
                if self.occ.get(&(m, c)).map_or(false, |ks| ks.contains(&p2.1)) {
                    let st = self.stamp_of(m, c, p2.1);
                    if best.map_or(true, |b| st > b.0) {
                        best = Some((st, c));
                    }
                }
            }
            let (_, c2) = best?;
            self.read_best(m, c2, p2.1, excl).map(|x| x.0)
        } else if self.live(m, p2.0, p2.1) {
            self.read_best(m, p2.0, p2.1, excl).map(|x| x.0)
        } else {
            None
        }
    }
    fn sq_write(&mut self, x: u32, id: u32, kd: u32, z: u32) {
        let cs = DER + id;
        self.write(x, cs, kd, z);
        let ks = self.occ.entry((x, cs)).or_default();
        if !ks.contains(&kd) {
            ks.push(kd);
        }
        let cl = self.dcl.entry((x, 3)).or_default();
        if !cl.contains(&cs) {
            cl.push(cs);
        }
        self.stamp.insert((x, cs, kd), self.t);
    }
    /// The most counted candidate patterns (diagnostics): (pattern, count).
    pub fn sq_top_cands(&self, n: usize) -> Vec<(((u32, u32), (u32, u32)), u32)> {
        let mut v: Vec<_> = self.sq_cand.iter().map(|(p, c)| (*p, *c)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v.truncate(n);
        v
    }
    /// The query weights' footprint (diagnostics): (stored entries, nonzero, at the 8-bit limit (sparse) or beyond 8
    /// bits (dense), bytes).
    pub fn qw_census(&self) -> (usize, usize, usize, usize) {
        if self.cfg.qsparse == 2 {
            let (nz, sat) = self.qr.census();
            (nz, nz, sat, self.qr.bytes())
        } else if self.cfg.qsparse == 1 {
            let (nz, sat) = self.qs.census();
            (self.qs.len, nz, sat, self.qs.bytes())
        } else {
            let nz = self.qw.iter().filter(|&&x| x != 0).count();
            let big = self.qw.iter().filter(|&&x| x > 127 || x < -128).count();
            (self.qw.len(), nz, big, self.qw.len() * 2)
        }
    }
    pub fn words_seen(&self) -> u64 {
        self.t
    }
    /// The open queries now (diagnostics).
    pub fn open_queries(&self) -> Vec<OpenQuery> {
        self.sq.clone()
    }

    fn live(&self, e: u32, c: u32, kd: u32) -> bool {
        !self.cfg.kinds || self.occ.get(&(e, c)).map(|v| v.contains(&kd)).unwrap_or(false)
    }
    fn stamp_of(&self, e: u32, c: u32, kd: u32) -> u64 {
        self.stamp.get(&(e, c, kd)).copied().unwrap_or(0)
    }

    /// The candidates for the next word: (value, weight), best first.
    pub fn recall(&mut self) -> Vec<(u32, f64)> {
        let ctx = self.ctx_key();
        let cue: Vec<u32> = (0..self.cfg.cue.min(self.hist.len())).map(|j| self.hist[self.hist.len() - 1 - j]).collect();
        if self.cfg.query {
            let mut props = self.query_recall(&cue);
            if self.cfg.lookback > 0 {
                // the slots' answer, its margin, and the question form; then the window's answer; answer from the
                // window where, for this form and margin, the window has been right more often than the slots
                let q = self.qbits.clone();
                self.wcands = self.window_cands(&cue, &q);
                let w0 = cue.first().copied().unwrap_or(u32::MAX) as u64;
                let w2 = cue.get(2).copied().unwrap_or(u32::MAX) as u64;
                let w3 = cue.get(3).copied().unwrap_or(u32::MAX) as u64;
                // the key: the question form, and both sources' margin buckets
                let key = (hash(hash(hash(w0, 7), w2), w3), Self::margin_bucket(&self.qcands) * 16 + Self::margin_bucket(&self.wcands));
                self.arb_key = key;
                self.slot_top = self.qcands.first().map(|x| x.1);
                let a = *self.arb.get(&key).unwrap_or(&[0; 4]);
                // window accuracy > slot accuracy, by cross-multiplication, with a prior of 1 right in 2; merged:
                // the window wins where its best route outscores the slots' best under the shared weights
                let use_win = !self.wcands.is_empty()
                    && if self.cfg.merge {
                        self.qcands.first().map_or(true, |s0| self.wcands[0].2 > s0.2)
                    } else {
                        (a[2] as u64 + 1) * (a[1] as u64 + 2) > (a[0] as u64 + 1) * (a[3] as u64 + 2)
                    };
                if use_win {
                    self.used_window += 1;
                    if let Some(&u) = self.hist.last() {
                        if self.boundary[u as usize] && self.mood[u as usize] >= 8 {
                            self.used_window_q += 1;
                        }
                    }
                    let lo = self.wcands.iter().map(|x| x.2).min().unwrap_or(0);
                    props = self.wcands.iter().map(|x| ((x.2 - lo + 1).min(u32::MAX as i64) as u32, x.1)).collect();
                }
                if let Some(&u) = self.hist.last() {
                    if self.boundary[u as usize] && self.mood[u as usize] >= 8 {
                        self.questions_seen += 1;
                    }
                }
                if std::env::var("SIT_WDBG").is_ok() {
                    eprintln!("WDBG {} slot {} win {} bucket {} use {}", self.t, self.slot_top.map_or(-1, |x| x as i64), self.wcands.first().map_or(-1, |x| x.1 as i64), key.1, use_win as u8);
                }
            }
            self.last = props.clone();
            let tot: f64 = props.iter().map(|x| x.0 as f64).sum();
            let mut out: Vec<(u32, f64)> = vec![];
            for (n, v) in props {
                match out.iter_mut().find(|x| x.0 == v) {
                    Some(x) => x.1 += n as f64 / tot.max(1.0),
                    None => out.push((v, n as f64 / tot.max(1.0))),
                }
            }
            return out;
        }
        let mut props: Vec<(u32, u32, i128)> = vec![];
        self.routes.clear();
        for (j, &e) in cue.iter().enumerate() {
            if !self.is_entity(e) {
                continue;
            }
            if self.cfg.bounce {
                // a loop of reads: stop, or follow the best-counted live slot of the current entity
                let (mut cur, mut lastc) = (e, 0u32);
                for step in 0..3u8 {
                    if step > 0 && *self.halt.get(&(ctx, self.kk(cur), step)).unwrap_or(&7) >= 8 {
                        props.push((lastc, cur, 0));
                        break;
                    }
                    let Some(h) = self.hop.get(&(ctx, j, step)) else { break };
                    let mut v: Vec<((u32, u32), u32)> = h.iter().map(|(&c, &n)| (c, n)).filter(|x| x.1 >= 2 && self.live(cur, x.0 .0, x.0 .1)).collect();
                    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                    v.truncate(6);
                    if v.is_empty() {
                        break;
                    }
                    if self.cfg.recency {
                        let maxn = v[0].1;
                        v.retain(|x| 2 * x.1 >= maxn);
                        v.sort_by(|a, b| self.stamp_of(cur, b.0 .0, b.0 .1).cmp(&self.stamp_of(cur, a.0 .0, a.0 .1)).then(b.1.cmp(&a.1)));
                    }
                    let ((c, kd), n) = v[0];
                    lastc = n;
                    let mut ex = cue.clone();
                    ex.push(cur);
                    match self.read_best(cur, c, kd, &ex) {
                        Some((val, _)) => cur = val,
                        None => break,
                    }
                }
                continue;
            }
            // (kind, first-slot recency, hits, value, misses, route)
            let mut cand: Vec<(u32, u64, u32, u32, u32, Route)> = vec![];
            if let Some(c1) = self.c1.get(&(ctx, j)) {
                let mut v: Vec<((u32, u32), u32)> = c1.iter().map(|(&c, &n)| (c, n)).filter(|x| x.1 >= 2).collect();
                v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                for &((c, kd), n) in v.iter().filter(|x| self.live(e, x.0 .0, x.0 .1)).take(self.cfg.nroutes.max(3)) {
                    if let Some((val, _)) = self.read_best(e, c, kd, &cue) {
                        let r: Route = (c, kd, u32::MAX, u32::MAX);
                        cand.push((kd, self.stamp_of(e, c, kd), n, val, *self.miss.get(&(ctx, j, r)).unwrap_or(&0), r));
                    }
                }
            }
            if let Some(c2) = self.c2.get(&(ctx, j)) {
                let mut v: Vec<(((u32, u32), (u32, u32)), u32)> = c2.iter().map(|(&c, &n)| (c, n)).filter(|x| x.1 >= 2).collect();
                v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                for &(((c1_, k1), (c2_, k2)), n) in v.iter().filter(|x| self.live(e, x.0 .0 .0, x.0 .0 .1)).take(self.cfg.nroutes.max(2)) {
                    if let Some((m, _)) = self.read_best(e, c1_, k1, &[e]) {
                        if !self.live(m, c2_, k2) {
                            continue;
                        }
                        if let Some((val, _)) = self.read_best(m, c2_, k2, &cue) {
                            let r: Route = (c1_, k1, c2_, k2);
                            cand.push((k2, self.stamp_of(e, c1_, k1), n, val, *self.miss.get(&(ctx, j, r)).unwrap_or(&0), r));
                        }
                    }
                }
            }
            for x in &cand {
                self.routes.push((ctx, j, x.5, x.3));
            }
            if let Ok(from) = std::env::var("SIT_DEBUG_FROM") {
                let last = *self.hist.last().unwrap_or(&0) as usize;
                if self.t >= from.parse::<u64>().unwrap_or(u64::MAX) && self.boundary[last] {
                    for x in &cand {
                        eprintln!("DBG {} j{} e{} route {:?} hits {} misses {} -> {} stamp {}", self.t, j, e, x.5, x.2, x.4, x.3, x.1);
                    }
                }
            }
            // a route's weight: its hits and misses; with --sit-precision ranked by hits / (hits + misses) (compared by
            // cross-multiplication), else by hits
            let better = |a: (u32, u32), b: (u32, u32), prec: bool| -> bool {
                // Go minus NoGo: hits minus misses (the striatal rule); a precision ranking let rare, lucky
                // routes win over well-tested ones
                if prec { (a.0 as i64 - a.1 as i64) > (b.0 as i64 - b.1 as i64) } else { a.0 > b.0 }
            };
            let prec = self.cfg.precision;
            if self.cfg.recency && !cand.is_empty() {
                // per kind: credible proposals (at least half the best), the newest first
                let mut bestw = (cand[0].2, cand[0].4);
                for x in &cand {
                    if better((x.2, x.4), bestw, prec) {
                        bestw = (x.2, x.4);
                    }
                }
                let half = |x: (u32, u32)| -> bool {
                    if prec { 2 * (x.0 as i64 - x.1 as i64) >= (bestw.0 as i64 - bestw.1 as i64) } else { self.cfg.cred * x.0 >= bestw.0 }
                };
                cand.retain(|x| half((x.2, x.4)));
                let mut kinds: Vec<u32> = cand.iter().map(|x| x.0).collect();
                kinds.sort_unstable();
                kinds.dedup();
                for kd in kinds {
                    let mut g: Vec<&(u32, u64, u32, u32, u32, Route)> = cand.iter().filter(|x| x.0 == kd).collect();
                    g.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
                    let mut gw = (g[0].2, g[0].4);
                    for x in &g {
                        if better((x.2, x.4), gw, prec) {
                            gw = (x.2, x.4);
                        }
                    }
                    // the group ranks by its best weight; inside it, newest first (kept by the stable sort)
                    for x in g {
                        props.push((gw.0, x.3, gw.1 as i128));
                    }
                }
            } else {
                props.extend(cand.into_iter().map(|x| (x.2, x.3, x.4 as i128)));
            }
        }
        // routes reliable enough (counts at least half the best), then the freshest (strongest) reading first
        if self.cfg.precision {
            // the third field holds the misses: best precision first, compared by cross-multiplication
            props.sort_by(|a, b| {
                let (ha, ma, hb, mb) = (a.0 as i128, a.2, b.0 as i128, b.2);
                (hb - mb).cmp(&(ha - ma)).then(b.0.cmp(&a.0))
            });
        } else {
            props.sort_by(|a, b| b.0.cmp(&a.0));
        }
        let props: Vec<(u32, u32)> = props.into_iter().map(|x| (x.0, x.1)).collect();
        self.last = props.clone();
        let tot: f64 = props.iter().map(|x| x.0 as f64).sum();
        let mut out: Vec<(u32, f64)> = vec![];
        for (n, v) in props {
            match out.iter_mut().find(|x| x.0 == v) {
                Some(x) => x.1 += n as f64 / tot.max(1.0),
                None => out.push((v, n as f64 / tot.max(1.0))),
            }
        }
        out
    }

    /// Learn from word w (after it has been predicted).
    pub fn observe(&mut self, w: u32) {
        let ctx = self.ctx_key();
        let cue: Vec<u32> = (0..self.cfg.cue.min(self.hist.len())).map(|j| self.hist[self.hist.len() - 1 - j]).collect();
        self.wcount[w as usize] += 1;
        // NoGo: every route proposed for this word that was wrong
        // (only the routes behind the answer actually given, the top proposal, are blamed: the bandit's rule,
        // only the taken action's outcome is seen)
        let given = self.last.first().map(|x| x.1);
        for &(cx, j, r, v) in &std::mem::take(&mut self.routes) {
            if Some(v) == given && v != w {
                *self.miss.entry((cx, j, r)).or_insert(0) += 1;
            }
        }
        // mood: after a boundary, was the store's top proposal this word? (up on a hit, down on half the misses)
        if let Some(&u) = self.hist.last() {
            if self.boundary[u as usize] {
                let ok = self.last.first().map(|x| x.1) == Some(w);
                let m = &mut self.mood[u as usize];
                if ok {
                    *m = (*m + 1).min(15);
                } else if hash(self.t, u as u64 + 777) & 1 == 0 {
                    *m = m.saturating_sub(1);
                }
            }
        }
        if self.cfg.query {
            if self.cfg.lookback > 0 {
                let wc = std::mem::take(&mut self.wcands);
                let q = self.qbits.clone();
                if self.cfg.merge {
                    // one pool: when the answer given was wrong, the best-scored route of either source that held w
                    // steps up and the given route steps down; the slots' own update then follows as usual
                    let given_win = self.last.first().map(|x| x.1) == wc.first().map(|x| x.1)
                        && wc.first().map_or(false, |t| self.qcands.first().map_or(true, |s0| t.2 > s0.2));
                    if given_win {
                        let top = wc[0].clone();
                        if top.1 != w {
                            let gw = wc.iter().find(|x| x.1 == w).map(|x| (x.0.clone(), x.2));
                            let gs = self.qcands.iter().find(|x| x.1 == w).map(|x| (x.0.clone(), x.2));
                            let good = match (gw, gs) {
                                (Some(a), Some(b)) => Some(if a.1 >= b.1 { a.0 } else { b.0 }),
                                (Some(a), None) => Some(a.0),
                                (None, Some(b)) => Some(b.0),
                                _ => None,
                            };
                            if let Some(gf) = good {
                                self.qupdate(&q, &gf, 1);
                                self.qupdate(&q, &top.0, -1);
                            }
                        }
                    } else if self.slot_top != Some(w) {
                        // the slots answered wrong: if the window held w, step its best such route up
                        if let Some(g) = wc.iter().find(|x| x.1 == w) {
                            let gf = g.0.clone();
                            self.qupdate(&q, &gf, 1);
                        }
                    }
                } else if let Some(top) = wc.first() {
                    if top.1 != w {
                        if let Some(good) = wc.iter().find(|x| x.1 == w) {
                            let (gf, tf) = (good.0.clone(), top.0.clone());
                            self.qupdate(&q, &gf, 1);
                            self.qupdate(&q, &tf, -1);
                        }
                    }
                }
                // the arbiter's counts: was each source right here?
                // (only where the two disagree: elsewhere the choice does not matter)
                let wt = wc.first().map(|x| x.1);
                if wt.is_some() && wt != self.slot_top {
                    let a = self.arb.entry(self.arb_key).or_insert([0; 4]);
                    a[0] += (self.slot_top == Some(w)) as u16;
                    a[1] += 1;
                    a[2] += (wt == Some(w)) as u16;
                    a[3] += 1;
                    if a[1] > 1000 {
                        a.iter_mut().for_each(|x| *x >>= 1);
                    }
                }
            }
            self.query_learn(w);
        }
        // which classes answer this context: those whose slot (of a cue word) holds w
        if !self.cfg.query && self.seen[w as usize] && !self.class_count.is_empty() {
            let mut top: Vec<(u64, u32)> = self.class_count.iter().enumerate().map(|(i, &c)| (c, i as u32)).collect();
            top.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            let mut top: Vec<u32> = top.into_iter().take(self.cfg.ntop).map(|x| x.1).collect();
            top.extend(self.comp.values().copied());
            let kw = self.kk(w);
            for (j, &e) in cue.iter().enumerate() {
                if e == w || !self.seen[e as usize] || !self.is_entity(e) {
                    continue;
                }
                let pool1: Vec<u32> = if self.cfg.live { self.live_classes(e, Some(kw)) } else { top.clone() };
                let hits: Vec<u32> = pool1.iter().copied().filter(|&c| self.read_best(e, c, kw, &[e]).map(|x| x.0) == Some(w)).collect();
                if !hits.is_empty() {
                    let m = self.c1.entry((ctx, j)).or_default();
                    for &c in &hits {
                        *m.entry((c, kw)).or_insert(0) += 1;
                    }
                    if self.cfg.bounce {
                        let h = self.hop.entry((ctx, j, 0)).or_default();
                        for &c in &hits {
                            *h.entry((c, kw)).or_insert(0) += 1;
                        }
                        let x = self.halt.entry((ctx, kw, 1)).or_insert(7);
                        *x = (*x + 1).min(15);
                    }
                } else {
                    let t10: Vec<u32> = if self.cfg.live { self.live_classes(e, None) } else { top.iter().take(10).copied().collect() };
                    for &c1_ in &t10 {
                        let k1s: Vec<u32> = if self.cfg.kinds { self.occ.get(&(e, c1_)).cloned().unwrap_or_default() } else { vec![self.kn] };
                        for k1 in k1s {
                            let Some((m, _)) = self.read_best(e, c1_, k1, &[e]) else { continue };
                            if m == w {
                                continue;
                            }
                            let t2: Vec<u32> = if self.cfg.live { self.live_classes(m, Some(kw)) } else { t10.clone() };
                            for &c2_ in &t2 {
                                if self.read_best(m, c2_, kw, &[m]).map(|x| x.0) == Some(w) {
                                    *self.c2.entry((ctx, j)).or_default().entry(((c1_, k1), (c2_, kw))).or_insert(0) += 1;
                                    *self.g2.entry(((c1_, k1), (c2_, kw))).or_insert(0) += 1;
                                    if self.cfg.bounce {
                                        *self.hop.entry((ctx, j, 0)).or_default().entry((c1_, k1)).or_insert(0) += 1;
                                        *self.hop.entry((ctx, j, 1)).or_default().entry((c2_, kw)).or_insert(0) += 1;
                                        let km = self.kk(m);
                                        let x = self.halt.entry((ctx, km, 1)).or_insert(7);
                                        *x = x.saturating_sub(1); // m was not the answer
                                        let x = self.halt.entry((ctx, kw, 2)).or_insert(7);
                                        *x = (*x + 1).min(15);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // segmentation, meaning, kinds' frames, then the facts of w's clause (buffered until its boundary)
        let n = self.hist.len();
        if n >= 2 {
            let (x, u) = (self.hist[n - 2], self.hist[n - 1]);
            self.seg_update(x, u, w);
        }
        if !(self.cfg.fpool && !self.cfg.sense) {
            self.mn.observe(w); // the meaning module (float inside) is only needed for its codes
        }
        if (self.cfg.kinds || self.cfg.fpool) && n >= 1 {
            let u = self.hist[n - 1];
            self.frame_bump(w, Self::fb(1, u as u64));
            if n >= 2 {
                let f = Self::fb(2, self.hist[n - 2] as u64 * 100_003 + u as u64);
                self.frame_bump(w, f);
            }
            self.frame_bump(u, Self::fb(3, w as u64));
        }
        let mut path: Vec<u32> = vec![];
        let back: Vec<u32> = self.clause.iter().rev().take(self.cfg.win).copied().collect();
        for s in back {
            if s != w {
                for dir in 0..2u32 {
                    let mut p = vec![dir];
                    if self.cfg.sense {
                        p.push(path.len() as u32);
                        p.extend(&path);
                        for (pi, &pw) in path.iter().enumerate() {
                            let ctx: Vec<u32> = [s, w].iter().chain(path.iter().enumerate().filter(|(k, _)| *k != pi).map(|(_, x)| x)).copied().collect();
                            let k = self.sense_pick(pw, &ctx, dir == 0).map(|k| k as u32).unwrap_or(u32::MAX);
                            p.push(k);
                        }
                    } else {
                        p.extend(&path);
                    }
                    *self.path_count.entry(p.clone()).or_insert(0) += 1;
                    let c = self.class_of(&p);
                    self.class_count[c as usize] += 1;
                    if dir == 0 {
                        self.buf.push((s, c, 0, w));
                    } else {
                        self.buf.push((w, c, 1, s));
                    }
                }
            }
            path.push(s);
            if self.boundary[s as usize] {
                break;
            }
        }
        if self.boundary[w as usize] || self.buf.len() > 64 {
            if self.cfg.mood && self.boundary[w as usize] && self.mood[w as usize] >= 8 {
                self.buf.clear(); // a question: a cue, not a fact
                self.dropped += 1;
            } else {
                self.commit();
                self.committed += 1;
            }
        }
        self.clause.push(w);
        if self.clause.len() > 16 {
            self.clause.drain(..8);
        }
        self.hist.push(w);
        if self.cfg.lookback > 0 {
            self.win.push(w);
            if self.win.len() > self.cfg.lookback {
                self.win.remove(0);
            }
        }
        if self.hist.len() > 64 {
            self.hist.drain(..32);
        }
        if !self.seen[w as usize] {
            self.seen[w as usize] = true;
            self.words.push(w);
        }
        // the clocks: slot i steps its phase every clock * 2^(i mod 7) words
        if self.cfg.clock > 0 {
            for i in 0..self.cfg.k {
                let per = self.cfg.clock << (i % 7);
                if (self.t + 1) % per == 0 {
                    self.ctx_ph[i] = (self.ctx_ph[i] + 1) & 3;
                }
            }
        }
        // the context drifts at learned boundaries
        if self.boundary[w as usize] {
            for j in 0..self.cfg.bdrift {
                let h = hash(self.t * 31 + j as u64, 97);
                let i = (h % self.cfg.k as u64) as usize;
                self.ctx_off[i] = (hash(h, 98) % self.cfg.d as u64) as u32;
                self.ctx_ph[i] = (hash(h, 99) & 3) as u8;
            }
        }
        self.t += 1;
        if self.cfg.sq_max > 0 {
            self.sq_open_sum += self.sq.len() as u64;
            self.sq_peak = self.sq_peak.max(self.sq.len());
            if self.t % 64 == 0 {
                self.sq_manage();
            }
        }
        if self.cfg.recluster > 0 && self.t % self.cfg.recluster as u64 == 0 {
            self.recluster();
        }
    }

    /// Write the buffered bindings of a clause into their slots.
    fn commit(&mut self) {
        let buf = std::mem::take(&mut self.buf);
        for (x, c, dir, v) in buf {
            let kd = self.kk(v);
            if self.cfg.supersede {
                let others: Vec<u32> = self.dcl.get(&(x, dir as u8)).cloned().unwrap_or_default();
                for c2 in others {
                    if c2 == c {
                        continue;
                    }
                    if let Some(ks) = self.occ.get_mut(&(x, c2)) {
                        if let Some(pos) = ks.iter().position(|&q| q == kd) {
                            ks.remove(pos);
                            let none = self.nw as u32;
                            self.write(x, c2, kd, none); // detach: the old relation now reads as nothing
                        }
                    }
                }
            }
            self.write(x, c, kd, v);
            let ks = self.occ.entry((x, c)).or_default();
            if !ks.contains(&kd) {
                ks.push(kd);
            }
            let cl = self.dcl.entry((x, dir as u8)).or_default();
            if !cl.contains(&c) {
                cl.push(c);
            }
            self.stamp.insert((x, c, kd), self.t);
            if self.cfg.sq_max > 0 {
                self.sq_on_fact(x, c, kd, v);
                if self.cfg.sq_mode > 0 && c < COMP && self.seen[v as usize] {
                    // the input proposes: the chains that start from this fact (x, c) = v, then v's newest slots
                    for (c2, k2, _) in self.newest_slots(v, 8) {
                        if c2 < COMP {
                            let e = self.sq_in.entry(((c, kd), (c2, k2))).or_insert((0, x));
                            e.0 = e.0.saturating_add(1);
                            e.1 = x;
                        }
                    }
                }
            }
            // data bouncing at write time: (x, c, v) and a counted pair ((c, kd), (c2, k2)) -> read v's (c2, k2)
            // slot now and keep the result in x's composite slot ("emma dropped the cup": where emma is now)
            if self.cfg.compose > 0 && c < COMP {
                let mut pairs: Vec<(((u32, u32), (u32, u32)), u32)> = self.g2.iter().map(|(&p, &n)| (p, n)).collect();
                pairs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                for &((p1, p2), _) in pairs.iter().take(self.cfg.compose) {
                    if p1 != (c, kd) {
                        continue;
                    }
                    if let Some((z, _)) = self.read_best(v, p2.0, p2.1, &[v, x]) {
                        let next = COMP + self.comp.len() as u32;
                        let cs = *self.comp.entry((p1, p2)).or_insert(next);
                        self.write(x, cs, p2.1, z);
                        let ks = self.occ.entry((x, cs)).or_default();
                        if !ks.contains(&p2.1) {
                            ks.push(p2.1);
                        }
                        let cl = self.dcl.entry((x, 2)).or_default();
                        if !cl.contains(&cs) {
                            cl.push(cs);
                        }
                        self.stamp.insert((x, cs, p2.1), self.t);
                    }
                }
            }
        }
    }
}
