"""Where-object questions by case, replayed symbolically from the story stream: the object held (its holder moved since
the pick-up, or not), or put down (its dropper moved since, or not). Then each run's top-answer accuracy per case.
usage: wo_cases.py IDS INFO.json PREFIX ..."""
import sys, json, numpy as np
ids = np.fromfile(sys.argv[1], dtype=np.uint32).astype(int); n = len(ids)
info = json.load(open(sys.argv[2])); voc = open(sys.argv[1].replace('.ids', '.txt')).read().split()
names, places, objects = set(info['names']), set(info['places']), set(info['objects'])
ans = {p: k for p, k in info['answers'] if n // 2 <= p < n and k == 'where-object'}
W = lambda s: voc.index(s)
loc = {}; held = {}; dropped_at = {}; moved_since = {}; cases = {}; clause = []
for t, w in enumerate(ids):
    if t in ans:
        o = [x for x in ids[t - 3:t] if x in objects][0]
        if o in held: cases[t] = 'held, holder moved since' if moved_since.get(o) else 'held, holder still'
        else: cases[t] = 'put down, dropper moved since' if moved_since.get(o) else 'put down, dropper still'
    clause.append(w)
    if voc[w] in ('.', '?'):
        c = [voc[x] for x in clause]; clause = []
        if '?' in c or not c: continue
        subj = W(c[0]) if c[0] in [voc[x] for x in names] else None
        if subj is None: continue
        if c[-2] in [voc[x] for x in places] and 'to' in c and 'the' in c and len(c) <= 6:
            loc[subj] = W(c[-2])
            for o, h in held.items():
                if h == subj: moved_since[o] = True
            for o, d in list(dropped_at.items()):
                if d == subj: moved_since[o] = True
        objs = [W(x) for x in c if x in [voc[y] for y in objects]]
        if objs:
            o = objs[0]
            if c[1] in ('got', 'grabbed', 'took', 'picked'): held[o] = subj; moved_since[o] = False; dropped_at.pop(o, None)
            elif c[1] in ('dropped', 'put'): held.pop(o, None); dropped_at[o] = subj; moved_since[o] = False
            elif c[1] in ('gave', 'handed'): held[o] = W(c[-2]); moved_since[o] = False
for pre in sys.argv[3:]:
    top = np.fromfile(pre + ".top", dtype=np.uint32).reshape(n, 8)
    by = {}
    for t, c in cases.items(): by.setdefault(c, []).append(top[t, 0] == ids[t])
    print(pre.split('/')[-1].ljust(14), "  ".join(f"{c}: {np.mean(v):4.0%} ({len(v)})" for c, v in sorted(by.items())))
