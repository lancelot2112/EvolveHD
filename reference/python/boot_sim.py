"""Bootstrapping depth: does solving an easy puzzle build parts for a harder one?

World (8 symbols per node, one symbol per tick, 85% rule-following):
  A, B, C, E   iid
  D1           irrelevant slow chain (dwell ~30);  D2 iid
  T2_t = LS1[A, B]                    (all inputs at t-1)   level 1: a pair
  T6_t = LS2[LS1[A, B], C]                                  level 2: needs level 1's intermediate
  T8_t = LS3[LS2[LS1[A, B], C], E]                          level 3
'nostone' world: T2 and T6 are replaced by iid noise, so nothing rewards building A*B first and
T8 is a bare 4-input puzzle.

Grammar (inline, shadow trials, screened candidates, as in graph_sim):
  bind(X, Y)   any two nodes, including binds and distilled nodes, if cells(X)*cells(Y) <= 512
  distill      when an edge bind -> T is kept with delta >= 0.1, add an 8-cell node Z[T] whose state is
               that edge's own prediction of T: compresses the bind down to the variable T cares about
  weights      optional: each kept edge votes with integer weight 1 + round(delta / 0.05), capped at 4
"""
import numpy as np
from graph_sim import K, D, CK, Edge, pick, mi, cmi, rand_code


class Node:
    def __init__(self, name, n, rng, kind='base', parts=None, edge=None):
        self.name, self.n, self.kind, self.parts, self.edge = name, n, kind, parts, edge
        self.codes = np.array([rand_code(rng) for _ in range(n)])
        self.hist = []


def latin(rng):
    base = rng.permutation(K)
    ls = np.array([[base[(i + j) % K] for j in range(K)] for i in range(K)])
    return ls[rng.permutation(K)][:, rng.permutation(K)]


class World:
    BASE = ['A', 'B', 'C', 'E', 'D1', 'D2', 'T2', 'T6', 'T8']

    def __init__(self, rng, nostone=False):
        self.r, self.nostone = rng, nostone
        self.ls1, self.ls2, self.ls3 = latin(rng), latin(rng), latin(rng)
        self.D1 = 0
        self.prev = {n: 0 for n in self.BASE}

    def noisy(self, v, p=0.85):
        return int(v) if self.r.random() < p else int(self.r.integers(K))

    def step(self):
        r, p = self.r, self.prev
        if r.random() < 1 / 30:
            self.D1 = int(r.integers(K))
        x1 = self.ls1[p['A']][p['B']]
        x2 = self.ls2[x1][p['C']]
        x3 = self.ls3[x2][p['E']]
        s = {k: int(r.integers(K)) for k in ('A', 'B', 'C', 'E', 'D2')}
        s['D1'] = self.D1
        if self.nostone:
            s['T2'], s['T6'] = int(r.integers(K)), int(r.integers(K))
        else:
            s['T2'], s['T6'] = self.noisy(x1), self.noisy(x2)
        s['T8'] = self.noisy(x3)
        self.prev = s
        return s


def run(seed, ticks=120000, nostone=False, distill=True, weights=False, max_trials=4,
        burn=4000, evalw=2000, screen_thr=0.02, syn_thr=0.3, conj_thr=0.2, max_cells=512,
        max_derived=12, window=4000):
    rng = np.random.default_rng(seed)
    world = World(rng, nostone)
    BASE = World.BASE
    nodes = {n: Node(n, K, rng) for n in BASE}
    derived = []                       # creation order = update order
    edges = {n: {n: Edge(nodes[n], nodes[n], rng)} for n in BASE}
    wt = {}                            # (dst, src) -> vote weight
    trials, trial_t0, stats, kept_since = {}, {}, {}, {}
    backoff, scr, priority, tried = {}, {}, set(), set()
    events = []
    acc = {n: [] for n in BASE}
    zmade = set()

    def add_bind(x, y, t, why):
        name = f'({x}*{y})'
        if name in nodes or len(derived) >= max_derived or nodes[x].n * nodes[y].n > max_cells:
            return None
        nd = Node(name, nodes[x].n * nodes[y].n, rng, 'bind', (x, y))
        nodes[name] = nd
        derived.append(nd)
        events.append((t, 'bind', name, why))
        return name

    def series(name, n):
        return np.array(nodes[name].hist[-n:])

    def contains(name, dst):
        nd = nodes[name]
        if name == dst:
            return True
        if nd.kind == 'bind':
            return contains(nd.parts[0], dst) or contains(nd.parts[1], dst)
        return False

    for t in range(ticks):
        obs = world.step()
        # ---- predict / learn
        for dst in BASE:
            sc = {}
            for s_, e in edges[dst].items():
                sc[s_] = e.scores() * wt.get((dst, s_), 1)
            total = sum(sc.values())
            y = obs[dst]
            ok = pick(total, rng) == y
            acc[dst].append(ok)
            for s_, v in sc.items():
                if s_ == dst:
                    continue
                wo = pick(total - v, rng) == y
                st = stats.setdefault((dst, s_), [0, 0])
                st[0] += int(ok and not wo)
                st[1] += int(wo and not ok)
            for (d, s_), e in trials.items():
                if d == dst:
                    w = pick(total + e.scores(), rng) == y
                    st = stats.setdefault((d, s_), [0, 0])
                    st[0] += int(w and not ok)
                    st[1] += int(ok and not w)
            for e in edges[dst].values():
                e.learn(y)
            for (d, s_), e in trials.items():
                if d == dst:
                    e.learn(y)
        # ---- observe; derived nodes in creation order
        for n in BASE:
            nodes[n].hist.append(obs[n])
        for nd in derived:
            if nd.kind == 'bind':
                x, y_ = nd.parts
                nd.hist.append(nodes[x].hist[-1] * nodes[y_].n + nodes[y_].hist[-1])
            else:   # distilled: the source edge's own prediction of its target, from the source's current state
                e = nd.edge
                i = e.src.hist[-1]
                nd.hist.append(int(np.argmax(e.masks[i][e.dst.codes].sum(1))))
        if t % 1000 != 999:
            continue
        # ---- trials
        for key in list(trials):
            t0 = trial_t0[key]
            if t - t0 == burn:
                stats[key] = [0, 0]
            elif t - t0 >= burn + evalw:
                b, c = stats.get(key, [0, 0])
                z = (b - c) / np.sqrt(b + c + 1)
                d = (b - c) / evalw
                e = trials.pop(key)
                if z >= 2.5 and d >= 0.01:
                    edges[key[0]][key[1]] = e
                    kept_since[key] = t
                    stats[key] = [0, 0]
                    if weights:
                        wt[key] = int(min(4, 1 + round(d / 0.05)))
                    events.append((t, 'keep', key[0], key[1], round(d, 4)))
                    if distill and nodes[key[1]].kind == 'bind' and d >= 0.1 and key[0] not in zmade:
                        zn = f'Z[{key[0]}]'
                        nd = Node(zn, K, rng, 'distill', edge=e)
                        nodes[zn] = nd
                        derived.append(nd)
                        zmade.add(key[0])
                        events.append((t, 'distill', zn, key[1]))
                else:
                    bo = backoff.setdefault(key, [0, 6000, 0])
                    bo[0], bo[1], bo[2] = t + bo[1], bo[1] * 2, bo[2] + 1
                    events.append((t, 'reject', key[0], key[1], round(d, 4)))
                    dst, s_ = key
                    if scr.get(key, 0) >= conj_thr and not contains(s_, dst):
                        nm = add_bind(*sorted((s_, dst)), t, 'conjoin')
                        if nm:
                            priority.add((dst, nm))
        # ---- prune
        for dst in BASE:
            for s_ in list(edges[dst]):
                if s_ == dst:
                    continue
                key = (dst, s_)
                if t - kept_since[key] >= 4000:
                    b, c = stats.get(key, [0, 0])
                    z = (b - c) / np.sqrt(b + c + 1)
                    d = (b - c) / (t - kept_since[key])
                    if z < 1.0 or d < 0.003:
                        del edges[dst][s_]
                        wt.pop(key, None)
                        backoff[key] = [t + 3000, 6000, 0]
                        events.append((t, 'prune', dst, s_, round(d, 4)))
                    elif weights:
                        wt[key] = int(min(4, 1 + round(d / 0.05)))
                    kept_since[key] = t
                    stats[key] = [0, 0]
        # ---- synergy -> bind
        if t % 5000 == 4999 and t >= window and len(derived) < max_derived:
            names = [n for n in nodes if len(nodes[n].hist) >= window]
            best = None
            for dst in BASE:
                if np.mean(acc[dst][-2000:]) > 0.6:     # already well predicted: no pressure
                    continue
                yn = series(dst, window)[1:]
                single = {}
                sh = int(rng.integers(100, window - 100))
                for x in names:
                    xs = series(x, window)[:-1]
                    single[x] = mi(xs, yn, nodes[x].n, K) - mi(np.roll(xs, sh), yn, nodes[x].n, K)
                for i, x in enumerate(names):
                    for y_ in names[i + 1:]:
                        if f'({x}*{y_})' in nodes or f'({y_}*{x})' in nodes:
                            continue
                        nx, ny = nodes[x].n, nodes[y_].n
                        if nx * ny > max_cells:
                            continue
                        pair = series(x, window)[:-1] * ny + series(y_, window)[:-1]
                        j = mi(pair, yn, nx * ny, K) - mi(np.roll(pair, sh), yn, nx * ny, K)
                        syn = j - max(single[x], 0) - max(single[y_], 0)
                        if syn > syn_thr and (best is None or syn > best[0]):
                            best = (syn, dst, x, y_)
            if best:
                add_bind(best[2], best[3], t, f'synergy {best[1]} {best[0]:.2f}')
        # ---- fill trial slots
        free = max_trials - len(trials)
        if free > 0 and t >= 3000:
            scored = []
            for dst in BASE:
                if any(k[0] == dst for k in trials):
                    continue
                yv = series(dst, window)
                for s_ in nodes:
                    if s_ == dst or s_ in edges[dst] or (dst, s_) in trials or s_ == f'Z[{dst}]':
                        continue
                    bo = backoff.get((dst, s_))
                    if bo and bo[0] > t:
                        continue
                    sv = series(s_, window)
                    n = min(len(yv), len(sv))
                    if n < 1000:
                        continue
                    a, b_ = yv[-n:], sv[-n:]
                    ns = nodes[s_].n
                    sh = int(rng.integers(100, n - 100))
                    v = cmi(b_[:-1], a[:-1], a[1:], ns, K) - cmi(np.roll(b_[:-1], sh), a[:-1], a[1:], ns, K)
                    scr[(dst, s_)] = v
                    if (dst, s_) in priority:
                        v += 10
                    if v > screen_thr:
                        scored.append((v, dst, s_))
            scored.sort(reverse=True)
            chosen = []
            for v, dst, s_ in scored:
                if len(chosen) >= free:
                    break
                if any(c[0] == dst for c in chosen):
                    continue
                chosen.append((dst, s_))
            for dst, s_ in chosen:
                trials[(dst, s_)] = Edge(nodes[dst], nodes[s_], rng)
                trial_t0[(dst, s_)] = t
                stats[(dst, s_)] = [0, 0]
                events.append((t, 'trial', dst, s_))
    accb = {n: [float(np.mean(acc[n][i:i + 1000])) for i in range(0, ticks, 1000)] for n in BASE}
    kept = sorted((d, s_, wt.get((d, s_), 1)) for d in BASE for s_ in edges[d] if s_ != d)
    return dict(acc=accb, events=events, kept=kept, derived=[nd.name for nd in derived])
