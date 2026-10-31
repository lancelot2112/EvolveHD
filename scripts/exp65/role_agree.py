"""How the role gate's decisions and its teacher's labels agree with the truth (is the previous segment a
description?). The dump (ROLE_DUMP=1) has one line per taught segment: segments after the first that are 16 bytes or
longer, in order. usage: role_agree.py STREAM DUMP"""
import sys, re
import numpy as np
s, dump = sys.argv[1], sys.argv[2]
starts = [int(x) for x in open(s + ".starts")]
descs = set(int(x) for x in open(s + ".descs"))
n = len(open(s + ".bin", "rb").read())
ends = starts[1:] + [n]
segs = [i for i in range(1, len(starts)) if ends[i] - starts[i] >= 16]
rows = [tuple(int(v) for v in re.findall(r"go=(\d) gain=[-\d.]+ label=(\d)", l)[0]) for l in open(dump) if l.startswith("ROLE")]
assert len(rows) == len(segs), (len(rows), len(segs))
go = np.array([r[0] for r in rows]); lab = np.array([r[1] for r in rows])
tru = np.array([starts[i - 1] in descs for i in segs]); isd = np.array([starts[i] in descs for i in segs])
half = len(rows) // 2
for name, sl in (("all", slice(None)), ("2nd half", slice(half, None))):
    g, l, t, d = go[sl], lab[sl], tru[sl], isd[sl]
    print(f"{name}: n {len(g)}  previous is a description {t.mean():.3f}")
    for nm, v in (("gate", g), ("teacher", l)):
        print(f"  {nm:<8} agrees {np.mean(v == t):.3f}  Go after a description {v[t].mean():.3f}  Go after a file {v[~t].mean():.3f}"
              f" (before a description {v[~t & d].mean():.3f}, before a file {v[~t & ~d].mean():.3f})")
    print(f"  gate agrees with teacher {np.mean(g == l):.3f}")
