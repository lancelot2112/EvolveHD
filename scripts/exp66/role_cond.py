"""Experiment 66: the role gate's decision and its teacher's label at the start of each real file, by the condition
before it (Rm: its magic entry, Rn: a note, W: another format's entry, N: nothing, so the segment before is a file),
pooled over the rotations; first files of a format apart from later ones. Also Go at the start of descriptions
(segment before is a file).
usage: role_cond.py STREAM_PATTERN [ROTATIONS]   (pattern contains {s}; reads {s}.items.json, .starts, .bin, _role.dump)"""
import sys, re, json, os
import numpy as np
pat = sys.argv[1]
rots = [int(x) for x in sys.argv[2].split(",")] if len(sys.argv) > 2 else range(4)
acc = {}
for s in rots:
    st = pat.format(s=s)
    if not os.path.exists(st + "_role.dump"):
        continue
    starts = [int(x) for x in open(st + ".starts")]
    n = os.path.getsize(st + ".bin")
    ends = starts[1:] + [n]
    segs = [i for i in range(1, len(starts)) if ends[i] - starts[i] >= 16]
    rows = [tuple(int(v) for v in re.findall(r"go=(\d) gain=[-\d.]+ label=(\d)", l)[0]) for l in open(st + "_role.dump")]
    assert len(rows) == len(segs), (len(rows), len(segs))
    at = {starts[i]: r for i, r in zip(segs, rows)}
    for it in json.load(open(st + ".items.json"))["items"]:
        fl = "first" if it["k"] == 0 else "later"
        if it["start"] in at:
            acc.setdefault((it["cond"], fl), []).append(at[it["start"]])
        if it["desc_len"] and it["desc_start"] in at:
            acc.setdefault(("desc", fl), []).append(at[it["desc_start"]])
print(f"{'before':<8}{'files':<7}{'n':>5}  {'gate Go':>8}  {'teacher Go':>10}  {'gate = teacher':>14}")
for c in ("Rm", "Rn", "W", "N", "desc"):
    for fl in ("first", "later"):
        r = np.array(acc.get((c, fl), [(0, 0)])).reshape(-1, 2)
        lbl = c if c != "desc" else "(desc)"
        print(f"{lbl:<8}{fl:<7}{len(r):>5}  {r[:, 0].mean():8.2f}  {r[:, 1].mean():10.2f}  {np.mean(r[:, 0] == r[:, 1]):14.2f}")
