"""Experiment 68: how well the expert reads the practice stream's own magic, by spelling. For made-up files read after
their own description (R) or a wrong one (W), the expert's cost on the magic bytes, by quarter of the practice stream,
for even-numbered formats (hex escapes in A and C) and odd ones (octal escapes in A and C) apart.
usage: practice.py NAME=ITEMS_JSON:BITS ...   (bits: float32 per byte of the run's stream, practice first)"""
import sys, json
import numpy as np
print(f"{'run':<10}{'formats':<8} | " + " | ".join(f"q{q} R    W   R-W" for q in range(4)))
for a in sys.argv[1:]:
    nm, rest = a.split("="); ip, bp = rest.split(":")
    items = json.load(open(ip))["items"]; b = np.fromfile(bp, dtype="<f4")
    n = len(items)
    for par, lbl in ((0, "even"), (1, "odd")):
        row = []
        for q in range(4):
            v = {"R": [], "W": []}
            for it in items[q * n // 4:(q + 1) * n // 4]:
                if it["format"] % 2 != par or it["cond"] not in v:
                    continue
                o, L, t = it["fields"][0]
                v[it["cond"]] += list(b[it["start"] + o:it["start"] + o + L])
            r, w = np.mean(v["R"]), np.mean(v["W"])
            row.append(f"{r:4.2f} {w:4.2f} {r - w:+5.2f}")
        print(f"{nm:<10}{lbl:<8} | " + " | ".join(row))
