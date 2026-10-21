"""Per-channel accuracy of the bitwise store (evolvehd assoc --hippo --out PREFIX writes PREFIX.chan): for each
question type, how often each channel's first candidate of the answer's kind is the answer, and how often the
offered channels' (the attention's choice) is.  usage: hippo_chan.py IDS INFO.json PREFIX [CUE] [K] [PHASE]"""
import sys, json
import numpy as np
ids = np.fromfile(sys.argv[1], dtype=np.uint32); n = len(ids); info = json.load(open(sys.argv[2]))
cue = int(sys.argv[4]) if len(sys.argv) > 4 else 4; K = int(sys.argv[5]) if len(sys.argv) > 5 else 8
P = int(sys.argv[6]) if len(sys.argv) > 6 else 0; np_ = P + 1 if P else 1
H2 = [int(x) for x in sys.argv[7].split(',')] if len(sys.argv) > 7 else []
ch = np.fromfile(sys.argv[3] + ".chan", dtype=np.uint32); nch = len(ch) // n // K; ch = ch.reshape(n, nch, K)
top = np.fromfile(sys.argv[3] + ".top", dtype=np.uint32).reshape(n, 2, K)
ph = lambda p: '' if not P else ('*' if p == P else str(p))
names = [f"{j}{d}{ph(p)}" for j in range(cue) for d in 'fb' for p in range(np_)]
for src in [P if P else 0] + H2:
    names += [f"{j}{d}{ph(src)}>{d2}" for j in range(cue) for d in 'fb' for d2 in 'fb']
ans = [(p, k) for p, k in info['answers'] if p >= n // 2]
places, people = set(info['places']), set(info['names'])
def first_kind(lst, kind):
    return next((int(x) for x in lst if int(x) in kind), None)
for k in ['where-person', 'where-object', 'who-has']:
    ps = [p for p, kk in ans if kk == k]; kind = people if k == 'who-has' else places
    acc = [np.mean([first_kind(ch[p, c], kind) == ids[p] for p in ps]) for c in range(nch)]
    best = np.argsort(acc)[::-1][:5]
    off = [np.mean([first_kind(top[p, h], kind) == ids[p] for p in ps]) for h in range(2)]
    print(f"{k:<13} best channels: " + ", ".join(f"{names[c]} {acc[c]:.0%}" for c in best) + f" | offered: hop1 {off[0]:.0%} hop2 {off[1]:.0%}")
