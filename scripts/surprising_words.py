"""What would keeping the surprising words be worth? Counts, prequential, scored on exact word identity.

The word stream keeps the V commonest words; the rest are "other". Bits per word for the exact word:
  in-vocabulary word w:  -log2 p(w)
  other word w:          -log2 p(other) - log2 q(w | other)
Baseline: p = order-2 Witten-Bell counts over the V slots + other; q = unigram counts over other words, with a
Witten-Bell escape to a flat 1/100,000 for words never seen.
Variants (weights chosen on the first fifth of the scored half, reported on the rest):
  story-512   the story op of experiment 24 (rows keyed by salient recent in-vocabulary content words)
  keep        remember surprising words: a decaying cache of recent "other" words, mixed into q
  keep+ctx    also use the kept words as story keys (rows keyed by them predict the slots)
usage: surprising_words.py BIN NBYTES V [H]"""
import sys, re, math
import numpy as np
from collections import Counter, defaultdict
txt = open(sys.argv[1], 'rb').read().decode('latin-1')[:int(sys.argv[2])]
V = int(sys.argv[3]); H = float(sys.argv[4]) if len(sys.argv) > 4 else 200.0
toks = [w.lower() for w in re.findall(r"[A-Za-z']+|[^\sA-Za-z']", txt)]
top = [w for w, _ in Counter(toks).most_common(V)]; ix = {w: i for i, w in enumerate(top)}
import os; FLAT = os.environ.get("FLAT") == "1"
OTHER = V; NV = V + 1; SKIP = 100; K = 16
x = [ix.get(w, OTHER) for w in toks]; n = len(x); h = n // 2; cal = h + (n - h) // 5
print(f"{sys.argv[1]}: {n} words, vocab {V}, other {sum(v == OTHER for v in x) / n:.3f}, "
      f"distinct other words {len(set(w for w, v in zip(toks, x) if v == OTHER))}", flush=True)
def wb(c, tot, y, base):
    if not tot: return base
    t = len(c); return (c.get(y, 0) + t * base) / (tot + t)
uni = Counter(); un = 0; big = defaultdict(Counter); bt = Counter(); tri = defaultdict(Counter); tt = Counter()
pb = np.zeros(n); p0s = np.zeros(n)
oc = Counter(); on = 0; qo = np.ones(n)            # other-word unigram
cache = {}; cg = 1.0; qc = np.zeros(n)             # surprising-word cache: salience per other word
# story rows: keyed by in-vocab content symbols (s512) or by kept other words too (sctx)
sal512 = {}; salk = {}; g = 1.0
rows5 = defaultdict(Counter); rt5 = Counter(); rowsk = defaultdict(Counter); rtk = Counter()
q512 = np.zeros(n); qk = np.zeros(n)
def topk(sal, k):
    return sorted(sal.items(), key=lambda kv: -kv[1])[:k]
set512 = []; setk = []
for i in range(n):
    y = x[i]; w = toks[i]
    p0 = (uni[y] + 1) / (un + NV); p0s[i] = p0
    a = x[i-1] if i else -1; b = (x[i-2], a) if i > 1 else None
    p1 = wb(big[a], bt[a], y, p0) if i else p0
    pb[i] = wb(tri[b], tt[b], y, p1) if b else p1
    # story predictions over slots
    for st, qarr, rows, rt in ((set512, q512, rows5, rt5), (setk, qk, rowsk, rtk)):
        if st:
            ws = sum(v for _, v in st)
            qarr[i] = sum(v * (rows[kk][y] + 0.5 * NV * p0) / (rt[kk] + 0.5 * NV) for kk, v in st) / ws
        else: qarr[i] = p0
    if y == OTHER:
        tq = len(oc); qo[i] = (oc[w] + tq * 1e-5) / (on + tq) if on else 1e-5
        tot = sum(cache.values()); qc[i] = cache.get(w, 0.0) / tot if tot else 0.0
    # updates
    uni[y] += 1; un += 1
    if i: big[a][y] += 1; bt[a] += 1
    if b: tri[b][y] += 1; tt[b] += 1
    for kk, _ in set512: rows5[kk][y] += 1; rt5[kk] += 1
    for kk, _ in setk: rowsk[kk][y] += 1; rtk[kk] += 1
    g *= 2 ** (1 / H)
    if y == OTHER:
        surprise = -math.log2(qo[i]) if not FLAT else 1.0   # how new this word is (FLAT=1: plain recency)
        cache[w] = cache.get(w, 0.0) + surprise * g
        salk[('o', w)] = salk.get(('o', w), 0.0) + surprise * g
        oc[w] += 1; on += 1
    elif y >= SKIP:
        sal512[y] = sal512.get(y, 0.0) + g; salk[('v', y)] = salk.get(('v', y), 0.0) + g
    if g > 1e100:
        for d in (cache, sal512, salk):
            for kk in d: d[kk] /= 1e100
        g /= 1e100
    if i % 256 == 0:   # prune faded entries (below 1/1000 of a fresh count)
        for d in (cache, sal512, salk):
            lo = g * 1e-3
            for kk in [kk for kk, v in d.items() if v < lo]: del d[kk]
    if (y == OTHER or y >= SKIP) and i % 8 == 0:
        set512 = [(kk, v / g) for kk, v in topk(sal512, K)]
        setk = [(kk, v / g) for kk, v in topk(salk, K)]
LAMS = [0.0, 0.01, 0.02, 0.05, 0.1, 0.15, 0.2, 0.3, 0.45, 0.6, 0.75]
isoth = np.array([v == OTHER for v in x])
def bits(ls, lc, qs):
    p = (1 - ls) * pb + ls * qs
    q = np.where(isoth, (1 - lc) * qo + lc * qc, 1.0)
    return -np.log2(p) - np.log2(q)
def best(qs, use_s, use_c):
    sl0 = slice(h, cal)
    grid = [(ls, lc) for ls in (LAMS if use_s else [0.0]) for lc in (LAMS if use_c else [0.0])]
    ls, lc = min(grid, key=lambda g_: bits(g_[0], g_[1], qs)[sl0].mean())
    return bits(ls, lc, qs)[cal:].mean(), ls, lc
base = bits(0, 0, pb)[cal:].mean()
oth = isoth[cal:].mean()
print(f"  baseline: {base:.3f} bits per word (of which other-word spelling {-(np.log2(qo)[cal:][isoth[cal:]]).sum() / (n - cal):.3f})")
for name, qs, us, uc in [("story-512", q512, 1, 0), ("keep (cache of surprising other words)", pb, 0, 1),
                         ("story-512 + keep", q512, 1, 1), ("keep+ctx (kept words also as story keys)", qk, 1, 1)]:
    b, ls, lc = best(qs, us, uc); print(f"  {name:<44} {b:.3f}  gain {base - b:+.4f}  (weights {ls}, {lc})", flush=True)
