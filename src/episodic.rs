//! A hippocampal store (experiment 31): fast, one-shot bindings between the content words of each clause,
//! recalled by cue.
//!
//! At every clause boundary, each pair of content words in the clause is bound (both directions), stamped with
//! the time; a later binding of the same pair overwrites the stamp. To recall, the content words of the current
//! clause and of the one before it cue the store: each partner b of a cue word scores 2^(-age/half) for that
//! binding (summed over cue words), cue words themselves excluded. A second hop follows the strongest partners
//! one step further (apple -> john -> office). Each hop's k strongest partners, normalised to sum to 1, are
//! offered to the word level as a copy channel (`Brain::set_copy`).
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
        }
    }

    /// The copy gains' context: the last `ctx_words` words, content words wildcarded, as one key.
    pub fn ctx_key(&self) -> Option<i32> {
        if self.cfg.ctx_words == 0 {
            return None;
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
        if w >= self.cfg.skip && w < u32::MAX - 1 { u32::MAX } else { w }
    }

    /// Learn from word w (after it has been predicted).
    pub fn observe(&mut self, w: u32) {
        self.t += 1;
        self.hist.push(w);
        if self.hist.len() > 16 {
            self.hist.remove(0);
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

    fn observe_roles(&mut self, w: u32) {
        if w >= self.cfg.skip {
            let r = self.intern_role((self.wild(self.ctx.0), self.wild(self.ctx.1)));
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
        let mut cue: Vec<u32> = self.rcur.iter().map(|x| x.0).chain(self.prev.iter().copied()).collect();
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
        let mut cue: Vec<u32> = self.rcur.iter().map(|x| x.0).chain(self.prev.iter().copied()).collect();
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
                            *chans.entry(ch).or_default().entry(c).or_insert(0.0) += wb * self.strength(tc);
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

    /// A readable name for channel j, given the vocabulary.
    pub fn chan_label(&self, j: usize, vocab: &[String]) -> String {
        let j = if self.cfg.both {
            if j < self.cfg.hops {
                return format!("plain hop{}", j + 1);
            }
            j - self.cfg.hops
        } else {
            j
        };
        let w = |x: u32| if x == u32::MAX { "*".to_string() } else { vocab.get(x as usize).cloned().unwrap_or_else(|| x.to_string()) };
        let role = |r: u32| { let (a, b) = self.role_names[r as usize]; format!("[{} {}]", w(a), w(b)) };
        let pair = |p: u32| { let (a, b) = self.pair_names[p as usize]; format!("{}->{}", role(a), role(b)) };
        let (h, p1, p2) = self.chan_names[j];
        if h == 1 { format!("hop1 {}", pair(p1)) } else { format!("hop2 {} then {}", pair(p1), pair(p2)) }
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
