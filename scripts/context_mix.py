"""Gain from non-adjacent context as a mixture with an order-1 model: p = (1-l) p1(y|prev) + l q(y|ctx).
Counts are prequential (learned as the stream goes); l is chosen on the first 20% of the second half and
reported on the rest. Contexts: skip-bigram (symbol d back), recency cache (last W non-"other" symbols),
and with CTX=tri the adjacent order-2 context (two previous symbols).
usage: context_mix.py syms FILE OTHER_SYMBOL | context_mix.py words FILE NBYTES VOCAB   [CTX=tri]
("words" splits text into lower-cased words and punctuation and keeps the VOCAB commonest; the rest are "other".)"""
import sys, math, re
import numpy as np
from collections import defaultdict, Counter, deque
if sys.argv[1] == 'syms':
    x = np.fromfile(sys.argv[2], dtype=np.uint32).tolist(); OTHER = int(sys.argv[3])
else:
    t = open(sys.argv[2], 'rb').read().decode('latin-1')[:int(sys.argv[3])]
    toks = [w.lower() for w in re.findall(r"[A-Za-z']+|[^\sA-Za-z']", t)]
    V = int(sys.argv[4]); top = [w for w, _ in Counter(toks).most_common(V)]; ix = {w: i for i, w in enumerate(top)}
    OTHER = V; x = [ix.get(w, OTHER) for w in toks]
n = len(x); V = max(x) + 1; h = n // 2; cal = h + (n - h) // 5
LAMS = [0.0, 0.01, 0.02, 0.05, 0.1, 0.2, 0.3, 0.45, 0.6, 0.75, 0.9]
def wb(c, y, base):
    if not c: return base
    t = len(c); return (c[y] + t * base) / (sum(c.values()) + t)
def evaluate(q_fn, name):
    """q_fn(i) -> (dist-like callable or None); mix with order-1 WB."""
    uni = Counter(); big = defaultdict(Counter); acc = {l: [0.0, 0.0] for l in LAMS}; st = q_fn('init')
    for i in range(1, n):
        y = x[i]; p0 = (uni[y] + 1) / (sum_uni[0] + V); p1 = wb(big.get(x[i-1]), y, p0)
        q = st.q(i, y)
        if i >= h:
            k = 0 if i < cal else 1
            for l in LAMS: acc[l][k] += math.log2((1 - l) * p1 + l * q)
        uni[y] += 1; sum_uni[0] += 1; big[x[i-1]][y] += 1; st.update(i, y)
    sum_uni[0] = 0
    best = max(LAMS, key=lambda l: acc[l][0])
    m = n - cal
    print(f"  {name:<44} gain {(acc[best][1] - acc[0.0][1]) / m:+.4f} bits/symbol (weight {best})", flush=True)
sum_uni = [0]
class Skip:
    def __init__(s, d): s.d = d; s.c = defaultdict(Counter); s.u = Counter(); s.n = 0
    def q(s, i, y):
        b = (s.u[y] + 1) / (s.n + V)
        return wb(s.c.get(x[i - s.d]), y, b) if i >= s.d else b
    def update(s, i, y):
        if i >= s.d: s.c[x[i - s.d]][y] += 1
        s.u[y] += 1; s.n += 1
class Tri:
    """Adjacent order-2 context: the two previous symbols together."""
    def __init__(s): s.c = defaultdict(Counter); s.u = Counter(); s.n = 0
    def q(s, i, y):
        b = (s.u[y] + 1) / (s.n + V)
        return wb(s.c.get((x[i - 2], x[i - 1])), y, b) if i >= 2 else b
    def update(s, i, y):
        if i >= 2: s.c[(x[i - 2], x[i - 1])][y] += 1
        s.u[y] += 1; s.n += 1
class Cache:
    def __init__(s, W): s.W = W; s.w = deque(); s.c = Counter()
    def q(s, i, y):
        k = len(s.w); return (s.c[y] / k if k and y != OTHER else 0.0) + 1e-9
    def update(s, i, y):
        if y == OTHER: return
        s.w.append(y); s.c[y] += 1
        if len(s.w) > s.W: s.c[s.w.popleft()] -= 1
print(f"{sys.argv[2]}: {n} symbols, vocab {V}, other {sum(v == OTHER for v in x) / n:.2f}")
import os
ONLY = os.environ.get('CTX')
if ONLY == 'tri':
    evaluate(lambda _i: Tri(), "adjacent: the two previous symbols")
    sys.exit()
for d in [2, 3, 5, 10]:
    evaluate(lambda _i, d=d: Skip(d), f"skip-bigram: symbol {d} back")
for W in [100, 1000]:
    evaluate(lambda _i, W=W: Cache(W), f"recency cache of last {W} non-other symbols")
