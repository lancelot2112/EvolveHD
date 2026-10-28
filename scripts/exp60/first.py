"""Bits on the fields a description states (magic, version, const), on files whose format the reader has not seen a
file of before, by condition; and on the second half of the stream only (after the expert has had time to learn).
usage: first.py STREAM NAME=BITS ..."""
import sys, json
import numpy as np
meta = json.load(open(sys.argv[1] + ".items.json"))["items"]
runs = [(a.split("=")[0], np.fromfile(a.split("=")[1], dtype="<f4").astype(float)) for a in sys.argv[2:]]
half = meta[len(meta) // 2]["start"]
seen = set(); acc = {}
for it in meta:
    first = it["format"] not in seen; seen.add(it["format"])
    for o, l, t in it["fields"]:
        if t in ("magic", "version", "const"):
            for nm, b in runs:
                for part in ("all", "2nd half"):
                    if part == "2nd half" and it["start"] < half:
                        continue
                    acc.setdefault((part, "first" if first else "later", t, it["cond"], nm), []).extend(b[it["start"] + o:it["start"] + o + l])
for part in ("all", "2nd half"):
    for fl in ("first", "later"):
        for t in ("magic", "version", "const"):
            row = f"  {part:<9}{fl:<6}{t:<8}"
            for c in "RWN":
                row += f" | {c} n {len(acc.get((part, fl, t, c, runs[0][0]), [])):>5} " + " ".join(f"{nm} {np.mean(acc.get((part, fl, t, c, nm), [np.nan])):5.2f}" for nm, _ in runs)
            print(row)
