"""The grown grammar on real text, with real matcher pools.

One target: the byte stream.  The byte node is a matcher pool over fixed random 32-of-2048 byte codes
(optionally corrupted by bit dropout and spurious bits).  Every derived node is either
  delay   view of the byte node d ticks back
  bind    REAL: a matcher pool (cap 1024, LRU eviction) over the shift-bind (X code OR shifted Y code)
          of its parts' winner codes; SYMBOLIC: an exact, unlimited pair table
  hold    X latched whenever the byte node shows a frequent-but-not-common value v (event gate)
  distill the next-byte guess of a kept bind edge, as an 8..160-state variable
Prime edges are writer masks (160 bits per source cell over the byte node's code space), as before.
The readout is a log-linear mix: p(cell) ~ exp(sum_e beta_e * score_e / 32); each beta is learned online.
Trials are shadow trials scored by paired log-loss (bits) instead of top-1 flips.
Every bit-per-byte figure is prequential: each byte is scored before it is learned from.
"""
import math
import numpy as np

D, CK = 2048, 32
LN2 = math.log(2)


def ent(*cols):
    key = cols[0].astype(np.int64)
    for c in cols[1:]:
        key = key * (int(c.max()) + 1) + c
    _, cnt = np.unique(key, return_counts=True)
    p = cnt / cnt.sum()
    return float(-(p * np.log2(p)).sum())


def mi(a, b):
    return ent(a) + ent(b) - ent(a, b)


def cmi(s, yp, yn):
    return ent(s, yp) + ent(yp, yn) - ent(yp) - ent(s, yp, yn)


# ------------------------------------------------------------------ nodes
class Node:
    kind = 'base'

    def __init__(self, name, cap, rng, T):
        self.name, self.cap, self.rng = name, cap, rng
        self.codes = np.zeros((cap, CK), np.int32)
        self.gen = np.zeros(cap, np.int32)
        self.alive = np.zeros(cap, bool)
        self.H = np.full(T, -1, np.int32)
        self.state = -1
        self.parts = ()
        self.born = 0
        self.extra = None

    def new_code(self, c):
        self.codes[c] = self.rng.choice(D, CK, replace=False)
        self.gen[c] += 1
        self.alive[c] = True


class Matcher(Node):
    """k-WTA (k = 1) matcher pool: recruit on novelty, LRU eviction at capacity, 1-bit Hebbian swaps."""

    HEBB = 1.0      # probability of a 1-bit Hebbian swap on a partial match

    def __init__(self, name, cap, rng, T, K, th_new):
        super().__init__(name, cap, rng, T)
        self.masks = np.zeros((cap, D), bool)
        self.K, self.th_new = K, th_new
        self.hebb = Matcher.HEBB
        self.last = np.zeros(cap, np.int64)
        self.born_t = np.zeros(cap, np.int64)
        self.recruits = self.evictions = 0

    def match(self, idx, t):
        ov = self.masks[:, idx].sum(1)
        ov[~self.alive] = -1
        b = int(np.argmax(ov))
        best = ov[b]
        if best < self.th_new:
            free = np.flatnonzero(~self.alive)
            if len(free):
                b = int(free[0])
            else:
                b = int(np.argmin(self.last))
                self.evictions += 1
            m = self.masks[b]
            m[:] = False
            m[self.rng.choice(idx, min(self.K, len(idx)), replace=False)] = True
            self.new_code(b)
            self.born_t[b] = t
            self.recruits += 1
        elif best < self.K and (self.hebb >= 1.0 or self.rng.random() < self.hebb):
            m = self.masks[b]
            inp = np.zeros(D, bool)
            inp[idx] = True
            add = np.flatnonzero(inp & ~m)
            rem = np.flatnonzero(m & ~inp)
            if len(add) and len(rem):
                m[rem[self.rng.integers(len(rem))]] = False
                m[add[self.rng.integers(len(add))]] = True
        self.last[b] = t
        return b


def consolidate(pool, thr):
    """Sleep-style merge: cells whose masks overlap by at least thr bits recognise the same thing;
    keep the older cell, retire the younger.  Returns the number of cells retired."""
    live = np.flatnonzero(pool.alive)
    if len(live) < 2:
        return 0
    M = pool.masks[live].astype(np.int32)
    ov = M @ M.T
    np.fill_diagonal(ov, 0)
    born = pool.born_t[live]
    gone = 0
    for i in range(len(live)):
        if not pool.alive[live[i]]:
            continue
        for j in np.flatnonzero(ov[i] >= thr):
            if pool.alive[live[j]] and pool.alive[live[i]]:
                young = live[j] if born[j] >= born[i] else live[i]
                pool.alive[young] = False
                pool.masks[young] = False
                gone += 1
    return gone


class ByteReal(Matcher):
    def __init__(self, rng, T, noise=(0.0, 0)):
        super().__init__('byte', 192, rng, T, K=24, th_new=10)   # 16 failed under noise: see results
        self.bytecode = np.array([rng.choice(D, CK, replace=False) for _ in range(256)])
        self.drop, self.spur = noise

    def observe(self, byte, t):
        idx = self.bytecode[byte]
        if self.drop > 0:
            idx = idx[self.rng.random(CK) >= self.drop]
        if self.spur > 0:
            idx = np.unique(np.concatenate([idx, self.rng.integers(0, D, self.spur)]))
        self.state = self.match(idx, t)
        self.H[t] = self.state
        return self.state


class ByteSym(Node):
    def __init__(self, rng, T, noise=None):
        super().__init__('byte', 256, rng, T)
        self.map = {}
        self.recruits = self.evictions = 0

    def observe(self, byte, t):
        c = self.map.get(byte)
        if c is None:
            c = self.map[byte] = len(self.map)
            self.new_code(c)
            self.recruits += 1
        self.state = c
        self.H[t] = c
        return c


class Delay:
    kind = 'delay'

    def __init__(self, byte, d, t):
        self.name, self.src, self.d, self.parts, self.extra, self.born = f'byte@{d}', byte, d, (byte.name,), d, t
        self.cap, self.codes, self.gen, self.alive = byte.cap, byte.codes, byte.gen, byte.alive
        self.H = np.full(len(byte.H), -1, np.int32)
        if t + 2 - d > 0:
            self.H[d - 1:t + 1] = byte.H[:t + 2 - d]
        self.state = int(self.H[t])

    def update(self, t):
        self.state = int(self.src.H[t - self.d + 1]) if t - self.d + 1 >= 0 else -1
        self.H[t] = self.state


class Hold:
    kind = 'hold'

    def __init__(self, X, Y, v, t, vlabel):
        self.name = f'hold({X.name}|{vlabel})'
        self.X, self.Y, self.v, self.parts, self.extra, self.born = X, Y, v, (X.name, Y.name), vlabel, t
        self.cap, self.codes, self.gen, self.alive = X.cap, X.codes, X.gen, X.alive
        self.H = np.full(len(X.H), -1, np.int32)
        x, y = X.H[:t + 1], Y.H[:t + 1]
        idx = np.where(y == v, np.arange(t + 1), -1)
        idx = np.maximum.accumulate(idx)
        self.H[:t + 1] = np.where(idx >= 0, x[np.maximum(idx, 0)], -1)
        self.state = int(self.H[t])

    def update(self, t):
        if self.Y.state == self.v:
            self.state = self.X.state
        self.H[t] = self.state


class BindReal(Matcher):
    kind = 'bind'

    def __init__(self, X, Y, rng, T, t, replay=20000, cap=1024):
        super().__init__(f'({X.name}*{Y.name})', cap, rng, T, K=48, th_new=40)
        self.X, self.Y, self.parts, self.born = X, Y, (X.name, Y.name), t
        self.cache, self.code_key, self.idx_cache = {}, {}, {}
        for tt in range(max(0, t - replay + 1), t + 1):      # build the pool from recent memory
            self._step(int(X.H[tt]), int(Y.H[tt]), tt)
        self.state = int(self.H[t])

    def _step(self, sx, sy, t):
        if sx < 0 or sy < 0:
            s = -1
        else:
            key = (sx, sy, int(self.X.gen[sx]), int(self.Y.gen[sy]))
            c = self.cache.get(key)
            idx = None
            if c is not None and self.alive[c]:
                idx = self.idx_cache[c]
                if self.code_key[c] == key:          # cell still holds this exact pair: full match, as match() would find
                    self.last[c] = t
                    s = c
                    self.H[t] = s
                    return s
            idx = np.unique(np.concatenate([self.X.codes[sx], (self.Y.codes[sy] + 977) % D]))
            r0 = self.recruits
            s = self.match(idx, t)
            if self.recruits > r0 or self.masks[s, idx].sum() >= self.K:
                self.cache[key] = s
                self.code_key[s] = key
                self.idx_cache[s] = idx
        self.H[t] = s
        return s

    def update(self, t):
        self.state = self._step(self.X.state, self.Y.state, t)


class BindSym(Node):
    kind = 'bind'

    def __init__(self, X, Y, rng, T, t, cap=65536):
        super().__init__(f'({X.name}*{Y.name})', cap, rng, T)
        self.X, self.Y, self.parts, self.born = X, Y, (X.name, Y.name), t
        self.map = {}
        self.codes = None            # codes made lazily
        self.codebook = {}
        xs, ys = X.H[:t + 1], Y.H[:t + 1]
        for tt in range(max(0, t - 40000 + 1), t + 1):
            self.H[tt] = self._id(int(xs[tt]), int(ys[tt]))
        self.state = int(self.H[t])
        self.recruits = len(self.map)
        self.evictions = 0

    def _id(self, a, b):
        if a < 0 or b < 0:
            return -1
        k = (a, b)
        c = self.map.get(k)
        if c is None:
            c = self.map[k] = len(self.map)
            self.gen[c] = 1
            self.alive[c] = True
        return c

    def code(self, c):
        cd = self.codebook.get(c)
        if cd is None:
            cd = self.codebook[c] = self.rng.choice(D, CK, replace=False)
        return cd

    def update(self, t):
        self.state = self._id(self.X.state, self.Y.state)
        self.H[t] = self.state


class CodeView:
    """Lets bind code lookups work the same for real nodes (codes array) and symbolic binds (lazy codebook)."""
    @staticmethod
    def get(node, c):
        if getattr(node, 'codes', None) is None:
            return node.code(c)
        return node.codes[c]


class Distill:
    """Compress: group a kept edge's source cells by what they predict (k-means on their predicted
    next-byte profiles).  State = cluster id; contexts that predict alike share a state."""
    kind = 'distill'

    def __init__(self, edge, target, rng, t, k=32):
        self.name = f'Z[{edge.src.name}]'
        self.edge, self.target, self.parts, self.extra, self.born = edge, target, (edge.src.name,), k, t
        self.k, self.rng = k, rng
        self.cap = k
        self.codes = np.array([rng.choice(D, CK, replace=False) for _ in range(k)])
        self.gen = np.ones(k, np.int32)
        self.alive = np.ones(k, bool)
        self.H = np.full(len(target.H), -1, np.int32)
        self.C = None
        self.assign = {}
        self.refit()
        src = edge.src.H[:t + 1]
        lo = max(0, t - 40000 + 1)
        for s_ in np.unique(src[lo:]):
            if s_ >= 0:
                self.H[lo:t + 1][src[lo:] == s_] = self.cluster(int(s_))
        self.state = int(self.H[t])

    def profile(self, c):
        r = self.edge.rows.get(c)
        if r is None or r[0] != self.edge.src.gen[c]:
            return None
        v = self.edge.row(c)[2].astype(float)
        v[~self.target.alive] = 0
        n = np.linalg.norm(v)
        return v / n if n > 0 else None

    def refit(self, iters=8):
        cells = [c for c in self.edge.rows]
        P = [self.profile(c) for c in cells]
        keep = [i for i, v in enumerate(P) if v is not None]
        if len(keep) < self.k:
            self.assign = {cells[i]: i % self.k for i in keep}
            return
        cells = [cells[i] for i in keep]
        P = np.array([P[i] for i in keep])
        C = self.C if self.C is not None and self.C.shape[1] == P.shape[1] else P[self.rng.choice(len(P), self.k, replace=False)]
        for _ in range(iters):
            a = np.argmax(P @ C.T, 1)
            for j in range(self.k):
                m = P[a == j]
                if len(m):
                    v = m.mean(0)
                    C[j] = v / (np.linalg.norm(v) + 1e-12)
        self.C = C
        self.assign = dict(zip(cells, np.argmax(P @ C.T, 1).tolist()))

    def cluster(self, s_):
        if s_ < 0:
            return -1
        a = self.assign.get(s_)
        if a is None:
            v = self.profile(s_)
            if v is None or self.C is None:
                return -1
            a = int(np.argmax(self.C @ v))
            self.assign[s_] = a
        return a

    def update(self, t):
        self.state = self.cluster(self.edge.src.state)
        self.H[t] = self.state


# ------------------------------------------------------------------ Prime edge with a learned gain
class TargetIndex:
    """Which target cells own each of the 2048 bits; rebuilt when the target pool recruits."""
    def __init__(self, target):
        self.target = target
        self.version = -1
        self.refresh()

    def refresh(self):
        tg = self.target
        owners = [[] for _ in range(D)]
        for c in np.flatnonzero(tg.alive):
            for bit in tg.codes[c]:
                owners[bit].append(c)
        mx = max(1, max(len(o) for o in owners))
        self.OWN = np.full((D, mx), -1, np.int64)
        for bit, o in enumerate(owners):
            self.OWN[bit, :len(o)] = o
        self.version += 1


class Edge:
    """Writer-mask edge.  A stack of L masks per source cell; level k is written with probability 2**-k
    (stochastic binary counters), so a successor's score grows with how often it follows.
    Score of target cell j = (sum over levels of |mask_k AND code_j|) / (32 L), kept as an integer
    count per (source cell, target cell) and updated incrementally as bits are swapped."""
    LEVELS, KW, SWAPS = 8, 640, 8

    def __init__(self, src, rng, tix, Kw=None, beta=0.0, levels=None):
        self.src, self.rng, self.tix = src, rng, tix
        self.Kw = Kw or Edge.KW
        self.L = levels or Edge.LEVELS
        self.prob = 2.0 ** -np.arange(self.L)
        self.rows = {}
        self.beta, self.v = beta, 0.01
        self.cur, self.sc = -1, None
        self.sum = self.sq = 0.0
        self.n = 0

    def row(self, i):
        """[gen, masks[L, D], counts[target cap], version] for source cell i."""
        g = self.src.gen[i]
        r = self.rows.get(i)
        if r is None or r[0] != g:
            m = np.zeros((self.L, D), bool)
            for k in range(self.L):
                m[k, self.rng.choice(D, self.Kw, replace=False)] = True
            r = self.rows[i] = [g, m, None, -1]
        if r[3] != self.tix.version:
            tg = self.tix.target
            r[2] = r[1][:, tg.codes].sum((0, 2)).astype(np.int32)
            r[3] = self.tix.version
        return r

    def score(self, alive_idx):
        self.cur = self.src.state
        if self.cur < 0:
            self.sc = None
            return None
        r = self.row(self.cur)
        self.sc = r[2][alive_idx] * (1.0 / (CK * self.L))
        return self.sc

    def learn(self, ycode, tgt):
        if self.cur < 0:
            return
        r = self.row(self.cur)
        M, cnt, OWN = r[1], r[2], self.tix.OWN
        rng = self.rng
        for k in np.flatnonzero(rng.random(self.L) < self.prob):
            m = M[k]
            add = ycode[~m[ycode]]
            if len(add) == 0:
                continue
            w = min(Edge.SWAPS, len(add))
            aa = add[rng.permutation(len(add))[:w]] if w < len(add) else add
            ra = rng.integers(D, size=4 * w + 8)
            ra = ra[m[ra] & ~tgt[ra]]
            ra = np.unique(ra)[:len(aa)]
            aa = aa[:len(ra)]                      # keep the mask's bit count constant
            m[ra] = False
            m[aa] = True
            dec = OWN[ra].ravel()
            inc = OWN[aa].ravel()
            np.subtract.at(cnt, dec[dec >= 0], 1)
            np.add.at(cnt, inc[inc >= 0], 1)

    def learn_beta(self, p, yi, lr=0.003):
        if self.sc is None or yi < 0:
            return
        g = self.sc[yi] - float(p @ self.sc)
        self.v = 0.999 * self.v + 0.001 * g * g
        self.beta += lr * g / (math.sqrt(self.v) + 1e-4)


def log_softmax_at(logits, yi):
    m = logits.max()
    z = np.exp(logits - m)
    s = z.sum()
    return z / s, (logits[yi] - m - math.log(s)) if yi >= 0 else None


# ------------------------------------------------------------------ the brain
class TextBrain:
    def __init__(self, data, seed=0, real=True, noise=(0.0, 0), grow=True, oracle=0, cfg=None, T=None, merge=0):
        self.data = data
        self.merge, self.merged = merge, 0
        self.rng = rng = np.random.default_rng(seed)
        self.T = T or len(data)
        self.real, self.grow = real, grow
        c = dict(burn=4000, evalw=8000, acc_gain=0.01, acc_z=2.5, prune_gain=0.003, prune_every=10000,
                 screen_thr=0.02, syn_thr=0.08, tmp_thr=0.02, W=30000, max_trials=3, max_derived=24,
                 distill_gain=0.05, eps=0.002, bind_cap=1024)
        c.update(cfg or {})
        self.cfg = c
        self.byte = (ByteReal if real else ByteSym)(rng, self.T, noise)
        self.tix = TargetIndex(self.byte)
        self.nodes = {'byte': self.byte}
        self.derived = []
        self.edges = {'byte': Edge(self.byte, rng, self.tix, beta=1.0)}
        self.trials, self.trial_t0, self.backoff, self.priority, self.scr = {}, {}, {}, set(), {}
        self.kept_since, self.gain_at_keep = {}, {}
        self.events = []
        self.bits = np.zeros(self.T)
        self.hit = np.zeros(self.T, bool)
        self.cellbyte = {}
        self.t = 0
        self.alive_idx = None
        self.frozen = False
        if oracle:
            self._oracle(oracle)

    # -------------- structure helpers
    def _add(self, nd, why):
        self.nodes[nd.name] = nd
        self.derived.append(nd)
        self.events.append((self.t, nd.kind, nd.name, why))
        self.priority.add(nd.name)

    def _bind(self, X, Y, why):
        name = f'({X.name}*{Y.name})'
        if name in self.nodes or f'({Y.name}*{X.name})' in self.nodes:
            return None
        if self.real:
            nd = BindReal(X, Y, self.rng, self.T, self.t, cap=self.cfg['bind_cap'])
        else:
            nd = BindSym(X, Y, self.rng, self.T, self.t)
        self._add(nd, why)
        return nd

    def _oracle(self, order):
        """Hand wiring: exact or real contexts of order 2..order (order 1 is the byte node's own edge)."""
        prev = self.byte
        taps = {}
        for k in range(2, order + 1):
            tp = Delay(self.byte, k, 0)
            self.nodes[tp.name] = tp
            self.derived.append(tp)
            taps[k] = tp
        for k in range(2, order + 1):
            X, Y = prev, taps[k]
            nd = BindReal(X, Y, self.rng, self.T, 0, replay=0, cap=self.cfg['bind_cap']) if self.real else BindSym(X, Y, self.rng, self.T, 0)
            self.nodes[nd.name] = nd
            self.derived.append(nd)
            self.edges[nd.name] = Edge(nd, self.rng, self.tix)
            self.kept_since[nd.name] = 0
            prev = nd

    def codes_of(self, node, s):
        return CodeView.get(node, s)

    # -------------- one tick
    def tick(self):
        t, rng, c = self.t, self.rng, self.cfg
        byte = self.data[t]
        if self.alive_idx is None:
            self._refresh_alive()
        ca = self.alive_idx
        scs = {}
        logits = np.zeros(len(self.alive_idx))
        for k, e in self.edges.items():
            s = e.score(ca)
            if s is not None:
                logits += e.beta * s
                scs[k] = s
        tsc = {}
        for k, e in self.trials.items():
            s = e.score(ca)
            if s is not None:
                tsc[k] = s
        r0 = self.byte.recruits
        y = self.byte.observe(byte, t)
        novel = self.byte.recruits > r0
        yi = -1 if novel else int(self.pos[y])
        if yi >= 0:
            p, lp = log_softmax_at(logits, yi)
            q = (1 - c['eps']) * math.exp(lp) + c['eps'] / 256
            self.hit[t] = int(np.argmax(logits)) == yi
        else:
            p = None
            q = c['eps'] / 256
        cb = self.cellbyte.setdefault(y, {})
        n_c = sum(cb.values())
        pb = (cb.get(byte, 0) + 0.02) / (n_c + 0.02 * 256)
        cb[byte] = cb.get(byte, 0) + 1
        bits = -math.log2(q) - math.log2(pb)
        self.bits[t] = bits
        if yi >= 0 and not self.frozen:
            base_loss = -math.log2(q)
            # trials (shadow): paired log-loss gain
            for k, s in tsc.items():
                e = self.trials[k]
                pw, lpw = log_softmax_at(logits + e.beta * s, yi)
                qw = (1 - c['eps']) * math.exp(lpw) + c['eps'] / 256
                d = base_loss + math.log2(qw)
                if t - self.trial_t0[k] >= c['burn']:
                    e.sum += d
                    e.sq += d * d
                    e.n += 1
                e.learn_beta(pw, yi)
            # kept edges: ablation gain
            for k, s in scs.items():
                if k == 'byte':
                    continue
                e = self.edges[k]
                _, lpo = log_softmax_at(logits - e.beta * s, yi)
                qo = (1 - c['eps']) * math.exp(lpo) + c['eps'] / 256
                d = -math.log2(qo) - base_loss
                e.sum += d
                e.sq += d * d
                e.n += 1
            for e in self.edges.values():
                e.learn_beta(p, yi)
            ycode = self.byte.codes[y]
            tgt = np.zeros(D, bool)
            tgt[ycode] = True
            for e in self.edges.values():
                e.learn(ycode, tgt)
            for e in self.trials.values():
                e.learn(ycode, tgt)
        if novel:
            self.alive_idx = None
        if self.real and self.merge and t % 1000 == 999:
            n = consolidate(self.byte, self.merge)
            if n:
                self.merged += n
                self.alive_idx = None
        for nd in self.derived:
            nd.update(t)
        if not self.frozen and self.grow and t % 1000 == 999:
            self.check()
        self.t += 1

    def _refresh_alive(self):
        self.tix.refresh()
        self.alive_idx = np.flatnonzero(self.byte.alive)
        self.codes_alive = self.byte.codes[self.alive_idx]
        self.pos = np.full(self.byte.cap, -1, np.int64)
        self.pos[self.alive_idx] = np.arange(len(self.alive_idx))

    # -------------- growth controller
    def series(self, nd, W):
        t = self.t
        return nd.H[t - W + 1:t + 1].astype(np.int64) + 1

    def check(self):
        t, c = self.t, self.cfg
        # trial decisions
        for k in list(self.trials):
            e = self.trials[k]
            if t - self.trial_t0[k] >= c['burn'] + c['evalw'] and e.n > 100:
                m = e.sum / e.n
                sd = math.sqrt(max(e.sq / e.n - m * m, 1e-12))
                z = m / (sd / math.sqrt(e.n))
                del self.trials[k]
                if m >= c['acc_gain'] and z >= c['acc_z']:
                    e.sum = e.sq = 0.0
                    e.n = 0
                    self.edges[k] = e
                    self.kept_since[k] = t
                    self.gain_at_keep[k] = m
                    self.events.append((t, 'keep', 'byte', k, round(m, 4)))
                    nd = self.nodes[k]
                    if nd.kind == 'bind' and m >= c['distill_gain'] and f'Z[{k}]' not in self.nodes \
                            and len(self.derived) < c['max_derived']:
                        self._add(Distill(e, self.byte, self.rng, t), k)
                else:
                    bo = self.backoff.get(k, (0, 10000))[1]
                    self.backoff[k] = (t + bo, bo * 2)
                    self.events.append((t, 'reject', 'byte', k, round(m, 4)))
        # prune kept edges
        for k in [k for k in self.edges if k != 'byte']:
            e = self.edges[k]
            if t - self.kept_since.get(k, 0) >= c['prune_every'] and e.n > 100:
                m = e.sum / e.n
                sd = math.sqrt(max(e.sq / e.n - m * m, 1e-12))
                z = m / (sd / math.sqrt(e.n))
                if m < c['prune_gain'] or z < 1.0:
                    del self.edges[k]
                    self.backoff[k] = (t + 10000, 10000)
                    self.events.append((t, 'prune', 'byte', k, round(m, 4)))
                e.sum = e.sq = 0.0
                e.n = 0
                self.kept_since[k] = t
        if t % 10000 == 9999:
            for nd in self.derived:
                if nd.kind == 'distill':
                    nd.refit()
        W = min(t + 1, c['W'])
        if W < 5000:
            return
        if t % 10000 == 9999 and len(self.derived) < c['max_derived']:
            self.temporal(W)
        if t % 10000 == 4999 and len(self.derived) < c['max_derived']:
            self.synergy(W)
        if t % 5000 == 4999:
            self.screen(W)
        if t % 10000 == 9999:
            self.gc()
        self.fill()

    def screen(self, W):
        rng = self.rng
        y = self.series(self.byte, W)
        yp, yn = y[:-1], y[1:]
        for k, nd in self.nodes.items():
            if k == 'byte':
                continue
            s = self.series(nd, W)[:-1]
            sh = int(rng.integers(500, W - 500))
            self.scr[k] = cmi(s, yp, yn) - cmi(np.roll(s, sh), yp, yn)

    def fill(self):
        c, t = self.cfg, self.t
        free = c['max_trials'] - len(self.trials)
        if free <= 0:
            return
        cands = []
        for k, v in self.scr.items():
            if k in self.edges or k in self.trials or k not in self.nodes:
                continue
            bo = self.backoff.get(k)
            if bo and bo[0] > t:
                continue
            if k in self.priority:
                v += 10
            if v > c['screen_thr']:
                cands.append((v, k))
        cands.sort(reverse=True)
        for v, k in cands[:free]:
            self.trials[k] = Edge(self.nodes[k], self.rng, self.tix)
            self.trial_t0[k] = t
            self.priority.discard(k)
            self.events.append((t, 'trial', 'byte', k))

    def temporal(self, W):
        rng = self.rng
        y = self.series(self.byte, W)
        yp, yn = y[:-1], y[1:]
        best = None
        H = self.byte.H
        t = self.t
        W = min(W, t + 1 - 16)
        y = self.series(self.byte, W)
        yp, yn = y[:-1], y[1:]
        for d in (2, 3, 4, 5, 6, 8, 12, 16):
            if f'byte@{d}' in self.nodes:
                continue
            s = H[t - W + 1 - (d - 1):t + 1 - (d - 1)].astype(np.int64) + 1
            s = s[:-1]
            sh = int(rng.integers(500, W - 500))
            v = cmi(s, yp, yn) - cmi(np.roll(s, sh), yp, yn)
            if v > self.cfg['tmp_thr'] and (best is None or v > best[0]):
                best = (v, 'delay', d)
        # event-gated holds: latch byte or a tap whenever the byte node shows a moderately frequent value
        bh = y - 1
        vals, cnt = np.unique(bh[bh >= 0], return_counts=True)
        evs = [int(v) for v, n in zip(vals, cnt) if 0.005 <= n / W <= 0.2]
        xs = [self.byte] + [nd for nd in self.derived if nd.kind == 'delay']
        for X in xs:
            xh = X.H[t - W + 1:t + 1]
            for v in evs:
                lab = self.label(v)
                if f'hold({X.name}|{lab})' in self.nodes:
                    continue
                idx = np.where(bh == v, np.arange(W), -1)
                idx = np.maximum.accumulate(idx)
                hs = (np.where(idx >= 0, xh[np.maximum(idx, 0)], -1).astype(np.int64) + 1)[:-1]
                sh = int(rng.integers(500, W - 500))
                val = cmi(hs, yp, yn) - cmi(np.roll(hs, sh), yp, yn)
                if val > self.cfg['tmp_thr'] and (best is None or val > best[0]):
                    best = (val, 'hold', (X.name, v))
        if best is None:
            return
        if best[1] == 'delay':
            nd = Delay(self.byte, best[2], t)
            self._add(nd, f'screen byte {best[0]:.2f}')
        else:
            X = self.nodes[best[2][0]]
            v = best[2][1]
            nd = Hold(X, self.byte, v, t, self.label(v))
            self._add(nd, f'screen byte {best[0]:.2f}')

    def label(self, cell):
        cb = self.cellbyte.get(cell, {})
        if not cb:
            return f'#{cell}'
        b = max(cb, key=cb.get)
        ch = chr(b)
        return {' ': "' '", '\n': "'\\n'"}.get(ch, repr(ch))

    def synergy(self, W):
        """Build the bind whose pair carries the most information about the next byte beyond its best part."""
        rng, c, t = self.rng, self.cfg, self.t
        y = self.series(self.byte, W)
        yn = y[1:]
        ser = {k: self.series(nd, W)[:-1] for k, nd in self.nodes.items()}
        sh = int(rng.integers(500, W - 500))
        single = {k: mi(s, yn) - mi(np.roll(s, sh), yn) for k, s in ser.items()}
        anchors = ['byte'] + [k for k in self.edges if k != 'byte'] + list(self.trials)
        best = None
        for a in dict.fromkeys(anchors):
            if a not in ser:
                continue
            for b in ser:
                if b == a or f'({a}*{b})' in self.nodes or f'({b}*{a})' in self.nodes:
                    continue
                if self.contains(b, a) or self.contains(a, b):
                    continue
                pair = ser[a] * (int(ser[b].max()) + 1) + ser[b]
                nd_ = len(np.unique(pair))
                if nd_ > W / 4 or (self.real and nd_ > 2 * c['bind_cap']):
                    continue                      # not enough data per context, or far beyond capacity
                j = mi(pair, yn) - mi(np.roll(pair, sh), yn)
                g = j - max(single[a], single[b], 0)
                if g > c['syn_thr'] and (best is None or g > best[0]):
                    best = (g, a, b)
        if best:
            self._bind(self.nodes[best[1]], self.nodes[best[2]], f'synergy byte {best[0]:.2f}')

    def contains(self, name, other):
        """Structural containment through binds and distills (delays and holds are leaves: other times)."""
        if name == other:
            return True
        nd = self.nodes.get(name)
        if nd is None or nd.kind not in ('bind', 'distill'):
            return False
        return any(self.contains(p, other) for p in nd.parts)

    def gc(self):
        t = self.t
        used = set(self.edges) | set(self.trials)
        changed = True
        while changed:
            changed = False
            for nd in self.derived:
                if nd.name in used:
                    for p in nd.parts:
                        if p not in used:
                            used.add(p)
                            changed = True
        tested = {e[3] for e in self.events if e[1] == 'trial'}
        for nd in list(self.derived):
            if nd.name not in used and nd.name in tested and t - nd.born > 15000:
                self.derived.remove(nd)
                del self.nodes[nd.name]
                self.scr.pop(nd.name, None)
                self.backoff.pop(nd.name, None)
                self.events.append((t, 'gc', nd.name, nd.kind))

    # -------------- run & report
    def run(self, n=None, log_every=25000):
        end = self.T if n is None else min(self.T, self.t + n)
        while self.t < end:
            self.tick()

    def summary(self, blocks=1000):
        T = self.t
        bpb = [float(self.bits[i:i + blocks].mean()) for i in range(0, T, blocks)]
        acc = [float(self.hit[i:i + blocks].mean()) for i in range(0, T, blocks)]
        info = dict(recruits=int(getattr(self.byte, 'recruits', len(getattr(self.byte, 'map', {})))),
                    byte_cells=int(self.byte.alive.sum()))
        # byte-pool purity: how many distinct bytes per cell / cells per byte over the last 50k
        lo = max(0, T - 50000)
        cells, bytes_ = self.byte.H[lo:T], np.frombuffer(self.data[lo:T], np.uint8).astype(np.int64)
        hc = ent(cells.astype(np.int64) + 1)
        hb = ent(bytes_)
        hj = ent(cells.astype(np.int64) + 1, bytes_)
        info['nmi_cell_byte'] = (hc + hb - hj) / max(hb, 1e-9)
        info['H_cell_given_byte'] = hj - hb
        nodes = []
        for nd in self.derived:
            r = dict(name=nd.name, kind=nd.kind, parts=list(nd.parts), born=int(nd.born), extra=nd.extra)
            if nd.kind == 'bind':
                r['cells'] = int(nd.alive.sum())
                r['evictions'] = int(getattr(nd, 'evictions', 0))
            nodes.append(r)
        kept = {k: dict(beta=round(e.beta, 3), gain_at_keep=round(self.gain_at_keep.get(k, 0), 4)) for k, e in self.edges.items()}
        return dict(bpb=bpb, acc=acc, events=self.events, nodes=nodes, kept=kept, info=info, ticks=T)
