"""A learned situation model on ternary memory (experiment 34): the pieces of experiments 32 and 33 together, on the
comprehension stories with held-out phrasings. Nothing about the words is given.

1. Clauses from learned boundaries (experiment 32's integer test on collisions with a stochastically kept draw).
2. Facts: as each word w arrives it is bound to every earlier word s of its clause (back to and including the first
   boundary), both ways. The relation code is the superposition of the codes of the words between them, read
   back as 4 phases per element ("went to the" and "walked to the" share "to the", so a new verb lands near its
   synonyms; the backward relation is a permutation of the forward one; adjacent words have a code of their own).
3. Fast memory, ternary: the fact (s, R, w) is written into a sparse trace at s's K active elements (from a hash of
   s, offset by a slowly drifting context), with phase s + R + w per element, adding +-1 to the real or imaginary
   part and clipping to -1, 0, +1 (agreeing writes hold, conflicting ones cancel to 0). Or, with --overwrite,
   each written part takes the sign (newest wins).
4. Slow memory, many levels: for each context (the words 1, 3 and 4 back) and cue position (the last 4 words), a
   query code per hop, as integer phasor sums (counters). When the next word w arrives, for each cue word e the
   fast trace is unbound by e and by w at e's active elements: what remains is the relation code of the facts
   (e, R, w) that would have produced w. It is added to that context's hop-1 query. For hop 2, the intermediate m
   with the strongest facts e -> m and m -> w (searched over the vocabulary) gives both relations. Each route's
   reliability (8-bit running hit rate) is kept beside it.
5. Prediction: each route with a learned query reads the fast trace (unbinding by e and the query, then scoring
   every word by correlation, ranked against its background); the proposals are ranked by route reliability.
Scored at the answer positions of the second half: the first candidate of the answer's kind, and the top
candidate overall.
usage: situation.py IDS INFO.json [--D 65536] [--K 256] [--overwrite] [--drift 4] [--bdrift 32]"""
import sys, json, argparse
import numpy as np
from collections import defaultdict
ap = argparse.ArgumentParser(); ap.add_argument('ids'); ap.add_argument('info')
ap.add_argument('--D', type=int, default=65536); ap.add_argument('--K', type=int, default=256)
ap.add_argument('--overwrite', action='store_true'); ap.add_argument('--drift', type=int, default=4)
ap.add_argument('--bdrift', type=int, default=32); ap.add_argument('--DR', type=int, default=1024)
a = ap.parse_args()
ids = np.fromfile(a.ids, dtype=np.uint32).astype(int).tolist(); n = len(ids)
info = json.load(open(a.info)); voc = open(a.ids.replace('.ids', '.txt')).read().split()
D, K, DR = a.D, a.K, a.DR; V = max(ids) + 1; ADJ = V
rng = np.random.default_rng(3)
PHI = rng.integers(0, 4, (V + 1, DR)).astype(np.int8)         # 4-phase relation codes (DR elements; element e
                                                              # of the trace uses relation element e mod DR)
VPH = rng.integers(0, 4, (V, D)).astype(np.int8)              # value codes
KIDX = rng.integers(0, D, (V, K))                             # each word's active elements as a key
KPH = rng.integers(0, 4, (V, K)).astype(np.int8)
ctx_off = rng.integers(0, D, K); ctx_ph = rng.integers(0, 4, K).astype(np.int8)
UNIT = np.array([1, 1j, -1, -1j])
re = np.zeros(D, np.int8); im = np.zeros(D, np.int8)          # the fast ternary trace (one, both directions
bg = np.zeros((V,), np.int64)                                 #  share it: backward relations are permuted)
def rel_code(path):
    """relation of a path: superposition of its words' phasors, read back as the nearest of 4 phases per element"""
    if not path: return PHI[ADJ]
    z = UNIT[PHI[path]].sum(0)
    return (np.round(np.angle(z) / (np.pi / 2)) % 4).astype(np.int8)
def back(r): return np.roll(r, 1)
def key(e):
    idx = (KIDX[e] + ctx_off) % D; return idx, (KPH[e] + ctx_ph) % 4
def write(e, r, v):
    idx, kp = key(e); p = (kp + r[idx % DR] + VPH[v, idx]) % 4
    dre = np.array([1, 0, -1, 0])[p]; dim = np.array([0, 1, 0, -1])[p]
    if a.overwrite:
        re[idx] = np.where(dre != 0, dre, re[idx]); im[idx] = np.where(dim != 0, dim, im[idx])
    else:
        re[idx] = np.clip(re[idx] + dre, -1, 1); im[idx] = np.clip(im[idx] + dim, -1, 1)
    bg[v] += 1
def unbind(e):
    idx, kp = key(e); z = (re[idx] + 1j * im[idx]) * UNIT[(-kp) % 4]; return idx, z
def read(e, q):
    """score every value for key e under relation q (q a phase vector)"""
    idx, z = unbind(e); z = z * UNIT[(-q[idx % DR]) % 4]
    return np.real(z[None, :] * np.conj(UNIT[VPH[:, idx]])).sum(1)       # V scores
def rel_evidence(e, w):
    """the trace unbound by e and by w: the relation phasors of facts (e, R, w), at e's active elements"""
    idx, z = unbind(e); return idx, z * np.conj(UNIT[VPH[w, idx]])
def qcode(acc):
    return (np.round(np.angle(acc) / (np.pi / 2)) % 4).astype(np.int8)
# ---- learned segmentation ----
seg_draw = {}; seg_hits = defaultdict(lambda: [0, 0, 0]); seg_n = defaultdict(int); boundary = set()
def seg_update(x, u, c, t):
    d = seg_draw.get(u)
    if d is not None:
        h = seg_hits[u]; h[0] += d[0] == x; h[1] += d[1] == c; h[2] += d == (x, c)
    seg_n[u] += 1
    if d is None or hash((t, u)) % 16 == 0: seg_draw[u] = (x, c)
    m = seg_n[u]
    if m >= 64 and m % 32 == 0:
        hp, hn, hj = seg_hits[u]; lo, hi = min(hp, hn), max(hp, hn)
        if 20 * hi <= 19 * m and lo ** 3 * hi ** 2 > m ** 2 * hj ** 3: boundary.add(u)
        else: boundary.discard(u)
# ---- slow memory: query codes per (context, cue position, hop) as phasor sums ----
Q = {}; relia = defaultdict(lambda: 128); relbook = {}         # 8-bit running hit rate per route
CUE = 4
ans = {p: k for p, k in info['answers']}
newq = set(info.get('newq', [])); newf = set(info.get('newfact', []))
places, names = set(info['places']), set(info['names'])
seen = np.zeros(V, bool); results = []; clause = []
def ranked(sc, excl):
    """values by excess over their background (subtraction), the cue words left out"""
    s = sc.copy(); s[~seen] = -1e9; s[list(excl)] = -1e9
    pos = np.maximum(s, 0); bt = bg.sum() + 1; st = pos.sum() + 1e-9
    ex = pos * bt - bg * st; ex[s <= 0] = -1e18
    return int(np.argmax(ex)), float(ex.max())
for t in range(n):
    w = ids[t]
    ctx = tuple(ids[t - j] if t - j >= 0 else -1 for j in (1, 3, 4))
    cue = [ids[t - 1 - j] for j in range(CUE) if t - 1 - j >= 0]
    # ---- predict ----
    props = []
    for j, e in enumerate(cue):
        k1 = (ctx, j, 1)
        if k1 in Q and isinstance(Q[k1], np.ndarray):
            v, s = ranked(read(e, qcode(Q[k1])), cue)
            if s > 0: props.append((relia[k1], v, k1))
        k2 = (ctx, j, 2)
        if k2 in Q:
            qa, qb = Q[k2]
            m, s = ranked(read(e, qcode(qa)), [e])
            if s > 0:
                v, s2 = ranked(read(m, qcode(qb)), cue)
                if s2 > 0: props.append((relia[k2], v, k2))
    props.sort(key=lambda x: -x[0])
    if t in ans and t >= n // 2:
        kind = names if ans[t] == 'who-has' else places
        fk = next((v for _, v, _ in props if v in kind), None)
        results.append((ans[t], t in newq, t in newf, fk == w, bool(props) and props[0][1] == w))
    for _, v, k in props:
        relia[k] += ((255 if v == w else 0) - relia[k]) >> 4
    # ---- learn the query codes from the facts that would have produced w ----
    # hop 1: the trace unbound by the cue word and by w is the relation of any fact (e, R, w); it is added to the
    # context's hop-1 query as it is: true relations add up coherently over many examples, noise averages out
    # (the slow memory's many levels are what make this work). Hop 2 is learned where hop 1 is unreliable: the
    # intermediate m and the first relation are the best (relation, value) read from e over the most frequent
    # relations seen, the second relation is the evidence for (m, ?, w).
    if seen[w]:
        top_rels = sorted(relbook.values(), key=lambda x: -x[1])[:20]
        for j, e in enumerate(cue):
            if e == w or not seen[e]: continue
            idx, z = rel_evidence(e, w)
            k1 = (ctx, j, 1)
            acc = Q.get(k1)
            if acc is None: acc = np.zeros(DR, complex)
            np.add.at(acc, idx % DR, z); Q[k1] = acc
            if relia[k1] < 128 and top_rels:
                R = np.array([r for r, _ in top_rels])                       # nr x D
                ie, ze = unbind(e)
                Z = ze[None, :] * UNIT[(-R[:, ie % DR]) % 4]                  # nr x K
                Sc = np.real(Z @ np.conj(UNIT[VPH[:, ie]]).T)                 # nr x V
                Sc[:, ~seen] = -1e9; Sc[:, [e, w]] = -1e9
                ri, m = np.unravel_index(np.argmax(Sc), Sc.shape)
                if Sc[ri, m] > 0.3 * K:
                    i2, z2 = rel_evidence(int(m), w)
                    k2 = (ctx, j, 2); qa, qb = Q.get(k2, (np.zeros(DR, complex), np.zeros(DR, complex)))
                    qa = qa + UNIT[R[ri]]; qb = qb.copy(); np.add.at(qb, i2 % DR, z2); Q[k2] = (qa, qb)
    # ---- segmentation, then write the facts of w's clause into the fast trace ----
    if t >= 2: seg_update(ids[t - 2], ids[t - 1], w, t)
    path = []
    for s in reversed(clause[-8:]):
        if s != w:
            r = rel_code(path); write(s, r, w); write(w, back(r), s)
            for rr, key_ in ((r, ('f',) + tuple(path)), (back(r), ('b',) + tuple(path))):
                if key_ in relbook: relbook[key_][1] += 1
                else: relbook[key_] = [rr, 1]
        path.append(s)
        if s in boundary: break
    clause.append(w); clause = clause[-16:]; seen[w] = True
    # the context drifts: a few slots per word, more at a boundary
    nd = a.drift + (a.bdrift if w in boundary else 0)
    sl = rng.integers(0, K, nd); ctx_off[sl] = rng.integers(0, D, nd); ctx_ph[sl] = rng.integers(0, 4, nd)
print(f"{n} words, D={D} K={K} {'overwrite' if a.overwrite else 'ternary'}; boundaries: {' '.join(voc[b] for b in sorted(boundary))}; routes {len(Q)}")
for k in ('where-person', 'where-object', 'who-has', 'all'):
    sel = [r for r in results if k == 'all' or r[0] == k]
    print(f"  {k:<13} first of the right kind {np.mean([r[3] for r in sel]):6.1%}   top overall {np.mean([r[4] for r in sel]):6.1%}   ({len(sel)})")
for lab, i in (("new question form", 1), ("new verb in the fact", 2)):
    sel = [r for r in results if r[i]]
    if sel: print(f"  {lab:<22} first of the right kind {np.mean([r[3] for r in sel]):6.1%}  ({len(sel)})")
