"""Shared inputs for the cortex-map experiments (26), on full word ids, all prequential (predict, then learn):
  p_base   exact-word probability of the experiment-25 counts system: order-2 Witten-Bell over the 512 commonest
           words + "other", and for "other" words a spelling that mixes unigram counts over words outside the 512
           with a decaying memory of recent ones (half-life 200 such words; weight chosen online from a grid)
  q_story  the experiment-25 story edge in counts: rows keyed by the 16 most salient recent content words (word id
           >= 100, half-life 200 content words), each counting the next word; salience-weighted mean, backing off
           to unigram counts over all words
  u_p      prequential unigram probability of the exact word over all words (escape 1e-5 for unseen words)
usage: cortex_base.py IDS OUT.npz"""
import sys, math, heapq
import numpy as np
from collections import Counter, defaultdict
ids = np.fromfile(sys.argv[1], dtype=np.uint32).astype(np.int64); n = len(ids)
BASE = 512; OTHER = BASE; NV = BASE + 1; SKIP = 100; K = 16; H = 200.0; BETA = 256.0
sym = np.minimum(ids, BASE)
def wb(c, tot, y, base):
    if not tot: return base
    return (c.get(y, 0) + len(c) * base) / (tot + len(c))
uni = np.zeros(NV); un = 0; big = defaultdict(Counter); bt = Counter(); tri = defaultdict(Counter); tt = Counter()
oc = Counter(); on = 0; keep = {}; ktot = 0.0; kg = 1.0
lams = [0.0, 0.05, 0.1, 0.2, 0.3, 0.45, 0.6, 0.75]; lloss = np.zeros(len(lams))
fc = Counter(); fn = 0
sal = {}; g = 1.0; rows = defaultdict(Counter); rt = Counter(); st = []
p_base = np.zeros(n); q_story = np.zeros(n); u_p = np.zeros(n)
for i in range(n):
    y = int(sym[i]); w = int(ids[i])
    p0 = (uni[y] + 1) / (un + NV)
    a = int(sym[i-1]) if i else -1; b = (int(sym[i-2]), a) if i > 1 else None
    p1 = wb(big[a], bt[a], y, p0) if i else p0
    p = wb(tri[b], tt[b], y, p1) if b else p1
    T = len(fc); up = (fc[w] + T * 1e-5) / (fn + T) if fn else 1e-5; u_p[i] = up
    if y == OTHER:
        tq = len(oc); q = (oc[w] + tq * 1e-5) / (on + tq) if on else 1e-5
        qc = keep.get(w, 0.0) / ktot if ktot else 0.0
        best = int(np.argmin(lloss))
        for j, l in enumerate(lams): lloss[j] -= math.log2((1 - l) * q + l * qc)
        p *= (1 - lams[best]) * q + lams[best] * qc
    p_base[i] = p
    if st:
        ws = sum(v for _, v in st)
        q_story[i] = sum(v * (rows[k][w] + BETA * up) / (rt[k] + BETA) for k, v in st) / ws
    else: q_story[i] = up
    # learn
    uni[y] += 1; un += 1
    if i: big[a][y] += 1; bt[a] += 1
    if b: tri[b][y] += 1; tt[b] += 1
    fc[w] += 1; fn += 1
    if y == OTHER:
        oc[w] += 1; on += 1
    if w >= BASE:
        kg *= 2 ** (1 / H); keep[w] = keep.get(w, 0.0) + kg; ktot += kg
        if kg > 1e100:
            for kk in keep: keep[kk] /= 1e100
            ktot /= 1e100; kg /= 1e100
        if i % 1024 == 0:
            keep = {kk: v for kk, v in keep.items() if v >= kg * 1e-4}; ktot = sum(keep.values())
    for k, _ in st: rows[k][w] += 1; rt[k] += 1
    if w >= SKIP:
        g *= 2 ** (1 / H); sal[w] = sal.get(w, 0.0) + g
        if g > 1e100:
            for kk in sal: sal[kk] /= 1e100
            g /= 1e100
        if i % 256 == 0: sal = {kk: v for kk, v in sal.items() if v >= g * 1e-3}
        st = [(kk, v / g) for kk, v in heapq.nlargest(K, sal.items(), key=lambda kv: kv[1])]
    if i % 500000 == 0: print(i, flush=True)
np.savez(sys.argv[2], ids=ids, p_base=p_base, q_story=q_story, u_p=u_p)
h = n // 2
print(f"base exact bits, second half: {-np.log2(p_base[h:]).mean():.3f}")
