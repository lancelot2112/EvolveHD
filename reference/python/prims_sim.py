"""The remaining primitives as grammar moves, plus a give-up rule for the attention loop.

Builds on curio_sim.Station (screened candidates, shadow trials, synergy/conjoin binds, distill,
delay taps, event-gated holds) and adds:

Correct  (cerebellum)  A shared granule expansion over the station's base nodes: one cell per joint value
          of every k-subset of sources (fixed random conjunctions, k = 3 by default), and one error-driven
          readout per unexplained target (Purkinje: weights move only when its own guess is wrong).
          Grown when a target has no kept edges and the screens have found nothing for 3 checks.
          Trials are long and adaptive: kept as soon as an eval window passes, rejected after 60k ticks.
Select   (basal ganglia) Chooses the station's action from context with Go/NoGo tables and a critic
          (delta = reward - V; NoGo and Go-LTD on dips). Context inputs are grown by screening which
          source carries reward information jointly with the action, then tested by interleaving ticks
          with and without the new input (actions can't be shadow-tested: only the taken action's
          reward is seen).
Broadcast (LC-like)      Phasic surprise: when a target's fast accuracy drops well below its slow average,
          its kept edges get an early re-check, its candidates lose their backoff, new trials into it get
          half the burn-in, and its edges learn 3x faster for 4000 ticks.
"""
import itertools
import numpy as np
from graph_sim import K, Edge, pick, mi, cmi, World as GraphWorld
from curio_sim import (Station, Node, EasyWorld, TemporalWorld, StagedWorld, NoiseTV, WORLDS as CW,
                       CEIL, ffill_hold)


# ------------------------------------------------------------------ worlds
class HardWorld:
    """A 3-input puzzle with no stepping stone: T = LS2[LS1[A, B], C]; every pair carries zero information."""
    BASE = ['A', 'B', 'C', 'D1', 'D2', 'T']
    TARGETS = ['T']

    def __init__(self, rng):
        from boot_sim import latin
        self.r = rng
        self.l1, self.l2 = latin(rng), latin(rng)
        self.prev = {n: 0 for n in self.BASE}
        self.D1 = 0

    def step(self):
        r, p = self.r, self.prev
        if r.random() < 1 / 30:
            self.D1 = int(r.integers(K))
        s = {k: int(r.integers(K)) for k in ('A', 'B', 'C', 'D2')}
        s['D1'] = self.D1
        v = self.l2[self.l1[p['A']][p['B']]][p['C']]
        s['T'] = int(v) if r.random() < 0.85 else int(r.integers(K))
        self.prev = s
        return s


class VeryHardWorld(NoiseTV):
    """The 4-input no-stepping-stone puzzle (boot_sim nostone): only T8 carries structure."""
    TARGETS = ['T8']


class PureNoise:
    BASE = ['A', 'B', 'C', 'T1', 'T2']
    TARGETS = ['T1', 'T2']

    def __init__(self, rng):
        self.r = rng

    def step(self):
        return {k: int(self.r.integers(K)) for k in self.BASE}


class Bloomer:
    """Pure noise until `awake` is switched on from outside, then T1 = map[X]."""
    BASE = ['X', 'Y', 'T1']
    TARGETS = ['T1']

    def __init__(self, rng):
        self.r, self.awake = rng, False
        self.m = rng.permutation(K)
        self.px = 0

    def step(self):
        r = self.r
        s = {'X': int(r.integers(K)), 'Y': int(r.integers(K))}
        s['T1'] = int(self.m[self.px]) if (self.awake and r.random() < 0.85) else int(r.integers(K))
        self.px = s['X']
        return s


class ActionWorld:
    """The station acts. Reward R_t = 1 if its action at t matches policy[C_{t-1}] (10% flipped)."""
    BASE = ['C', 'E', 'D2', 'Act', 'R']
    TARGETS = ['R']
    ACTION = True

    def __init__(self, rng):
        self.r = rng
        self.pol = rng.integers(K, size=K)
        self.pc = 0

    def step(self, act):
        r = self.r
        rew = int(act == self.pol[self.pc])
        if r.random() < 0.1:
            rew = 1 - rew
        s = {'C': int(r.integers(K)), 'E': int(r.integers(K)), 'D2': int(r.integers(K)), 'Act': int(act), 'R': rew}
        self.pc = s['C']
        return s


class SwapWorld(GraphWorld):
    BASE = ['SL', 'A', 'B', 'D1', 'D2', 'T1', 'T2', 'T3', 'T4']
    TARGETS = ['T1', 'T2', 'T3', 'T4']

    def __init__(self, rng):
        super().__init__(rng, swap_at=40000)


WORLDS = dict(CW, hard=HardWorld, veryhard=VeryHardWorld, noise=PureNoise, bloomer=Bloomer, action=ActionWorld,
              swap=SwapWorld)
CEIL = dict(CEIL, T=0.869, R=0.86)


# ------------------------------------------------------------------ Correct: granule expansion + error-driven readout
class Granules:
    def __init__(self, srcs, rng, k=3, max_subsets=120):
        self.srcs = srcs
        subs = list(itertools.combinations(range(len(srcs)), k))
        if len(subs) > max_subsets:
            subs = [subs[i] for i in rng.choice(len(subs), max_subsets, replace=False)]
        self.sub = np.array(subs)
        self.mult = K ** np.arange(k)
        self.S, self.ncell = len(subs), K ** k
        self.t_cached, self.idx = -1, None

    def active(self, t):
        if self.t_cached != t:
            vals = np.array([s.hist[-1] for s in self.srcs])
            self.idx = (vals[self.sub] * self.mult).sum(1)
            self.t_cached = t
        return self.idx


class CorrectEdge:
    """Purkinje-like readout from the shared granule layer to one target; learns only on its own errors."""
    def __init__(self, dst, gran, rng, clip=24):
        self.dst, self.g, self.rng, self.clip = dst, gran, rng, clip
        self.W = np.zeros((gran.S, gran.ncell, dst.n), np.int16)
        self.ar = np.arange(gran.S)
        self.src = None
        self.t = 0

    def scores(self):
        idx = self.g.active(self.t)
        self.cur = idx
        self.last = self.W[self.ar, idx].sum(0).astype(float)
        return self.last.copy()

    def learn(self, y):
        guess = int(np.argmax(self.last + self.rng.random(len(self.last)) * 1e-3))
        if guess != y:
            idx = self.cur
            self.W[self.ar, idx, y] = np.minimum(self.W[self.ar, idx, y] + 1, self.clip)
            self.W[self.ar, idx, guess] = np.maximum(self.W[self.ar, idx, guess] - 1, -self.clip)
        self.t += 1


# ------------------------------------------------------------------ Station with the full primitive set
class FullStation(Station):
    def __init__(self, name, seed, cerebellum=True, k_sub=3, select=True, broadcast=True, cb_max_age=60000, max_derived=16,
                 per_target=2, tmp_additive=True, max_trials=6, **kw):
        self.name = name
        super().__init__('easy', seed, max_derived=max_derived, **kw)          # builds generic state; world replaced below
        rng = self.rng
        self.name = name
        self.world = WORLDS[name](rng)
        self.BASE, self.TARGETS = self.world.BASE, self.world.TARGETS
        self.nodes = {n: Node(n, K, rng) for n in self.BASE}
        self.derived = []
        self.edges = {n: {n: Edge(self.nodes[n], self.nodes[n], rng)} for n in self.BASE}
        self.acc = {n: [] for n in self.BASE}
        self.cfg.update(cerebellum=cerebellum, k_sub=k_sub, select=select, broadcast=broadcast, cb_max_age=cb_max_age,
                        per_target=per_target, tmp_additive=tmp_additive, max_trials=max_trials)
        self.gran, self.cb_names, self.dry = None, set(), {n: 0 for n in self.BASE}
        self.cb_hist, self.cb_pass = {}, {}
        # broadcast state
        self.ema_f = {n: None for n in self.BASE}
        self.ema_s = {n: None for n in self.BASE}
        self.alert = {}
        self.alert_start = {}
        self.refract = {}
        # select state
        self.action = getattr(self.world, 'ACTION', False)
        self.reward = []
        if self.action:
            self.sel_ctx = ['bias']
            self.go, self.nogo, self.V = {}, {}, {}
            self._init_ctx('bias', 1)
            self.sel_trial = None
            self.sel_rejected, self.sel_backoff = {}, {}

    # ---------------- Select
    def _init_ctx(self, name, n):
        self.go[name] = np.zeros((n, K))
        self.nogo[name] = np.zeros((n, K))
        self.V[name] = np.full(n, 0.2)

    def _cell(self, name):
        if name == 'bias':
            return 0
        h = self.nodes[name].hist
        return h[-1] if h else 0

    def choose(self):
        ctx = list(self.sel_ctx)
        if self.sel_trial is not None and self.t % 2 == 0:
            ctx.append(self.sel_trial['src'])
        q = sum(self.go[c][self._cell(c)] - self.nogo[c][self._cell(c)] for c in ctx)
        if self.rng.random() < 0.05:
            return int(self.rng.integers(K))
        return int(np.argmax(q + self.rng.random(K) * 1e-3))

    def learn_select(self, act, rew):
        for c in set(self.sel_ctx) | ({self.sel_trial['src']} if self.sel_trial else set()):
            cell = self.sel_cells[c]
            d = rew - self.V[c][cell]
            self.V[c][cell] += 0.05 * d
            if d > 0:
                self.go[c][cell, act] += d
            else:
                self.nogo[c][cell, act] -= d
                self.go[c][cell, act] = max(0.0, self.go[c][cell, act] + d)

    # ---------------- one tick
    def tick(self):
        rng, c = self.rng, self.cfg
        if self.action:
            self.sel_cells = {k: self._cell(k) for k in set(self.sel_ctx) | ({self.sel_trial['src']} if self.sel_trial else set())}
            act = self.choose()
            obs = self.world.step(act)
            self.reward.append(obs['R'])
            self.learn_select(act, obs['R'])
            if self.sel_trial is not None and self.t - self.sel_trial['t0'] >= self.cfg['burn']:
                self.sel_trial['r'][self.t % 2].append(obs['R'])
        else:
            obs = self.world.step()
        for dst in self.BASE:
            for e in list(self.edges[dst].values()) + [e for (d, s_), e in self.trials.items() if d == dst]:
                if isinstance(e, CorrectEdge):
                    e.t = self.t
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
            boost = 3 if self.alert.get(dst, -1) > self.t else 1
            for e in self.edges[dst].values():
                for _ in range(boost if not isinstance(e, CorrectEdge) else 1):
                    e.learn(y)
            for (d, s_), e in self.trials.items():
                if d == dst:
                    e.learn(y)
            if c['broadcast']:
                self._broadcast(dst, ok)
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
                nd.hist.append(h[-nd.extra] if len(h) >= nd.extra else 0)
            elif nd.kind == 'hold':
                x, y_ = nd.parts
                prev = nd.hist[-1] if nd.hist else 0
                nd.hist.append(self.nodes[x].hist[-1] if self.nodes[y_].hist[-1] == nd.extra else prev)
        if self.t % 1000 == 999:
            self.check()
        self.t += 1

    def _broadcast(self, dst, ok):
        f, s = self.ema_f[dst], self.ema_s[dst]
        f = ok if f is None else f + (ok - f) / 300
        s = ok if s is None else s + (ok - s) / 5000
        self.ema_f[dst], self.ema_s[dst] = f, s
        if self.t > 6000 and s > 0.3 and s - f > 0.12 and self.refract.get(dst, -1) < self.t:
            self.alert[dst] = self.t + 4000
            self.refract[dst] = self.t + 8000
            for s_ in self.edges[dst]:
                if s_ != dst:
                    self.stats[(dst, s_)] = [0, 0]
                    self.kept_since[(dst, s_)] = self.t - 2000      # re-check in ~2000 ticks
            for key in list(self.backoff):
                if key[0] == dst:
                    del self.backoff[key]
            # trials into dst were chosen on the old world's statistics: drop them, and let the
            # screens look only at data since the alarm
            prev = self.alert_start.get(dst)
            for key in [k for k in self.trials if k[0] == dst and not isinstance(self.trials[k], CorrectEdge)
                        and (prev is None or self.trial_t0[k] < prev)]:
                del self.trials[key]          # only trials chosen before the change; keep ones started since an earlier alarm
            self.alert_start[dst] = self.t
            self.events.append((self.t, 'broadcast', dst, f'accuracy fell from {s:.2f} to {f:.2f}'))

    # ---------------- controller additions
    def check(self):
        t, c = self.t, self.cfg
        # cerebellar trials: long and adaptive, handled before the generic trial logic
        held = {}
        for key in list(self.trials):
            if not isinstance(self.trials[key], CorrectEdge):
                continue
            e = self.trials.pop(key)
            t0 = self.trial_t0[key]
            age = t - t0
            if age >= c['burn'] + c['evalw'] and (age - c['burn']) % c['evalw'] == 0:
                b, cc = self.stats.get(key, [0, 0])
                z = (b - cc) / np.sqrt(b + cc + 1)
                d = (b - cc) / c['evalw']
                self.cb_hist.setdefault(key, []).append(d)
                passed = self.cb_pass.get(key, 0) + 1 if (z >= 2.5 and d >= 0.01) else 0
                self.cb_pass[key] = passed
                if passed >= 2:          # many looks over a long trial: require two windows in a row
                    self.edges[key[0]][key[1]] = e
                    self.kept_since[key] = t
                    self.stats[key] = [0, 0]
                    if c['weights']:
                        self.wt[key] = int(min(4, 1 + round(d / 0.05)))
                    self.events.append((t, 'keep', key[0], key[1], round(d, 4)))
                    continue
                if age >= c['cb_max_age']:
                    self.events.append((t, 'reject', key[0], key[1], round(d, 4)))
                    continue
                self.stats[key] = [0, 0]
            elif age == c['burn']:
                self.stats[key] = [0, 0]
            held[key] = e
        # broadcast: halve burn-in for trials into alerted targets
        if c['broadcast']:
            for key in self.trials:
                if self.alert.get(key[0], -1) > t and not isinstance(self.trials[key], CorrectEdge):
                    if t - self.trial_t0[key] < c['burn'] // 2:
                        self.trial_t0[key] -= c['burn'] // 2
        n_before = len(self.events)
        super().check()
        self.trials.update(held)
        if t % 10000 == 9999:
            self._gc()
        # Select: interleaved trial of a new context input
        if self.action and c['select']:
            self._select_check()
        # dry-screen tracking and cerebellum growth
        if t % 5000 == 4999 and c['cerebellum']:
            new = self.events[n_before:]
            for dst in self.BASE:
                if (self.action and dst in ('Act', 'R')) or np.mean(self.acc[dst][-2000:]) >= 0.6:
                    self.dry[dst] = 0
                    continue
                kept_any = any(s_ != dst for s_ in self.edges[dst])
                best = max([v for (d, s_), v in self.scr.items() if d == dst and s_ not in self.edges[dst]] or [0])
                built = any(e[1] in ('bind', 'delay', 'hold') and dst in str(e[3]) for e in new)
                self.dry[dst] = 0 if (kept_any or best >= 0.05 or built) else self.dry[dst] + 1
            # readouts are cheap next to the shared granule layer: every unexplained target gets one
            ready = [d for d in self.BASE if self.dry[d] >= 3 and f'cb[{d}]' not in self.cb_names]
            if ready:
                if self.gran is None:
                    srcs = [self.nodes[n] for n in self.BASE if not (self.action and n in ('Act', 'R'))]
                    self.gran = Granules(srcs, self.rng, k=c['k_sub'])
                    self.events.append((t, 'granules', f'granule layer: {self.gran.S} subsets x {self.gran.ncell} cells', [s.name for s in srcs]))
                for d in ready:
                    nm = f'cb[{d}]'
                    self.cb_names.add(nm)
                    self.trials[(d, nm)] = CorrectEdge(self.nodes[d], self.gran, self.rng)
                    self.trial_t0[(d, nm)] = t
                    self.stats[(d, nm)] = [0, 0]
                    self.events.append((t, 'trial', d, nm))

    def win(self, dst, W):
        a = self.alert_start.get(dst)
        if self.cfg['broadcast'] and a is not None and self.t - a < W:
            return max(1000, self.t - a)
        return W

    def _gc(self):
        """Sleep-style cleanup: drop derived nodes nothing uses, so the structure budget is recycled."""
        used = set()
        for dst in self.BASE:
            used.update(s_ for s_ in self.edges[dst] if s_ != dst)
        used.update(s_ for (_, s_) in self.trials)
        if self.action:
            used.update(self.sel_ctx)
            if self.sel_trial:
                used.add(self.sel_trial['src'])
        changed = True
        while changed:
            changed = False
            for nd in self.derived:
                if nd.name in used and nd.parts:
                    for p_ in nd.parts:
                        if p_ in self.nodes and p_ not in used:
                            used.add(p_)
                            changed = True
        doomed = set()
        for nd in self.derived:
            born = next((e[0] for e in self.events if len(e) > 2 and e[2] == nd.name and e[1] == nd.kind), 0)
            tested = any(e[1] == 'trial' and e[3] == nd.name for e in self.events)
            if nd.name not in used and self.t - born > 15000 and tested:
                doomed.add(nd.name)
        changed = True
        while changed:                     # never remove a part of a node that survives
            changed = False
            for nd in self.derived:
                if nd.name not in doomed and nd.parts:
                    for p_ in nd.parts:
                        if p_ in doomed:
                            doomed.discard(p_)
                            changed = True
        for nd in list(self.derived):
            if nd.name in doomed:
                self.derived.remove(nd)
                del self.nodes[nd.name]
                self.priority = {k for k in self.priority if k[1] != nd.name}
                for key in [k for k in self.backoff if k[1] == nd.name]:
                    del self.backoff[key]
                for key in [k for k in self.scr if k[1] == nd.name]:
                    del self.scr[key]
                if nd.kind == 'distill':
                    self.zmade.discard(nd.name[2:-1])
                self.events.append((self.t, 'gc', nd.name, nd.kind))

    def _select_check(self):
        t, c, rng = self.t, self.cfg, self.rng
        tr = self.sel_trial
        if tr is not None:
            if t - tr['t0'] >= c['burn'] + 4000:
                on, off = np.array(tr['r'][0]), np.array(tr['r'][1])
                d = on.mean() - off.mean()
                se = np.sqrt(on.var() / len(on) + off.var() / len(off) + 1e-9)
                if d / se >= 2.5 and d >= 0.02:
                    self.sel_ctx.append(tr['src'])
                    self.events.append((t, 'keep', 'Select', tr['src'], round(float(d), 4)))
                else:
                    for k_ in ('go', 'nogo', 'V'):
                        getattr(self, k_).pop(tr['src'], None)
                    bo = self.sel_backoff.get(tr['src'], 10000)
                    self.sel_rejected[tr['src']] = t + bo
                    self.sel_backoff[tr['src']] = bo * 2
                    self.events.append((t, 'reject', 'Select', tr['src'], round(float(d), 4)))
                self.sel_trial = None
            return
        if t % 5000 != 4999 or t < 5000:
            return
        W = min(4000, len(self.reward) - 2)
        R = np.array(self.reward[-W:])
        A = np.array(self.nodes['Act'].hist[-W:])
        best = None
        for s_ in self.nodes:
            if s_ in ('Act', 'R') or s_ in self.sel_ctx or self.sel_rejected.get(s_, -1) > t:
                continue
            S = np.array(self.nodes[s_].hist[-W - 1:-1])
            if len(S) < W:
                continue
            n = self.nodes[s_].n
            sh = int(rng.integers(100, W - 100))
            j = mi(S * K + A, R, n * K, 2) - mi(np.roll(S, sh) * K + A, R, n * K, 2)
            if j > 0.02 and (best is None or j > best[0]):
                best = (j, s_)
        if best:
            self._init_ctx(best[1], self.nodes[best[1]].n)
            self.sel_trial = dict(src=best[1], t0=t, r=([], []), mi=float(best[0]))
            self.events.append((t, 'trial', 'Select', best[1]))

    def add_node(self, nd, why):
        if not hasattr(self, 'ever'):
            self.ever = []
        self.ever.append(dict(name=nd.name, kind=nd.kind, parts=list(nd.parts) if nd.parts else [], extra=nd.extra, born=self.t))
        super().add_node(nd, why)

    # ---------------- reporting
    def run_block(self, n=1000):
        if self.action:
            start = len(self.reward)
            for _ in range(n):
                self.tick()
            return float(np.mean(self.reward[start:]))
        return super().run_block(n)

    def progress(self, x, last=1000):
        if self.action and x == 'R':
            a = np.mean(self.reward[-last:])
            return (a - 0.2) / (0.86 - 0.2)
        a = np.mean(self.acc[x][-last:])
        return (a - 0.125) / (CEIL[x] - 0.125)

    def opportunity(self):
        o = super().opportunity()
        # a long cerebellar trial whose shadow gain is rising is still on the table
        for key, hist in self.cb_hist.items():
            if key in self.trials and len(hist) >= 2 and hist[-1] >= 0.01 and hist[-1] > hist[-2]:
                o = max(o, 0.2)
        if self.action and self.sel_trial is not None and self.sel_trial.get('mi', 0) > 0.05:
            o = max(o, 0.2)
        return o

    def pending_fallback(self):
        """True while a brute-force move is on the way or still on probation, so the station shouldn't be given up yet."""
        if not self.cfg['cerebellum']:
            return False
        if any(0 < v < 3 for d, v in self.dry.items() if not (self.action and d in ('Act', 'R'))):
            return True                      # dry streak building toward a granule layer
        for key, e in self.trials.items():
            if isinstance(e, CorrectEdge) and self.t - self.trial_t0[key] < 20000:
                return True                  # cerebellar trial still on probation
        return False

    def summary(self):
        s = super().summary()
        s['ever'] = getattr(self, 'ever', [])
        s['reward'] = [float(np.mean(self.reward[i:i + 1000])) for i in range(0, len(self.reward), 1000)] if self.action else None
        s['select_ctx'] = list(self.sel_ctx) if self.action else None
        s['granules'] = None if self.gran is None else dict(subsets=int(self.gran.S), cells=int(self.gran.S * self.gran.ncell),
                                                             srcs=[x.name for x in self.gran.srcs])
        return s


def run_full(name, seed, ticks, **kw):
    s = FullStation(name, seed, **kw)
    for _ in range(ticks // 1000):
        s.run_block()
    return s.summary()


# ------------------------------------------------------------------ attention loop with a give-up rule
def slope_progress(v, m=10):
    """Expected gain over the next 5 visits from a least-squares slope over the last m visits, if significant."""
    y = np.array(v[-m:], float)
    n = len(y)
    if n < 4:
        return 0.0
    x = np.arange(n) - (n - 1) / 2
    slope = (x * (y - y.mean())).sum() / (x * x).sum()
    resid = y - y.mean() - slope * x
    se = np.sqrt(max((resid ** 2).sum() / (n - 2), 0.007 ** 2) / (x * x).sum())
    return max(0.0, slope - 2 * se) * 5


def run_budget(seed, policy, blocks=200, giveup=False, wake_at=None, fallback_aware=False, slope_lp=False,
               names=('easy', 'staged', 'temporal', 'action', 'hard', 'noise', 'bloomer'), patience=6):
    rng = np.random.default_rng(seed + 4242)
    st = [FullStation(n, seed * 10 + i) for i, n in enumerate(names)]
    m = len(st)
    visits = [[] for _ in st]
    struct = [[] for _ in st]
    quiet = [0] * m            # consecutive visits with no sign of life
    dormant_until = [-1] * m
    dormant_back = [10] * m
    order, dormancy = [], []
    for b in range(blocks):
        if wake_at is not None and b == wake_at:
            for s in st:
                if s.name == 'bloomer':
                    s.world.awake = True
        if policy == 'round-robin' or b < 2 * m:
            i = b % m
        else:
            score = []
            for j, v in enumerate(visits):
                if giveup and dormant_until[j] > b:
                    score.append(-1.0)
                    continue
                n = len(v)
                if n < 4:
                    score.append(10.0)
                    continue
                if slope_lp:
                    lp = slope_progress(v)
                else:
                    k = min(4, n // 2)
                    a, b_ = np.array(v[-k:]), np.array(v[-2 * k:-k])
                    se = np.sqrt(a.var() / k + b_.var() / k + 2 * 0.007 ** 2 / k)
                    lp = max(0.0, abs(a.mean() - b_.mean()) - 2 * se)
                build = 0.02 * sum(struct[j][-4:])
                score.append(lp + build + 0.05 * st[j].opportunity() + 0.002 * np.sqrt(np.log(b + 1) / n))
            i = int(np.argmax(score))
        n_ev = len(st[i].events)
        a = st[i].run_block()
        ev = sum(1 for e in st[i].events[n_ev:] if e[1] in ('keep', 'bind', 'distill', 'delay', 'hold', 'granules'))
        struct[i].append(ev)
        visits[i].append(a)
        order.append(i)
        if giveup and b >= 2 * m:
            v = visits[i]
            k = min(4, len(v) // 2)
            alive = ev > 0 or st[i].opportunity() > 0 or (fallback_aware and st[i].pending_fallback())
            if k >= 2:
                aa, bb = np.array(v[-k:]), np.array(v[-2 * k:-k])
                se = np.sqrt(aa.var() / k + bb.var() / k + 2 * 0.007 ** 2 / k)
                alive = alive or (aa.mean() - bb.mean()) > 2 * se or (slope_lp and slope_progress(v) > 0)
            quiet[i] = 0 if alive else quiet[i] + 1
            if alive:
                dormant_back[i] = 10
            if quiet[i] >= patience:
                dormant_until[i] = b + dormant_back[i]
                dormancy.append((b, st[i].name, dormant_back[i]))
                dormant_back[i] = min(80, dormant_back[i] * 2)
                quiet[i] = patience - 1          # one quiet recheck visit sends it back to sleep
    final = {}
    for s in st:
        s.run_block()
        final[s.name] = {x: float(s.progress(x, 1000)) for x in s.TARGETS}
    return dict(order=order, names=list(names), final=final, dormancy=dormancy,
                stations={s.name: s.summary() for s in st})
