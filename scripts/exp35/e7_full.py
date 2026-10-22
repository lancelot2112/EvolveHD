"""E7: the situation model as one loop of vectors, with the pieces the thought experiments pointed to (experiment 35).
Base: scripts/situation_slots.py (learned clauses; facts in one ternary slot per (entity, relation class); a slow
memory of full counts learning which classes answer each context, and class pairs for two hops). Switches:
  --recency   each slot keeps the time it was last written (one 16-bit cell per slot: E6b's winner). For each cue
              word, the proposals from its classes are ranked by the recency of the cue word's own slot (the first
              hop), then by count: the cue's newest relation wins over a stale one. Cue words are ranked by their
              best count, as in the base. --recmin r: only proposals with at least r of the cue word's best
              count compete on recency (Select proposes, recency arbitrates).
  --both      learn the one-hop and two-hop counts at every answer (the base learns two-hop only when no one-hop
              class held the word).
  --compose   data bouncing at write time: when a fact (x, s1, y) is written and (s1, s2) is one of the most
              counted two-hop pairs, y's (s2) slot is read now and the result written into x's composite slot
              (s1.s2): "emma dropped the cup" also stores where emma is at that moment.
  --bitproto  meaning and relation classes from bit-local counters: each word's meaning is 4-bit counters over its
              left and right neighbours (halved together when one saturates); its code is the counters at a
              quarter of the largest or more. A path's code is the OR of its words' codes; a class is 4-bit
              counters over the codes of the paths assigned to it, read the same way; a path joins the class it
              overlaps most (Jaccard) if at least --pool, else starts one. No reclustering: path assignments are
              looked up again every 2,000 words, the prototypes keep their counts.
usage: e7_full.py IDS INFO.json [--pool 0.5] [--recency] [--both] [--compose] [--bitproto]"""
import sys, json, argparse, time
import numpy as np
from collections import defaultdict, Counter
ap = argparse.ArgumentParser(); ap.add_argument('ids'); ap.add_argument('info')
ap.add_argument('--D', type=int, default=65536); ap.add_argument('--K', type=int, default=64)
ap.add_argument('--mag', type=int, default=1); ap.add_argument('--N', type=int, default=40)
ap.add_argument('--drift', type=int, default=0); ap.add_argument('--bdrift', type=int, default=1)
ap.add_argument('--pool', type=float, default=0.5)
ap.add_argument('--recency', action='store_true'); ap.add_argument('--both', action='store_true')
ap.add_argument('--compose', action='store_true'); ap.add_argument('--bitproto', action='store_true')
ap.add_argument('--ncomp', type=int, default=6, help='how many of the most counted two-hop pairs are composed')
ap.add_argument('--limit', type=int, default=0)
ap.add_argument('--oracle', action='store_true', help='at answer positions, also test whether the answer is reachable in one or two reads from the entity, over every class')
ap.add_argument('--recmin', type=float, default=0.0, help='with --recency: only proposals whose count is at least this share of the cue word\'s best count')
a = ap.parse_args()
t0 = time.time()
ids = np.fromfile(a.ids, dtype=np.uint32).astype(int).tolist()
if a.limit: ids = ids[:a.limit]
n = len(ids)
info = json.load(open(a.info)); voc = open(a.ids.replace('.ids', '.txt')).read().split()
D, K = a.D, a.K; V = len(voc)
rng = np.random.default_rng(3)
VPH = rng.integers(0, 4, (V, D)).astype(np.int8)
UNIT = np.array([1, 1j, -1, -1j]); CU = np.conj(UNIT[VPH]).astype(np.complex64); DRE = np.array([1, 0, -1, 0]); DIM = np.array([0, 1, 0, -1])
re = np.zeros(D, np.int8); im = np.zeros(D, np.int8); bg = np.zeros(V, np.int64)
ctx_off = rng.integers(0, D, K); ctx_ph = rng.integers(0, 4, K)
sig_id = {}; sig_count = Counter(); cls_count = Counter(); stamp = {}
# ---------------- meaning and pooling ----------------
mean = np.zeros((V, 2 * V))                            # float counts (base) or 4-bit counters (--bitproto)
path_class = {}; protos = []                           # protos: [code (float sum or 4-bit counters), name, members]
def meaning_code(w):
    m = mean[w]; mx = m.max()
    return (m >= max(1, mx / 4)) if mx > 0 else np.zeros(2 * V, bool)
def path_code(sig):
    path = sig[1:]
    if not path: return None
    if a.bitproto:
        c = np.zeros(2 * V, bool)
        for x in path: c |= meaning_code(x)
        return c
    m = mean[list(path)]; m = m / (np.linalg.norm(m, axis=1, keepdims=True) + 1e-9)
    return m.sum(0)
def proto_read(ps):
    mx = ps.max(); return ps >= max(1, mx / 4)
def cls(sig):
    if a.pool <= 0 or len(sig) == 1: return sig
    c = path_class.get(sig)
    if c is not None: return c
    v = path_code(sig); best_i, best_s = -1, a.pool
    for i, (ps, name, mem) in enumerate(protos):
        if name[0] != sig[0]: continue
        if a.bitproto:
            pr = proto_read(ps); u = (pr | v).sum()
            sc = (pr & v).sum() / u if u else 0.0
        else:
            sc = float(v @ ps) / (np.linalg.norm(v) * np.linalg.norm(ps) + 1e-9)
        if sc > best_s: best_i, best_s = i, sc
    if best_i < 0:
        protos.append([v.astype(np.int64) if a.bitproto else v.copy(), sig, 1]); c = sig
    else:
        if a.bitproto:
            ps = protos[best_i][0]; ps += v
            if ps.max() > 15: ps >>= 1                      # 4-bit: halve all together when one saturates
        else: protos[best_i][0] += v
        protos[best_i][2] += 1; c = protos[best_i][1]
    path_class[sig] = c; return c
def recluster():
    global protos, path_class
    if a.bitproto:
        path_class = {}                                  # look the paths up again; prototypes keep their counts
    else:
        paths = [sg for sg, _ in sig_count.most_common() if len(sg) > 1]
        protos, path_class = [], {}
        for sg in paths: cls(sg)
    cls_count.clear()
    for sg, c in sig_count.items(): cls_count[cls(sg)] += c
def sid(sig):
    if sig not in sig_id: sig_id[sig] = len(sig_id)
    return sig_id[sig]
_kc = {}; _kk = {}; ctx_ver = [0]
def keyc(e, s):
    c = _kk.get((e, s))
    if c is not None and c[0] == ctx_ver[0]: return c[1], c[2]
    if (e, s) not in _kc:
        h = np.random.default_rng(e * 1_000_003 + s * 7919 + 17); _kc[(e, s)] = (h.integers(0, D, K), h.integers(0, 4, K))
    i0, p0 = _kc[(e, s)]; I, P = (i0 + ctx_off) % D, (p0 + ctx_ph) % 4
    _kk[(e, s)] = (ctx_ver[0], I, P); return I, P
def write(e, s, v, t):
    idx, kp = keyc(e, s); p = (kp + VPH[v, idx]) % 4
    re[idx] = np.clip(re[idx] + a.mag * DRE[p], -1, 1); im[idx] = np.clip(im[idx] + a.mag * DIM[p], -1, 1); bg[v] += 1
    stamp[(e, s)] = t
def read_many(e, ss):
    I = np.empty((len(ss), K), int); P = np.empty((len(ss), K), int)
    for r, s in enumerate(ss): I[r], P[r] = keyc(e, s)
    z = ((re[I] + 1j * im[I]) * UNIT[(-P) % 4]).astype(np.complex64)
    return np.real(np.einsum('rk,vrk->rv', z, CU[:, I]))
def best(sc, excl):
    s = sc.copy(); s[:, ~seen] = -1e9
    if excl: s[:, list(excl)] = -1e9
    pos = np.maximum(s, 0); bt = bg.sum() + 1; st = pos.sum(1, keepdims=True) + 1e-9
    ex = pos * bt - bg[None, :] * st; ex[s <= 0] = -1e18
    return ex.argmax(1), ex.max(1)
# ---------------- learned segmentation (experiment 32) ----------------
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
# ---------------- slow memory ----------------
C1 = defaultdict(Counter); C2 = defaultdict(Counter); G2 = Counter(); comp_id = {}
CUE = 4
ans = {p: k for p, k in info['answers']}
newq = set(info.get('newq', [])); newf = set(info.get('newfact', []))
places, names = set(info['places']), set(info['names'])
seen = np.zeros(V, bool); results = []; clause = []; oracle = []
def composite(s1, s2):
    if (s1, s2) not in comp_id: comp_id[(s1, s2)] = sid(('c', s1, s2))
    return comp_id[(s1, s2)]
for t in range(n):
    w = ids[t]
    ctx = tuple(ids[t - j] if t - j >= 0 else -1 for j in (1, 3, 4))
    cue = [ids[t - 1 - j] for j in range(CUE) if t - 1 - j >= 0]
    # ---- predict ----
    props = []
    for j, e in enumerate(cue):
        cand = []                                          # (first-hop recency, count, value)
        c1 = C1.get((ctx, j))
        if c1:
            ss = [s for s, c in c1.most_common(3) if c >= 2]
            if ss:
                v, x = best(read_many(e, ss), cue)
                for r, s in enumerate(ss):
                    if x[r] > 0: cand.append((stamp.get((e, s), -1), c1[s], int(v[r])))
        c2 = C2.get((ctx, j))
        if c2:
            for (s1, s2), c in c2.most_common(2):
                if c < 2: continue
                m, x = best(read_many(e, [s1]), [e])
                if x[0] > 0:
                    v, x2 = best(read_many(int(m[0]), [s2]), cue)
                    if x2[0] > 0: cand.append((stamp.get((e, s1), -1), c, int(v[0])))
        if a.recency and cand:
            # this cue word's proposals, newest first-hop slot first, ranked among cue words by their best count
            top_c = max(c for _, c, _ in cand); cand = [p for p in cand if p[1] >= a.recmin * top_c]
            cand.sort(key=lambda p: (-p[0], -p[1]))
            props += [(top_c - i * 1e-6, v) for i, (_, _, v) in enumerate(cand)]
        else:
            props += [(c, v) for _, c, v in cand]
    props.sort(key=lambda p: -p[0])
    if a.oracle and t in ans and t >= n // 2 and len(cue) > 1:
        e = cue[1]; top = [sid(s_) for s_, _ in cls_count.most_common(a.N)]
        v, x = best(read_many(e, top), [e]); r1 = any(v[r] == w and x[r] > 0 for r in range(len(top)))
        r2 = False
        if not r1:
            for r in range(len(top)):
                if x[r] <= 0: continue
                v2, x2 = best(read_many(int(v[r]), top), [int(v[r])])
                if any(v2[q] == w and x2[q] > 0 for q in range(len(top))): r2 = True; break
        oracle.append((ans[t], r1, r2))
    if t in ans and t >= n // 2:
        kind = names if ans[t] == 'who-has' else places
        fk = next((v for _, v in props if v in kind), None)
        results.append((ans[t], t in newq, t in newf, fk == w, bool(props) and props[0][1] == w))
    # ---- learn which classes (and pairs) answer this context ----
    if seen[w]:
        top = [sid(s) for s, _ in cls_count.most_common(a.N)]
        if a.compose: top += list(comp_id.values())
        for j, e in enumerate(cue):
            if e == w or not seen[e] or not top: continue
            v, x = best(read_many(e, top), [e])
            hits = [top[r] for r in range(len(top)) if v[r] == w and x[r] > 0]
            for s in hits: C1[(ctx, j)][s] += 1
            if not hits or a.both:
                t10 = top[:10]
                for r in range(len(t10)):
                    m = int(v[r])
                    if x[r] <= 0 or m == w: continue
                    v2, x2 = best(read_many(m, t10), [m])
                    for r2 in range(len(t10)):
                        if v2[r2] == w and x2[r2] > 0:
                            C2[(ctx, j)][(t10[r], t10[r2])] += 1; G2[(t10[r], t10[r2])] += 1
    # ---- segmentation, then the facts of w's clause ----
    if t >= 2: seg_update(ids[t - 2], ids[t - 1], w, t)
    path = []; comps = [p for p, _ in G2.most_common(a.ncomp)] if a.compose else []
    for s in reversed(clause[-8:]):
        if s != w:
            fs_, bs_ = ('f',) + tuple(path), ('b',) + tuple(path)
            for sg in (fs_, bs_): sig_count[sg] += 1; cls_count[cls(sg)] += 1
            fwd = sid(cls(fs_)); bwd = sid(cls(bs_))
            write(s, fwd, w, t); write(w, bwd, s, t)
            for (x_, s1, y_) in ((s, fwd, w), (w, bwd, s)):
                for (p1, p2) in comps:
                    if p1 != s1: continue
                    m, xx = best(read_many(y_, [p2]), [y_, x_])
                    if xx[0] > 0: write(x_, composite(p1, p2), int(m[0]), t)
        path.append(s)
        if s in boundary: break
    clause.append(w); clause = clause[-16:]; seen[w] = True
    if a.bitproto:
        for (row, col) in ((w, ids[t - 1]), (ids[t - 1], V + w)) if t >= 1 else ():
            mean[row, col] += 1
            if mean[row].max() > 15: mean[row] = np.floor(mean[row] / 2)
    elif t >= 1:
        mean[w, ids[t - 1]] += 1; mean[ids[t - 1], V + w] += 1
    if a.pool > 0 and t % 2000 == 1999: recluster()
    nd = a.drift + (a.bdrift if w in boundary else 0)
    if nd:
        sl = rng.integers(0, K, nd); ctx_off[sl] = rng.integers(0, D, nd); ctx_ph[sl] = rng.integers(0, 4, nd); ctx_ver[0] += 1
flags = ' '.join(f for f in ('recency', 'both', 'compose', 'bitproto') if getattr(a, f)) + (f' recmin {a.recmin}' if a.recmin else '')
print(f"{n} words, {time.time() - t0:.0f} s, pool={a.pool} [{flags}]; boundaries: {' '.join(voc[b] for b in sorted(boundary))}; paths {len(sig_count)}, classes {len(set(cls(x) for x in sig_count))}, composites {len(comp_id)}")
for k in ('where-person', 'where-object', 'who-has', 'all'):
    sel = [r for r in results if k == 'all' or r[0] == k]
    print(f"  {k:<13} first of the right kind {np.mean([r[3] for r in sel]):6.1%}   top overall {np.mean([r[4] for r in sel]):6.1%}   ({len(sel)})")
for lab, i in (("new question form", 1), ("new verb in the fact", 2)):
    sel = [r for r in results if r[i]]
    if sel: print(f"  {lab:<22} first of the right kind {np.mean([r[3] for r in sel]):6.1%}  ({len(sel)})")
if oracle:
    for k in ('where-person', 'where-object', 'who-has'):
        sel = [o for o in oracle if o[0] == k]
        print(f"  reachable from the entity, {k:<13}: one read {np.mean([o[1] for o in sel]):6.1%}   within two {np.mean([o[1] or o[2] for o in sel]):6.1%}")
