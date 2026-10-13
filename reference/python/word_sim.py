"""Word experiment: chunking by surprise.

Adds to TextBrain:
  surprise   a Broadcast-style statistic: running mean and sd of the byte's bits (EMA, 2000 ticks)
  boundary   event B_t = bits_t > mu_t + z * sd_t   (a surprising byte starts a new chunk)
  chunk      recurrent bind: chunk_t = bind(chunk_{t-1}, byte_t), reset to bind(START, byte_t) on B_t.
             REAL: matcher pool (cap 4096, LRU, 48-bit masks over 64-bit input); SYMBOLIC: exact trie.
             Its state is "the bytes since the last boundary" (word-so-far, if boundaries fall on words).
  prevword   hold: the completed chunk latched at each boundary (the previous word, if aligned)
Both are offered as grammar moves (virtual screens over stored history; z chosen by the screen),
then go through the same shadow trials as everything else.
"""
import math
import numpy as np
import text_sim as ts
from text_sim import TextBrain, Matcher, Node, cmi, D, CK


def heldout_gain(s, yp, yn):
    """Bits per tick that context (s, y_t) adds over y_t alone for predicting y_{t+1}, measured on held-out
    data: counts from the first half of the window, Witten-Bell interpolation, log-loss on the second half.
    Unlike the shift-null MI screen, this stays honest for contexts with thousands of states."""
    n = len(s)
    h = n // 2
    V = int(yn.max()) + 1
    p0 = np.bincount(yn[:h], minlength=V) + 1.0
    p0 /= p0.sum()
    cb, cs = {}, {}
    for a, c, y in zip(yp[:h].tolist(), s[:h].tolist(), yn[:h].tolist()):
        d = cb.setdefault(a, {})
        d[y] = d.get(y, 0) + 1
        d = cs.setdefault((c, a), {})
        d[y] = d.get(y, 0) + 1
    tot = 0.0
    for a, c, y in zip(yp[h:].tolist(), s[h:].tolist(), yn[h:].tolist()):
        d = cb.get(a)
        pb = p0[y]
        if d:
            nb = sum(d.values())
            pb = (d.get(y, 0) + len(d) * pb) / (nb + len(d))
        d = cs.get((c, a))
        ps = pb
        if d:
            ns = sum(d.values())
            ps = (d.get(y, 0) + len(d) * pb) / (ns + len(d))
        tot += math.log2(ps) - math.log2(pb)
    return tot / (n - h)


class ChunkReal(Matcher):
    kind = 'chunk'

    def __init__(self, brain, z, T, t, cap=4096, replay=20000):
        super().__init__(f'chunk(z={z})', cap, brain.rng, T, K=48, th_new=40)
        self.brain, self.z, self.parts, self.extra, self.born = brain, z, ('byte', 'surprise'), z, t
        self.start = brain.rng.choice(D, CK, replace=False)
        self.cache, self.code_key = {}, {}
        prev = -1
        for tt in range(max(0, t - replay + 1), t + 1):
            prev = self._step(prev, int(brain.byte.H[tt]), brain.boundary(tt, z), tt)
        self.state = prev

    def _step(self, prev, b, reset, t):
        if b < 0:
            s = -1
        else:
            if reset or prev < 0:
                key = (-1, 0, b, int(self.brain.byte.gen[b]))
                pc = self.start
            else:
                key = (prev, int(self.gen[prev]), b, int(self.brain.byte.gen[b]))
                pc = self.codes[prev]
            c = self.cache.get(key)
            if c is not None and self.alive[c] and self.code_key.get(c) == key:
                self.last[c] = t
                s = c
            else:
                idx = np.unique(np.concatenate([(pc + 333) % D, self.brain.byte.codes[b]]))
                r0 = self.recruits
                s = self.match(idx, t)
                if self.recruits > r0 or self.masks[s, idx].sum() >= self.K:
                    self.cache[key] = s
                    self.code_key[s] = key
        self.H[t] = s
        return s

    def update(self, t):
        self.state = self._step(self.state, self.brain.byte.state, self.brain.boundary(t, self.z), t)


class ChunkSym(Node):
    kind = 'chunk'

    def __init__(self, brain, z, T, t):
        super().__init__(f'chunk(z={z})', 1 << 17, brain.rng, T)
        self.brain, self.z, self.parts, self.extra, self.born = brain, z, ('byte', 'surprise'), z, t
        self.map = {}
        self.codes = None
        self.codebook = {}
        prev = -1
        for tt in range(max(0, t - 40000 + 1), t + 1):
            prev = self._step(prev, int(brain.byte.H[tt]), brain.boundary(tt, z), tt)
        self.state = prev
        self.recruits, self.evictions = len(self.map), 0

    def code(self, c):
        cd = self.codebook.get(c)
        if cd is None:
            cd = self.codebook[c] = self.rng.choice(D, CK, replace=False)
        return cd

    def _step(self, prev, b, reset, t):
        if b < 0:
            s = -1
        else:
            k = (-1 if (reset or prev < 0) else prev, b)
            s = self.map.get(k)
            if s is None:
                s = self.map[k] = len(self.map)
                self.gen[s] = 1
                self.alive[s] = True
        self.H[t] = s
        return s

    def update(self, t):
        self.state = self._step(self.state, self.brain.byte.state, self.brain.boundary(t, self.z), t)


class PrevWord:
    """hold(chunk one tick back | boundary): the chunk completed at the last boundary."""
    kind = 'hold'

    def __init__(self, brain, chunk, t):
        self.name = f'prevword[{chunk.name}]'
        self.brain, self.chunk, self.parts, self.extra, self.born = brain, chunk, (chunk.name, 'surprise'), chunk.z, t
        self.cap, self.gen, self.alive = chunk.cap, chunk.gen, chunk.alive
        self.codes = chunk.codes
        if getattr(chunk, 'codes', None) is None:
            self.code = chunk.code
        self.H = np.full(len(chunk.H), -1, np.int32)
        st = -1
        for tt in range(max(1, t - 40000 + 1), t + 1):
            if brain.boundary(tt, chunk.z):
                st = int(chunk.H[tt - 1])
            self.H[tt] = st
        self.state = st

    def update(self, t):
        if self.brain.boundary(t, self.chunk.z) and t >= 1:
            self.state = int(self.chunk.H[t - 1])
        self.H[t] = self.state


class WordBrain(TextBrain):
    def __init__(self, data, words=True, prevword=True, zs=(0.5, 1.0, 1.5), **kw):
        super().__init__(data, **kw)
        self.words, self.allow_prev, self.zs = words, prevword, zs
        self.mu = np.zeros(self.T)
        self.sd = np.ones(self.T)
        self._m, self._v = 4.0, 1.0
        self.chunk = None
        self.prev = None

    def boundary(self, t, z):
        return self.bits[t] > self.mu[t] + z * self.sd[t]

    def tick(self):
        t = self.t
        self.mu[t], self.sd[t] = self._m, math.sqrt(self._v)
        super().tick()
        b = self.bits[t]
        a = 1 / 2000
        d = b - self._m
        self._m += a * d
        self._v = (1 - a) * self._v + a * d * d

    def _virtual_chunk(self, z, W):
        t = self.t
        lo = t - W + 1
        ids, m, prev = np.full(W, -1, np.int64), {}, -1
        bh = self.byte.H
        for i, tt in enumerate(range(lo, t + 1)):
            b = int(bh[tt])
            if b < 0:
                prev = -1
                continue
            k = (-1 if (self.boundary(tt, z) or prev < 0) else prev, b)
            prev = m.setdefault(k, len(m))
            ids[i] = prev
        return ids

    def temporal(self, W):
        super().temporal(W)
        if not self.words or len(self.derived) >= self.cfg['max_derived']:
            return
        rng, t = self.rng, self.t
        y = self.series(self.byte, W)
        yp, yn = y[:-1], y[1:]
        if self.chunk is None:
            best = None
            for z in self.zs:
                s = self._virtual_chunk(z, W)[:-1] + 1
                v = heldout_gain(s, yp, yn)
                self.events.append((t, 'screen-chunk', f'z={z}', round(v, 3)))
                if v > self.cfg['tmp_thr'] and (best is None or v > best[0]):
                    best = (v, z)
            if best:
                nd = (ChunkReal(self, best[1], self.T, t) if self.real else ChunkSym(self, best[1], self.T, t))
                self.chunk = nd
                self._add(nd, f'screen byte {best[0]:.2f}')
        elif self.prev is None and self.allow_prev:
            c = self.chunk
            ch = c.H[t - W + 1:t + 1]
            bnd = np.array([self.boundary(tt, c.z) for tt in range(t - W + 1, t + 1)])
            idx = np.where(bnd, np.arange(W), -1)
            idx = np.maximum.accumulate(idx)
            prevc = np.where(idx >= 1, ch[np.maximum(idx - 1, 0)], -1)
            s = prevc[:-1].astype(np.int64) + 1
            v = heldout_gain(s, yp, yn)
            self.events.append((t, 'screen-prevword', c.name, round(v, 3)))
            if v > self.cfg['tmp_thr']:
                nd = PrevWord(self, c, t)
                self.prev = nd
                self._add(nd, f'screen byte {v:.2f}')

    def gc(self):
        super().gc()
        if self.chunk is not None and self.chunk.name not in self.nodes:
            self.chunk = None
        if self.prev is not None and self.prev.name not in self.nodes:
            self.prev = None

    def word_report(self, lo, hi):
        """Boundary alignment with real word starts, and how many completed chunks are one whole word."""
        if self.chunk is None:
            return None
        z = self.chunk.z
        data = self.data
        isl = np.array([chr(c).isalpha() or chr(c) == "'" for c in data[lo - 1:hi]])
        bnd = np.array([self.boundary(t, z) for t in range(lo, hi)])
        word_start = isl[1:] & ~isl[:-1]
        prec = float((word_start & bnd).sum() / max(1, bnd.sum()))
        rec = float((word_start & bnd).sum() / max(1, word_start.sum()))
        starts = np.flatnonzero(bnd) + lo
        whole = 0
        segs = 0
        examples = []
        for a, b in zip(starts[:-1], starts[1:]):
            seg = data[a:b].decode('latin-1')
            segs += 1
            core = seg.rstrip(' \n,.;:!?"-')
            starts_word = a == 0 or not (chr(data[a - 1]).isalpha() or chr(data[a - 1]) == "'")
            ends_word = len(seg) > len(core) or b >= len(data) or not chr(data[b]).isalpha()
            if core.isalpha() and len(core) >= 2 and starts_word and ends_word:
                whole += 1
            if len(examples) < 40:
                examples.append(seg)
        return dict(z=z, boundaries=int(bnd.sum()), word_starts=int(word_start.sum()), precision=prec, recall=rec,
                    segments=segs, whole_word_segments=whole / max(1, segs), examples=examples,
                    mean_seg_len=float(np.mean(np.diff(starts))) if len(starts) > 1 else None)
