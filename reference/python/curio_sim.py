"""Stations, temporal grammar moves, and a Select loop that allocates attention by learning progress.

A Station is one world plus one grown brain (the boot_sim grammar: screened candidates, parallel shadow
trials, synergy/conjoin binds, distill), extended with two temporal moves:
  delay(X, d)        X as it was d ticks back (taps d = 2, 4, 8, 16)
  hold(X | Y = v)    X's value latched whenever the rare event Y = v occurs, held until the next one
                     (an event-gated working-memory slot: Gate + persistence)
Temporal candidates are screened virtually: their series is computed from stored history, and a node is
built only if it passes the screen.

Worlds (8 symbols, 85% rule-following unless noted):
  easy      X, Y, D2 iid;  T1 = map[X],  T3 = map'[Y]
  staged    as boot_sim: T2 = LS1[A,B], T6 = LS2[LS1[A,B],C], T8 = LS3[...,E]
  temporal  X, D2 iid; F a rare flag (5%); K = new value V when F fires, noise otherwise
            Td = map[X at t-8];  Q = map[V] with prob 0.3, noise otherwise (needs the last flagged K)
  noisetv   boot_sim 'nostone': looks structured, nothing learnable by this grammar
"""
import numpy as np
from graph_sim import K, D, Edge, pick, mi, cmi, rand_code
from boot_sim import World as BootWorld, latin


class Node:
    def __init__(self, name, n, rng, kind='base', parts=None, edge=None, extra=None):
        self.name, self.n, self.kind, self.parts, self.edge, self.extra = name, n, kind, parts, edge, extra
        self.codes = np.array([rand_code(rng) for _ in range(n)])
        self.hist = []


class EasyWorld:
    BASE = ['X', 'Y', 'D2', 'T1', 'T3']
    TARGETS = ['T1', 'T3']

    def __init__(self, rng):
        self.r = rng
        self.m1, self.m3 = rng.permutation(K), rng.permutation(K)
        self.prev = {n: 0 for n in self.BASE}

    def step(self):
        r, p = self.r, self.prev
        s = {k: int(r.integers(K)) for k in ('X', 'Y', 'D2')}
        s['T1'] = int(self.m1[p['X']]) if r.random() < 0.85 else int(r.integers(K))
        s['T3'] = int(self.m3[p['Y']]) if r.random() < 0.85 else int(r.integers(K))
        self.prev = s
        return s


class TemporalWorld:
    BASE = ['X', 'K', 'F', 'D2', 'Td', 'Q']
    TARGETS = ['Td', 'Q']

    def __init__(self, rng, p_flag=0.05, p_ans=0.3, delay=8):
        self.r, self.pf, self.pa, self.delay = rng, p_flag, p_ans, delay
        self.mapd, self.mapq = rng.permutation(K), rng.permutation(K)
        self.xh = [0] * 64
        self.V = 0
        self.prev = {n: 0 for n in self.BASE}

    def step(self):
        r = self.r
        s = {'X': int(r.integers(K)), 'D2': int(r.integers(K))}
        Vprev = self.V                      # value as of t-1 (includes a flag at t-1)
        # Td_t = map[X_{t-delay}]
        xd = self.xh[-self.delay]
        s['Td'] = int(self.mapd[xd]) if r.random() < 0.85 else int(r.integers(K))
        s['Q'] = int(self.mapq[Vprev]) if r.random() < self.pa else int(r.integers(K))
        if r.random() < self.pf:
            s['F'] = 1
            self.V = int(r.integers(K))
            s['K'] = self.V
        else:
            s['F'] = 0
            s['K'] = int(r.integers(K))
        self.xh = self.xh[1:] + [s['X']]
        self.prev = s
        return s


class StagedWorld(BootWorld):
    TARGETS = ['T2', 'T6', 'T8']


class NoiseTV(BootWorld):
    TARGETS = ['T2', 'T6', 'T8']

    def __init__(self, rng):
        super().__init__(rng, nostone=True)


WORLDS = {'easy': EasyWorld, 'staged': StagedWorld, 'temporal': TemporalWorld, 'noisetv': NoiseTV}
CEIL = {'T1': 0.869, 'T3': 0.869, 'T2': 0.869, 'T6': 0.869, 'T8': 0.869, 'Td': 0.869, 'Q': 0.3 + 0.7 / 8}


def ffill_hold(x, y, v):
    """hold(x | y = v): value of x at the most recent tick where y == v (0 before the first)."""
    idx = np.where(y == v, np.arange(len(y)), -1)
    idx = np.maximum.accumulate(idx)
    out = np.where(idx >= 0, x[np.maximum(idx, 0)], 0)
    return out


class Station:
    def __init__(self, name, seed, temporal=True, delay_only=False, distill=True, weights=True, max_trials=4,
                 burn=4000, evalw=2000, screen_thr=0.02, syn_thr=0.3, conj_thr=0.2, tmp_thr=0.1,
                 max_cells=512, max_derived=12, window=4000):
        self.name = name
        self.rng = rng = np.random.default_rng(seed)
        self.world = WORLDS[name](rng)
        self.BASE, self.TARGETS = self.world.BASE, self.world.TARGETS
        self.cfg = dict(temporal=temporal, delay_only=delay_only, distill=distill, weights=weights, max_trials=max_trials,
                        burn=burn, evalw=evalw, screen_thr=screen_thr, syn_thr=syn_thr, conj_thr=conj_thr,
                        tmp_thr=tmp_thr, max_cells=max_cells, max_derived=max_derived, window=window)
        self.nodes = {n: Node(n, K, rng) for n in self.BASE}
        self.derived = []
        self.edges = {n: {n: Edge(self.nodes[n], self.nodes[n], rng)} for n in self.BASE}
        self.wt, self.trials, self.trial_t0, self.stats, self.kept_since = {}, {}, {}, {}, {}
        self.backoff, self.scr, self.priority = {}, {}, set()
        self.events, self.zmade = [], set()
        self.acc = {n: [] for n in self.BASE}
        self.t = 0

    # ---------------------------------------------------------------- helpers
    def series(self, name, n):
        return np.array(self.nodes[name].hist[-n:])

    def contains(self, name, dst):
        nd = self.nodes[name]
        if name == dst:
            return True
        if nd.kind == 'bind':
            return self.contains(nd.parts[0], dst) or self.contains(nd.parts[1], dst)
        return False

    def add_node(self, nd, why):
        self.nodes[nd.name] = nd
        self.derived.append(nd)
        self.events.append((self.t, nd.kind, nd.name, why))

    def add_bind(self, x, y, why):
        c = self.cfg
        name = f'({x}*{y})'
        if name in self.nodes or len(self.derived) >= c['max_derived'] or self.nodes[x].n * self.nodes[y].n > c['max_cells']:
            return None
        nd = Node(name, self.nodes[x].n * self.nodes[y].n, self.rng, 'bind', (x, y))
        # bind series from parts' full history so screening can start at once
        hx, hy = self.nodes[x].hist, self.nodes[y].hist
        n = min(len(hx), len(hy))
        nd.hist = [hx[len(hx) - n + i] * self.nodes[y].n + hy[len(hy) - n + i] for i in range(n)]
        self.add_node(nd, why)
        return name

    # ---------------------------------------------------------------- one tick
    def tick(self):
        rng, c = self.rng, self.cfg
        obs = self.world.step()
        for dst in self.BASE:
            sc = {s_: e.scores() * self.wt.get((dst, s_), 1) for s_, e in self.edges[dst].items()}
            total = sum(sc.values())
            y = obs[dst]
            ok = pick(total, rng) == y
            self.acc[dst].append(ok)
            for s_, v in sc.items():
                if s_ == dst:
                    continue
                wo = pick(total - v, rng) == y
                st = self.stats.setdefault((dst, s_), [0, 0])
                st[0] += int(ok and not wo)
                st[1] += int(wo and not ok)
            for (d, s_), e in self.trials.items():
                if d == dst:
                    w = pick(total + e.scores(), rng) == y
                    st = self.stats.setdefault((d, s_), [0, 0])
                    st[0] += int(w and not ok)
                    st[1] += int(ok and not w)
            for e in self.edges[dst].values():
                e.learn(y)
            for (d, s_), e in self.trials.items():
                if d == dst:
                    e.learn(y)
        for n in self.BASE:
            self.nodes[n].hist.append(obs[n])
        for nd in self.derived:
            if nd.kind == 'bind':
                x, y_ = nd.parts
                nd.hist.append(self.nodes[x].hist[-1] * self.nodes[y_].n + self.nodes[y_].hist[-1])
            elif nd.kind == 'distill':
                e = nd.edge
                nd.hist.append(int(np.argmax(e.masks[e.src.hist[-1]][e.dst.codes].sum(1))))
            elif nd.kind == 'delay':
                h = self.nodes[nd.parts[0]].hist
                d = nd.extra
                nd.hist.append(h[-d] if len(h) >= d else 0)
            elif nd.kind == 'hold':
                x, y_ = nd.parts
                v = nd.extra
                prev = nd.hist[-1] if nd.hist else 0
                nd.hist.append(self.nodes[x].hist[-1] if self.nodes[y_].hist[-1] == v else prev)
        if self.t % 1000 == 999:
            self.check()
        self.t += 1

    # ---------------------------------------------------------------- growth controller
    def check(self):
        t, rng, c = self.t, self.rng, self.cfg
        nodes, edges = self.nodes, self.edges
        for key in list(self.trials):
            t0 = self.trial_t0[key]
            if t - t0 == c['burn']:
                self.stats[key] = [0, 0]
            elif t - t0 >= c['burn'] + c['evalw']:
                b, cc = self.stats.get(key, [0, 0])
                z = (b - cc) / np.sqrt(b + cc + 1)
                d = (b - cc) / c['evalw']
                e = self.trials.pop(key)
                if z >= 2.5 and d >= 0.01:
                    edges[key[0]][key[1]] = e
                    self.kept_since[key] = t
                    self.stats[key] = [0, 0]
                    if c['weights']:
                        self.wt[key] = int(min(4, 1 + round(d / 0.05)))
                    self.events.append((t, 'keep', key[0], key[1], round(d, 4)))
                    if c['distill'] and nodes[key[1]].kind == 'bind' and d >= 0.1 and key[0] not in self.zmade:
                        zn = f'Z[{key[0]}]'
                        nd = Node(zn, K, rng, 'distill', parts=(key[1],), edge=e)
                        self.zmade.add(key[0])
                        self.add_node(nd, key[1])
                else:
                    bo = self.backoff.setdefault(key, [0, 6000, 0])
                    bo[0], bo[1], bo[2] = t + bo[1], bo[1] * 2, bo[2] + 1
                    self.events.append((t, 'reject', key[0], key[1], round(d, 4)))
                    dst, s_ = key
                    if self.scr.get(key, 0) >= c['conj_thr'] and not self.contains(s_, dst):
                        nm = self.add_bind(*sorted((s_, dst)), 'conjoin')
                        if nm:
                            self.priority.add((dst, nm))
        for dst in self.BASE:
            for s_ in list(edges[dst]):
                if s_ == dst:
                    continue
                key = (dst, s_)
                if t - self.kept_since[key] >= 4000:
                    b, cc = self.stats.get(key, [0, 0])
                    z = (b - cc) / np.sqrt(b + cc + 1)
                    d = (b - cc) / (t - self.kept_since[key])
                    if z < 1.0 or d < 0.003:
                        del edges[dst][s_]
                        self.wt.pop(key, None)
                        self.backoff[key] = [t + 3000, 6000, 0]
                        self.events.append((t, 'prune', dst, s_, round(d, 4)))
                    elif c['weights']:
                        self.wt[key] = int(min(4, 1 + round(d / 0.05)))
                    self.kept_since[key] = t
                    self.stats[key] = [0, 0]
        W = c['window']
        if t % 5000 == 4999 and t >= W and len(self.derived) < c['max_derived']:
            pressured = [d for d in self.BASE if np.mean(self.acc[d][-2000:]) < 0.6]
            self.synergy(pressured, W)
            if c['temporal']:
                self.temporal(pressured, W)
        self.fill_trials(W)

    def synergy(self, pressured, W):
        rng, nodes = self.rng, self.nodes
        names = [n for n in nodes if len(nodes[n].hist) >= W]
        best = None
        for dst in pressured:
            yn = self.series(dst, W)[1:]
            sh = int(rng.integers(100, W - 100))
            single = {}
            for x in names:
                xs = self.series(x, W)[:-1]
                single[x] = mi(xs, yn, nodes[x].n, K) - mi(np.roll(xs, sh), yn, nodes[x].n, K)
            for i, x in enumerate(names):
                for y_ in names[i + 1:]:
                    if f'({x}*{y_})' in nodes or f'({y_}*{x})' in nodes:
                        continue
                    nx, ny = nodes[x].n, nodes[y_].n
                    if nx * ny > self.cfg['max_cells']:
                        continue
                    pair = self.series(x, W)[:-1] * ny + self.series(y_, W)[:-1]
                    j = mi(pair, yn, nx * ny, K) - mi(np.roll(pair, sh), yn, nx * ny, K)
                    syn = j - max(single[x], 0) - max(single[y_], 0)
                    if syn > self.cfg['syn_thr'] and (best is None or syn > best[0]):
                        best = (syn, dst, x, y_)
        if best:
            nm = self.add_bind(best[2], best[3], f'synergy {best[1]} {best[0]:.2f}')
            if nm:
                self.priority.add((best[1], nm))     # one-shot: trial it for the target it was built for

    def temporal(self, pressured, W):
        """Virtual screen of delay taps and event-gated holds over base nodes; build the best one."""
        rng, nodes = self.rng, self.nodes
        best = None
        H = W + 32
        for dst in pressured:
            yv = self.series(dst, W)
            for x in self.BASE:
                xh = self.series(x, H)
                if len(xh) < H:
                    continue
                cands = []
                for d in (2, 4, 8, 16):
                    if f'{x}@{d}' in nodes:
                        continue
                    cands.append((f'{x}@{d}', 'delay', (x,), d, xh[H - W - (d - 1): H - (d - 1)]))
                if not self.cfg['delay_only']:
                    for y_ in self.BASE:
                        if y_ == x:
                            continue
                        yh = self.series(y_, H)
                        vals, cnt = np.unique(yh, return_counts=True)
                        for v, n_ in zip(vals, cnt):
                            f = n_ / H
                            if 0.005 < f <= 0.1 and f'hold({x}|{y_}={v})' not in nodes:
                                cands.append((f'hold({x}|{y_}={v})', 'hold', (x, y_), int(v), ffill_hold(xh, yh, v)[-W:]))
                for name, kind, parts, extra, sv in cands:
                    sh = int(rng.integers(100, W - 100))
                    if self.cfg.get('tmp_additive', False):
                        # information an additive edge can use directly (conjunctions are left to synergy/conjoin)
                        v_ = mi(sv[:-1], yv[1:], K, K) - mi(np.roll(sv[:-1], sh), yv[1:], K, K)
                    else:
                        v_ = cmi(sv[:-1], yv[:-1], yv[1:], K, K) - cmi(np.roll(sv[:-1], sh), yv[:-1], yv[1:], K, K)
                    if v_ > self.cfg['tmp_thr'] and (best is None or v_ > best[0]):
                        best = (v_, dst, name, kind, parts, extra)
        if best:
            v_, dst, name, kind, parts, extra = best
            nd = Node(name, K, rng, kind, parts, extra=extra)
            xh = np.array(nodes[parts[0]].hist)
            if kind == 'delay':
                nd.hist = list(np.concatenate([np.zeros(extra - 1, int), xh[:len(xh) - (extra - 1)]]))
            else:
                nd.hist = list(ffill_hold(xh, np.array(nodes[parts[1]].hist), extra))
            self.add_node(nd, f'screen {dst} {v_:.2f}')
            self.priority.add((dst, name))

    def fill_trials(self, W):
        t, rng, c = self.t, self.rng, self.cfg
        free = c['max_trials'] - len(self.trials)
        if free <= 0 or t < 3000:
            return
        scored = []
        for dst in self.BASE:
            if sum(1 for k in self.trials if k[0] == dst) >= self.cfg.get('per_target', 1):
                continue
            Wd = self.win(dst, W)
            yv = self.series(dst, Wd)
            for s_ in self.nodes:
                if s_ == dst or s_ in self.edges[dst] or (dst, s_) in self.trials or s_ == f'Z[{dst}]':
                    continue
                bo = self.backoff.get((dst, s_))
                if bo and bo[0] > t:
                    continue
                sv = self.series(s_, Wd)
                n = min(len(yv), len(sv))
                if n < 1000:
                    continue
                a, b_ = yv[-n:], sv[-n:]
                ns = self.nodes[s_].n
                sh = int(rng.integers(100, n - 100))
                v = cmi(b_[:-1], a[:-1], a[1:], ns, K) - cmi(np.roll(b_[:-1], sh), a[:-1], a[1:], ns, K)
                self.scr[(dst, s_)] = v
                if (dst, s_) in self.priority:
                    v += 10
                if v > c['screen_thr']:
                    scored.append((v, dst, s_))
        scored.sort(reverse=True)
        chosen = []
        for v, dst, s_ in scored:
            if len(chosen) >= free:
                break
            if sum(1 for ch in chosen if ch[0] == dst) + sum(1 for k in self.trials if k[0] == dst) >= self.cfg.get('per_target', 1):
                continue
            chosen.append((dst, s_))
        for dst, s_ in chosen:
            self.trials[(dst, s_)] = Edge(self.nodes[dst], self.nodes[s_], rng)
            self.trial_t0[(dst, s_)] = t
            self.stats[(dst, s_)] = [0, 0]
            self.priority.discard((dst, s_))         # priority buys one trial, not a permanent slot
            self.events.append((t, 'trial', dst, s_))

    def win(self, dst, W):
        return W

    def opportunity(self):
        """Bits the screens see for unsolved targets that no kept edge has captured yet."""
        best = 0.0
        for dst in self.BASE:
            if np.mean(self.acc[dst][-2000:]) >= 0.6:
                continue
            for (d, s_), v in self.scr.items():
                if d == dst and s_ not in self.edges[dst]:
                    best = max(best, v)
        return min(1.0, max(0.0, best - 0.05))

    # ---------------------------------------------------------------- reporting
    def run_block(self, n=1000):
        start = len(self.acc[self.TARGETS[0]])
        for _ in range(n):
            self.tick()
        return float(np.mean([np.mean(self.acc[x][start:]) for x in self.TARGETS]))

    def progress(self, x, last=2000):
        """Normalised accuracy on target x over the last `last` ticks: 0 = chance, 1 = ceiling."""
        a = np.mean(self.acc[x][-last:])
        return (a - 0.125) / (CEIL[x] - 0.125)

    def summary(self):
        accb = {n: [float(np.mean(self.acc[n][i:i + 1000])) for i in range(0, len(self.acc[n]), 1000)] for n in self.BASE}
        kept = sorted((d, s_, self.wt.get((d, s_), 1)) for d in self.BASE for s_ in self.edges[d] if s_ != d)
        return dict(acc=accb, events=self.events, kept=kept, derived=[nd.name for nd in self.derived],
                    derived_info=[dict(name=nd.name, kind=nd.kind, parts=list(nd.parts) if nd.parts else [], extra=nd.extra) for nd in self.derived],
                    ticks=self.t)


def run_station(name, seed, ticks, **kw):
    s = Station(name, seed, **kw)
    for _ in range(ticks // 1000):
        s.run_block()
    return s.summary()


# ---------------------------------------------------------------- Select: attention across stations
def run_select(seed, policy, blocks=160, eps=0.15, names=('easy', 'staged', 'temporal', 'noisetv'), eval_blocks=2):
    rng = np.random.default_rng(seed + 999)
    st = [Station(n, seed * 10 + i) for i, n in enumerate(names)]
    visits = [[] for _ in st]       # block accuracy per visit
    struct_events = [[] for _ in st]
    order = []
    for b in range(blocks):
        if b < 2 * len(st) or policy == 'round-robin':
            i = b % len(st)
        elif rng.random() < eps and policy != 'progress+':
            i = int(rng.integers(len(st)))
        elif policy == 'surprise':     # go where error is highest
            i = int(np.argmin([np.mean(v[-3:]) for v in visits]))
        elif policy == 'progress':     # go where accuracy is changing fastest (learning progress)
            lp = []
            for v in visits:
                if len(v) < 4:
                    lp.append(1.0)
                else:
                    k = min(3, len(v) // 2)
                    lp.append(abs(np.mean(v[-k:]) - np.mean(v[-2 * k:-k])))
            i = int(np.argmax(lp))
        elif policy == 'opportunity':  # progress+ plus what the screens say is still on the table
            score = []
            for j, v in enumerate(visits):
                n = len(v)
                if n < 4:
                    score.append(10.0)
                    continue
                k = min(4, n // 2)
                a, b_ = np.array(v[-k:]), np.array(v[-2 * k:-k])
                se = np.sqrt(a.var() / k + b_.var() / k + 2 * 0.007 ** 2 / k)
                lp = max(0.0, abs(a.mean() - b_.mean()) - 2 * se)
                build = 0.02 * sum(struct_events[j][-4:])
                score.append(lp + build + 0.05 * st[j].opportunity() + 0.002 * np.sqrt(np.log(b + 1) / n))
            i = int(np.argmax(score))
        elif policy == 'progress+':   # significant progress + structural activity + exploration bonus
            score = []
            for j, v in enumerate(visits):
                n = len(v)
                if n < 4:
                    score.append(10.0)
                    continue
                k = min(4, n // 2)
                a, b_ = np.array(v[-k:]), np.array(v[-2 * k:-k])
                se = np.sqrt(a.var() / k + b_.var() / k + 2 * 0.007 ** 2 / k)
                lp = max(0.0, abs(a.mean() - b_.mean()) - 2 * se)
                recent = struct_events[j][-4:]
                build = 0.02 * sum(recent)
                score.append(lp + build + 0.01 * np.sqrt(np.log(b + 1) / n))
            i = int(np.argmax(score))
        n_ev = len(st[i].events)
        a = st[i].run_block()
        struct_events[i].append(sum(1 for e in st[i].events[n_ev:] if e[1] in ('keep', 'bind', 'distill', 'delay', 'hold')))
        visits[i].append(a)
        order.append(i)
    # final evaluation: run each station briefly (learning continues) and score its targets
    final = {}
    for s in st:
        for _ in range(eval_blocks):
            s.run_block()
        final[s.name] = {x: float(s.progress(x, 1000 * eval_blocks)) for x in s.TARGETS}
    return dict(order=order, names=list(names), final=final,
                stations={s.name: s.summary() for s in st})
