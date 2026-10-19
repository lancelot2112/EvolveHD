"""Prototype (exact counts) of a factored readout for a phrase level, on a word stream.
Words are cut into phrases where word surprise (order-2 Witten-Bell, prequential) exceeds its running mean
+ Z sds, like the engine's chunk cut. For each word, at position j in its phrase (j known causally: the phrase
started at the last cut), heads predict it from the previous phrase:
  cell      : q(w | j, previous phrase as one unit)                  -- what a slot/cell readout can use
  factored  : one head per role of the previous phrase, q(w | j, part_r), r = its last 1..R words and its first
  skip      : q(w | word 2 back), q(w | word 3 back)                 -- non-adjacent words without phrases
Each head (and the union of heads) is mixed with the order-2 word model, weights chosen on the first fifth
of the scored half, gain reported on the rest (bits per word, beyond order-2 counts).
usage: factored_proto.py FILE.syms VOCAB [Z]"""
import sys, math
import numpy as np
from collections import defaultdict, Counter
x = np.fromfile(sys.argv[1], dtype=np.uint32).tolist(); V = int(sys.argv[2]); Z = float(sys.argv[3]) if len(sys.argv) > 3 else 0.5
n = len(x); h = n // 2; cal = h + (n - h) // 5

def wb(c, y, base):
    if not c: return base
    t = len(c); return (c[y] + t * base) / (sum(c.values()) + t)

class Head:
    def __init__(s): s.c = defaultdict(Counter); s.u = Counter(); s.n = 0
    def q(s, key, y):
        b = (s.u[y] + 1) / (s.n + V)
        return wb(s.c.get(key), y, b) if key is not None else b
    def add(s, key, y):
        if key is not None: s.c[key][y] += 1
        s.u[y] += 1; s.n += 1

uni = Counter(); c1 = defaultdict(Counter); c2 = defaultdict(Counter); ntot = 0
m_s = 0.0; v_s = 1.0
heads = {k: Head() for k in ['cell', 'last1', 'last2', 'last3', 'first', 'skip2', 'skip3']}
groups = {'cell': ['cell'], 'factored (last 1-3, first)': ['last1', 'last2', 'last3', 'first'],
          'factored without last1': ['last2', 'last3', 'first'], 'skip 2,3 back': ['skip2', 'skip3'],
          'skip + factored': ['skip2', 'skip3', 'last1', 'last2', 'last3', 'first'],
          'skip + cell': ['skip2', 'skip3', 'cell']}
LAMS = [0.0, 0.02, 0.05, 0.1, 0.2, 0.35]
acc = {g: defaultdict(lambda: [0.0, 0.0]) for g in groups}
PAIRS = {'skip | factored': ['last1', 'last2', 'last3', 'first'], 'skip | cell': ['cell']}
W2 = [(a, b) for a in [0.0, 0.05, 0.1, 0.15] for b in [0.0, 0.02, 0.05, 0.1, 0.15] if a + b < 0.5]
acc2 = {g: defaultdict(lambda: [0.0, 0.0]) for g in PAIRS}
start = 0; prev = None; cuts = 0; plen = []
for t in range(n):
    y = x[t]
    p0 = (uni[y] + 1) / (ntot + V)
    p1 = wb(c1.get(x[t-1]) if t >= 1 else None, y, p0)
    p2 = wb(c2.get((x[t-2], x[t-1])) if t >= 2 else None, y, p1)
    j = min(t - start, 7)
    keys = {}
    if prev is not None:
        keys = {'cell': (j, prev), 'last1': (j, prev[-1]), 'last2': (j, prev[-2]) if len(prev) > 1 else None,
                'last3': (j, prev[-3]) if len(prev) > 2 else None, 'first': (j, prev[0])}
    keys['skip2'] = x[t-2] if t >= 2 else None; keys['skip3'] = x[t-3] if t >= 3 else None
    qs = {k: heads[k].q(keys.get(k), y) for k in heads}
    if t >= h:
        k = 0 if t < cal else 1
        for g, hs in groups.items():
            qg = sum(qs[a] for a in hs) / len(hs)            # equal blend of the group's heads
            for l in LAMS: acc[g][l][k] += math.log2((1 - l) * p2 + l * qg)
        qsk = (qs['skip2'] + qs['skip3']) / 2
        for g, hs in PAIRS.items():
            qg = sum(qs[a] for a in hs) / len(hs)
            for (a, b) in W2: acc2[g][(a, b)][k] += math.log2((1 - a - b) * p2 + a * qsk + b * qg)
    # learn
    for kk in heads: heads[kk].add(keys.get(kk), y)
    uni[y] += 1; ntot += 1
    if t >= 1: c1[x[t-1]][y] += 1
    if t >= 2: c2[(x[t-2], x[t-1])][y] += 1
    s = -math.log2(p2); d = s - m_s; m_s += 0.001 * d; v_s = 0.999 * v_s + 0.001 * d * d
    # cut before the next word if this word was surprising (the phrase ends here)
    if t > start and s > m_s + Z * math.sqrt(v_s):
        prev = tuple(x[start:t + 1]); plen.append(len(prev)); start = t + 1; cuts += 1
print(f"{sys.argv[1]}: {n} words, {cuts} phrases, mean length {np.mean(plen):.2f}")
for g in PAIRS:
    best = max(W2, key=lambda w: acc2[g][w][0]); m = n - cal
    sk = max([w for w in W2 if w[1] == 0.0], key=lambda w: acc2[g][w][0])
    print(f"  {g:28s} gain {(acc2[g][best][1] - acc2[g][(0.0, 0.0)][1]) / m:+.4f} (weights skip {best[0]}, other {best[1]}); skip alone {(acc2[g][sk][1] - acc2[g][(0.0, 0.0)][1]) / m:+.4f}")
for g in groups:
    best = max(LAMS, key=lambda l: acc[g][l][0]); m = n - cal
    print(f"  {g:28s} gain {(acc[g][best][1] - acc[g][0.0][1]) / m:+.4f} bits/word beyond order-2 counts (weight {best})")
