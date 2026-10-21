"""A learned situation model on ternary memory, one slot per (entity, relation) (experiment 34).

As scripts/situation.py, but a fact (s, relation, w) is written at active elements chosen from the entity *and*
the relation together, so each entity has a slot per relation and a new fact in a slot replaces the old one
(ternary: a write adds +-m to the real or imaginary part and clips to -1, 0, +1; m = 1 lets a conflicting write
cancel to 0, m = 2 makes the newest sign win). Relations are the exact paths of words between the pair (with a
direction), so synonyms are different relations: which relations answer which question is learned, from
consistency, in a slow many-level memory: counts per (context, cue position, relation), or per pair of relations
for two hops. When the next word w arrives, the store reads each cue word's slots for the most frequent relations;
a relation whose slot holds w (it ranks first) earns a count for that context. Prediction reads the slots of the
context's best-counted relations. The slots sit in a slowly drifting context (a few of each key's element offsets
redrawn per word), so old stories fade.
usage: situation_slots.py IDS INFO.json [--D 65536] [--K 64] [--mag 1] [--N 40]"""
import sys, json, argparse
import numpy as np
from collections import defaultdict, Counter
ap = argparse.ArgumentParser(); ap.add_argument('ids'); ap.add_argument('info')
ap.add_argument('--D', type=int, default=65536); ap.add_argument('--K', type=int, default=64)
ap.add_argument('--mag', type=int, default=1); ap.add_argument('--N', type=int, default=40)
ap.add_argument('--drift', type=int, default=2); ap.add_argument('--bdrift', type=int, default=8)
a = ap.parse_args()
ids = np.fromfile(a.ids, dtype=np.uint32).astype(int).tolist(); n = len(ids)
info = json.load(open(a.info)); voc = open(a.ids.replace('.ids', '.txt')).read().split()
D, K = a.D, a.K; V = max(ids) + 1
rng = np.random.default_rng(3)
VPH = rng.integers(0, 4, (V, D)).astype(np.int8)
UNIT = np.array([1, 1j, -1, -1j]); DRE = np.array([1, 0, -1, 0]); DIM = np.array([0, 1, 0, -1])
re = np.zeros(D, np.int8); im = np.zeros(D, np.int8); bg = np.zeros(V, np.int64)
ctx_off = rng.integers(0, D, K); ctx_ph = rng.integers(0, 4, K)
sig_id = {}; sig_count = Counter()
def sid(sig):
    if sig not in sig_id: sig_id[sig] = len(sig_id)
    return sig_id[sig]
def key(e, s):
    h = np.random.default_rng(e * 1_000_003 + s * 7919 + 17)
    return (h.integers(0, D, K) + ctx_off) % D, (h.integers(0, 4, K) + ctx_ph) % 4
_kc = {}
def keyc(e, s):
    """keys cached per (entity, relation); the context offsets are added at use"""
    if (e, s) not in _kc:
        h = np.random.default_rng(e * 1_000_003 + s * 7919 + 17); _kc[(e, s)] = (h.integers(0, D, K), h.integers(0, 4, K))
    i0, p0 = _kc[(e, s)]; return (i0 + ctx_off) % D, (p0 + ctx_ph) % 4
def write(e, s, v):
    idx, kp = keyc(e, s); p = (kp + VPH[v, idx]) % 4
    re[idx] = np.clip(re[idx] + a.mag * DRE[p], -1, 1); im[idx] = np.clip(im[idx] + a.mag * DIM[p], -1, 1); bg[v] += 1
def read_many(e, ss):
    """scores of every value in e's slots for relations ss: len(ss) x V"""
    I = np.empty((len(ss), K), int); P = np.empty((len(ss), K), int)
    for r, s in enumerate(ss): I[r], P[r] = keyc(e, s)
    z = (re[I] + 1j * im[I]) * UNIT[(-P) % 4]                         # unbound, len(ss) x K
    return np.real(np.einsum('rk,vrk->rv', z, np.conj(UNIT[VPH[:, I]])))
def best(sc, excl):
    """per row: the value with the largest excess over its background (subtraction), and that excess"""
    s = sc.copy(); s[:, ~seen] = -1e9
    if excl: s[:, list(excl)] = -1e9
    pos = np.maximum(s, 0); bt = bg.sum() + 1; st = pos.sum(1, keepdims=True) + 1e-9
    ex = pos * bt - bg[None, :] * st; ex[s <= 0] = -1e18
    return ex.argmax(1), ex.max(1)
# learned segmentation (experiment 32)
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
# slow memory: counts per (context, cue position) over relations (hop 1) and relation pairs (hop 2)
C1 = defaultdict(Counter); C2 = defaultdict(Counter)
CUE = 4
ans = {p: k for p, k in info['answers']}
newq = set(info.get('newq', [])); newf = set(info.get('newfact', []))
places, names = set(info['places']), set(info['names'])
seen = np.zeros(V, bool); results = []; clause = []
for t in range(n):
    w = ids[t]
    ctx = tuple(ids[t - j] if t - j >= 0 else -1 for j in (1, 3, 4))
    cue = [ids[t - 1 - j] for j in range(CUE) if t - 1 - j >= 0]
    # ---- predict: each cue word's slots for the context's best-counted relations ----
    props = []
    for j, e in enumerate(cue):
        c1 = C1.get((ctx, j))
        if c1:
            ss = [s for s, c in c1.most_common(3) if c >= 2]
            if ss:
                v, x = best(read_many(e, ss), cue)
                for r, s in enumerate(ss):
                    if x[r] > 0: props.append((c1[s], int(v[r])))
        c2 = C2.get((ctx, j))
        if c2:
            for (s1, s2), c in c2.most_common(2):
                if c < 2: continue
                m, x = best(read_many(e, [s1]), [e])
                if x[0] > 0:
                    v, x2 = best(read_many(int(m[0]), [s2]), cue)
                    if x2[0] > 0: props.append((c, int(v[0])))
    props.sort(key=lambda p: -p[0])
    if t in ans and t >= n // 2:
        kind = names if ans[t] == 'who-has' else places
        fk = next((v for _, v in props if v in kind), None)
        results.append((ans[t], t in newq, t in newf, fk == w, bool(props) and props[0][1] == w))
    # ---- learn which relations answer this context: relations whose slot holds w ----
    if seen[w]:
        top = [sig_id[s] for s, _ in sig_count.most_common(a.N)]
        for j, e in enumerate(cue):
            if e == w or not seen[e] or not top: continue
            v, x = best(read_many(e, top), [e])
            hits = [top[r] for r in range(len(top)) if v[r] == w and x[r] > 0]
            for s in hits: C1[(ctx, j)][s] += 1
            if not hits:
                # two hops: through the best value of each of the 10 most frequent relations
                t10 = top[:10]; ms = [int(v[r]) for r in range(len(t10)) if x[r] > 0]
                for r, m in zip(range(len(t10)), [int(v[r]) for r in range(len(t10))]):
                    if x[r] <= 0 or m == w: continue
                    v2, x2 = best(read_many(m, t10), [m])
                    for r2 in range(len(t10)):
                        if v2[r2] == w and x2[r2] > 0: C2[(ctx, j)][(t10[r], t10[r2])] += 1
    # ---- segmentation, then the facts of w's clause, each in its (entity, relation) slot ----
    if t >= 2: seg_update(ids[t - 2], ids[t - 1], w, t)
    path = []
    for s in reversed(clause[-8:]):
        if s != w:
            fwd = sid(('f',) + tuple(path)); bwd = sid(('b',) + tuple(path))
            write(s, fwd, w); write(w, bwd, s); sig_count[('f',) + tuple(path)] += 1; sig_count[('b',) + tuple(path)] += 1
        path.append(s)
        if s in boundary: break
    clause.append(w); clause = clause[-16:]; seen[w] = True
    nd = a.drift + (a.bdrift if w in boundary else 0)
    sl = rng.integers(0, K, nd); ctx_off[sl] = rng.integers(0, D, nd); ctx_ph[sl] = rng.integers(0, 4, nd)
print(f"{n} words, D={D} K={K} mag={a.mag}; boundaries: {' '.join(voc[b] for b in sorted(boundary))}; relations {len(sig_id)}")
for k in ('where-person', 'where-object', 'who-has', 'all'):
    sel = [r for r in results if k == 'all' or r[0] == k]
    print(f"  {k:<13} first of the right kind {np.mean([r[3] for r in sel]):6.1%}   top overall {np.mean([r[4] for r in sel]):6.1%}   ({len(sel)})")
for lab, i in (("new question form", 1), ("new verb in the fact", 2)):
    sel = [r for r in results if r[i]]
    if sel: print(f"  {lab:<22} first of the right kind {np.mean([r[3] for r in sel]):6.1%}  ({len(sel)})")
