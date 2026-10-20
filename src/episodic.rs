//! A hippocampal store (experiment 30): fast, one-shot bindings between the content words of each clause,
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
}

pub struct Episodic {
    pub cfg: EpiCfg,
    links: HashMap<u32, HashMap<u32, u64>>,
    cur: Vec<u32>,
    prev: Vec<u32>,
    t: u64,
}

impl Episodic {
    pub fn new(cfg: EpiCfg) -> Self {
        Episodic { cfg, links: HashMap::new(), cur: vec![], prev: vec![], t: 0 }
    }

    /// Learn from word w (after it has been predicted).
    pub fn observe(&mut self, w: u32) {
        self.t += 1;
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
            v.sort_unstable_by(|x, y| y.1.partial_cmp(&x.1).unwrap().then(x.0.cmp(&y.0)));
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
