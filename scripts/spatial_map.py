"""A learned map for the spatial stories (experiment 32), integer phases throughout.

Place codes: each room's position in the current story is a phase vector (D elements, L phases each).
Direction cells: each relation word r has a step code s_r (a phase vector); "X is r of Y" means p_X = p_Y + s_r.
Nothing about the words is given: the step codes start random and are learned across stories.

Within a story (best guesses that improve with every fact):
  - the first room mentioned is pinned at a fresh random code; a fact with both rooms unplaced and no link to
    placed rooms starts a weak anchor of its own
  - every fact adds evidence for both its rooms (p_Y + s_r for X, p_X - s_r for Y); after each fact a few
    relaxation sweeps recompute every room's position as the phase of its summed evidence (phasor sums of
    integer cos/sin tables; the phase read back as the table entry it matches best), so a new fact re-places
    whole groups of rooms and contradictions average out
  - a question "what is r of Y ?" looks for the room whose code best matches p_Y + s_r
Across stories (relation learning): at each story's end, for every fact "X is r of Y" whose rooms the other facts
still connect, the map is rebuilt without it (leave one out) and the phase difference p_X - p_Y there is added to
s_r's accumulator, and s_r is re-read from it.
usage: spatial_map.py IDS INFO.json [D] [L]"""
import sys, json
import numpy as np
from collections import defaultdict
ids = np.fromfile(sys.argv[1], dtype=np.uint32); info = json.load(open(sys.argv[2]))
voc = open(sys.argv[1].replace('.ids', '.txt')).read().split(); W = {w: i for i, w in enumerate(voc)}
D = int(sys.argv[3]) if len(sys.argv) > 3 else 256; L = int(sys.argv[4]) if len(sys.argv) > 4 else 16
rng = np.random.default_rng(7)
C = np.round(64 * np.cos(2 * np.pi * np.arange(L) / L)).astype(np.int64)
S = np.round(64 * np.sin(2 * np.pi * np.arange(L) / L)).astype(np.int64)
def phase_of(re, im):        # the phase index whose phasor best matches (re, im), per element: integer argmax
    return (re[:, None] * C[None, :] + im[:, None] * S[None, :]).argmax(1)
def sim(a, b): return int(C[(a - b) % L].sum())
dir_ids = info['dirs']; room_ids = set(info['rooms'])
racc = {r: (C[rng.integers(0, L, D)], S[rng.integers(0, L, D)]) for r in dir_ids}   # random start
step = {r: phase_of(*racc[r]) for r in dir_ids}
qpos = {q[0]: q for q in info['questions']}
res = []; story = -1; curve = []
def relax(facts, pins):
    pos = {}
    for _ in range(12):
        ev = defaultdict(lambda: [np.zeros(D, np.int64), np.zeros(D, np.int64)])
        for room, (code, w) in pins.items():
            ev[room][0] += w * C[code]; ev[room][1] += w * S[code]
        for (a, r, b) in facts:
            if b in pos:
                t = (pos[b] + step[r]) % L; ev[a][0] += C[t]; ev[a][1] += S[t]
            if a in pos:
                t = (pos[a] - step[r]) % L; ev[b][0] += C[t]; ev[b][1] += S[t]
        pos = {room: phase_of(e[0], e[1]) for room, e in ev.items()}
    return pos
i = 0; n = len(ids); facts = []; pins = {}; pos = {}; rooms_seen = []; nstory = 0
while i < n:
    w = int(ids[i])
    if w == W['new']:
        # story end: learn the step codes from this story's map
        # leave-one-out: each fact's relation is measured on the map built without it (so the evidence is not
        # just the current step code read back); only when the other facts still place both rooms together
        for fi, (a, r, b) in enumerate(facts):
            rest = facts[:fi] + facts[fi + 1:]
            grp = {b}; changed = True
            while changed:
                changed = False
                for (x, _, y) in rest:
                    if (x in grp) != (y in grp): grp |= {x, y}; changed = True
            if a not in grp: continue
            p2 = relax(rest, {b: (pos[b], 8)})
            if a in p2 and b in p2:
                d = (p2[a] - p2[b]) % L; racc[r] = (racc[r][0] + C[d], racc[r][1] + S[d])
        step = {r: phase_of(*racc[r]) for r in dir_ids}
        if nstory in (0, 1, 2, 5, 10, 20, 50, 100):
            curve.append((nstory + 1, sim((step[W['north']] + step[W['south']]) % L, np.zeros(D, int)) / (64 * D),
                          sim((step[W['east']] + step[W['west']]) % L, np.zeros(D, int)) / (64 * D)))
        facts, pins, pos, rooms_seen = [], {}, {}, []; nstory += 1; i += 3; continue
    if w == W['the'] and i + 7 < n and ids[i + 2] == W['is'] and int(ids[i + 3]) in dir_ids:
        a, r, b = int(ids[i + 1]), int(ids[i + 3]), int(ids[i + 6])
        facts.append((a, r, b)); rooms_seen += [x for x in (a, b) if x not in rooms_seen]
        if not pins: pins[b] = (rng.integers(0, L, D), 8)                   # the first room, pinned
        elif a not in pos and b not in pos: pins[b] = (rng.integers(0, L, D), 1)   # a weak anchor
        # a weak anchor is dropped once its group links up with the pinned room's (the facts then place it)
        main = next(r_ for r_, (_, w_) in pins.items() if w_ == 8)
        grp = {main}; changed = True
        while changed:
            changed = False
            for (x, _, y) in facts:
                if (x in grp) != (y in grp): grp |= {x, y}; changed = True
        pins = {r_: v for r_, v in pins.items() if v[1] == 8 or r_ not in grp}
        pos = relax(facts, pins); i += 8; continue
    if w == W['what']:
        r, y = int(ids[i + 2]), int(ids[i + 5]); p = i + 7
        if p in qpos:
            ans = int(ids[p]); _, nf, direct, det = qpos[p]
            # Y's group: the rooms linked to it through facts so far (their places are relative to each other)
            grp = {y}; changed = True
            while changed:
                changed = False
                for (a, _, b) in facts:
                    if (a in grp) != (b in grp): grp |= {a, b}; changed = True
            t = (pos[y] + step[r]) % L if y in pos else None
            inside = [(sim(pos[x], t), x) for x in grp if x != y and x in pos] if t is not None else []
            if inside and max(inside)[0] > 64 * D // 5:
                guess = max(inside)[1]                                   # the map says which room is there
            else:
                # best guess: not one of Y's group (their places are known not to be there), preferring rooms
                # already mentioned in this story; the fewer such rooms, the better the guess
                others = [x for x in rooms_seen if x not in grp] or [x for x in room_ids if x not in grp]
                guess = sorted(others)[nstory % len(others)] if others else -1
            res.append((nstory, nf, direct, det, guess == ans))
        i += 9; continue
    i += 1
res = np.array(res)
print(f"D={D} L={L}: {len(res)} questions")
for lo, hi in ((0, 50), (50, 200), (200, 1000), (1000, 99999)):
    m = (res[:, 0] >= lo) & (res[:, 0] < hi)
    if not m.any(): continue
    seg = res[m]
    row = [f"stories {lo}-{min(hi, nstory)}:"]
    for lab, sel in (("stated", seg[:, 2] == 1), ("implied only", (seg[:, 2] == 0) & (seg[:, 3] == 1)), ("not determined", seg[:, 3] == 0)):
        row.append(f"{lab} {seg[sel, 4].mean():.0%} ({sel.sum()})")
    print("  " + "  ".join(row))
last = res[res[:, 0] >= 1000]
print("  last 1000 stories, not determined yet, by facts so far:", "  ".join(
    f"{a}-{b}: {last[(last[:, 1] >= a) & (last[:, 1] <= b) & (last[:, 3] == 0), 4].mean():.0%}" for a, b in ((1, 3), (4, 6), (7, 9), (10, 12))))
print("  last 1000 stories, by facts so far (determined questions):", "  ".join(
    f"{a}-{b}: {last[(last[:, 1] >= a) & (last[:, 1] <= b) & (last[:, 3] == 1), 4].mean():.0%}" for a, b in ((1, 3), (4, 6), (7, 9), (10, 12))))
inv = {W['north']: W['south'], W['east']: W['west']}
for a, b in inv.items():
    print(f"  learned: {voc[a]} + {voc[b]} = {sim((step[a] + step[b]) % L, np.zeros(D, int)) / (64 * D):.2f} of zero (1 = exact opposites); "
          f"{voc[a]} vs {voc[b]} overlap {sim(step[a], step[b]) / (64 * D):+.2f}")
print(f"  north vs east overlap {sim(step[W['north']], step[W['east']]) / (64 * D):+.2f}")
print("  relation learning (stories seen: north+south, east+west, 1 = exact opposites):", "  ".join(f"{n}: {a:.2f}/{b:.2f}" for n, a, b in curve))
