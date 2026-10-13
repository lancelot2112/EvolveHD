"""Typed-interface growth sims for HyperdimBinaryBrain.

World: hidden topic (6 topics, dwell ~100 ticks) chooses which of 48 words are in play and
their successors (2 per word, 0.75/0.25). Words shared by two topics have different
successors in each, so the previous word alone is ambiguous; the topic disambiguates.

Primitives used:
  Node      - matcher pool (expand/associate), k-WTA k=1, recruit on novelty, Hebbian swaps,
              optional hardening (frozen + immune after N wins), optional sleep pruning.
              Output = the winner cell's fixed 32-of-2048 code (fixed sparsity).
  Edge      - typed Prime edge into the word node W: per source cell a 128-bit writer mask over
              W's code space that moves toward the code of the word that actually followed.
              Score_j = sum_e popcount(writer_e[src winner] & code_j). Learned on driven steps only.
  Compress  - a Node reading the OR of the last H word-cell codes (temporal pooling).

Growth controller (pressure -> operator):
  plateau in surprise          -> trial a candidate Prime edge (burn-in, then paired ablation)
  kept edge stops paying       -> prune (hysteresis: accept z>=2.5 & d>=1%, prune z<1 or d<0.3%)
  candidates exhausted + plateau + lagged co-activity in W -> grow a Compress node, offer it as a candidate
"""
import numpy as np
from collections import deque

D = 2048
CK = 32


def rand_code(rng):
    return rng.choice(D, CK, replace=False)


def emit(rng, code, noise):
    keep = rng.random(CK) >= noise
    return np.unique(np.concatenate([code[keep], rng.integers(0, D, CK - keep.sum())]))


class World:
    def __init__(self, rng, notopic=False, p_switch=0.03, T=6, V=48, swap_at=None,
                 word_noise=0.1, sig_noise=0.15, chain_p=0.01):
        self.chain_p = chain_p
        self.rng, self.V, self.p = rng, V, p_switch
        self.word_noise, self.sig_noise, self.swap_at = word_noise, sig_noise, swap_at
        if notopic:
            T = 1
            sets = [np.arange(V)]
        else:
            perm = rng.permutation(V)
            base = np.array_split(perm, T)
            sets = []
            for k in range(T):
                others = np.setdiff1d(np.arange(V), base[k])
                sets.append(np.concatenate([base[k], rng.choice(others, 4, replace=False)]))
        self.T, self.sets = T, sets
        self.succ = []
        for k in range(T):
            d = {}
            for w in sets[k]:
                pool = sets[k][sets[k] != w]
                d[int(w)] = rng.choice(pool, 2, replace=False)
            self.succ.append(d)
        self.wcode = np.array([rand_code(rng) for _ in range(V)])
        self.tcode = np.array([rand_code(rng) for _ in range(6)])
        self.ccode = np.array([rand_code(rng) for _ in range(6)])
        self.ncode = np.array([rand_code(rng) for _ in range(16)])
        self.topic = 0
        self.word = int(sets[0][0])
        self.chain = 0
        self.t = 0

    def step(self):
        r = self.rng
        if self.T > 1 and r.random() < self.p:
            self.topic = int(r.choice([k for k in range(self.T) if k != self.topic]))
            self.word = int(r.choice(self.sets[self.topic]))
        else:
            s = self.succ[self.topic][self.word]
            self.word = int(s[0] if r.random() < 0.75 else s[1])
        if r.random() < self.chain_p:
            self.chain = int(r.choice([c for c in range(6) if c != self.chain]))
        nsym = int(r.integers(16))
        phase2 = self.swap_at is not None and self.t >= self.swap_at
        tsig = emit(r, self.tcode[self.topic % 6], self.sig_noise)
        csig = emit(r, self.ccode[self.chain], self.sig_noise)
        obs = {
            'word': emit(r, self.wcode[self.word], self.word_noise),
            'S1': csig if phase2 else tsig,
            'S2': tsig if phase2 else csig,
            'N': emit(r, self.ncode[nsym], self.sig_noise),
            'TOPIC': tsig,   # oracle source for reference wirings
        }
        self.t += 1
        return obs


class Node:
    def __init__(self, rng, cap, K=32, th_fire=12, th_new=18, young=5, harden=None,
                 init_frac=1.0, hebb_p=1.0):
        self.init_frac, self.hebb_p = init_frac, hebb_p
        self.rng, self.cap, self.K = rng, cap, K
        self.th_fire, self.th_new, self.young, self.harden = th_fire, th_new, young, harden
        self.masks = np.zeros((cap, D), bool)
        self.alive = np.zeros(cap, bool)
        self.wins = np.zeros(cap, int)
        self.born = np.zeros(cap, int)
        self.last = np.zeros(cap, int)
        self.gen = np.zeros(cap, int)
        self.codes = np.zeros((cap, CK), int)
        self.hist = []
        self.recruits = self.evictions = self.prunes = 0
        self.events = []  # ticks of recruit/evict/prune (churn)

    def immune(self):
        if self.harden is None:
            return np.zeros(self.cap, bool)
        return self.alive & (self.wins >= self.harden)

    def recruit(self, idx, t):
        free = np.flatnonzero(~self.alive)
        if len(free):
            c = int(free[0])
        else:
            cand = np.flatnonzero(self.alive & ~self.immune())
            if not len(cand):
                return -1
            c = int(cand[np.argmin(self.last[cand])])
            self.evictions += 1
        m = np.zeros(D, bool)
        if self.init_frac < 1.0:   # immature cell: only part of the mask comes from the input
            k_in = int(round(self.K * self.init_frac))
            m[self.rng.choice(idx, min(k_in, len(idx)), replace=False)] = True
            extra = self.rng.choice(np.flatnonzero(~m), self.K - m.sum(), replace=False)
            m[extra] = True
        elif len(idx) >= self.K:
            m[self.rng.choice(idx, self.K, replace=False)] = True
        else:
            m[idx] = True
            extra = self.rng.choice(np.flatnonzero(~m), self.K - len(idx), replace=False)
            m[extra] = True
        self.masks[c] = m
        self.alive[c] = True
        self.wins[c] = 0
        self.born[c] = t
        self.gen[c] += 1
        self.codes[c] = rand_code(self.rng)
        self.recruits += 1
        self.events.append(t)
        return c

    def step(self, idx, t, learn=True):
        if idx is None or len(idx) == 0:
            self.hist.append(-1)
            return -1
        ov = self.masks[:, idx].sum(1)
        ov[~self.alive] = -1
        b = int(np.argmax(ov))
        best = ov[b]
        if best < self.th_new and learn:
            c = self.recruit(idx, t)
            if c < 0:
                b = b if best >= self.th_fire else -1
            else:
                b = c
        elif best < self.th_fire:
            b = -1
        elif learn and not (self.harden is not None and self.wins[b] >= self.harden) \
                and (self.hebb_p >= 1.0 or self.rng.random() < self.hebb_p):
            n = 2 if self.wins[b] < self.young else 1
            m = self.masks[b]
            inp = np.zeros(D, bool)
            inp[idx] = True
            add = np.flatnonzero(inp & ~m)
            rem = np.flatnonzero(m & ~inp)
            n = min(n, len(add), len(rem))
            if n:
                m[self.rng.choice(rem, n, replace=False)] = False
                m[self.rng.choice(add, n, replace=False)] = True
        if b >= 0:
            self.wins[b] += 1
            self.last[b] = t
        self.hist.append(b)
        return b

    def sleep_prune(self, t, age_min, wins_min):
        dead = self.alive & ~self.immune() & ((t - self.born) > age_min) & (self.wins < wins_min)
        for c in np.flatnonzero(dead):
            self.alive[c] = False
            self.prunes += 1
            self.events.append(t)

    def churn(self, t, window):
        return sum(1 for e in self.events if e > t - window)


class Edge:
    def __init__(self, name, src, delay, rng, Kw=None):
        # writer mask size: small for the immediate self tap (few successors, keep frequency
        # weighting), larger for context edges that must cover many targets
        Kw = Kw or (48 if name == 'self' else 160)
        self.name, self.src, self.delay, self.rng, self.Kw = name, src, delay, rng, Kw
        self.masks = np.zeros((src.cap, D), bool)
        self.gen = -np.ones(src.cap, int)

    def winner(self):
        h = self.src.hist
        return h[-self.delay] if len(h) >= self.delay else -1

    def mask(self, i):
        if self.gen[i] != self.src.gen[i]:
            self.masks[i] = False
            self.masks[i, self.rng.choice(D, self.Kw, replace=False)] = True
            self.gen[i] = self.src.gen[i]
        return self.masks[i]

    def scores(self, W):
        i = self.cur = self.winner()
        if i < 0:
            return np.zeros(W.cap)
        s = self.mask(i)[W.codes].sum(1).astype(float)
        s[~W.alive] = 0
        return s

    def learn(self, W, y, swaps=1):
        i = self.cur
        if i < 0 or y < 0:
            return
        m = self.mask(i)
        tgt = np.zeros(D, bool)
        tgt[W.codes[y]] = True
        add = np.flatnonzero(tgt & ~m)
        rem = np.flatnonzero(m & ~tgt)
        n = min(swaps, len(add), len(rem))
        if n:
            m[self.rng.choice(rem, n, replace=False)] = False
            m[self.rng.choice(add, n, replace=False)] = True


def pick(total, W, rng):
    s = total + rng.random(W.cap) * 1e-3
    s[~W.alive] = -np.inf
    return int(np.argmax(s))


def lagged_coactivity(hist, cap, lo=3, hi=20, n=6000):
    """G-statistic per dof for W-cell co-occurrence at lags lo..hi (beyond first-order succession)."""
    h = np.array([x for x in hist[-n:] if x >= 0])
    if len(h) < 1000:
        return 0.0
    O = np.zeros(cap * cap)
    for lag in range(lo, hi + 1):
        O += np.bincount(h[:-lag] * cap + h[lag:], minlength=cap * cap)
    O = O.reshape(cap, cap)
    row, col = O.sum(1, keepdims=True), O.sum(0, keepdims=True)
    E = row * col / O.sum()
    ok = (E > 5)
    G = 2 * np.sum(O[ok] * np.log(np.where(O[ok] > 0, O[ok], 1) / E[ok]))
    dof = max(1, ok.sum() - (row > 0).sum() - (col > 0).sum() + 1)
    return G / dof


def nmi(a, b):
    a, b = np.asarray(a), np.asarray(b)
    ok = a >= 0
    a, b = a[ok], b[ok]
    if len(a) == 0:
        return 0.0
    _, ai = np.unique(a, return_inverse=True)
    _, bi = np.unique(b, return_inverse=True)
    J = np.zeros((ai.max() + 1, bi.max() + 1))
    np.add.at(J, (ai, bi), 1)
    P = J / J.sum()
    pa, pb = P.sum(1, keepdims=True), P.sum(0, keepdims=True)
    nz = P > 0
    I = np.sum(P[nz] * np.log(P[nz] / (pa @ pb)[nz]))
    Hb = -np.sum(pb[pb > 0] * np.log(pb[pb > 0]))
    return I / Hb if Hb > 0 else 0.0


# ---------------------------------------------------------------- run
def run(seed, ticks, mode='controller', fixed=(), notopic=False, swap_at=None,
        candidates=('S1', 'S2', 'N', 'W2'), coact=False, force_compress_at=None,
        lower='stable', upper=None, coact_thr=None, record_topics=False,
        burn=4000, reset_on_prune=False, p_switch=0.03, chain_p=0.01,
        shadow=False, untried_first=False, exhaust_after=None, recode_at=None, recode_frac=0.3):
    """mode: 'controller' (growth operators) or 'fixed' (edges in `fixed` from t=0).
    lower: 'stable' or 'churny' (sleep pruning of unconsolidated cells, slow hardening).
    upper: None or dict(grow='t0'|'gated', harden=None|int) for Exp C.
    """
    rng = np.random.default_rng(seed)
    world = World(rng, notopic=notopic, swap_at=swap_at, p_switch=p_switch, chain_p=chain_p)
    if lower == 'churny':
        W = Node(rng, 96, th_fire=10, th_new=13, young=0, init_frac=0.5, hebb_p=0.1)
    else:
        W = Node(rng, 96, th_new=14)
    src = {k: Node(rng, 32, th_new=14) for k in ('S1', 'S2', 'N', 'TOPIC')}
    P = None
    H = 12
    edges = {'self': Edge('self', W, 1, rng)}

    def make_edge(name):
        if name == 'W2':
            return Edge('W2', W, 2, rng)
        if name == 'P':
            return Edge('P', P, 1, rng)
        return Edge(name, src[name], 1, rng)

    if mode == 'fixed':
        for f in fixed:
            if f not in ('P', 'self'):
                edges[f] = make_edge(f)

    # controller state
    BURN, EVAL, PRUNE_BLOCK = burn, 2000, 4000
    cand = {c: {'next': 3000, 'backoff': 6000} for c in candidates} if mode == 'controller' else {}
    trial = None
    kept_stats = {}      # name -> [b, c, since]
    events = []
    last_change = 0
    acc_blocks = []
    blk = []
    compress_grown = False
    coact_log = []
    topics = []
    p_hist = []

    for t in range(ticks):
        obs = world.step()
        # predict word t from context at t-1
        sc = {k: e.scores(W) for k, e in edges.items()}
        tname = trial[0] if (shadow and trial is not None) else None
        # shadow trial: the trial edge is scored but does not vote in the acted-on prediction,
        # and kept edges are ablated against a baseline that excludes it
        total = sum(v for k, v in sc.items() if k != tname)
        pred = pick(total, W, rng)
        abl = {k: pick(total - s, W, rng) for k, s in sc.items() if k not in ('self', tname)}
        y = W.step(obs['word'], t)
        correct = (pred == y) and y >= 0
        blk.append(correct)
        if tname is not None:
            pw = pick(total + sc[tname], W, rng)
            cw = (pw == y) and y >= 0
            st = kept_stats.setdefault(tname, [0, 0, t])
            st[0] += int(cw and not correct)
            st[1] += int(correct and not cw)
        for k, e in edges.items():
            e.learn(W, y)
        if recode_at is not None and t == recode_at:
            # lower node reorganises (e.g. a sleep merge/re-centre): a fraction of live cells get new codes
            live = np.flatnonzero(W.alive)
            for c in rng.choice(live, int(len(live) * recode_frac), replace=False):
                W.codes[c] = rand_code(rng)
                W.gen[c] += 1
            events.append((t, 'recode', 'W'))
        if lower == 'churny' and t % 1000 == 999:
            W.sleep_prune(t, 800, 6)
        # sources (and the compress node) see time t only after the word-t prediction is made
        for k, n in src.items():
            n.step(obs[k], t)
        # Exp C / B: upper compress node
        if P is None and upper is not None:
            grow = upper['grow'] == 't0' or (t >= 4000 and W.churn(t, 2000) <= upper.get('gate', 1))
            if grow:
                P = Node(rng, 16, K=96, th_fire=20, th_new=34, young=5, harden=upper.get('harden'))
                edges['P'] = make_edge('P')
                events.append((t, 'grow', 'P'))
        if P is None and force_compress_at is not None and t == force_compress_at:
            P = Node(rng, 16, K=96, th_fire=20, th_new=34)
            cand['P'] = {'next': t + 3000, 'backoff': 6000}
            events.append((t, 'grow', 'P'))
            compress_grown = True
        if P is not None:
            recent = [x for x in W.hist[-H:] if x >= 0]
            pooled = np.unique(W.codes[recent].ravel()) if recent else None
            P.step(pooled, t)
            p_hist.append(P.hist[-1])
        if record_topics:
            topics.append(world.topic)


        # bookkeeping for ablations
        for k, p_ in abl.items():
            wo = (p_ == y) and y >= 0
            st = kept_stats.setdefault(k, [0, 0, t])
            st[0] += int(correct and not wo)
            st[1] += int(wo and not correct)

        if t % 1000 == 999:
            acc_blocks.append(float(np.mean(blk)))
            blk = []
            if mode != 'controller':
                continue
            # trial decision
            if trial is not None:
                name, t0 = trial
                if t - t0 >= BURN + EVAL:
                    b, c, _ = kept_stats[name]
                    z = (b - c) / np.sqrt(b + c + 1)
                    d = (b - c) / EVAL
                    if z >= 2.5 and d >= 0.01:
                        events.append((t, 'keep', name, round(d, 4)))
                        kept_stats[name] = [0, 0, t]
                    else:
                        events.append((t, 'reject', name, round(d, 4)))
                        del edges[name]
                        kept_stats.pop(name, None)
                        cand[name]['next'] = t + cand[name]['backoff']
                        cand[name]['backoff'] *= 2
                        cand[name]['rej'] = cand[name].get('rej', 0) + 1
                    trial = None
                    last_change = t
                elif t - t0 == BURN:
                    kept_stats[name] = [0, 0, t]   # start the paired eval after burn-in
            # prune check on kept edges
            for name in [k for k in edges if k != 'self' and (trial is None or k != trial[0])]:
                b, c, since = kept_stats.get(name, [0, 0, t])
                if t - since >= PRUNE_BLOCK:
                    z = (b - c) / np.sqrt(b + c + 1)
                    d = (b - c) / (t - since)
                    if z < 1.0 or d < 0.003:
                        events.append((t, 'prune', name, round(d, 4)))
                        del edges[name]
                        kept_stats.pop(name, None)
                        cand[name]['next'] = t + 3000
                        cand[name]['backoff'] = 6000
                        if reset_on_prune:   # a kept edge failing is evidence the world changed
                            for c in cand.values():
                                c['next'] = min(c['next'], t + 1000)
                                c['backoff'] = 6000
                        last_change = t
                    else:
                        kept_stats[name] = [0, 0, t]
            # plateau -> new trial
            if trial is None and len(acc_blocks) >= 6 and t - last_change >= 4000:
                recent, older = np.mean(acc_blocks[-2:]), np.mean(acc_blocks[-6:-3])
                if recent - older < 0.01:
                    avail = [c for c in cand if c not in edges and cand[c]['next'] <= t]
                    if untried_first:
                        fresh = [c for c in avail if not cand[c].get('tried')]
                        avail = fresh or avail
                    # candidates rejected exhaust_after times no longer block the structural operator
                    exhausted = exhaust_after is not None and all(
                        cand[c].get('rej', 0) >= exhaust_after for c in cand if c not in edges)
                    if exhausted and coact and not compress_grown:
                        avail = []
                    if avail:
                        name = avail[rng.integers(len(avail))]
                        cand[name]['tried'] = True
                        edges[name] = make_edge(name)
                        trial = (name, t)
                        kept_stats[name] = [0, 0, t]
                        events.append((t, 'trial', name))
                    elif coact and not compress_grown:
                        g = lagged_coactivity(W.hist, W.cap)
                        coact_log.append((t, round(g, 2)))
                        if coact_thr is not None and g > coact_thr:
                            P = Node(rng, 16, K=96, th_fire=20, th_new=34)
                            cand['P'] = {'next': t + 3000, 'backoff': 6000}
                            compress_grown = True
                            events.append((t, 'grow', 'P', round(g, 2)))
                            last_change = t

    out = dict(acc=acc_blocks, events=events, edges=sorted(edges), coact=coact_log,
               W_cells=int(W.alive.sum()), W_recruits=W.recruits, W_prunes=W.prunes,
               W_churn_t=[int(e) for e in W.events])
    if P is not None:
        out['P_cells'] = int(P.alive.sum())
        out['P_recruits'] = P.recruits
        out['P_evictions'] = P.evictions
        if record_topics and len(p_hist):
            n = min(20000, len(p_hist))
            out['P_nmi'] = nmi(p_hist[-n:], topics[-n:])
            # stale fraction: P mask bits sitting on codes of no live W cell
            live = np.zeros(D, bool)
            live[W.codes[W.alive].ravel()] = True
            pm = P.masks[P.alive]
            out['P_stale'] = float(1 - (pm & live).sum() / max(1, pm.sum()))
    return out
