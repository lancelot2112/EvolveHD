"""Top-1 accuracy by kind in each stretch of the book, for several runs (to compare a sharded read with a sequential
one stretch by stretch). usage: by_region.py DOC PARTS RUN... (PARTS: comma-separated word positions)"""
import sys, json
import numpy as np
doc, cuts, runs = sys.argv[1], [int(x) for x in sys.argv[2].split(",")], sys.argv[3:]
info = json.load(open(doc + ".info.json"))
for r in runs:
    rk = np.fromfile(r + ".rank", dtype=np.uint8)
    print(r.split("/")[-1])
    for kind in ["where-person", "who-has", "where-object", "home", "owner", "found"]:
        row = []
        for a, b in zip(cuts, cuts[1:]):
            ps = [p for p, k in info["answers"] if k == kind and a <= p < b]
            row.append(f"{100 * np.mean([rk[p] == 0 for p in ps]):5.1f}%" if ps else "   -  ")
        print(f"  {kind:13s}", " ".join(row))
