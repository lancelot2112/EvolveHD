"""Counts version of the engine's story edge, on a symbol stream: rows keyed by the K most salient recent
content symbols (salience decays with half-life H content symbols), the prediction the salience-weighted mean
of the rows' distributions, every row (or one, drawn by weight) counting each next symbol. Gain beyond
order-2 Witten-Bell counts, as a mixture (weight chosen on the first fifth of the scored half).
usage: story_proto.py SYMS [K H write1 ...]   e.g. 16 200 0   (other = the largest symbol; content >= 100)"""
import sys, math, random
import numpy as np
from collections import Counter, defaultdict
x = np.fromfile(sys.argv[1], dtype=np.uint32).tolist(); OTHER = max(x); NV = OTHER + 1; SKIP = 100
n = len(x); h = n // 2; cal = h + (n - h) // 5
def wb(c, tot, y, base):
    if not tot: return base
    t = len(c); return (c.get(y, 0) + t * base) / (tot + t)
uni = Counter(); un = 0; big = defaultdict(Counter); bt = Counter(); tri = defaultdict(Counter); tt = Counter()
pb = np.zeros(n); p0s = np.zeros(n)
for i in range(n):
    y = x[i]; p0 = (uni[y] + 1) / (un + NV); p0s[i] = p0
    a = x[i-1] if i else -1; b = (x[i-2], a) if i > 1 else None
    p1 = wb(big[a], bt[a], y, p0) if i else p0
    pb[i] = wb(tri[b], tt[b], y, p1) if b else p1
    uni[y] += 1; un += 1
    if i: big[a][y] += 1; bt[a] += 1
    if b: tri[b][y] += 1; tt[b] += 1
def story(K, H, w1):
    rng = random.Random(1); sal = np.zeros(NV); g = 1.0; rows = defaultdict(Counter); rt = Counter()
    q = np.zeros(n); setk = []
    for i in range(n):
        y = x[i]
        if setk:
            ws = sum(w for _, w in setk)
            q[i] = sum(w * (rows[k][y] + 0.5 * NV * p0s[i]) / (rt[k] + 0.5 * NV) for k, w in setk) / ws
            upd = setk
            if w1:
                u = rng.random() * ws
                for kw in setk:
                    if u < kw[1]: upd = [kw]; break
                    u -= kw[1]
                else: upd = [setk[-1]]
            for k, _ in upd: rows[k][y] += 1; rt[k] += 1
        else: q[i] = p0s[i]
        if SKIP <= y < OTHER:
            g *= 2 ** (1 / H); sal[y] += g
            if g > 1e100: sal /= 1e100; g /= 1e100
            nz = np.nonzero(sal)[0]; top = nz[np.argsort(-sal[nz])[:K]]
            setk = [(int(k), sal[k] / g) for k in top]
    return q
LAMS = [0.0, 0.01, 0.02, 0.05, 0.1, 0.15, 0.2, 0.3, 0.45]
def gain(q):
    best = max(LAMS, key=lambda l: np.log2((1 - l) * pb[h:cal] + l * q[h:cal]).mean())
    return np.log2((1 - best) * pb[cal:] + best * q[cal:]).mean() - np.log2(pb[cal:]).mean(), best
print(f"{sys.argv[1]}: {n} symbols; order-2 WB {-np.log2(pb[cal:]).mean():.3f} bits", flush=True)
args = sys.argv[2:] or ['16', '200', '0']
for j in range(0, len(args), 3):
    K, H, w1 = int(args[j]), float(args[j+1]), int(args[j+2])
    g, l = gain(story(K, H, w1)); print(f"  K {K:<3} half-life {H:<6g} write {'one' if w1 else 'all'}: gain {g:+.4f} (weight {l})", flush=True)
