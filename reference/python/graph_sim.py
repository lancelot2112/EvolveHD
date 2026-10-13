"""Generative wiring on an 8-node graph: inline (screened, parallel shadow trials, synergy-grown
bind nodes) vs population search.

World (8 symbols per node, one symbol per tick):
  L    hidden slow latent (dwell ~30)
  SL   noisy view of L (85%)                  -> phase 2: SL and D1 swap channels
  A,B  iid
  D1   irrelevant slow chain (dwell ~30)
  D2   iid
  T1   T1_t = map[L_{t-1}]                  70%   needs SL (single edge)
  T3   T3_t = map'[T1_{t-1}]                85%   needs T1 (internal edge)
  T2   T2_t = LatinSquare[A_{t-1}, B_{t-1}] 85%   needs A AND B: each alone carries zero information
  T4   T4_t = perm_{L_{t-1}}(T4_{t-1})      85%   needs its own state AND SL (a context x self conjunction)

Every node is a target: it predicts its own next symbol from Prime edges. Each node's cells are
its symbols, each with a fixed 32-of-2048 code (the matcher layer was validated in earlier sims).
A bind node over (X, Y) has 64 cells, one per symbol pair: the symbolic stand-in for shift-bind
followed by k-WTA cleanup (audit sim 2).

Edges: per source cell a writer mask over the target's code space, 1-bit swap toward the code
that followed. Score_j = sum_e popcount(mask_e[src cell] & code_j).
"""
import numpy as np

K, D, CK = 8, 2048, 32
BASE = ['SL', 'A', 'B', 'D1', 'D2', 'T1', 'T2', 'T3', 'T4']
TRUE1 = {('T1', 'SL'), ('T3', 'T1'), ('T2', 'A*B'), ('T4', 'SL*T4')}   # phase-1 ground truth (dst, src)


def rand_code(rng):
    return rng.choice(D, CK, replace=False)


class World:
    def __init__(self, rng, swap_at=None):
        self.r, self.swap_at = rng, swap_at
        self.map1 = rng.permutation(K)
        self.map3 = rng.permutation(K)
        self.perm4 = np.array([rng.permutation(K) for _ in range(K)])
        base = rng.permutation(K)
        self.ls = np.array([[base[(i + j) % K] for j in range(K)] for i in range(K)])
        self.ls = self.ls[rng.permutation(K)][:, rng.permutation(K)]
        self.L = self.D1 = 0
        self.prev = {n: 0 for n in BASE}
        self.t = 0

    def noisy(self, v, p=0.85):
        return int(v) if self.r.random() < p else int(self.r.integers(K))

    def step(self):
        r, p = self.r, self.prev
        L_prev = self.L
        if r.random() < 1 / 30:
            self.L = int(r.integers(K))
        if r.random() < 1 / 30:
            self.D1 = int(r.integers(K))
        s = {}
        s['T1'] = self.noisy(self.map1[L_prev], 0.7)
        s['T3'] = self.noisy(self.map3[p['T1']])
        s['T4'] = self.noisy(self.perm4[L_prev][p['T4']])
        s['T2'] = self.noisy(self.ls[p['A']][p['B']])
        s['A'], s['B'], s['D2'] = (int(r.integers(K)) for _ in range(3))
        sl, d1 = self.noisy(self.L), self.D1
        if self.swap_at is not None and self.t >= self.swap_at:
            sl, d1 = d1, sl
        s['SL'], s['D1'] = sl, d1
        self.prev = s
        self.t += 1
        return s


class Node:
    def __init__(self, name, n, rng, parts=None):
        self.name, self.n, self.parts = name, n, parts
        self.codes = np.array([rand_code(rng) for _ in range(n)])
        self.hist = []


class Edge:
    def __init__(self, dst, src, rng, Kw=None):
        self.dst, self.src, self.rng = dst, src, rng
        self.Kw = Kw or (48 if src is dst else 128)
        self.masks = np.zeros((src.n, D), bool)
        for i in range(src.n):
            self.masks[i, rng.choice(D, self.Kw, replace=False)] = True
        self.cur = -1

    def scores(self):
        self.cur = self.src.hist[-1] if self.src.hist else -1
        if self.cur < 0:
            return np.zeros(self.dst.n)
        return self.masks[self.cur][self.dst.codes].sum(1).astype(float)

    def learn(self, y):
        if self.cur < 0:
            return
        m = self.masks[self.cur]
        code = self.dst.codes[y]
        add = code[~m[code]]
        if len(add) == 0:
            return
        tgt = np.zeros(D, bool)
        tgt[code] = True
        rem = np.flatnonzero(m & ~tgt)
        m[self.rng.choice(rem)] = False
        m[self.rng.choice(add)] = True


def pick(s, rng):
    return int(np.argmax(s + rng.random(len(s)) * 1e-3))


def mi(x, y, nx, ny):
    J = np.bincount(x * ny + y, minlength=nx * ny).reshape(nx, ny).astype(float)
    P = J / J.sum()
    px, py = P.sum(1, keepdims=True), P.sum(0, keepdims=True)
    nz = P > 0
    return float(np.sum(P[nz] * np.log2(P[nz] / (px @ py)[nz])))


def cmi(s, yp, yn, ns, ny):
    """I(yn ; s | yp) in bits."""
    J = np.bincount((s * ny + yp) * ny + yn, minlength=ns * ny * ny).reshape(ns, ny, ny).astype(float)
    P = J / J.sum()
    p_syp = P.sum(2, keepdims=True)
    p_ypyn = P.sum(0, keepdims=True)
    p_yp = P.sum((0, 2), keepdims=True)
    nz = P > 0
    num = P * p_yp
    den = p_syp * p_ypyn
    return float(np.sum(P[nz] * np.log2(num[nz] / den[nz])))


class Brain:
    def __init__(self, rng, wiring=(), binds=()):
        self.rng = rng
        self.nodes = {n: Node(n, K, rng) for n in BASE}
        self.binds = {}
        for pair in binds:
            self.add_bind(*pair)
        self.edges = {n: {n: Edge(self.nodes[n], self.nodes[n], rng)} for n in BASE}   # dst -> src -> edge
        for dst, src in wiring:
            self.edges[dst][src] = Edge(self.nodes[dst], self.node(src), rng)
        self.trials = {}    # (dst, src) -> Edge in shadow
        self.acc = {n: [] for n in BASE}
        self.wt = {}   # (dst, src) -> integer vote weight

    def node(self, name):
        return self.nodes.get(name) or self.binds[name]

    def add_bind(self, x, y):
        name = f'{x}*{y}'
        if name not in self.binds:
            self.binds[name] = Node(name, K * K, self.rng, parts=(x, y))
        return name

    def tick(self, obs, stats=None):
        """Predict every node's symbol, then observe and learn. stats: dict to accumulate ablations."""
        rng = self.rng
        for dst in BASE:
            sc = {s: e.scores() * self.wt.get((dst, s), 1) for s, e in self.edges[dst].items()}
            total = sum(sc.values())
            pred = pick(total, rng)
            y = obs[dst]
            ok = pred == y
            self.acc[dst].append(ok)
            if stats is not None:
                for s, v in sc.items():
                    if s == dst:
                        continue
                    wo = pick(total - v, rng) == y
                    st = stats.setdefault((dst, s), [0, 0])
                    st[0] += int(ok and not wo)
                    st[1] += int(wo and not ok)
                for (d, s), e in self.trials.items():
                    if d != dst:
                        continue
                    w = pick(total + e.scores(), rng) == y
                    st = stats.setdefault((d, s), [0, 0])
                    st[0] += int(w and not ok)
                    st[1] += int(ok and not w)
            for e in self.edges[dst].values():
                e.learn(y)
            for (d, s), e in self.trials.items():
                if d == dst:
                    if e.cur < 0 and e.src.hist:   # trial edge scored above only when stats on
                        e.cur = e.src.hist[-1]
                    e.learn(y)
        for n in BASE:
            self.nodes[n].hist.append(obs[n])
        for name, b in self.binds.items():
            x, y_ = b.parts
            b.hist.append(obs[x] * K + obs[y_])


# ------------------------------------------------------------------ inline controller
def run_inline(seed, ticks=100000, swap_at=50000, mode='screen', max_trials=4, synergy=True,
               conjoin=True, weights=False, burn=4000, evalw=2000, screen_thr=0.02, syn_thr=0.3, conj_thr=0.2,
               max_binds=8, window=4000):
    rng = np.random.default_rng(seed)
    world = World(rng, swap_at=swap_at)
    br = Brain(rng)
    stats = {}
    trial_t0 = {}
    kept_since = {}
    backoff = {}     # (dst,src) -> [next_allowed, backoff, rejections]
    events = []
    tried = set()
    scr = {}         # last screen score per (dst, src)
    priority = set() # (dst, bind) candidates created by the conjoin rule

    def conjoin_check(t, key):
        dst, s = key
        if not conjoin or s not in BASE or scr.get(key, 0) < conj_thr or len(br.binds) >= max_binds:
            return
        x, y = sorted((s, dst), key=BASE.index)
        if f'{x}*{y}' in br.binds:
            return
        name = br.add_bind(x, y)
        priority.add((dst, name))
        events.append((t, 'conjoin', dst, name, round(scr[key], 3)))

    def candidates(t):
        srcs = BASE + list(br.binds)
        out = []
        for dst in BASE:
            for s in srcs:
                if s == dst or s in br.edges[dst] or (dst, s) in br.trials:
                    continue
                b = backoff.get((dst, s))
                if b and b[0] > t:
                    continue
                out.append((dst, s))
        return out

    def series(name, n):
        return np.array(br.node(name).hist[-n:])

    for t in range(ticks):
        obs = world.step()
        # make sure trial edges see the right source cell (scores() sets cur)
        br.tick(obs, stats)
        if t % 1000 != 999:
            continue
        # finish trials
        for key in list(br.trials):
            t0 = trial_t0[key]
            if t - t0 == burn:
                stats[key] = [0, 0]
            elif t - t0 >= burn + evalw:
                b, c = stats.get(key, [0, 0])
                z = (b - c) / np.sqrt(b + c + 1)
                d = (b - c) / evalw
                e = br.trials.pop(key)
                if z >= 2.5 and d >= 0.01:
                    br.edges[key[0]][key[1]] = e
                    kept_since[key] = t
                    if weights:
                        br.wt[key] = int(min(4, 1 + round(d / 0.05)))
                    stats[key] = [0, 0]
                    events.append((t, 'keep', key[0], key[1], round(d, 4)))
                else:
                    bo = backoff.setdefault(key, [0, 6000, 0])
                    bo[0], bo[1], bo[2] = t + bo[1], bo[1] * 2, bo[2] + 1
                    events.append((t, 'reject', key[0], key[1], round(d, 4)))
                    # information the screen saw but an additive edge could not use -> bind it with the target
                    conjoin_check(t, key)
        # prune kept edges
        for dst in BASE:
            for s in list(br.edges[dst]):
                if s == dst:
                    continue
                key = (dst, s)
                if t - kept_since[key] >= 4000:
                    b, c = stats.get(key, [0, 0])
                    z = (b - c) / np.sqrt(b + c + 1)
                    d = (b - c) / (t - kept_since[key])
                    if z < 1.0 or d < 0.003:
                        del br.edges[dst][s]
                        br.wt.pop(key, None)
                        backoff[key] = [t + 3000, 6000, 0]
                        events.append((t, 'prune', dst, s, round(d, 4)))
                        conjoin_check(t, key)
                    elif weights:
                        br.wt[key] = int(min(4, 1 + round(d / 0.05)))
                    kept_since[key] = t
                    stats[key] = [0, 0]
        # synergy screen -> grow bind nodes
        if synergy and t % 5000 == 4999 and len(br.binds) < max_binds and t >= window:
            best = None
            for dst in BASE:
                yn = series(dst, window)[1:]
                for i, x in enumerate(BASE):
                    for y in BASE[i + 1:]:
                        if f'{x}*{y}' in br.binds:
                            continue
                        xs, ys = series(x, window)[:-1], series(y, window)[:-1]
                        sh = int(rng.integers(100, window - 100))
                        pair = xs * K + ys
                        j = mi(pair, yn, K * K, K) - mi(np.roll(pair, sh), yn, K * K, K)
                        a = mi(xs, yn, K, K) - mi(np.roll(xs, sh), yn, K, K)
                        b = mi(ys, yn, K, K) - mi(np.roll(ys, sh), yn, K, K)
                        syn = j - max(a, 0) - max(b, 0)
                        if syn > syn_thr and (best is None or syn > best[0]):
                            best = (syn, dst, x, y)
            if best:
                name = br.add_bind(best[2], best[3])
                events.append((t, 'grow', best[1], name, round(best[0], 3)))
        # fill trial slots
        free = max_trials - len(br.trials)
        if free > 0 and t >= 3000:
            cands = candidates(t)
            if mode == 'random':
                fresh = [c for c in cands if c not in tried]
                pool = fresh or cands
                rng.shuffle(pool)
                chosen = pool[:free]
            else:
                scored = []
                for dst, s in cands:
                    yp_yn = series(dst, window)
                    src = series(s, window)
                    n = min(len(yp_yn), len(src))
                    if n < 1000:
                        continue
                    yv, sv = yp_yn[-n:], src[-n:]
                    ns = br.node(s).n
                    sh = int(rng.integers(100, n - 100))
                    sc_ = cmi(sv[:-1], yv[:-1], yv[1:], ns, K) - cmi(np.roll(sv[:-1], sh), yv[:-1], yv[1:], ns, K)
                    scr[(dst, s)] = sc_
                    if (dst, s) in priority:
                        sc_ += 10.0
                    if sc_ > screen_thr:
                        scored.append((sc_, dst, s))
                scored.sort(reverse=True)
                chosen = []
                for sc_, dst, s in scored:
                    if len(chosen) >= free:
                        break
                    if any(c[0] == dst for c in chosen) or any(k[0] == dst for k in br.trials):
                        continue     # at most one trial per target at a time
                    chosen.append((dst, s))
            for dst, s in chosen:
                br.trials[(dst, s)] = Edge(br.nodes[dst], br.node(s), rng)
                trial_t0[(dst, s)] = t
                stats[(dst, s)] = [0, 0]
                tried.add((dst, s))
                events.append((t, 'trial', dst, s))
    acc = {n: [float(np.mean(br.acc[n][i:i + 1000])) for i in range(0, ticks, 1000)] for n in BASE}
    kept = sorted((d, s) for d in BASE for s in br.edges[d] if s != d)
    return dict(acc=acc, events=events, kept=kept, binds=sorted(br.binds))


# ------------------------------------------------------------------ fixed wiring (for the GA)
def run_fixed(seed, wiring, binds, ticks=20000, score_from=10000, w=1):
    world = World(np.random.default_rng(seed))            # same world for every genome
    br = Brain(np.random.default_rng(seed + 1000), wiring=wiring, binds=binds)
    for dst, src in wiring:
        br.wt[(dst, src)] = w
    for t in range(ticks):
        br.tick(world.step())
    per = {n: float(np.mean(br.acc[n][score_from:])) for n in BASE}
    return per
