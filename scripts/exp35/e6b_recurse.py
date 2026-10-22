"""E6b: chained questions answered by bouncing reads through a ternary slot state.
World: 6 people, 6 places, 6 objects, 4 boxes. Events: a person moves; picks up a free object or box; drops what
they hold (it is then in their place); puts a held object or box into a box they hold (no cycles).
"where is X?" needs 1 hop (a person, a dropped thing), 2 (held), 3 (in a held box), 4 (box in a held box), ...
Facts are written as they happen into one ternary slot per (entity, relation) -- relations: in-place, held-by,
inside -- K = 128 active elements in D = 65,536, newest write wins (magnitude 2), in a context of K offsets of
which one is redrawn per event with probability d (d = 0: no drift). A thing's old relations are never erased: when an apple is
dropped, its held-by slot still names its last holder. A read returns the best entity and its strength (the
excess of its score over the next best, per element).
Answering: start at X; at each step read X's three slots; follow one; stop at a place (an entity with no strong
slot of its own); at most 6 steps.
  hop2      follow the strongest slot, but give up after 2 steps (the situation model's explicit hop-2)
  strongest follow the strongest slot (drift makes older facts weaker, so recency picks the current relation)
  first     follow relations in a fixed order (inside, held-by, in-place) whenever readable: no recency
  stamp     follow the slot written last, from an exact 16-bit write time kept per slot (one cell each)
  learned   Select: success counts per (relation, rank of its strength among the readable slots), learned from
            the answers (reward at the end, credited to every step taken); epsilon-greedy while learning
Accuracy on questions after a warm-up, by hops needed."""
import numpy as np, sys
from trace import UNIT, DRE, DIM
NP, NL, NO, NB = 6, 6, 6, 4
P = list(range(NP)); L = list(range(NP, NP + NL)); O = list(range(NP + NL, NP + NL + NO))
B = list(range(NP + NL + NO, NP + NL + NO + NB)); NE = NP + NL + NO + NB
REL = ('in', 'held', 'inside')
def run(policy, drift, D=65536, K=128, steps=30000, seed=3, verbose=False):
    r = np.random.default_rng(seed)
    VPH = r.integers(0, 4, (NE, D)).astype(np.int8)
    base = r.integers(0, D, (NE, 3, K)); bph = r.integers(0, 4, (NE, 3, K))
    off = r.integers(0, D, K); oph = r.integers(0, 4, K)
    re = np.zeros(D, np.int16); im = np.zeros(D, np.int16)
    stamp = np.zeros((NE, 3), np.int64); tnow = [0]
    def key(e, rel): return (base[e, rel] + off) % D, (bph[e, rel] + oph) % 4
    def write(e, rel, v):
        stamp[e, rel] = tnow[0]
        idx, kp = key(e, rel); p = (kp + VPH[v, idx]) % 4
        re[idx] = np.clip(re[idx] + 2 * DRE[p], -1, 1); im[idx] = np.clip(im[idx] + 2 * DIM[p], -1, 1)
    def read(e, rel):
        idx, kp = key(e, rel); z = (re[idx] + 1j * im[idx]) * UNIT[(-kp) % 4]
        sc = np.real(np.conj(UNIT[VPH[:, idx]]) @ z); sc[e] = -1e9
        o = np.argsort(-sc); return int(o[0]), (sc[o[0]] - max(sc[o[1]], 0)) / K
    # true world
    at = {p: int(r.choice(L)) for p in P}; holder = {}; inside = {}; dropped = {}
    for x in O + B: dropped[x] = int(r.choice(L)); write(x, 0, dropped[x])
    for p in P: write(p, 0, at[p])
    def place(x):
        h = 0
        while True:
            if x in at: return at[x], h + 1
            if x in inside: x = inside[x]; h += 1; continue
            if x in holder: x = holder[x]; h += 1; continue
            return dropped[x], h + 1
    def held_by(p): return [x for x, q in holder.items() if q == p]
    def contains(b, x):
        while x in inside:
            if inside[x] == b: return True
            x = inside[x]
        return False
    Q = np.ones((3, 3)) ; N = np.ones((3, 3)) * 2      # learned success rates per (relation, strength rank)
    res = {}; thr = 0.05
    for t in range(steps):
        tnow[0] = t + 1
        ev = r.random(); p = int(r.choice(P))
        if ev < 0.35:
            at[p] = int(r.choice(L)); write(p, 0, at[p])
        elif ev < 0.6:
            free = [x for x in O + B if x not in holder and x not in inside]
            if free:
                x = int(r.choice(free)); holder[x] = p; dropped.pop(x, None); write(x, 1, p)
        elif ev < 0.8:
            hs = held_by(p)
            if hs:
                x = int(r.choice(hs)); del holder[x]; dropped[x] = at[p]; write(x, 0, at[p])
        else:
            hs = held_by(p); bs = [b for b in hs if b in B]
            if hs and bs:
                x = int(r.choice(hs)); b = int(r.choice(bs))
                if x != b and not contains(x, b) and not (x in B and contains(b, x)) and x not in inside:
                    del holder[x]; inside[x] = b; write(x, 2, b)
        if drift and r.random() < drift:
            sl = int(r.integers(0, K)); off[sl] = r.integers(0, D); oph[sl] = r.integers(0, 4)
        # a question
        if r.random() < 0.3:
            x = int(r.choice(P + O + B)); ans, hops = place(x)
            cur = x; trail = []; ok = False
            for step in range(6):
                if cur in L: break
                reads = [read(cur, rl) for rl in range(3)]
                cand = [rl for rl in range(3) if reads[rl][1] > thr]
                if not cand and policy != 'stamp': break
                rank = {rl: i for i, rl in enumerate(sorted(cand, key=lambda rl: -reads[rl][1]))}
                for rl_ in range(3): rank.setdefault(rl_, 2)
                if policy == 'stamp': cand = [rl for rl in range(3) if stamp[cur, rl] > 0]; rl = max(cand, key=lambda rl: stamp[cur, rl]) if cand else None
                if policy == 'stamp' and rl is None: break
                if policy == 'stamp': pass
                elif policy in ('strongest', 'hop2'): rl = min(cand, key=lambda rl: rank[rl])
                elif policy == 'first': rl = [c for c in (2, 1, 0) if c in cand][0]
                else:
                    if t < steps // 2 and r.random() < 0.1: rl = int(r.choice(cand))
                    else: rl = max(cand, key=lambda rl: Q[rl, rank[rl]] / N[rl, rank[rl]])
                trail.append((rl, rank[rl])); cur = reads[rl][0]
                if policy == 'hop2' and step == 1: break
            ok = cur == ans
            if policy == 'learned':
                for rl, rk in trail: Q[rl, rk] += ok; N[rl, rk] += 1
            if t > steps // 2:
                h = min(hops, 5); res.setdefault(h, []).append(ok)
    if verbose and policy == 'learned': print("   learned success rate per relation (rows: in, held, inside) by strength rank:\n", np.round(Q / N, 2))
    return res
print("accuracy by hops needed (questions in the second half)")
for drift in (0, 1 / 64, 1 / 16, 1 / 4):
    for pol in (('stamp',) if drift == 0 else ()) + ('hop2', 'first', 'strongest', 'learned'):
        res = run(pol, drift, verbose=(drift == 1 / 16))
        tot = np.mean([v for vs in res.values() for v in vs])
        print(f"drift {drift:.3f}  {pol:>9}: " + "  ".join(f"{h} hop{'s' if h > 1 else ' '} {np.mean(res[h]):4.0%} ({len(res[h])})" for h in sorted(res)) + f"   all {tot:4.0%}", flush=True)
