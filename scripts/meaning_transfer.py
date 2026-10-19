"""Does meaning carry across a story, beyond the nearby words? Counts gate, whole words.

Baseline: order-2 Witten-Bell word counts (backing off to order 1, then unigram), prequential.
Long-range contexts, each mixed with the baseline (weights chosen on the first fifth of the scored half,
gain reported on the rest, bits per word):
  cache W     the last W content words, predicting themselves again (literal repetition)
  book        unigram counts of the current book so far (topic, names, style of this story)
  assoc       words that co-occurred in the same 500-word passages with the last 500 content words,
              excluding the word itself (meaning by association: "ship" -> "sea", "whale"); prequential,
              updated after each passage.
usage: meaning_transfer.py BIN IDX VOCAB"""
import sys, re, math
import numpy as np
from collections import Counter, defaultdict, deque
txt = open(sys.argv[1], 'rb').read().decode('latin-1')
books = [(l.split('\t')[0], int(l.split('\t')[1])) for l in open(sys.argv[2])]
V = int(sys.argv[3])
toks, bk = [], []
starts = [s for _, s in books]
bi = 0
for m in re.finditer(r"[A-Za-z']+|[^\sA-Za-z']", txt):
    while bi + 1 < len(starts) and m.start() >= starts[bi + 1]: bi += 1
    toks.append(m.group().lower()); bk.append(bi)
top = [w for w, _ in Counter(toks).most_common(V)]; ix = {w: i for i, w in enumerate(top)}
OTHER = V; x = np.array([ix.get(w, OTHER) for w in toks]); bk = np.array(bk)
n = len(x); h = n // 2; cal = h + (n - h) // 5; NV = V + 1
FUNC = 100  # the 100 commonest words (and punctuation) are not "content"
content = (x >= FUNC) & (x != OTHER)
print(f"{n} words, vocab {V}, other {np.mean(x == OTHER):.3f}, content {content.mean():.3f}", flush=True)

def wb(c, tot, y, base):
    if not tot: return base
    t = len(c); return (c.get(y, 0) + t * base) / (tot + t)
# baseline, prequential order-2 WB
uni = np.zeros(NV); un = 0; big = defaultdict(Counter); bt = Counter(); tri = defaultdict(Counter); tt = Counter()
pb = np.zeros(n)
for i in range(n):
    y = int(x[i]); p0 = (uni[y] + 1) / (un + NV)
    a = int(x[i-1]) if i else -1; b = (int(x[i-2]), a) if i > 1 else None
    p1 = wb(big[a], bt[a], y, p0) if i else p0
    p2 = wb(tri[b], tt[b], y, p1) if b else p1
    pb[i] = p2
    uni[y] += 1; un += 1
    if i: big[a][y] += 1; bt[a] += 1
    if b: tri[b][y] += 1; tt[b] += 1
print("baseline done", flush=True)

def cache(W):
    q = np.full(n, 1e-9); w = deque(); c = Counter()
    for i in range(n):
        y = int(x[i])
        if w and y != OTHER: q[i] += c[y] / len(w)
        if content[i]:
            w.append(y); c[y] += 1
            if len(w) > W: c[w.popleft()] -= 1
    return q
def book():
    q = np.zeros(n); c = np.zeros(NV); tot = 0; cur = -1
    for i in range(n):
        if bk[i] != cur: c[:] = 0; tot = 0; cur = bk[i]
        y = int(x[i]); q[i] = (c[y] + 0.1) / (tot + 0.1 * NV); c[y] += 1; tot += 1
    return q
def assoc(P=500, W=500, every=25):
    """q(y) proportional to sum over recent distinct content words w (w != y) of P(y | w in passage)."""
    M = np.zeros((NV, NV), dtype=np.float32); occ = np.zeros(NV, dtype=np.float32)
    q = np.full(n, 1e-9); w = deque(); c = Counter(); qv = None; last = -every
    for i in range(n):
        if i % P == 0 and i:
            ws = np.unique(x[i-P:i][content[i-P:i]])
            M[np.ix_(ws, ws)] += 1; occ[ws] += 1
        if i - last >= every:
            last = i; ks = np.array([k for k, v in c.items() if v > 0], dtype=int)
            if len(ks) and occ[ks].min() > 0:
                R = M[ks] / occ[ks][:, None]  # P(y in passage | w in passage)
                R[np.arange(len(ks)), ks] = 0  # no self: association, not repetition
                qv = R.sum(0); qv[:FUNC] = 0; qv[OTHER] = 0; s = qv.sum(); qv = qv / s if s > 0 else None
            else: qv = None
        y = int(x[i])
        if qv is not None and content[i]: q[i] += qv[y]
        if content[i]:
            w.append(int(x[i])); c[int(x[i])] += 1
            if len(w) > W: c[w.popleft()] -= 1
    return q

LAMS = [0.0, 0.01, 0.02, 0.05, 0.1, 0.15, 0.2, 0.3, 0.45]
def gain(qs):
    """Mix baseline with the given contexts, grid over each weight."""
    import itertools
    best, bl = None, None
    sl0 = slice(h, cal); sl1 = slice(cal, n)
    base1 = np.log2(pb[sl1]).mean()
    for ls in itertools.product(LAMS, repeat=len(qs)):
        if sum(ls) >= 0.9: continue
        p = (1 - sum(ls)) * pb + sum(l * q for l, q in zip(ls, qs))
        s0 = np.log2(p[sl0]).mean()
        if best is None or s0 > best: best, bl = s0, ls
    p = (1 - sum(bl)) * pb + sum(l * q for l, q in zip(bl, qs))
    return np.log2(p[sl1]).mean() - base1, bl
print(f"baseline (order-2 WB), scored part: {-np.log2(pb[cal:]).mean():.3f} bits/word", flush=True)
Q = {}
for W in [100, 1000, 10000]: Q[f'cache {W}'] = cache(W)
Q['book'] = book()
print("cache/book done", flush=True)
Q['assoc'] = assoc()
for k in Q: g, l = gain([Q[k]]); print(f"  {k:<32} {g:+.4f} (weight {l})", flush=True)
for combo in [('cache 1000', 'assoc'), ('cache 1000', 'book'), ('cache 1000', 'book', 'assoc')]:
    g, l = gain([Q[k] for k in combo]); print(f"  {' + '.join(combo):<32} {g:+.4f} (weights {l})", flush=True)
