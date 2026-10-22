"""Experiment 36: the forum's changes to the situation model, one switch each (scripts/exp35/e7_full.py is the base).

Base, as e7_full.py:
- clauses come from learned boundaries;
- each word is bound both ways to the earlier words of its clause, with the path between them pooled into
  relation classes by meaning (cosine 0.5);
- facts go into a ternary slot per (entity, class), with K = 64 active elements in a sparse trace of 65,536;
- a slow memory counts which class answers each context (C1), and which pair of classes answers in two hops (C2).

Switches:
  --mood      questions do not write. A clause's bindings are buffered until the boundary that closes it. Each
              boundary word has a 4-bit counter of "the store's top proposal was the next word": up by 1 when it was,
              down by 1 with probability 1/2 when it wasn't (a stochastic decrement, so it climbs whenever the store is
              right more than a third of the time; a symmetric counter only climbs above half). A clause
              closed by a boundary whose counter is in its upper half ("?") is a cue, not a fact, and its buffer is
              dropped.
  --story     (needs --kinds) the context is fully redrawn at a story start, and does not drift inside a story.
              Each word has a 4-bit novelty counter: 20 words after it, it steps up if fewer than a fifth of the
              content words that followed also came in the 20 before it, else steps down with probability 1/4 (a
              stochastic decrement, so a word that is followed by new entities a third of the time still climbs).
              Content = words of the kinds with at least 4 members. A word whose counter is in its upper half
              redraws all K context slots and starts a new epoch (old slots read as empty).
  --kinds     slots are keyed on (entity, class, kind of the value). Kinds are learned by
              - 4-bit counters per word over its frames (word before, two words before, word after), halved
                together when one saturates;
              - leader clustering of the words' frame codes (Jaccard >= 0.4), redone every 2,000 words; a kind is
                named by its most frequent word.
              C1 counts (class, kind); C2 counts pairs of them.
  --recency   among one cue word's proposals of the same kind, the most recently written first-hop slot wins
              (a rank within the entity); cue words are ranked by their best count.
  --compose   data bouncing at write time. When a fact (x, s1, y) is committed and (s1, s2) is among the --ncomp most
              counted two-hop pairs, y's s2 slot is read now and the result is written to x's composite slot: "emma
              dropped the cup" also stores where emma is now.
  --supersede detach: when x's slot of kind k is written under one class, x's other slots of kind k in the same
              direction are overwritten with NONE (a reserved value), so an entity keeps one live value per
              (direction, kind).
  --intpool   integer pooling: meaning as 4-bit counters read at a quarter of their maximum (a shift), a path's code
              as the OR of its words' codes, classes as 4-bit counter prototypes, joined at Jaccard >= --pool
              compared by cross-multiplication (no floats, no division). Prototypes are rebuilt every 2,000 words.
  --bounce    reasoning as a loop of reads instead of the C1/C2 tables. The question code is the context (the words 1,
              3 and 4 back). At each step the loop either stops, or reads one of the current entity's live slots and
              moves to the value read.
              - The choice of slot is learned as success counts per (question, cue position, step).
              - The stop is a 4-bit up/down counter per (question, kind of the current entity, step): stepped up
                where the answer was reached, down at an intermediate.
              - With --recency, the credible choices (at least half the best count) go newest first.
              - At most 3 steps.
  --table     the fact slots as a tagged table (one exact value per slot, a story epoch as its tag) instead of the
              sparse trace: the CPU seat's proposal for exact keys.
Diagnostics:
- scored on the top proposal (no kind filter), also kind-first for comparison;
- the oracle finds the question's entity by role (the name or object in the question), and asks whether the
  answer is in any of its occupied slots;
- --seed sets the codes, keys and context.
usage: sit2.py IDS INFO.json [--seed 3] [--mood] [--story] [--kinds] [--recency] [--compose] [--supersede]"""
import sys, json, argparse, time
import numpy as np
from collections import defaultdict, Counter, deque
ap = argparse.ArgumentParser(); ap.add_argument('ids'); ap.add_argument('info')
ap.add_argument('--D', type=int, default=65536); ap.add_argument('--K', type=int, default=64)
ap.add_argument('--N', type=int, default=40); ap.add_argument('--bdrift', type=int, default=1)
ap.add_argument('--pool', type=float, default=0.5); ap.add_argument('--seed', type=int, default=3)
for f in ('mood', 'story', 'kinds', 'recency', 'compose', 'supersede', 'oracle', 'table', 'intpool', 'bounce'): ap.add_argument('--' + f, action='store_true')
ap.add_argument('--ncomp', type=int, default=6); ap.add_argument('--recmin', type=float, default=0.5)
ap.add_argument('--kthr', type=float, default=0.4); ap.add_argument('--limit', type=int, default=0)
a = ap.parse_args()
t0 = time.time()
ids = np.fromfile(a.ids, dtype=np.uint32).astype(int).tolist()
if a.limit: ids = ids[:a.limit]
n = len(ids)
info = json.load(open(a.info)); voc = open(a.ids.replace('.ids', '.txt')).read().split()
D, K = a.D, a.K; V = len(voc); NONE = V                # one reserved value code past the vocabulary
rng = np.random.default_rng(a.seed)
VPH = rng.integers(0, 4, (V + 1, D)).astype(np.int8)
UNIT = np.array([1, 1j, -1, -1j]); CU = np.conj(UNIT[VPH]).astype(np.complex64)
DRE = np.array([1, 0, -1, 0]); DIM = np.array([0, 1, 0, -1])
re = np.zeros(D, np.int8); im = np.zeros(D, np.int8); bg = np.zeros(V + 1, np.int64)
ctx_off = rng.integers(0, D, K); ctx_ph = rng.integers(0, 4, K)
sig_id = {}; sig_count = Counter(); cls_count = Counter(); stamp = {}
# ---------------- meaning and pooling (base) ----------------
mean = np.zeros((V, 2 * V)); path_class = {}; protos = []
def mcode(x):
    m = mean[x]; mx = int(m.max()); return m >= max(1, mx >> 2)          # counters at a quarter of the max (shift)
def path_code(sig):
    path = sig[1:]
    if not path: return None
    if a.intpool:
        c = np.zeros(2 * V, bool)
        for x in path: c |= mcode(x)
        return c
    m = mean[list(path)]; m = m / (np.linalg.norm(m, axis=1, keepdims=True) + 1e-9)
    return m.sum(0)
PN, PD = int(round(a.pool * 100)), 100                 # pooling threshold as a ratio of integers
def cls(sig):
    if a.pool <= 0 or len(sig) == 1: return sig
    c = path_class.get(sig)
    if c is not None: return c
    v = path_code(sig); best_i, best_s = -1, None
    for i, (ps, name, mem) in enumerate(protos):
        if name[0] != sig[0]: continue
        if a.intpool:
            pr = ps >= max(1, int(ps.max()) >> 2); inter = int((pr & v).sum()); uni = int((pr | v).sum())
            # Jaccard >= threshold, compared by cross-multiplication; the best by inter * best_uni > best_inter * uni
            if uni and PD * inter >= PN * uni and (best_s is None or inter * best_s[1] > best_s[0] * uni):
                best_i, best_s = i, (inter, uni)
        else:
            sc = float(v @ ps) / (np.linalg.norm(v) * np.linalg.norm(ps) + 1e-9)
            if sc > (best_s if best_s is not None else a.pool): best_i, best_s = i, sc
    if best_i < 0: protos.append([v.astype(np.int64) if a.intpool else v.copy(), sig, 1]); c = sig
    else:
        if a.intpool:
            ps = protos[best_i][0]; ps += v
            if ps.max() > 15: ps >>= 1                    # 4-bit prototype counters, halved together
        else: protos[best_i][0] += v
        protos[best_i][2] += 1; c = protos[best_i][1]
    path_class[sig] = c; return c
def recluster():
    global protos, path_class
    paths = [sg for sg, _ in sig_count.most_common() if len(sg) > 1]
    protos, path_class = [], {}
    for sg in paths: cls(sg)
    cls_count.clear()
    for sg, c in sig_count.items(): cls_count[cls(sg)] += c
def sid(sig):
    if sig not in sig_id: sig_id[sig] = len(sig_id)
    return sig_id[sig]
# ---------------- kinds (learned value types) ----------------
FB = 512
frame = np.zeros((V, FB), np.int64); wcount = np.zeros(V, np.int64)
kind = np.full(V + 1, 127)                              # 127 = no kind yet (all one kind: the base)
def fbucket(*x): return hash(x) % FB
def frame_update(t):
    w = ids[t]
    feats = [fbucket(1, ids[t - 1])] if t >= 1 else []
    if t >= 2: feats.append(fbucket(2, ids[t - 2], ids[t - 1]))
    for f in feats: frame[w, f] += 1
    if t >= 1: frame[ids[t - 1], fbucket(3, w)] += 1
    for x in (w, ids[t - 1] if t >= 1 else w):
        if frame[x].max() > 15: frame[x] >>= 1          # 4-bit: halve together when one saturates
def recluster_kinds():
    order = [w for w in np.argsort(-wcount) if wcount[w] > 0]
    leaders = []; members = defaultdict(list)
    for w in order:
        c = frame[w] >= max(1, int(frame[w].max()) >> 2)
        for (lw, lc) in leaders:
            u = (c | lc).sum()
            if u and 5 * int((c & lc).sum()) >= 2 * int(u): members[lw].append(w); break   # Jaccard >= 2/5, cross-multiplied
        else: leaders.append((w, c)); members[w].append(w)
    # keep each kind's name across reclusterings: a cluster takes the old name most of its members had
    used = set(); new = {}
    for lw, (_, _) in sorted(((lw, (0, 0)) for lw, _ in leaders), key=lambda x: -len(members[x[0]])):
        old = Counter(int(kind[w]) for w in members[lw] if kind[w] != 127)
        name = next((o for o, _ in old.most_common() if o not in used), None)
        if name is None: name = lw if lw not in used else next(w for w in range(V) if w not in used)
        used.add(name); new[lw] = name
    for lw, ms in members.items():
        for w in ms: kind[w] = new[lw]
    return len(leaders)
nkinds = 1
def kk(v): return int(kind[v]) if a.kinds else 127
def slot(c_sid, k): return c_sid * 128 + k
# ---------------- the fast trace ----------------
_kc = {}; _kk = {}; ctx_ver = [0]; epoch = [0]
SALT = a.seed * 104729
def keyc(e, s):
    c = _kk.get((e, s))
    if c is not None and c[0] == ctx_ver[0]: return c[1], c[2]
    if (e, s) not in _kc:
        h = np.random.default_rng(e * 1_000_003 + s * 7919 + 17 + SALT); _kc[(e, s)] = (h.integers(0, D, K), h.integers(0, 4, K))
    i0, p0 = _kc[(e, s)]; I, P = (i0 + ctx_off) % D, (p0 + ctx_ph) % 4
    _kk[(e, s)] = (ctx_ver[0], I, P); return I, P
occ = defaultdict(dict)                                 # (entity, class) -> {kind: epoch}, the slots' valid bits
table = {}
def write(e, s, v, t, mag=1):
    if a.table:                                         # a tagged table: one exact cell per slot, story epoch as tag
        table[(e, s)] = (v, epoch[0]); bg[v] += 1; stamp[(e, s)] = t; return
    idx, kp = keyc(e, s); p = (kp + VPH[v, idx]) % 4
    re[idx] = np.clip(re[idx] + mag * DRE[p], -1, 1); im[idx] = np.clip(im[idx] + mag * DIM[p], -1, 1); bg[v] += 1
    stamp[(e, s)] = t
def read_many(e, ss):
    if a.table:
        out = np.zeros((len(ss), V + 1))
        for r, s in enumerate(ss):
            c = table.get((e, s))
            if c is not None and c[1] == epoch[0]: out[r, c[0]] = K
        return out
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
def live_kinds(e, c):
    return [k for k, ep in occ.get((e, c), {}).items() if ep == epoch[0]]
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
# ---------------- mood and story counters ----------------
mood = np.zeros(V, np.int64); nov = np.zeros(V, np.int64); pending = deque(); last_reset = [-99]; resets = [0]
rng_n = np.random.default_rng(a.seed + 1)
def content(x): return x in content_set
content_set = set()
# ---------------- slow memory ----------------
C1 = defaultdict(Counter); C2 = defaultdict(Counter); G2 = Counter(); comp_id = {}
H = defaultdict(Counter); HALT = defaultdict(lambda: 7)         # per-hop choice counts and the learned stop
CUE = 4
ans = {p: k for p, k in info['answers']}
newq = set(info.get('newq', [])); newf = set(info.get('newfact', []))
places, names, objects = set(info['places']), set(info['names']), set(info['objects'])
seen = np.zeros(V + 1, bool); results = []; clause = []; oracle = []; buf = []; dropped = [0, 0]
def composite(p1, p2):
    if (p1, p2) not in comp_id: comp_id[(p1, p2)] = sid(('c', p1, p2))
    return comp_id[(p1, p2)]
def commit(t):
    comps = [p for p, _ in G2.most_common(a.ncomp)] if a.compose else []
    for (x, c, d, v) in buf:
        k = kk(v); write(x, slot(c, k), v, t)
        if a.supersede:
            for (c_other, kinds_) in [(cc, occ[(x, cc)]) for cc in dclasses[(x, d)] if cc != c]:
                if kinds_.get(k) == epoch[0]:
                    write(x, slot(c_other, k), NONE, t, mag=2); del kinds_[k]
        occ[(x, c)][k] = epoch[0]; dclasses[(x, d)].add(c)
        for (p1, p2) in comps:
            if p1 != (c, k): continue
            (c2, k2) = p2
            m, xx = best(read_many(v, [slot(c2, k2)]), [v, x])
            if xx[0] > 0:
                cs = composite(p1, p2); write(x, slot(cs, k2), int(m[0]), t); occ[(x, cs)][k2] = epoch[0]
    buf.clear()
dclasses = defaultdict(set)
for t in range(n):
    w = ids[t]; wcount[w] += 1
    ctx = tuple(ids[t - j] if t - j >= 0 else -1 for j in (1, 3, 4))
    cue = [ids[t - 1 - j] for j in range(CUE) if t - 1 - j >= 0]
    # ---- predict ----
    props = []
    if a.bounce:
        for j, e in enumerate(cue):
            cur = e; lastc = 0
            for step in range(3):
                if step > 0 and HALT[(ctx, kk(cur), step)] >= 8:
                    props.append((lastc, cur)); break
                h = H.get((ctx, j, step))
                if not h: break
                live = [(c, k) for (c, k), n_ in h.most_common(6) if n_ >= 2 and (not a.kinds or k in live_kinds(cur, c))]
                if not live: break
                if a.recency:                              # credible choices (at least half the best count), newest first
                    top_c = max(h[x] for x in live); live = [x for x in live if 2 * h[x] >= top_c]
                    live.sort(key=lambda x: -stamp.get((cur, slot(*x)), -1))
                (c, k) = live[0]; lastc = h[(c, k)]
                v, x = best(read_many(cur, [slot(c, k)]), [cur] + cue)
                if x[0] <= 0 or int(v[0]) == NONE: break
                cur = int(v[0])
    for j, e in enumerate(cue if not a.bounce else []):
        cand = []                                         # (kind, first-hop recency, count, value)
        c1 = C1.get((ctx, j))
        if c1:
            ss = [s for s, c in c1.most_common(3) if c >= 2]
            if ss:
                v, x = best(read_many(e, [slot(c, k) for c, k in ss]), cue)
                for r, (c, k) in enumerate(ss):
                    if x[r] > 0 and int(v[r]) != NONE: cand.append((k, stamp.get((e, slot(c, k)), -1), c1[(c, k)], int(v[r])))
        c2 = C2.get((ctx, j))
        if c2:
            for ((c1_, k1), (c2_, k2)), c in c2.most_common(2):
                if c < 2: continue
                m, x = best(read_many(e, [slot(c1_, k1)]), [e])
                if x[0] > 0 and int(m[0]) != NONE:
                    v, x2 = best(read_many(int(m[0]), [slot(c2_, k2)]), cue)
                    if x2[0] > 0 and int(v[0]) != NONE: cand.append((k2, stamp.get((e, slot(c1_, k1)), -1), c, int(v[0])))
        if not cand: continue
        if a.recency:
            top_c = max(p[2] for p in cand); cand = [p for p in cand if 2 * p[2] >= top_c]      # at least half the best count
            bykind = defaultdict(list)
            for p in cand: bykind[p[0]].append(p)
            for k, ps in bykind.items():
                ps.sort(key=lambda p: (-p[1], -p[2]))
                props += [(max(p[2] for p in ps) - i * 1e-6, p[3]) for i, p in enumerate(ps)]
        else:
            props += [(p[2], p[3]) for p in cand]
    props.sort(key=lambda p: -p[0])
    if t in ans and t >= n // 2:
        if a.oracle:
            ent = next((x for x in ids[max(0, t - 6):t - 1][::-1] if x in names or x in objects), None)
            if ent is not None:
                ss = [slot(c, k) for (e_, c), kd in occ.items() if e_ == ent for k, ep in kd.items() if ep == epoch[0]]
                r1 = False
                if ss:
                    sc = read_many(ent, ss); sc[:, ~seen] = -1e9; r1 = bool((sc.argmax(1) == w).any())
                oracle.append((ans[t], r1))
        kd_ = names if ans[t] == 'who-has' else places
        fk = next((v for _, v in props if v in kd_), None)
        results.append((ans[t], t in newq, t in newf, fk == w, bool(props) and props[0][1] == w))
    # ---- mood: did the store predict the word after this boundary? ----
    if t >= 1 and ids[t - 1] in boundary:
        u = ids[t - 1]; ok = bool(props) and props[0][1] == w
        if ok: mood[u] = min(15, mood[u] + 1)
        elif rng_n.random() < 0.5: mood[u] = max(0, mood[u] - 1)           # stochastic decrement: climbs above 1/3 right
    # ---- learn which classes (and pairs) answer this context ----
    if seen[w]:
        top = [sid(s) for s, _ in cls_count.most_common(a.N)]
        if a.compose: top += list(comp_id.values())
        kw = kk(w)
        for j, e in enumerate(cue):
            if e == w or not seen[e] or not top: continue
            v, x = best(read_many(e, [slot(c, kw) for c in top]), [e])
            hits = [top[r] for r in range(len(top)) if v[r] == w and x[r] > 0]
            for c in hits: C1[(ctx, j)][(c, kw)] += 1
            if a.bounce and hits:
                for c in hits: H[(ctx, j, 0)][(c, kw)] += 1
                hk = (ctx, kw, 1); HALT[hk] = min(15, HALT[hk] + 1)
            if not hits:
                t10 = top[:10]
                for c in t10:
                    for k1 in (live_kinds(e, c) if a.kinds else [127]):
                        m, xm = best(read_many(e, [slot(c, k1)]), [e]); m = int(m[0])
                        if xm[0] <= 0 or m == w or m == NONE: continue
                        v2, x2 = best(read_many(m, [slot(c2, kw) for c2 in t10]), [m])
                        for r2, c2 in enumerate(t10):
                            if v2[r2] == w and x2[r2] > 0:
                                p = ((c, k1), (c2, kw)); C2[(ctx, j)][p] += 1; G2[p] += 1
                                if a.bounce:
                                    H[(ctx, j, 0)][(c, k1)] += 1; H[(ctx, j, 1)][(c2, kw)] += 1
                                    h1 = (ctx, kk(m), 1); HALT[h1] = max(0, HALT[h1] - 1)   # m was not the answer
                                    h2 = (ctx, kw, 2); HALT[h2] = min(15, HALT[h2] + 1)
    # ---- segmentation, kinds, novelty ----
    if t >= 2: seg_update(ids[t - 2], ids[t - 1], w, t)
    if a.kinds: frame_update(t)
    if a.story:
        pending.append((t, w, set(x for x in ids[max(0, t - 20):t] if content(x))))
        while pending and pending[0][0] <= t - 20:
            t_, u, before = pending.popleft()
            after = set(x for x in ids[t_ + 1:t_ + 21] if content(x))
            if after:
                if 5 * len(before & after) < len(after): nov[u] = min(15, nov[u] + 1)
                elif rng_n.random() < 0.25: nov[u] = max(0, nov[u] - 1)      # stochastic decrement
    # ---- the facts of w's clause, buffered until its boundary ----
    path = []
    for s in reversed(clause[-8:]):
        if s != w:
            fs_, bs_ = ('f',) + tuple(path), ('b',) + tuple(path)
            for sg in (fs_, bs_): sig_count[sg] += 1; cls_count[cls(sg)] += 1
            buf.append((s, sid(cls(fs_)), 'f', w)); buf.append((w, sid(cls(bs_)), 'b', s))
        path.append(s)
        if s in boundary: break
    clause.append(w); clause = clause[-16:]; seen[w] = True
    if w in boundary or len(buf) > 64:
        if a.mood and w in boundary and mood[w] >= 8: dropped[0] += 1; buf.clear()
        else: dropped[1] += 1; commit(t)
    if t >= 1:
        mean[w, ids[t - 1]] += 1; mean[ids[t - 1], V + w] += 1
        if a.intpool:
            for x in (w, ids[t - 1]):
                if mean[x].max() > 15: mean[x] //= 2                     # 4-bit meaning counters (a shift)
    if t % 2000 == 1999:
        if a.pool > 0: recluster()
        if a.kinds:
            nkinds = recluster_kinds()
            sizes = Counter(int(kind[x]) for x in range(V) if wcount[x])
            content_set = set(x for x in range(V) if wcount[x] and sizes[int(kind[x])] >= 4)   # words of the big kinds
    if a.story and nov[w] >= 8 and t - last_reset[0] > 5:
        ctx_off[:] = rng.integers(0, D, K); ctx_ph[:] = rng.integers(0, 4, K); ctx_ver[0] += 1; epoch[0] += 1
        last_reset[0] = t; resets[0] += 1
    elif not a.story and w in boundary and a.bdrift:
        sl = rng.integers(0, K, a.bdrift); ctx_off[sl] = rng.integers(0, D, a.bdrift); ctx_ph[sl] = rng.integers(0, 4, a.bdrift); ctx_ver[0] += 1
flags = ' '.join(f for f in ('mood', 'story', 'kinds', 'recency', 'compose', 'supersede', 'table', 'intpool', 'bounce') if getattr(a, f)) + f' pool {a.pool}'
print(f"{n} words, {time.time() - t0:.0f} s, seed {a.seed} [{flags}]; boundaries: {' '.join(voc[b] for b in sorted(boundary))}; classes {len(set(cls(x) for x in sig_count))}, kinds {nkinds}, composites {len(comp_id)}")
qb = [voc[u] for u in sorted(boundary) if mood[u] >= 8]
print(f"  question boundaries: {' '.join(qb) or '-'}; clauses committed {dropped[1]}, dropped {dropped[0]}; story resets {resets[0]} (story words: {' '.join(voc[u] for u in range(V) if nov[u] >= 8) or '-'})")
if a.kinds:
    groups = defaultdict(list)
    for x in range(V):
        if wcount[x]: groups[int(kind[x])].append(voc[x])
    print("  kinds: " + " | ".join(' '.join(g) for g in sorted(groups.values(), key=len, reverse=True)))
for k in ('where-person', 'where-object', 'who-has', 'all'):
    sel = [r for r in results if k == 'all' or r[0] == k]
    print(f"  {k:<13} top {np.mean([r[4] for r in sel]):6.1%}   first of the right kind {np.mean([r[3] for r in sel]):6.1%}   ({len(sel)})")
for lab, i in (("new question form", 1), ("new verb in the fact", 2)):
    sel = [r for r in results if r[i]]
    if sel: print(f"  {lab:<22} top {np.mean([r[4] for r in sel]):6.1%}  ({len(sel)})")
if oracle:
    for k in ('where-person', 'where-object', 'who-has'):
        sel = [o for o in oracle if o[0] == k]
        if sel: print(f"  answer in one of the entity's live slots, {k:<13}: {np.mean([o[1] for o in sel]):6.1%}")
