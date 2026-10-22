"""Experiment 36, step 4: a question state that drives each hop and the stop (the forum's HRM proposal), tested on
experiment 35's box world with the oracle stop removed and a distractor relation added.

World (as e6b_recurse.py): 6 people, 6 places, 6 objects, 4 boxes; people move, pick up, drop, and put things into
boxes they hold. New:
- a frequent distractor relation, "looks at": a person looks at a random object or person;
- a second question, "what does X look at?".
Facts go into one ternary slot per (entity, relation): in, held-by, inside, looks-at. K = 128 of 65,536 elements;
the newest write wins.

Each step of answering reads the current entity's four slots and either stops (the answer is the current entity)
or follows one slot. Steps are capped at 8. The question code q (2 bits: where / looks-at) and the current entity's
kind (person, place, object or box; given here, learned in the stories) are the policy's inputs.

Policies:
  stamp-oracle     follow the slot written last (an exact write time per slot); stop at a place (the oracle stop of
                   experiment 35)
  stamp-kind       follow the slot written last; stop when the current entity's kind is the one most often answered
                   for this question (the learned stop below)
  learned          Go/NoGo: for each (q, kind of the current entity, relation, the relation's recency rank within the
                   entity) a pair of 8-bit counts (Go: the answer came out right with this step taken; NoGo: it
                   didn't), halved together at saturation. Follow the readable slot with the highest Go / (Go + NoGo), compared by
                   cross-multiplication (no division).
                   Credit: at the answer, every step taken is credited, last first. Stop: a 4-bit up/down counter per
                   (q, kind, steps taken: 0, 1, 2+), stepped at every visited entity by "this entity is the answer";
                   stop when it is in its upper half. Exploration 10% in the first half.
  learned-norank   the same without the recency rank (the policy sees q, kind and relation only)
  supersede        no write times: a new in / held-by / inside fact writes NONE into the entity's other two
                   location slots, so one location slot stays readable; follow it (or looks-at, under the learned
                   Go/NoGo without the rank); learned stop.
Scored on the second half: by question type and by hops needed for "where"."""
import numpy as np, sys
sys.path.insert(0, __file__.rsplit('/', 2)[0] + '/exp35')
from trace import UNIT, DRE, DIM
NP, NL, NO, NB = 6, 6, 6, 4
P = list(range(NP)); L = list(range(NP, NP + NL)); O = list(range(NP + NL, NP + NL + NO))
B = list(range(NP + NL + NO, NP + NL + NO + NB)); NE = NP + NL + NO + NB; NONE = NE
KIND = np.array([0] * NP + [1] * NL + [2] * NO + [3] * NB)
NR = 4                                                        # in, held, inside, looks
def run(policy, D=65536, K=128, steps=30000, seed=3):
    r = np.random.default_rng(seed)
    VPH = r.integers(0, 4, (NE + 1, D)).astype(np.int8)
    base = r.integers(0, D, (NE, NR, K)); bph = r.integers(0, 4, (NE, NR, K))
    re = np.zeros(D, np.int16); im = np.zeros(D, np.int16)
    stamp = np.zeros((NE, NR), np.int64); tnow = [0]
    def write(e, rel, v):
        stamp[e, rel] = tnow[0]
        idx, kp = base[e, rel], bph[e, rel]; p = (kp + VPH[v, idx]) % 4
        re[idx] = np.clip(re[idx] + 2 * DRE[p], -1, 1); im[idx] = np.clip(im[idx] + 2 * DIM[p], -1, 1)
        if policy == 'supersede' and rel < 3:
            for o in range(3):
                if o != rel and stamp[e, o] > 0:
                    idx2, kp2 = base[e, o], bph[e, o]; p2 = (kp2 + VPH[NONE, idx2]) % 4
                    re[idx2] = np.clip(re[idx2] + 2 * DRE[p2], -1, 1); im[idx2] = np.clip(im[idx2] + 2 * DIM[p2], -1, 1)
    def read(e, rel):
        idx, kp = base[e, rel], bph[e, rel]; z = (re[idx] + 1j * im[idx]) * UNIT[(-kp) % 4]
        sc = np.real(np.conj(UNIT[VPH[:, idx]]) @ z); sc[e] = -1e9
        o = np.argsort(-sc); return int(o[0]), (sc[o[0]] - max(sc[o[1]], 0)) / K
    at = {p: int(r.choice(L)) for p in P}; holder = {}; inside = {}; dropped = {}; looks = {}
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
    go = np.ones((2, 4, NR, NR + 1), np.int64); nogo = np.ones((2, 4, NR, NR + 1), np.int64)
    halt = np.full((2, 4, 3), 7)
    res = {}; thr = 0.05; nsteps = []
    for t in range(steps):
        tnow[0] = t + 1
        ev = r.random(); p = int(r.choice(P))
        if ev < 0.25:
            at[p] = int(r.choice(L)); write(p, 0, at[p])
        elif ev < 0.45:
            free = [x for x in O + B if x not in holder and x not in inside]
            if free: x = int(r.choice(free)); holder[x] = p; dropped.pop(x, None); write(x, 1, p)
        elif ev < 0.6:
            hs = held_by(p)
            if hs: x = int(r.choice(hs)); del holder[x]; dropped[x] = at[p]; write(x, 0, at[p])
        elif ev < 0.72:
            hs = held_by(p); bs = [b for b in hs if b in B]
            if hs and bs:
                x = int(r.choice(hs)); b = int(r.choice(bs))
                if x != b and not contains(x, b) and not (x in B and contains(b, x)) and x not in inside:
                    del holder[x]; inside[x] = b; write(x, 2, b)
        else:                                                   # the distractor: frequent, recent, never a place
            tgt = int(r.choice([x for x in O + P if x != p])); looks[p] = tgt; write(p, 3, tgt)
        if r.random() >= 0.3: continue
        if r.random() < 0.7:
            q = 0; x = int(r.choice(P + O + B)); ans_, hops = place(x)
        else:
            cand = [pp for pp in P if pp in looks]
            if not cand: continue
            q = 1; x = int(r.choice(cand)); ans_, hops = looks[x], 1
        cur = x; trail = []; visited = []; learn = t < steps // 2
        for step in range(8):
            k = int(KIND[cur]) if cur < NE else 0
            visited.append((k, cur, min(step, 2)))
            # stop?
            if policy == 'stamp-oracle':
                if (q == 0 and cur in L) or (q == 1 and step == 1): break
            elif halt[q, k, min(step, 2)] >= 8: break
            reads = [read(cur, rl) for rl in range(NR)] if cur < NE else []
            cand = [rl for rl in range(len(reads)) if reads[rl][1] > thr and reads[rl][0] != NONE]
            if not cand: break
            order = sorted(cand, key=lambda rl: -stamp[cur, rl]); rank = {rl: i for i, rl in enumerate(order)}
            if policy.startswith('stamp'): rl = order[0]
            else:
                rk = (lambda rl: rank[rl]) if policy == 'learned' else (lambda rl: NR)
                if learn and r.random() < 0.1: rl = int(r.choice(cand))
                else:                                     # the best Go / (Go + NoGo), compared by cross-multiplication
                    rl = cand[0]
                    for c_ in cand[1:]:
                        g1, n1 = go[q, k, rl, rk(rl)], nogo[q, k, rl, rk(rl)]; g2, n2 = go[q, k, c_, rk(c_)], nogo[q, k, c_, rk(c_)]
                        if g2 * (g1 + n1) > g1 * (g2 + n2): rl = c_
                trail.append((k, rl, rk(rl)))
            cur = reads[rl][0]
        ok = cur == ans_
        # learning: stop counters at every visited entity; Go/NoGo for every step taken
        for (k, e, sb) in visited:
            halt[q, k, sb] = min(15, halt[q, k, sb] + 1) if e == ans_ else max(0, halt[q, k, sb] - 1)
        for (k, rl, rk) in reversed(trail):
            (go if ok else nogo)[q, k, rl, rk] += 1
            if go[q, k, rl, rk] + nogo[q, k, rl, rk] > 255: go[q, k, rl, rk] >>= 1; nogo[q, k, rl, rk] >>= 1
        if t > steps // 2:
            key_ = ('looks', 1) if q == 1 else ('where', min(hops, 5))
            res.setdefault(key_, []).append(ok); nsteps.append(len(visited) - 1 - (hops if q == 0 else 1))
    return res, np.mean(nsteps)
print("accuracy, second half; where-questions by hops needed (5 = 5 or more); overshoot = steps taken minus needed")
for pol in ('stamp-oracle', 'stamp-kind', 'learned-norank', 'learned', 'supersede'):
    for seed in (3, 4, 5):
        res, ov = run(pol, seed=seed)
        allw = [v for k, vs in res.items() if k[0] == 'where' for v in vs]
        print(f"{pol:>15} seed {seed}: " + "  ".join(f"{k[0]}{k[1] if k[0] == 'where' else ''} {np.mean(v):4.0%}" for k, v in sorted(res.items())) +
              f"   where all {np.mean(allw):4.0%}   overshoot {ov:+.2f}", flush=True)
