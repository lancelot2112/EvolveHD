"""Meaning codes in the 2,048-bit space (experiment 27). One pass, prequential.

Every word has a fixed random identity code (32 of 2,048 bits). Its meaning is learned as it reads: a counter per
bit, M[w], to which each co-occurrence adds the identity code of the other word (content words, those outside the
100 commonest, within --win content words of each other, both directions), weighted by the other word's rarity
(log of total / its count so far). The meaning code m[w] is the --K strongest bits of M[w]. Two words that keep the
same company share bits, whether or not they ever meet: the groupings.
The story state is a decaying sum (half-life 200 content words) of the meaning codes of recent content words; its
code is the --k most lit bits. Readouts, both predicting the exact next word:
  q_bits  rows keyed by the story code's bits, each counting the next word (learned)
  q_mean  meaning readout: each story-code bit spreads its share over the words whose meaning codes contain it, by
          frequency so far (a snapshot of all meaning codes every --snap words), so a story about the sea can
          name a sea word the story hasn't used yet
--identity: build the story state from identity codes instead of meaning codes (the control: same compression,
no meaning). Scored as cortex_map.py, beyond the experiment-25 counts system (cortex_base.py).
usage: meaning_codes.py IDS WORDS.txt BASE.npz OUT [--K 64] [--k 64] [--win 16] [--identity] [--snap 20000]"""
import sys, math, argparse, json, itertools
import numpy as np, scipy.sparse as spm
ap = argparse.ArgumentParser()
ap.add_argument('ids'); ap.add_argument('words'); ap.add_argument('base'); ap.add_argument('out')
ap.add_argument('--K', type=int, default=64); ap.add_argument('--k', type=int, default=64)
ap.add_argument('--win', type=int, default=16); ap.add_argument('--identity', action='store_true')
ap.add_argument('--snap', type=int, default=20000); ap.add_argument('--half', type=float, default=200.0)
ap.add_argument('--report', type=int, default=1)
ap.add_argument('--raw', action='store_true')   # choose meaning bits by raw count instead of by distinctiveness
ap.add_argument('--D', type=int, default=2048)   # width of the space
# fast mapping (hippocampal): on each of a word's first --fast-n occurrences, add the story code lit at that moment
# to its counters with this weight per bit, so a new word starts out meaning what the story is about
ap.add_argument('--fast', type=float, default=0.0); ap.add_argument('--fast-n', type=int, default=8)
A = ap.parse_args()
rng = np.random.default_rng(1)
ids = np.fromfile(A.ids, dtype=np.uint32).astype(np.int64); n = len(ids)
words = open(A.words, encoding='latin-1').read().split('\n')
B = np.load(A.base); p_base, q_story, u_p = B['p_base'], B['q_story'], B['u_p']
NW = int(ids.max()) + 1; SKIP = 100; D = A.D; CK = 32; BETA = 256.0
ident = np.stack([rng.choice(D, CK, replace=False) for _ in range(NW)])   # identity codes
# the control's codes: random, the same size as a meaning code (K bits), for content words only
ident_k = np.stack([rng.choice(D, A.K, replace=False) for _ in range(NW)]) if A.identity else None
M = np.zeros((NW, D), dtype=np.float32)
fc = np.zeros(NW, dtype=np.int64); ctot = 0
Mcol = np.zeros(D, dtype=np.float64)          # how much each bit is used across all words
def mcode(w):
    if w < SKIP: return None                    # only content words have codes, in both versions
    if A.identity: return ident_k[w]
    row = M[w]
    if not row.any(): return None
    kk = min(A.K, int(np.count_nonzero(row)))
    if not A.raw:
        row = row / (Mcol + Mcol.mean() + 1e-9)   # distinctiveness: a bit busy for every word says little
    return np.argpartition(-row, kk - 1)[:kk]
recent = []                                    # last --win content words
g = 1.0; act = np.zeros(D); code = np.zeros(0, dtype=np.int64); incode = np.zeros(D, dtype=bool)
rows = np.zeros((D, NW), dtype=np.int32); nb = np.zeros(D, dtype=np.int64)
q_bits = np.zeros(n); q_mean = np.zeros(n); prior = np.zeros(n, dtype=np.int64)
snap = None; snapU = None; snapu = None
def snapshot():
    """Meaning codes of every word seen so far, as a sparse word x bit matrix, with frequency weights."""
    global snap, snapU, snapu
    seen = np.nonzero(fc >= 1)[0]
    r, c = [], []
    for w in seen:
        m = mcode(w)
        if m is not None: r.append(np.full(len(m), w)); c.append(m)
    if not r: return
    r = np.concatenate(r); c = np.concatenate(c)
    snap = spm.csr_matrix((np.ones(len(r), dtype=np.float32), (r, c)), shape=(NW, D))
    snapu = fc.astype(np.float64).copy()
    snapU = np.asarray(snap.T @ snapu).ravel()
# demo: record story codes at occurrences of a few words, for the attention demo
# (the demo records the first 400 occurrences of each word; the story code at an occurrence already includes the
# word's own meaning code)
DEMO = [w for w in ('light', 'spring', 'bank', 'fire', 'court', 'heart', 'charge', 'state', 'ball', 'deck')
        if w in words]
didx = {words.index(w): w for w in DEMO}; dcodes = {w: [] for w in DEMO}
for i in range(n):
    w = int(ids[i]); prior[i] = fc[w]
    if len(code):
        r = rows[code, w].astype(np.float64); q_bits[i] = ((r + BETA * u_p[i]) / (nb[code] + BETA)).mean()
        if snap is not None and w < snap.shape[0]:
            row = snap.getrow(w)
            if row.nnz and snapu[w] > 0:
                bb = row.indices[incode[row.indices]]
                bb = bb[snapU[bb] > 0]
                q_mean[i] = (snapu[w] / snapU[bb]).sum() / len(code)
    else:
        q_bits[i] = u_p[i]
    if len(code):
        rows[code, w] += 1; nb[code] += 1
    fc[w] += 1
    if w >= SKIP:
        ctot += 1
        if not A.identity:
            wt_w = math.log(ctot / fc[w]) + 1e-3
            for c_ in recent:
                if c_ == w: continue
                wc = math.log(ctot / fc[c_]) + 1e-3
                M[w, ident[c_]] += wc; M[c_, ident[w]] += wt_w
                Mcol[ident[c_]] += wc; Mcol[ident[w]] += wt_w
        if A.fast > 0 and not A.identity and fc[w] <= A.fast_n and len(code):
            M[w, code] += A.fast; Mcol[code] += A.fast
        recent.append(w)
        if len(recent) > A.win: recent.pop(0)
        m = mcode(w)
        g *= 2 ** (1 / A.half)
        if m is not None: act[m] += g
        if g > 1e100: act /= 1e100; g /= 1e100
        nz = int(np.count_nonzero(act)); kk = min(A.k, nz)
        if kk:
            code = np.argpartition(-act, kk - 1)[:kk]; incode[:] = False; incode[code] = True
        if w in didx and len(dcodes[didx[w]]) < 400: dcodes[didx[w]].append((i, code.copy()))
    if i % A.snap == A.snap - 1: snapshot()
    if i % 250000 == 0: print(f"t={i}", flush=True)
# ---- scoring (as cortex_map.py)
h = n // 2; cal = h + (n - h) // 5; sc = slice(cal, n); ca = slice(h, cal)
LAMS = [0.0, 0.02, 0.05, 0.1, 0.15, 0.2, 0.3, 0.45]
def fit(qs):
    best = None
    for ls in itertools.product(LAMS, repeat=len(qs)):
        if sum(ls) >= 0.95: continue
        p = (1 - sum(ls)) * p_base[ca] + sum(l * q[ca] for l, q in zip(ls, qs))
        v = np.log2(p).mean()
        if best is None or v > best[0]: best = (v, ls)
    ls = best[1]
    return -np.log2((1 - sum(ls)) * p_base + sum(l * q for l, q in zip(ls, qs))), ls
base_bits = -np.log2(p_base)
last = {}; storynew = np.zeros(n, dtype=bool)
for i in range(n):
    w = int(ids[i]); storynew[i] = w >= SKIP and (w not in last or i - last[w] > 2000); last[w] = i
rare = prior < 10
out = {'args': vars(A), 'base': float(base_bits[sc].mean())}
print(f"{A.out}: base {out['base']:.4f} bits per exact word (scored part)")
for name, qs in {'story': [q_story], 'bits': [q_bits], 'mean': [q_mean], 'story+bits': [q_story, q_bits],
                 'story+mean': [q_story, q_mean], 'story+bits+mean': [q_story, q_bits, q_mean]}.items():
    bb, ls = fit(qs); d = base_bits - bb
    m_ = lambda mask: float(d[sc][mask[sc]].mean())
    out[name] = {'gain': float(d[sc].mean()), 'rare': m_(rare), 'storynew': m_(storynew), 'weights': ls}
    print(f"  {name:<16} gain {d[sc].mean():+.4f}   on rare words {m_(rare):+.4f}   on words new to the story {m_(storynew):+.4f}   weights {ls}", flush=True)
json.dump(out, open(A.out + '.json', 'w'), indent=1)
if not A.report or A.identity: sys.exit()
# ---- groupings
lines = []
W = lambda w: words[w] if w < len(words) else str(w)
cnt = fc
pool = np.nonzero((cnt >= 20) & (np.arange(NW) >= SKIP))[0]
rr, cc = [], []
for w in pool:
    m = mcode(w)
    if m is not None: rr.append(np.full(len(m), w)); cc.append(m)
C = spm.csr_matrix((np.ones(sum(len(x) for x in rr), dtype=np.float32), (np.concatenate(rr), np.concatenate(cc))), shape=(NW, D))
Cp = C[pool]
def neighbours(vec_bits, k=12, exclude=()):
    v = np.zeros(D, dtype=np.float32); v[vec_bits] = 1
    s = Cp @ v
    o = np.argsort(-s)
    res = []
    for j in o:
        if pool[j] in exclude: continue
        res.append(int(pool[j]))
        if len(res) == k: break
    return res
lines.append(f"Meaning codes: {A.K} bits of {D} per word ({'raw counts' if A.raw else 'distinctive bits'}); {len(pool)} content words seen 20+ times.")
lines.append("\nNearest words by shared meaning bits:")
for s in ['whale', 'sea', 'ship', 'king', 'god', 'love', 'heart', 'horse', 'sword', 'gold', 'rabbit', 'emma', 'brutus',
          'dinner', 'money', 'tree', 'river', 'night', 'war', 'church', 'letter', 'jump', 'light', 'spring', 'fire']:
    if s in words:
        w = words.index(s)
        if cnt[w] >= 20:
            if mcode(w) is not None:
                lines.append(f"  {s}: " + " ".join(W(x) for x in neighbours(mcode(w), exclude=(w,))))
# locality on the same measure as experiment 26: share of each word's 10 strongest associates among its nearest 2%
cw = ids[ids >= SKIP]; r_, c_ = [], []
for d_ in range(1, 11): r_.append(cw[:-d_]); c_.append(cw[d_:])
r_ = np.concatenate(r_); c_ = np.concatenate(c_)
Mx = spm.coo_matrix((np.ones(len(r_)), (r_, c_)), shape=(NW, NW)).tocsr(); Mx = Mx + Mx.T
Mx.setdiag(0); Mx.eliminate_zeros(); tot = Mx.sum(); rs = np.asarray(Mx.sum(1)).ravel()
cnt50 = [w for w in pool if np.bincount(cw, minlength=NW)[w] >= 50] if False else None
c3 = np.bincount(cw, minlength=NW)
cand = [int(w) for w in pool if c3[w] >= 50]
kn = max(1, int(0.02 * len(pool)))
prng = np.random.default_rng(2)
loc, ch = [], []
for w in prng.choice(cand, min(2000, len(cand)), replace=False):
    row = Mx.getrow(w); js, vs = row.indices, row.data; ok = vs >= 5; js, vs = js[ok], vs[ok]
    if len(js) < 10: continue
    pmi = np.log(vs * tot / (rs[w] * rs[js])); top = js[np.argsort(-pmi)[:10]]
    near = set(neighbours(mcode(w), k=kn, exclude=(w,)))
    loc.append(np.mean([x in near for x in top])); ch.append(np.mean([x in near for x in prng.choice(cand, 10)]))
lines.append(f"\nLocality: of each word's 10 strongest associates (as in experiment 26), the share among its nearest 2% by shared meaning bits: {np.mean(loc):.3f} (random words: {np.mean(ch):.3f}; {len(loc)} words)")
# attention: a word's meaning code ANDed with the story code where it occurs, in different books
lines.append("\nAttention: a word's meaning bits that the story has lit, at occurrences far apart, and the words nearest them:")
for s, occ in dcodes.items():
    w = words.index(s)
    if cnt[w] < 20 or len(occ) < 2: continue
    m = set(mcode(w).tolist())
    picks = [occ[0], occ[len(occ) // 2], occ[-1]]
    lines.append(f"  {s} (all bits): " + " ".join(W(x) for x in neighbours(np.array(sorted(m)), k=8, exclude=(w,))))
    for i0, cd in picks:
        att = np.array(sorted(m & set(cd.tolist())))
        if len(att) == 0: lines.append(f"    at word {i0}: nothing lit"); continue
        ctx = " ".join(words[int(x)] for x in ids[max(0, i0 - 8):i0 + 1])
        lines.append(f"    at word {i0} (\"...{ctx}\"), {len(att)} bits lit: " + " ".join(W(x) for x in neighbours(att, k=8, exclude=(w,))))
open(A.out + '.groups.txt', 'w').write("\n".join(lines) + "\n")
print("\n".join(lines))
