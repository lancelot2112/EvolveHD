"""Bits per file byte on the formats stream, by condition (R: own description before, W: another format's, N: none),
by field type, and for each format's first file against its later files. Columns: one per per-byte bits file.
usage: score.py STREAM NAME=BITS_FILE ..."""
import sys, json
import numpy as np
s = sys.argv[1]; meta = json.load(open(s + ".items.json"))["items"]
runs = [(a.split("=")[0], np.fromfile(a.split("=")[1], dtype="<f4").astype(float)) for a in sys.argv[2:]]
seen = set(); rows = []
for it in meta:
    first = it["format"] not in seen; seen.add(it["format"])
    for o, l, t in it["fields"]:
        for j in range(l):
            rows.append((it["cond"], t, first, it["start"] + o + j))
rows = np.array(rows, dtype=object); pos = rows[:, 3].astype(int)
def show(label, m):
    print(f"  {label:<26} n {m.sum():>7}" + "".join(f"  {nm} {b[pos[m]].mean():6.3f}" for nm, b in runs))
print("file bytes, by condition:")
for c in "RWN": show(c, rows[:, 0] == c)
print("by field type and condition:")
for t in ["magic", "version", "count", "kind", "value", "name", "const"]:
    for c in "RWN": show(f"{t} {c}", (rows[:, 1] == t) & (rows[:, 0] == c))
print("first file of each format against later ones:")
for f in (True, False):
    for c in "RWN": show(f"{'first' if f else 'later'} {c}", (rows[:, 2] == f) & (rows[:, 0] == c))
d = [(it["desc_start"], it["desc_len"]) for it in meta if it["desc_len"]]
dm = np.concatenate([np.arange(a, a + l) for a, l in d])
print("description bytes:" + "".join(f"  {nm} {b[dm].mean():6.3f}" for nm, b in runs))
