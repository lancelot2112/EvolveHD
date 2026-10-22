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
//!   Slots are keyed on (entity, class, kind of the value), and C1/C2 count (class, kind).
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
    pub seed: u64,
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
    mood: Vec<u8>,
    buf: Vec<(u32, u32, u8, u32)>,                  // buffered bindings: (entity, class, direction, value)
    stamp: HashMap<(u32, u32, u32), u64>,           // last write time per slot (entity, class, kind)
    occ: HashMap<(u32, u32), Vec<u32>>,             // live kinds per (entity, class): the slots' valid bits
    dcl: HashMap<(u32, u8), Vec<u32>>,              // classes written per (entity, direction)
    hop: HashMap<(u64, usize, u8), HashMap<(u32, u32), u32>>,
    g2: HashMap<((u32, u32), (u32, u32)), u32>,       // two-hop pairs counted over all contexts
    comp: HashMap<((u32, u32), (u32, u32)), u32>,     // composite classes, numbered from COMP
    halt: HashMap<(u64, u32, u8), u8>,
    pub committed: u64,
    pub dropped: u64,
}

const DRE: [i8; 4] = [1, 0, -1, 0];
const FB: usize = 512;
const COMP: u32 = 1 << 24; // composite class ids start here, apart from the pooled classes
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
            mood: vec![0; nw],
            buf: vec![],
            stamp: HashMap::new(),
            occ: HashMap::new(),
            dcl: HashMap::new(),
            hop: HashMap::new(),
            g2: HashMap::new(),
            comp: HashMap::new(),
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
            self.boundary[u as usize] = 20 * hi <= 19 * n && lo * lo * lo * hi * hi > n * n * hj * hj * hj;
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
        let map: HashMap<u32, u32> = rep.into_iter().map(|(old, p)| (old, self.path_class[&p])).collect();
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
        // the slots written under the old class numbers are no longer addressed: forget their valid bits
        self.occ.clear();
        self.dcl.clear();
        self.stamp.clear();
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
        let sc: Vec<(u32, i64)> = self
            .words
            .iter()
            .filter(|w| !excl.contains(w))
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
        let mut props: Vec<(u32, u32, i128)> = vec![];
        for (j, &e) in cue.iter().enumerate() {
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
            // (kind, first-slot recency, count, value)
            let mut cand: Vec<(u32, u64, u32, u32)> = vec![];
            if let Some(c1) = self.c1.get(&(ctx, j)) {
                let mut v: Vec<((u32, u32), u32)> = c1.iter().map(|(&c, &n)| (c, n)).filter(|x| x.1 >= 2).collect();
                v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                for &((c, kd), n) in v.iter().take(3) {
                    if let Some((val, _)) = self.read_best(e, c, kd, &cue) {
                        cand.push((kd, self.stamp_of(e, c, kd), n, val));
                    }
                }
            }
            if let Some(c2) = self.c2.get(&(ctx, j)) {
                let mut v: Vec<(((u32, u32), (u32, u32)), u32)> = c2.iter().map(|(&c, &n)| (c, n)).filter(|x| x.1 >= 2).collect();
                v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
                for &(((c1_, k1), (c2_, k2)), n) in v.iter().take(2) {
                    if let Some((m, _)) = self.read_best(e, c1_, k1, &[e]) {
                        if let Some((val, _)) = self.read_best(m, c2_, k2, &cue) {
                            cand.push((k2, self.stamp_of(e, c1_, k1), n, val));
                        }
                    }
                }
            }
            if self.cfg.recency && !cand.is_empty() {
                // per kind: credible proposals (at least half the cue word's best count), the newest first
                let maxn = cand.iter().map(|x| x.2).max().unwrap();
                cand.retain(|x| 2 * x.2 >= maxn);
                let mut kinds: Vec<u32> = cand.iter().map(|x| x.0).collect();
                kinds.sort_unstable();
                kinds.dedup();
                for kd in kinds {
                    let mut g: Vec<&(u32, u64, u32, u32)> = cand.iter().filter(|x| x.0 == kd).collect();
                    g.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
                    let top = g.iter().map(|x| x.2).max().unwrap();
                    // the group ranks by its best count; inside it, newest first (the order is kept by a stable sort)
                    for x in g {
                        props.push((top, x.3, 0));
                    }
                }
            } else {
                props.extend(cand.into_iter().map(|x| (x.2, x.3, 0)));
            }
        }
        // routes reliable enough (counts at least half the best), then the freshest (strongest) reading first
        let maxc = props.iter().map(|x| x.0).max().unwrap_or(0);
        if self.cfg.clock > 0 {
            props.retain(|x| 2 * x.0 >= maxc);
            props.sort_by(|a, b| b.2.cmp(&a.2));
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
        // which classes answer this context: those whose slot (of a cue word) holds w
        if self.seen[w as usize] && !self.class_count.is_empty() {
            let mut top: Vec<(u64, u32)> = self.class_count.iter().enumerate().map(|(i, &c)| (c, i as u32)).collect();
            top.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            let mut top: Vec<u32> = top.into_iter().take(self.cfg.ntop).map(|x| x.1).collect();
            top.extend(self.comp.values().copied());
            let kw = self.kk(w);
            for (j, &e) in cue.iter().enumerate() {
                if e == w || !self.seen[e as usize] {
                    continue;
                }
                let hits: Vec<u32> = top.iter().copied().filter(|&c| self.read_best(e, c, kw, &[e]).map(|x| x.0) == Some(w)).collect();
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
                    let t10: Vec<u32> = top.iter().take(10).copied().collect();
                    for &c1_ in &t10 {
                        let k1s: Vec<u32> = if self.cfg.kinds { self.occ.get(&(e, c1_)).cloned().unwrap_or_default() } else { vec![self.kn] };
                        for k1 in k1s {
                            let Some((m, _)) = self.read_best(e, c1_, k1, &[e]) else { continue };
                            if m == w {
                                continue;
                            }
                            for &c2_ in &t10 {
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
        self.mn.observe(w);
        if self.cfg.kinds && n >= 1 {
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
                        self.stamp.insert((x, cs, p2.1), self.t);
                    }
                }
            }
        }
    }
}
