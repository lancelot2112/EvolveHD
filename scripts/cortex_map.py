"""A growing cortex map and a compressed story code (experiment 26). One pass, prequential: every word is
predicted before anything learns from it.

Stage 1, the map. A line of spots that starts empty and grows. A content word (id >= 100) gets a spot on first
sight, inserted next to a recent story word drawn by salience (so a word first met at sea lands in the sea region).
Each later occurrence draws a context spot the same way and moves the word's spot part of the way toward it
(eta = max(eta_min, 0.5/sqrt(occurrences))), shifting the spots in between one place each: a run of swaps. A word
whose recent pulls split into two far-apart groups gets a second spot at the other group (a sense); an occurrence
uses the spot nearest its pull.
Stage 2, the compression. The story state lights spots (salience decaying with a half-life of 200 content words);
D bits pool the line in two offset banks of windows (each spot feeds 2 bits); the code is the k most lit bits.
Two readouts, both predicting the exact next word:
  q_bits  rows keyed by the code's bits, each counting the next word (learned)
  q_reg   the lit regions name their own words: each code bit spreads its share over the words whose spots it pools,
          by frequency so far (no learning beyond the map; a snapshot of the map every 256 words)
Scored on exact-word bits beyond the experiment-25 counts system (cortex_base.py), as mixtures whose weights are
chosen on the first fifth of the second half and reported on the rest.
usage: cortex_map.py IDS WORDS.txt BASE.npz OUT [--D 2048] [--k 32] [--eta-min 0.02] [--senses 2]
       [--place context|random] [--move 1] [--report 1]"""
import sys, math, random, argparse, json
import numpy as np
from collections import deque
ap = argparse.ArgumentParser()
ap.add_argument('ids'); ap.add_argument('words'); ap.add_argument('base'); ap.add_argument('out')
ap.add_argument('--D', type=int, default=2048); ap.add_argument('--k', type=int, default=32)
ap.add_argument('--eta-min', type=float, default=0.02); ap.add_argument('--senses', type=int, default=2)
ap.add_argument('--place', default='context'); ap.add_argument('--move', type=int, default=1)
ap.add_argument('--target', default='draw')   # draw: one context spot drawn by salience; median: the salience-weighted median of the lit spots
ap.add_argument('--mutual', type=float, default=0.0)
# assoc: the cortex keeps association links (counts of each content word with the 16 most lit words when it
# occurs); a word moves toward the weighted median position of its 8 strongest links by PMI (2+ co-occurrences)
ap.add_argument('--links', type=int, default=16)  # also move the drawn context spot toward the word, at this fraction of eta
ap.add_argument('--report', type=int, default=1); ap.add_argument('--half', type=float, default=200.0)
A = ap.parse_args()
rng = random.Random(1); nrng = np.random.default_rng(1)
ids = np.fromfile(A.ids, dtype=np.uint32).astype(np.int64); n = len(ids)
words = open(A.words, encoding='latin-1').read().split('\n')
B = np.load(A.base); p_base, q_story, u_p = B['p_base'], B['q_story'], B['u_p']
NW = int(ids.max()) + 1; SKIP = 100; D = A.D; K = A.k; HB = D // 2; BETA = 256.0
# ---- the map
cap = 1 << 16
order = np.zeros(cap, dtype=np.int64)          # spot id at each position
spos = np.zeros(cap, dtype=np.int64)           # position of each spot
sword = np.zeros(cap, dtype=np.int64)          # word of each spot
N = 0; S = 0                                   # spots placed; spot ids used
wspots = {}                                    # word -> [spot ids]
fc = np.zeros(NW, dtype=np.int64)              # occurrences so far
res = {}                                       # word -> recent pulls (as fractions of the line)
nsplit = 0; moves = 0; moved = 0
def grow():
    global cap, order, spos, sword
    cap *= 2
    for name in ('order', 'spos', 'sword'):
        a = globals()[name]; b = np.zeros(cap, dtype=np.int64); b[:len(a)] = a; globals()[name] = b
def insert(w, q):
    global N, S
    if S >= cap or N >= cap: grow()
    s = S; S += 1
    q = min(max(q, 0), N)
    order[q + 1:N + 1] = order[q:N].copy(); spos[order[q + 1:N + 1]] += 1
    order[q] = s; spos[s] = q; sword[s] = w; N += 1
    wspots.setdefault(w, []).append(s)
    return s
def move(s, q):
    p = spos[s]; q = min(max(q, 0), N - 1)
    if q < p:
        order[q + 1:p + 1] = order[q:p].copy(); spos[order[q + 1:p + 1]] += 1
    elif q > p:
        order[p:q] = order[p + 1:q + 1].copy(); spos[order[p:q]] -= 1
    else: return 0
    order[q] = s; spos[s] = q; return abs(q - p)
def bits_of(pos, n_):
    a = (pos * HB) // max(n_, 1)
    b = HB + np.minimum(HB - 1, (pos * HB + max(n_, 1) // 2) // max(n_, 1))
    return a, b
# ---- story state: salience per spot (for drawing context), activation per bit (for the code)
H = A.half; g = 1.0; sal = {}; act = np.zeros(D)
code = np.zeros(0, dtype=np.int64); incode = np.zeros(D, dtype=bool)
def draw_context(exclude):
    if not sal: return None
    items = [(s, v) for s, v in sal.items() if sword[s] != exclude]
    if not items: return None
    tot = sum(v for _, v in items); u = rng.random() * tot
    for s, v in items:
        if u < v: return s
        u -= v
    return items[-1][0]
# ---- readouts
rows = np.zeros((D, NW), dtype=np.int32); nb = np.zeros(D, dtype=np.int64)
q_bits = np.zeros(n); q_reg = np.zeros(n)
snap_a = snap_b = None; snap_u = None; snap_U = None; snap_S = 0
def snapshot():
    global snap_a, snap_b, snap_u, snap_U, snap_S
    snap_S = S
    a, b = bits_of(spos[:S], N)
    nsp = np.array([len(wspots[w]) for w in sword[:S]]) if S else np.zeros(0)
    u = fc[sword[:S]] / np.maximum(nsp, 1)
    snap_a, snap_b, snap_u = a, b, u
    snap_U = np.bincount(a, weights=u, minlength=D) + np.bincount(b, weights=u, minlength=D)
snapshot()
prior = np.zeros(n, dtype=np.int64)            # occurrences of the word before this tick
links = {}; ltot = [0]; lcnt = np.zeros(NW, dtype=np.int64); tcache = {}; recent = deque(maxlen=A.links)
for i in range(n):
    w = int(ids[i]); prior[i] = fc[w]
    # predict
    if len(code):
        r = rows[code, w].astype(np.float64); q_bits[i] = ((r + BETA * u_p[i]) / (nb[code] + BETA)).mean()
        qr = 0.0
        for s in wspots.get(w, ()):
            if s < snap_S and snap_u[s] > 0:
                for bb in (snap_a[s], snap_b[s]):
                    if incode[bb] and snap_U[bb] > 0: qr += snap_u[s] / snap_U[bb]
        q_reg[i] = qr / len(code)
    else:
        q_bits[i] = u_p[i]
    # learn
    if len(code):
        rows[code, w] += 1; nb[code] += 1
    fc[w] += 1
    if w >= SKIP:
        c = draw_context(w)
        tau = spos[c] if c is not None else rng.randrange(N + 1)
        if A.target == 'assoc':
            lw = links.setdefault(w, {})
            for cx in recent:
                if cx != w: lw[cx] = lw.get(cx, 0) + 1; ltot[0] += 1; lcnt[w] += 1; lcnt[cx] += 1
            if w in wspots and (fc[w] < 64 or fc[w] % 8 == 0 or w not in tcache):
                cand = [(cx, k_) for cx, k_ in lw.items() if k_ >= 2 and cx in wspots]
                if cand:
                    sc_ = [(math.log(k_ * ltot[0] / (lcnt[w] * lcnt[cx])), cx) for cx, k_ in cand]
                    sc_ = [z for z in sorted(sc_, reverse=True)[:8] if z[0] > 0]
                    if sc_:
                        ps = np.array([spos[wspots[cx][0]] for _, cx in sc_]); vs = np.array([v for v, _ in sc_])
                        o = np.argsort(ps); ps, vs = ps[o], vs[o]
                        groups = [(ps, vs)]
                        if len(ps) >= 6:
                            j = int(np.argmax(np.diff(ps)))
                            if ps[j + 1] - ps[j] > 0.1 * N and 3 <= j + 1 <= len(ps) - 3:
                                groups = [(ps[:j + 1], vs[:j + 1]), (ps[j + 1:], vs[j + 1:])]
                        meds = []
                        for gp, gv in groups:
                            cs = np.cumsum(gv); meds.append(int(gp[np.searchsorted(cs, cs[-1] / 2)]))
                        tcache[w] = meds
                        sp = wspots[w]
                        if len(meds) == 2 and len(sp) < A.senses and fc[w] >= 32:
                            for f in meds:
                                if min(abs(spos[x] - f) for x in sp) > 0.05 * N:
                                    insert(w, f); nsplit += 1; break
        elif A.target == 'median' and c is not None:
            it = [(spos[x], v) for x, v in sal.items() if sword[x] != w]
            ps = np.array([a for a, _ in it]); vs = np.array([b for _, b in it]); o = np.argsort(ps)
            cs = np.cumsum(vs[o]); tau = int(ps[o][np.searchsorted(cs, cs[-1] / 2)])
        if w not in wspots:
            q = (tau + 1) if A.place == 'context' and c is not None else rng.randrange(N + 1)
            s = insert(w, q)
        else:
            sp = wspots[w]
            s = min(sp, key=lambda x: abs(spos[x] - tau))
            if A.target == 'assoc':
                meds = tcache.get(w)
                if A.move and meds:
                    tgt = min(meds, key=lambda f: abs(spos[s] - f))
                    eta = max(A.eta_min, 0.5 / math.sqrt(fc[w]))
                    p = spos[s]; d = move(s, p + int(round(eta * (tgt - p)))); moves += 1; moved += d
            elif A.move and c is not None:
                eta = max(A.eta_min, 0.5 / math.sqrt(fc[w]))
                p = spos[s]; d = move(s, p + int(round(eta * (tau - p)))); moves += 1; moved += d
                if A.mutual > 0:
                    pc = spos[c]; ec = A.mutual * max(A.eta_min, 0.5 / math.sqrt(max(fc[sword[c]], 1)))
                    move(c, pc + int(round(ec * (spos[s] - pc))))
                if A.senses > 1 and fc[w] >= 32 and A.target != 'assoc':
                    rv = res.setdefault(w, deque(maxlen=16)); rv.append(tau / N)
                    if len(rv) == 16 and fc[w] % 16 == 0 and len(sp) < A.senses:
                        v = np.sort(np.array(rv))
                        best = min(range(5, 12), key=lambda j: v[:j].var() * j + v[j:].var() * (16 - j))
                        g1, g2 = v[:best], v[best:]
                        if np.median(g2) - np.median(g1) > 0.1:
                            far = [np.median(g1), np.median(g2)]
                            # recruit a spot at the group no existing spot is near
                            for f in far:
                                if min(abs(spos[x] / N - f) for x in sp) > 0.05:
                                    s2 = insert(w, int(f * N)); nsplit += 1; break
                            rv.clear()
        # light the story state
        g *= 2 ** (1 / H)
        sal[s] = sal.get(s, 0.0) + g
        recent.append(w)
        a_, b_ = bits_of(np.array([spos[s]]), N); act[a_[0]] += g; act[b_[0]] += g
        if g > 1e100:
            for kk in sal: sal[kk] /= 1e100
            act /= 1e100; g /= 1e100
        if i % 256 == 0: sal = {kk: v for kk, v in sal.items() if v >= g * 1e-3}
        nz = np.count_nonzero(act)
        kk = min(K, nz)
        code = np.argpartition(-act, kk - 1)[:kk] if kk else np.zeros(0, dtype=np.int64)
        incode[:] = False; incode[code] = True
    if i % 256 == 255: snapshot()
    if i % 250000 == 0:
        print(f"t={i} spots={N} words={len(wspots)} senses+={nsplit} mean move={moved / max(moves, 1):.1f}", flush=True)
# ---- scoring
h = n // 2; cal = h + (n - h) // 5; sc = slice(cal, n); ca = slice(h, cal)
LAMS = [0.0, 0.02, 0.05, 0.1, 0.15, 0.2, 0.3, 0.45]
import itertools
def fit(qs):
    best = None
    for ls in itertools.product(LAMS, repeat=len(qs)):
        if sum(ls) >= 0.95: continue
        p = (1 - sum(ls)) * p_base[ca] + sum(l * q[ca] for l, q in zip(ls, qs))
        v = np.log2(p).mean()
        if best is None or v > best[0]: best = (v, ls)
    ls = best[1]
    p = (1 - sum(ls)) * p_base + sum(l * q for l, q in zip(ls, qs))
    return -np.log2(p), ls
base_bits = -np.log2(p_base)
# subsets of the scored part: rare words (seen fewer than 10 times before), words new to the story (not in the
# previous 2,000 words)
last = {}; storynew = np.zeros(n, dtype=bool)
for i in range(n):
    w = int(ids[i]); storynew[i] = w >= SKIP and (w not in last or i - last[w] > 2000); last[w] = i
rare = prior < 10
out = {'args': vars(A), 'spots': int(N), 'words_mapped': len(wspots), 'senses_added': nsplit,
       'base': float(base_bits[sc].mean()), 'rare_share': float(rare[sc].mean()), 'storynew_share': float(storynew[sc].mean())}
combos = {'story': [q_story], 'bits': [q_bits], 'reg': [q_reg], 'story+bits': [q_story, q_bits],
          'story+reg': [q_story, q_reg], 'story+bits+reg': [q_story, q_bits, q_reg]}
print(f"{A.out}: base {out['base']:.4f} bits per exact word (scored part); rare {out['rare_share']:.3f}, new to story {out['storynew_share']:.3f}")
for name, qs in combos.items():
    bb, ls = fit(qs); d = base_bits - bb
    m = lambda mask: float(d[sc][mask[sc]].mean())
    out[name] = {'gain': float(d[sc].mean()), 'rare': m(rare), 'storynew': m(storynew), 'weights': ls}
    print(f"  {name:<16} gain {d[sc].mean():+.4f}   on rare words {m(rare):+.4f}   on words new to the story {m(storynew):+.4f}   weights {ls}", flush=True)
json.dump(out, open(A.out + '.json', 'w'), indent=1)
np.savez_compressed(A.out + '.map.npz', order=order[:N], sword=sword[:S], spos=spos[:S])
np.savez_compressed(A.out + '.npz', q_bits=q_bits[h:], q_reg=q_reg[h:])
if not A.report: sys.exit()
# ---- stage 1 report: what the map looks like
lines = []
W = lambda w: words[w] if w < len(words) else str(w)
lines.append(f"{N} spots for {len(wspots)} words, {nsplit} second spots (senses)")
lines.append("\nRegions (12 consecutive spots at 16 evenly spaced places along the line):")
for j in range(16):
    p = int(j * (N - 12) / 15)
    lines.append(f"  {p:>6}: " + " ".join(W(int(sword[order[x]])) for x in range(p, p + 12)))
# associates from co-occurrence (evaluation only): content words within 10 content positions, PPMI
import scipy.sparse as spm
cw = ids[ids >= SKIP]
r_, c_ = [], []
for d_ in range(1, 11): r_.append(cw[:-d_]); c_.append(cw[d_:])
r_ = np.concatenate(r_); c_ = np.concatenate(c_)
M = spm.coo_matrix((np.ones(len(r_)), (r_, c_)), shape=(NW, NW)).tocsr(); M = M + M.T
M.setdiag(0); M.eliminate_zeros()  # a word is not its own associate
tot = M.sum(); rs = np.asarray(M.sum(1)).ravel()
cnt = np.bincount(cw, minlength=NW)
cand = [w for w in wspots if cnt[w] >= 50]
loc, chance = [], []
span = 0.01 * N
for w in rng.sample(cand, min(2000, len(cand))):
    row = M.getrow(w); js = row.indices; vs = row.data
    ok = vs >= 5
    js, vs = js[ok], vs[ok]
    if len(js) < 10: continue
    pmi = np.log(vs * tot / (rs[w] * rs[js]))
    top = js[np.argsort(-pmi)[:10]]
    mine = [spos[s] for s in wspots[w]]
    near = lambda x: any(abs(spos[s2] - m) <= span for s2 in wspots.get(int(x), []) for m in mine)
    loc.append(np.mean([near(x) for x in top]))
    rnd = rng.sample(cand, 10); chance.append(np.mean([near(x) for x in rnd]))
lines.append(f"\nLocality: of each word's 10 strongest associates (PPMI, words seen 50+ times), the share with a spot within 1% of the line of one of its spots: {np.mean(loc):.3f} (random words: {np.mean(chance):.3f}; {len(loc)} words)")
split = sorted([w for w, sp in wspots.items() if len(sp) > 1], key=lambda w: -fc[w])
lines.append(f"\nWords with a second spot (commonest 40 of {len(split)}), with 5 neighbours of each spot:")
for w in split[:40]:
    parts = []
    for s in wspots[w]:
        p = spos[s]; nb_ = [W(int(sword[order[x]])) for x in range(max(0, p - 2), min(N, p + 3)) if x != p]
        parts.append("[" + " ".join(nb_) + "]")
    lines.append(f"  {W(w)} ({fc[w]}): " + "  ".join(parts))
open(A.out + '.map.txt', 'w').write("\n".join(lines) + "\n")
print(lines[0]); print([l for l in lines if l.startswith("\nLocality")][0].strip())
