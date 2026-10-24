"""Score found questions on a book with volumes (longdoc_gen.py vols=V) by where the fact is: in the volume being
read, or in an earlier volume (where only a chain volume > part > chapter finds it).
usage: score_levels.py DOC RUN [SKIP_FRACTION]"""
import sys, json, collections, math
import numpy as np
doc, run = sys.argv[1], sys.argv[2]; skip = float(sys.argv[3]) if len(sys.argv) > 3 else 0.1
ids = np.fromfile(doc + ".ids", dtype=np.uint32); info = json.load(open(doc + ".info.json"))
voc = open(doc + ".txt").read().split("\n"); V = voc.index("volume"); DOT = voc.index(".")
top = np.fromfile(run + ".top", dtype=np.uint32).reshape(len(ids), -1)
heads = [i for i in range(len(ids)) if ids[i] == V and (i == 0 or ids[i - 1] == DOT)]
g = collections.defaultdict(list)
for (p, kind), d in zip(info["answers"], info["dist"]):
    if kind != "found" or p < skip * len(ids): continue
    f = p - d; cur = max(h for h in heads if h <= p)
    where = "this volume" if f >= cur else "earlier volume"
    b = "8k-64k" if d < 65536 else "64k-512k" if d < 524288 else "512k+"
    if d < 8192: b = "<8k"
    g[(where, b)].append(top[p, 0] == ids[p])
for where in ["this volume", "earlier volume"]:
    print(f"  found, fact in {where:>14}: " + "  ".join(f"{b} {np.mean(g[(where, b)]):6.1%} ({len(g[(where, b)])})" for b in ["<8k", "8k-64k", "64k-512k", "512k+"] if g[(where, b)]))
