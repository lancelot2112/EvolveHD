//! A hippocampal store (experiment 31): fast, one-shot bindings between the content words of each clause,
//! recalled by cue.
//!
//! At every clause boundary, each pair of content words in the clause is bound (both directions), stamped with
//! the time; a later binding of the same pair overwrites the stamp. To recall, the content words of the current
//! clause and of the one before it cue the store: each partner b of a cue word scores 2^(-age/half) for that
//! binding (summed over cue words), cue words themselves excluded. A second hop follows the strongest partners
//! one step further (apple -> john -> office). Each hop's k strongest partners, normalised to sum to 1, are
//! offered to the word level as a copy channel (`Brain::set_copy`).
use crate::meaning::{Meaning, MeaningCfg};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct EpiCfg {
    /// Ids below this are not content words.
    pub skip: u32,
    /// Clause boundaries.
    pub bounds: Vec<u32>,
    /// Half-life of a binding, in words.
    pub half: f64,
    /// Partners offered per hop.
    pub k: usize,
    pub hops: usize,
    /// Learned roles: bind per pair of roles (each word's role = its two preceding words, content words
    /// replaced by a wildcard), giving one recall channel per role pair (and per chain of them for hop 2).
    pub roles: bool,
    /// With roles, also offer the plain (role-free) channels, first.
    pub both: bool,
    /// Words of (wildcarded) context in the copy gains' context key; 0 = the brain's previous cell.
    pub ctx_words: usize,
    /// Roles and gate contexts as meaning classes: the context's words' meaning codes (learned online, every
    /// word, in `mdim` bits), rotated by position and ORed, matched to prototypes (overlap >= `mbar`, else a
    /// new class is recruited); instead of exact word contexts.
    pub mclass: bool,
    pub mdim: usize,
    pub mbar: f64,
    /// Fully learned mode: no content-word split, no clause boundaries, no wildcards. Each word is bound to the
    /// words of a fading window before it (`win` words), the cue is the last `cue` words, roles and gates are
    /// meaning classes over all words, and each word's recall weight is learned from whether recalling it
    /// predicted the next word.
    pub learned: bool,
    pub win: usize,
    pub cue: usize,
    /// Most role classes and gate classes (when full, a context joins its nearest class).
    pub role_cap: usize,
    pub gate_cap: usize,
}

/// Online classes of codes: each prototype is the strongest bits of the counts of the codes it matched.
struct Classes {
    d: usize,
    counts: Vec<Vec<f32>>,
    members: Vec<Vec<bool>>,
    size: Vec<usize>,
    seen: Vec<u32>,
    bar: f64,
    /// For each bit, the classes whose prototype holds it (for matching by overlap counts).
    owners: Vec<Vec<u32>>,
    cap: usize,
}

impl Classes {
    fn new(d: usize, bar: f64) -> Self {
        Classes {
            d,
            counts: vec![],
            members: vec![],
            size: vec![],
            seen: vec![],
            bar,
            owners: vec![vec![]; d],
            cap: 1024,
        }
    }
    /// The class of a code (bit positions), learning from it.
    fn assign(&mut self, code: &[u32]) -> u32 {
        if code.is_empty() {
            return u32::MAX;
        }
        let mut hits: HashMap<u32, u32> = HashMap::new();
        for &b in code {
            for &j in &self.owners[b as usize] {
                *hits.entry(j).or_insert(0) += 1;
            }
        }
        let mut best = (0.0, usize::MAX);
        let mut hv: Vec<(u32, u32)> = hits.into_iter().collect();
        hv.sort_unstable();
        for (j, h) in hv {
            let ov = h as f64 / code.len().max(self.size[j as usize]) as f64;
            if ov > best.0 {
                best = (ov, j as usize);
            }
        }
        let full = self.counts.len() >= self.cap;
        let j = if best.1 != usize::MAX && (best.0 >= self.bar || full) {
            best.1
        } else {
            self.counts.push(vec![0.0; self.d]);
            self.members.push(vec![false; self.d]);
            self.size.push(0);
            self.seen.push(0);
            self.counts.len() - 1
        };
        for &b in code {
            self.counts[j][b as usize] += 1.0;
        }
        // the prototype is re-read from its counts every 32 assignments (and at first)
        self.seen[j] += 1;
        if self.seen[j] > 1 && self.seen[j] % 32 != 0 {
            return j as u32;
        }
        // the prototype: the code-sized set of its most counted bits
        let k = code.len();
        let c = &self.counts[j];
        let mut idx: Vec<u32> = (0..self.d as u32)
            .filter(|&i| c[i as usize] > 0.0)
            .collect();
        if idx.len() > k {
            idx.select_nth_unstable_by(k - 1, |&a, &b| c[b as usize].total_cmp(&c[a as usize]));
            idx.truncate(k);
        }
        let m = &mut self.members[j];
        for b in 0..self.d {
            if m[b] {
                m[b] = false;
                self.owners[b].retain(|&x| x != j as u32);
            }
        }
        for &b in &idx {
            m[b as usize] = true;
            self.owners[b as usize].push(j as u32);
        }
        self.size[j] = idx.len();
        j as u32
    }
}

pub struct Episodic {
    pub cfg: EpiCfg,
    links: HashMap<u32, HashMap<u32, u64>>,
    cur: Vec<u32>,
    prev: Vec<u32>,
    t: u64,
    // learned-role store
    rlinks: HashMap<u32, HashMap<(u32, u32), u64>>,
    rcur: Vec<(u32, u32)>,
    ctx: (u32, u32),
    role_ids: HashMap<(u32, u32), u32>,
    pub role_names: Vec<(u32, u32)>,
    chan_ids: HashMap<(u32, u32, u32), u32>,
    /// Channel descriptions: (hop, role pair id of hop 1, role pair id of hop 2 or u32::MAX).
    pub chan_names: Vec<(u32, u32, u32)>,
    pair_ids: HashMap<(u32, u32), u32>,
    pub pair_names: Vec<(u32, u32)>,
    hist: Vec<u32>,
    mn: Option<Meaning>,
    role_cls: Option<Classes>,
    gate_cls: Option<Classes>,
    wild_code: Vec<u32>,
    gate_now: Option<i32>,
    // learned mode
    recent: Vec<(u32, u32, u64)>,
    uw: Vec<f64>,
    last_q: HashMap<u32, f64>,
    code_cache: HashMap<u32, (u64, Vec<u32>)>,
}

impl Episodic {
    pub fn new(cfg: EpiCfg) -> Self {
        Episodic {
            cfg,
            links: HashMap::new(),
            cur: vec![],
            prev: vec![],
            t: 0,
            rlinks: HashMap::new(),
            rcur: vec![],
            ctx: (u32::MAX - 1, u32::MAX - 1),
            role_ids: HashMap::new(),
            role_names: vec![],
            chan_ids: HashMap::new(),
            chan_names: vec![],
            pair_ids: HashMap::new(),
            pair_names: vec![],
            hist: vec![],
            mn: None,
            role_cls: None,
            gate_cls: None,
            wild_code: vec![],
            gate_now: None,
            recent: vec![],
            uw: vec![],
            last_q: HashMap::new(),
            code_cache: HashMap::new(),
        }
    }

    /// A word's meaning code (or identity code), cached and refreshed every 256 words.
    fn cached_code(&mut self, w: u32) -> Vec<u32> {
        if let Some((t0, c)) = self.code_cache.get(&w) {
            if self.t - t0 < 256 {
                return c.clone();
            }
        }
        let c = self.mn.as_mut().unwrap().code_or_ident(w);
        self.code_cache.insert(w, (self.t, c.clone()));
        c
    }

    /// Turn on meaning classes (needs the vocabulary size).
    pub fn with_meaning(mut self, nw: usize) -> Self {
        self.uw = vec![1.0; nw];
        if self.cfg.mclass || self.cfg.learned {
            let mut mc = MeaningCfg::default();
            mc.d = self.cfg.mdim;
            mc.skip = 0;
            mc.seed = 7;
            self.wild_code = crate::rng::Rng::new(11)
                .choose_distinct(mc.d, mc.k_word)
                .into_iter()
                .map(|x| x as u32)
                .collect();
            self.mn = Some(Meaning::new(mc, nw));
            let mut rc = Classes::new(self.cfg.mdim, self.cfg.mbar);
            rc.cap = self.cfg.role_cap;
            let mut gc = Classes::new(self.cfg.mdim, self.cfg.mbar);
            gc.cap = self.cfg.gate_cap;
            self.role_cls = Some(rc);
            self.gate_cls = Some(gc);
        }
        self
    }

    /// The code of a context: each word's meaning code (content words: the wildcard's), rotated by its distance
    /// back, ORed.
    fn ctx_code(&mut self, words: &[u32]) -> Vec<u32> {
        let d = self.cfg.mdim;
        let mut out: Vec<u32> = vec![];
        for (back, &w) in words.iter().rev().enumerate() {
            let c = if !self.cfg.learned && w >= self.cfg.skip && w < u32::MAX - 1 {
                self.wild_code.clone()
            } else if w >= u32::MAX - 1 {
                continue;
            } else {
                self.cached_code(w)
            };
            let rot = (back * 7919) % d;
            out.extend(c.iter().map(|&b| ((b as usize + rot) % d) as u32));
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// The copy gains' context: the last `ctx_words` words, content words wildcarded, as one key.
    pub fn ctx_key(&self) -> Option<i32> {
        if self.cfg.ctx_words == 0 {
            return None;
        }
        if self.cfg.mclass || self.cfg.learned {
            return self.gate_now;
        }
        let mut h: u64 = 1469598103934665603;
        for &w in self.hist.iter().rev().take(self.cfg.ctx_words) {
            h = (h ^ self.wild(w) as u64).wrapping_mul(1099511628211);
        }
        Some((h & 0x3fff_ffff) as i32)
    }

    fn intern_role(&mut self, ctx: (u32, u32)) -> u32 {
        let n = self.role_ids.len() as u32;
        *self.role_ids.entry(ctx).or_insert_with(|| {
            self.role_names.push(ctx);
            n
        })
    }
    fn intern_pair(&mut self, p: (u32, u32)) -> u32 {
        let n = self.pair_ids.len() as u32;
        *self.pair_ids.entry(p).or_insert_with(|| {
            self.pair_names.push(p);
            n
        })
    }
    fn intern_chan(&mut self, c: (u32, u32, u32)) -> u32 {
        let n = self.chan_ids.len() as u32;
        *self.chan_ids.entry(c).or_insert_with(|| {
            self.chan_names.push(c);
            n
        })
    }
    /// A word's context with content words replaced by a wildcard (u32::MAX).
    fn wild(&self, w: u32) -> u32 {
        if w >= self.cfg.skip && w < u32::MAX - 1 {
            u32::MAX
        } else {
            w
        }
    }

    /// Learn from word w (after it has been predicted).
    pub fn observe(&mut self, w: u32) {
        self.t += 1;
        self.hist.push(w);
        if self.hist.len() > 16 {
            self.hist.remove(0);
        }
        if self.cfg.learned {
            self.observe_learned(w);
            return;
        }
        if self.cfg.mclass {
            self.mn.as_mut().unwrap().observe(w);
            if self.cfg.ctx_words > 0 {
                let n = self.hist.len();
                let last: Vec<u32> = self.hist[n.saturating_sub(self.cfg.ctx_words)..].to_vec();
                let code = self.ctx_code(&last);
                self.gate_now = Some(self.gate_cls.as_mut().unwrap().assign(&code) as i32);
            }
        }
        if self.cfg.roles {
            self.observe_roles(w);
            return;
        }
        if w >= self.cfg.skip {
            self.cur.push(w);
        }
        if self.cfg.bounds.contains(&w) {
            let t = self.t;
            let c = std::mem::take(&mut self.cur);
            for &a in &c {
                for &b in &c {
                    if a != b {
                        self.links.entry(a).or_default().insert(b, t);
                    }
                }
            }
            self.prev = c;
        }
    }

    fn observe_learned(&mut self, w: u32) {
        // learn each recalled word's weight: up if it was the word that came, down otherwise
        let q = std::mem::take(&mut self.last_q);
        for (b, qb) in q {
            let target = if b == w { 1.0 } else { 0.0 };
            let u = &mut self.uw[b as usize];
            *u = (*u * (0.05 * (target - qb)).exp()).clamp(1e-3, 10.0);
        }
        self.mn.as_mut().unwrap().observe(w);
        // the word's role: the meaning class of the two words before it
        let n = self.hist.len();
        let ctx: Vec<u32> = self.hist[n.saturating_sub(3)..n - 1].to_vec();
        let code = self.ctx_code(&ctx);
        let c = self.role_cls.as_mut().unwrap().assign(&code);
        let r = self.intern_role((c, u32::MAX - 2));
        // bind it to the words of the window before it, both ways
        let t = self.t;
        let recent = self.recent.clone();
        for &(b, rb, _) in recent.iter().rev().take(self.cfg.win) {
            if b != w {
                let p1 = self.intern_pair((rb, r));
                self.rlinks.entry(b).or_default().insert((w, p1), t);
                let p2 = self.intern_pair((r, rb));
                self.rlinks.entry(w).or_default().insert((b, p2), t);
            }
        }
        self.recent.push((w, r, t));
        if self.recent.len() > 64 {
            self.recent.remove(0);
        }
        // the gate's context class: the last `ctx_words` words
        if self.cfg.ctx_words > 0 {
            let last: Vec<u32> = self.hist[n.saturating_sub(self.cfg.ctx_words)..].to_vec();
            let code = self.ctx_code(&last);
            self.gate_now = Some(self.gate_cls.as_mut().unwrap().assign(&code) as i32);
        }
    }

    /// Learned-mode recall: cue = the last `cue` words; partners weighted by binding strength and their learned
    /// recall weight; a plain channel per hop plus one channel per role pair (and chain).
    pub fn recall_learned(&mut self) -> Vec<Vec<(u32, f64)>> {
        let mut cue: Vec<u32> = self
            .recent
            .iter()
            .rev()
            .take(self.cfg.cue)
            .map(|x| x.0)
            .collect();
        cue.sort_unstable();
        cue.dedup();
        let mut plain: Vec<HashMap<u32, f64>> = vec![HashMap::new(); self.cfg.hops];
        let mut chans: HashMap<u32, HashMap<u32, f64>> = HashMap::new();
        let mut hop1: Vec<(u32, u32, f64)> = vec![];
        for &a in &cue {
            if let Some(m) = self.rlinks.get(&a) {
                for (&(b, pr), &tb) in m {
                    let st = self.strength(tb) * self.uw[b as usize];
                    if !cue.contains(&b) && st > 1e-12 {
                        hop1.push((b, pr, st));
                    }
                }
            }
        }
        for &(b, pr, st) in &hop1 {
            let e = plain[0].entry(b).or_insert(0.0);
            *e = e.max(st);
            let c = self.intern_chan((1, pr, u32::MAX));
            *chans.entry(c).or_default().entry(b).or_insert(0.0) += st;
        }
        if self.cfg.hops >= 2 {
            let mut v: Vec<(u32, f64)> = plain[0].iter().map(|(&b, &x)| (b, x)).collect();
            v.sort_unstable_by(|x, y| y.1.total_cmp(&x.1).then(x.0.cmp(&y.0)));
            v.truncate(self.cfg.k);
            let s: f64 = v.iter().map(|x| x.1).sum();
            for (b, wb) in v {
                if s <= 0.0 {
                    break;
                }
                let links: Vec<((u32, u32), u64)> = match self.rlinks.get(&b) {
                    Some(m) => m.iter().map(|(&k, &t)| (k, t)).collect(),
                    None => continue,
                };
                for ((c, _pr2), tc) in links {
                    if !cue.contains(&c) && c != b {
                        let st = wb / s * self.strength(tc) * self.uw[c as usize];
                        *plain[1].entry(c).or_insert(0.0) += st;
                    }
                }
            }
        }
        let norm = |m: HashMap<u32, f64>, k: usize| -> Vec<(u32, f64)> {
            let mut v: Vec<(u32, f64)> = m.into_iter().filter(|x| x.1 > 1e-12).collect();
            v.sort_unstable_by(|x, y| y.1.total_cmp(&x.1).then(x.0.cmp(&y.0)));
            v.truncate(k);
            let s: f64 = v.iter().map(|x| x.1).sum();
            if s > 0.0 {
                v.iter_mut().for_each(|x| x.1 /= s);
            }
            v
        };
        let mut out: Vec<Vec<(u32, f64)>> = vec![];
        let k = self.cfg.k;
        for m in plain {
            out.push(norm(m, k));
        }
        // what was recalled (summed over the plain channels), for learning the recall weights
        self.last_q.clear();
        for v in &out {
            for &(b, x) in v {
                *self.last_q.entry(b).or_insert(0.0) += x / out.len() as f64;
            }
        }
        let mut roles = vec![vec![]; self.chan_ids.len()];
        for (ch, m) in chans {
            roles[ch as usize] = norm(m, k);
        }
        out.extend(roles);
        out
    }

    fn observe_roles(&mut self, w: u32) {
        if w >= self.cfg.skip {
            let r = if self.cfg.mclass {
                let code = self.ctx_code(&[self.ctx.0, self.ctx.1]);
                let c = self.role_cls.as_mut().unwrap().assign(&code);
                self.intern_role((c, u32::MAX - 2))
            } else {
                self.intern_role((self.wild(self.ctx.0), self.wild(self.ctx.1)))
            };
            self.rcur.push((w, r));
        }
        self.ctx = (self.ctx.1, w);
        if self.cfg.bounds.contains(&w) {
            let t = self.t;
            let c = std::mem::take(&mut self.rcur);
            for &(a, ra) in &c {
                for &(b, rb) in &c {
                    if a != b {
                        let pr = self.intern_pair((ra, rb));
                        self.rlinks.entry(a).or_default().insert((b, pr), t);
                    }
                }
            }
            self.prev = c.iter().map(|x| x.0).collect();
            self.cur.clear();
        }
    }

    /// Learned-role recall: one channel per role pair (hop 1) and per chain of two (hop 2), indexed by channel
    /// id; channels with nothing recalled are empty.
    pub fn recall_both(&mut self) -> Vec<Vec<(u32, f64)>> {
        // the plain store mirrors the role store's clauses: rebuild its links lazily from the role links
        let mut out = self.recall_plain_from_roles();
        out.extend(self.recall_roles());
        out
    }

    /// Role-free recall over the role store (pairs merged, the newest stamp kept).
    fn recall_plain_from_roles(&self) -> Vec<Vec<(u32, f64)>> {
        let mut cue: Vec<u32> = self
            .rcur
            .iter()
            .map(|x| x.0)
            .chain(self.prev.iter().copied())
            .collect();
        cue.sort_unstable();
        cue.dedup();
        let partners = |a: u32| -> HashMap<u32, u64> {
            let mut m: HashMap<u32, u64> = HashMap::new();
            if let Some(l) = self.rlinks.get(&a) {
                for (&(b, _), &t) in l {
                    let e = m.entry(b).or_insert(0);
                    *e = (*e).max(t);
                }
            }
            m
        };
        let mut out = vec![];
        let mut front: Vec<(u32, f64)> = cue.iter().map(|&c| (c, 1.0)).collect();
        for _ in 0..self.cfg.hops {
            let mut r: HashMap<u32, f64> = HashMap::new();
            for &(a, wa) in &front {
                for (b, tb) in partners(a) {
                    if !cue.contains(&b) {
                        *r.entry(b).or_insert(0.0) += wa * self.strength(tb);
                    }
                }
            }
            let mut v: Vec<(u32, f64)> = r.into_iter().filter(|x| x.1 > 1e-12).collect();
            v.sort_unstable_by(|x, y| y.1.total_cmp(&x.1).then(x.0.cmp(&y.0)));
            v.truncate(self.cfg.k);
            let s: f64 = v.iter().map(|x| x.1).sum();
            if s > 0.0 {
                v.iter_mut().for_each(|x| x.1 /= s);
            }
            front = v.clone();
            out.push(v);
        }
        out
    }

    pub fn recall_roles(&mut self) -> Vec<Vec<(u32, f64)>> {
        let mut cue: Vec<u32> = self
            .rcur
            .iter()
            .map(|x| x.0)
            .chain(self.prev.iter().copied())
            .collect();
        cue.sort_unstable();
        cue.dedup();
        let mut chans: HashMap<u32, HashMap<u32, f64>> = HashMap::new();
        let mut hop1: Vec<(u32, u32, f64)> = vec![]; // (partner, pair, strength)
        for &a in &cue {
            if let Some(m) = self.rlinks.get(&a) {
                for (&(b, pr), &tb) in m {
                    let st = self.strength(tb);
                    if !cue.contains(&b) && st > 1e-12 {
                        hop1.push((b, pr, st));
                    }
                }
            }
        }
        for &(b, pr, st) in &hop1 {
            let c = self.intern_chan((1, pr, u32::MAX));
            *chans.entry(c).or_default().entry(b).or_insert(0.0) += st;
        }
        if self.cfg.hops >= 2 {
            // follow each hop-1 channel's strongest partners one step further
            let mut by1: HashMap<u32, Vec<(u32, f64)>> = HashMap::new();
            for &(b, pr, st) in &hop1 {
                by1.entry(pr).or_default().push((b, st));
            }
            let mut keys: Vec<u32> = by1.keys().copied().collect();
            keys.sort_unstable();
            for pr1 in keys {
                let mut v = by1.remove(&pr1).unwrap();
                v.sort_unstable_by(|x, y| y.1.total_cmp(&x.1).then(x.0.cmp(&y.0)));
                v.truncate(self.cfg.k);
                let s: f64 = v.iter().map(|x| x.1).sum();
                if s <= 0.0 {
                    continue;
                }
                for (b, wb) in v {
                    let wb = wb / s;
                    let links: Vec<((u32, u32), u64)> = match self.rlinks.get(&b) {
                        Some(m) => m.iter().map(|(&k, &t)| (k, t)).collect(),
                        None => continue,
                    };
                    for ((c, pr2), tc) in links {
                        if !cue.contains(&c) && c != b {
                            let ch = self.intern_chan((2, pr1, pr2));
                            *chans.entry(ch).or_default().entry(c).or_insert(0.0) +=
                                wb * self.strength(tc);
                        }
                    }
                }
            }
        }
        let mut out = vec![vec![]; self.chan_ids.len()];
        for (ch, m) in chans {
            let mut v: Vec<(u32, f64)> = m.into_iter().collect();
            v.sort_unstable_by(|x, y| y.1.total_cmp(&x.1).then(x.0.cmp(&y.0)));
            v.truncate(self.cfg.k);
            let s: f64 = v.iter().map(|x| x.1).sum();
            if s > 0.0 {
                v.iter_mut().for_each(|x| x.1 /= s);
            }
            out[ch as usize] = v;
        }
        out
    }

    /// Counts for reports: (role classes, gate classes, recall channels).
    pub fn stats(&self) -> (usize, usize, usize) {
        (
            self.role_cls.as_ref().map(|c| c.counts.len()).unwrap_or(0),
            self.gate_cls.as_ref().map(|c| c.counts.len()).unwrap_or(0),
            self.chan_ids.len(),
        )
    }

    /// A readable name for channel j, given the vocabulary.
    pub fn chan_label(&self, j: usize, vocab: &[String]) -> String {
        let j = if self.cfg.both || self.cfg.learned {
            if j < self.cfg.hops {
                return format!("plain hop{}", j + 1);
            }
            j - self.cfg.hops
        } else {
            j
        };
        let w = |x: u32| {
            if x == u32::MAX {
                "*".to_string()
            } else {
                vocab
                    .get(x as usize)
                    .cloned()
                    .unwrap_or_else(|| x.to_string())
            }
        };
        let role = |r: u32| {
            let (a, b) = self.role_names[r as usize];
            if b == u32::MAX - 2 {
                format!("[class {a}]")
            } else {
                format!("[{} {}]", w(a), w(b))
            }
        };
        let pair = |p: u32| {
            let (a, b) = self.pair_names[p as usize];
            format!("{}->{}", role(a), role(b))
        };
        let (h, p1, p2) = self.chan_names[j];
        if h == 1 {
            format!("hop1 {}", pair(p1))
        } else {
            format!("hop2 {} then {}", pair(p1), pair(p2))
        }
    }

    fn strength(&self, t_bind: u64) -> f64 {
        2f64.powf(-((self.t - t_bind) as f64) / self.cfg.half)
    }

    /// The partners recalled for the next word, per hop: (word id, weight), weights summing to 1.
    pub fn recall(&self) -> Vec<Vec<(u32, f64)>> {
        let mut cue: Vec<u32> = self.cur.iter().chain(self.prev.iter()).copied().collect();
        cue.sort_unstable();
        cue.dedup();
        let mut out = vec![];
        let mut front: Vec<(u32, f64)> = cue.iter().map(|&c| (c, 1.0)).collect();
        for _ in 0..self.cfg.hops {
            let mut r: HashMap<u32, f64> = HashMap::new();
            for &(a, wa) in &front {
                if let Some(m) = self.links.get(&a) {
                    for (&b, &tb) in m {
                        if !cue.contains(&b) {
                            *r.entry(b).or_insert(0.0) += wa * self.strength(tb);
                        }
                    }
                }
            }
            let mut v: Vec<(u32, f64)> = r.into_iter().collect();
            v.sort_unstable_by(|x, y| y.1.total_cmp(&x.1).then(x.0.cmp(&y.0)));
            v.truncate(self.cfg.k);
            let s: f64 = v.iter().map(|x| x.1).sum();
            if s > 0.0 {
                v.iter_mut().for_each(|x| x.1 /= s);
            }
            front = v.clone();
            out.push(v);
        }
        out
    }
}
