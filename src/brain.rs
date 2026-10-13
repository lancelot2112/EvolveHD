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
use crate::info::{ent, ent_nested, roll, CmiScreen};
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
    pub bind_cap: usize,
    pub hebb: f64,
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
            bind_cap: 1024,
            hebb: 1.0,
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
                self.pool.last[c] = t as u64;
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

pub enum Kind {
    Byte,
    Delay { d: usize },
    Hold { v: i32 },
    BindReal(Box<BindReal>),
    BindSym(Box<BindSym>),
    Distill(Box<Distill>),
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
            Kind::Taken => "?",
        }
    }
}

pub struct Node {
    pub name: String,
    pub kind: Kind,
    pub parts: Vec<usize>,
    pub born: usize,
    pub extra: String,
    pub h: Vec<i32>,
    pub state: i32,
    pub removed: bool,
}

pub struct ByteNode {
    pub pool: Pool,
    pub bytecode: Vec<Code>,
    pub sym: HashMap<u8, usize>,
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
    pub data: Vec<u8>,
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
    pub novel: Vec<bool>,
    pub hit: Vec<bool>,
    cellbyte: Vec<[u32; 256]>,
    cellbyte_n: Vec<u32>,
    tix: TargetIndex,
    alive_idx: Vec<usize>,
    pos: Vec<i64>,
    alive_dirty: bool,
    pub merged: usize,
    /// Seconds spent in: [tick body, temporal, synergy, screen, other controller work].
    pub prof: [f64; 5],
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
        let tt = data.len();
        let mut rng = Rng::new(cfg.seed);
        let mut pool = if cfg.real {
            Pool::new(192, 24, 10)
        } else {
            Pool::new(256, 0, 0)
        };
        pool.hebb = cfg.hebb;
        let bytecode = (0..256).map(|_| bits::rand_code(&mut rng)).collect();
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
            h: vec![-1; tt],
            state: -1,
            removed: false,
        };
        let mut b = Brain {
            data,
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
            backoff: HashMap::new(),
            priority: HashSet::new(),
            scr: HashMap::new(),
            kept_since: HashMap::new(),
            gain_at_keep: HashMap::new(),
            events: vec![],
            tried: HashSet::new(),
            bits: vec![0.0; tt],
            bits_map: vec![0.0; tt],
            novel: vec![false; tt],
            hit: vec![false; tt],
            cellbyte: vec![[0; 256]; cap],
            cellbyte_n: vec![0; cap],
            tix: TargetIndex::new(),
            alive_idx: vec![],
            pos: vec![-1; cap],
            alive_dirty: true,
            merged: 0,
            prof: [0.0; 5],
        };
        if cfg.oracle > 0 {
            b.oracle(cfg.oracle);
        }
        b
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
            Kind::BindSym(_) | Kind::Distill(_) => 1,
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
            h: vec![-1; self.tt],
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
            nd.h[d - 1..d - 1 + n].copy_from_slice(&self.nodes[0].h[..n]);
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
        for tt in 0..=t {
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
            let mut b = BindReal::new(self.cfg.bind_cap, self.cfg.hebb);
            let mut h = vec![-1; self.tt];
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
            let mut h = vec![-1; self.tt];
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
            self.slab.push(Edge::new(i, 0.0));
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
        for i in 0..256 {
            if cb[i] > cb[b] {
                b = i;
            }
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
        let mut h = vec![-1; self.tt];
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

    fn observe(&mut self, byte: u8, t: usize) -> (i32, bool) {
        let r0 = self.byte.pool.recruits;
        let s = if self.cfg.real {
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
        let eps = self.cfg.eps;
        let byte = self.data[t];
        if self.alive_dirty {
            self.refresh_alive();
        }
        let na = self.alive_idx.len();
        let mut logits = vec![0.0; na];
        let kept = self.kept.clone();
        let trials = self.trials.clone();
        for &e in &kept {
            if self.score_edge(e) {
                let (beta, sc) = (self.slab[e].beta, self.slab[e].sc.as_ref().unwrap());
                logits.iter_mut().zip(sc).for_each(|(l, s)| *l += beta * s);
            }
        }
        let mut tsc = vec![];
        for &e in &trials {
            if self.score_edge(e) {
                tsc.push(e);
            }
        }
        let (y, novel) = self.observe(byte, t);
        let yi: i64 = if novel { -1 } else { self.pos[y as usize] };
        let mut p = vec![];
        let q;
        if yi >= 0 && na > 0 {
            let (pp, lp) = log_softmax_at(&logits, yi as usize);
            p = pp;
            q = (1.0 - eps) * lp.exp() + eps / 256.0;
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
            q = eps / 256.0;
        }
        let yu = y as usize;
        let pb = (self.cellbyte[yu][byte as usize] as f64 + 0.02)
            / (self.cellbyte_n[yu] as f64 + 0.02 * 256.0);
        self.cellbyte[yu][byte as usize] += 1;
        self.cellbyte_n[yu] += 1;
        self.bits[t] = -q.log2() - pb.log2();
        self.bits_map[t] = -pb.log2();
        self.novel[t] = novel;
        if yi >= 0 && na > 0 {
            let yi = yi as usize;
            let base_loss = -q.log2();
            for &e in &tsc {
                let (beta, sc) = (self.slab[e].beta, self.slab[e].sc.clone().unwrap());
                let lw: Vec<f64> = logits.iter().zip(&sc).map(|(l, s)| l + beta * s).collect();
                let (pw, lpw) = log_softmax_at(&lw, yi);
                let qw = (1.0 - eps) * lpw.exp() + eps / 256.0;
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
                let (beta, sc) = (self.slab[e].beta, self.slab[e].sc.as_ref().unwrap());
                let lo: Vec<f64> = logits.iter().zip(sc).map(|(l, s)| l - beta * s).collect();
                let (_, lpo) = log_softmax_at(&lo, yi);
                let qo = (1.0 - eps) * lpo.exp() + eps / 256.0;
                let d = -qo.log2() - base_loss;
                let ed = &mut self.slab[e];
                ed.sum += d;
                ed.sq += d * d;
                ed.n += 1;
            }
            for &e in &kept {
                self.slab[e].learn_beta(&p, yi, 0.003);
            }
            let ycode = self.byte.pool.codes[yu];
            let tgt = bits::from_positions(&ycode);
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
        }
        if novel {
            self.alive_dirty = true;
        }
        if self.cfg.real && self.cfg.merge > 0 && t % 1000 == 999 {
            let n = self.byte.pool.consolidate(self.cfg.merge);
            if n > 0 {
                self.merged += n;
                self.alive_dirty = true;
            }
        }
        for i in self.derived.clone() {
            self.update_node(i, t);
        }
        if self.cfg.grow && t % 1000 == 999 {
            let c0 = std::time::Instant::now();
            self.check();
            self.prof[4] += c0.elapsed().as_secs_f64();
        }
        self.t += 1;
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
            Kind::Byte | Kind::Taken => return,
        };
        self.nodes[i].state = s;
        self.nodes[i].h[t] = s;
    }

    // ---------------------------------------------------------------- growth controller
    fn series(&self, n: usize, w: usize) -> Vec<i64> {
        let t = self.t;
        self.nodes[n].h[t + 1 - w..=t]
            .iter()
            .map(|&x| x as i64 + 1)
            .collect()
    }

    /// Source node of edge e.
    pub fn slab_src(&self, e: usize) -> usize {
        self.slab[e].src
    }

    fn name_of_edge(&self, e: usize) -> String {
        self.nodes[self.slab[e].src].name.clone()
    }

    fn check(&mut self) {
        let t = self.t;
        let c = self.cfg.clone();
        // trial decisions
        for e in self.trials.clone() {
            if t - self.trial_t0[&e] >= c.burn + c.evalw && self.slab[e].n > 100 {
                let (m, z) = self.slab[e].stats();
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
                }
            }
        }
        // prune kept edges
        for e in self.kept[1..].to_vec() {
            let k = self.name_of_edge(e);
            if t - self.kept_since.get(&k).copied().unwrap_or(0) >= c.prune_every
                && self.slab[e].n > 100
            {
                let (m, z) = self.slab[e].stats();
                if m < c.prune_gain || z < 1.0 {
                    self.kept.retain(|&x| x != e);
                    self.backoff.insert(k.clone(), (t + 10000, 10000));
                    self.events.push(Event {
                        t,
                        kind: "prune".into(),
                        a: "byte".into(),
                        b: Some(k.clone()),
                        v: Some(m),
                    });
                }
                self.slab[e].reset_stats();
                self.kept_since.insert(k, t);
            }
        }
        if t % 10000 == 9999 {
            for i in self.derived.clone() {
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
        self.fill();
    }

    fn screen(&mut self, w: usize) {
        let y = self.series(0, w);
        let (yp, yn) = (&y[..w - 1], &y[1..]);
        let sc = CmiScreen::new(yp, yn);
        let mut out = vec![];
        for n in self.derived.clone() {
            let s = self.series(n, w);
            let sh = self.rng.range(500, w - 500);
            out.push((self.nodes[n].name.clone(), sc.shift_screen(&s[..w - 1], sh)));
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
            if v > self.cfg.screen_thr {
                cands.push((v, k.clone()));
            }
        }
        cands.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap().then_with(|| b.1.cmp(&a.1)));
        for (_, k) in cands.into_iter().take(free) {
            let n = self.live(&k).unwrap();
            self.slab.push(Edge::new(n, 0.0));
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
        let t = self.t;
        let w = w.min(t + 1 - 16);
        let y = self.series(0, w);
        let (yp, yn) = (&y[..w - 1], &y[1..]);
        let sc = CmiScreen::new(yp, yn);
        let thr = self.cfg.tmp_thr;
        // (value, Some(d)) for a delay, (value, None, x, v) for a hold
        let mut best: Option<(f64, Option<usize>, usize, i32)> = None;
        for d in [2usize, 3, 4, 5, 6, 8, 12, 16] {
            if self.live(&format!("byte@{d}")).is_some() {
                continue;
            }
            let h = &self.nodes[0].h;
            let s: Vec<i64> = h[t + 1 - w - (d - 1)..t + 1 - (d - 1)]
                .iter()
                .map(|&x| x as i64 + 1)
                .collect();
            let sh = self.rng.range(500, w - 500);
            let v = sc.shift_screen(&s[..w - 1], sh);
            if v > thr && best.map_or(true, |b| v > b.0) {
                best = Some((v, Some(d), 0, 0));
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
        for &x in &xs {
            for &v in &evs {
                let lab = self.label(v);
                if self
                    .live(&format!("hold({}|{})", self.nodes[x].name, lab))
                    .is_some()
                {
                    continue;
                }
                let xh = &self.nodes[x].h[t + 1 - w..=t];
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
                let sh = self.rng.range(500, w - 500);
                let val = sc.shift_screen(&hs[..w - 1], sh);
                if val > thr && best.map_or(true, |b| val > b.0) {
                    best = Some((val, None, x, v));
                }
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

    fn contains(&self, n: usize, other: usize) -> bool {
        if n == other {
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
        let single: HashMap<usize, f64> = ser
            .iter()
            .map(|(&n, s)| (n, mi_yn(s).0 - mi_yn(&roll(s, sh)).0))
            .collect();
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
                let (sa, sb) = (&ser[&a], &ser[&b]);
                let m = sb.iter().copied().max().unwrap_or(0) + 1;
                let pair: Vec<i64> = sa.iter().zip(sb).map(|(x, z)| x * m + z).collect();
                let (m0, nd) = mi_yn(&pair);
                if nd > w / 4 || (self.cfg.real && nd > 2 * self.cfg.bind_cap) {
                    continue; // not enough data per context, or far beyond capacity
                }
                let j = m0 - mi_yn(&roll(&pair, sh)).0;
                let g = j - single[&a].max(single[&b]).max(0.0);
                if g > self.cfg.syn_thr && best.map_or(true, |bb| g > bb.0) {
                    best = Some((g, a, b));
                }
            }
        }
        if let Some((g, a, b)) = best {
            self.bind(a, b, format!("synergy byte {g:.2}"));
        }
    }

    fn gc(&mut self) {
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
            if !used.contains(&n) && self.tried.contains(&nd.name) && t - nd.born > 15000 {
                let (name, kind) = (nd.name.clone(), nd.kind.label().to_string());
                self.derived.retain(|&x| x != n);
                self.by_name.remove(&name);
                self.nodes[n].removed = true;
                self.scr.remove(&name);
                self.backoff.remove(&name);
                // free memory the node no longer needs, but keep its frozen state
                if let Kind::BindReal(_) | Kind::BindSym(_) = self.nodes[n].kind {
                    self.nodes[n].h = Vec::new();
                }
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

    // ---------------------------------------------------------------- run and report
    pub fn run(&mut self) {
        while self.t < self.tt {
            self.tick();
        }
    }

    /// Mean bits per byte over [lo, hi).
    pub fn bpb(&self, lo: usize, hi: usize) -> f64 {
        let hi = hi.min(self.t);
        self.bits[lo..hi].iter().sum::<f64>() / (hi - lo) as f64
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
        let cells: Vec<i64> = self.nodes[0].h[lo..tt]
            .iter()
            .map(|&x| x as i64 + 1)
            .collect();
        let by: Vec<i64> = self.data[lo..tt].iter().map(|&x| x as i64).collect();
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
        let _ = write!(o, "\"ticks\":{},\"secs\":{}}}", tt, json_f(secs));
        o
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
