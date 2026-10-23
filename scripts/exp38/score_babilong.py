"""Score a run on a BABILong stream: top-1 at each answer position, by (task, length), over the documents from
index SKIP on in each group (the earlier ones are where the reader learns). usage: score_babilong.py DOC RUN [SKIP]"""
import sys, json, collections
import numpy as np
doc, run = sys.argv[1], sys.argv[2]; skip = int(sys.argv[3]) if len(sys.argv) > 3 else 0
ids = np.fromfile(doc + ".ids", dtype=np.uint32); info = json.load(open(doc + ".info.json"))
top = np.fromfile(run + ".top", dtype=np.uint32).reshape(len(ids), -1)
g = collections.defaultdict(list)
for p, task, length, k, dl in info["answers"]:
    if k >= skip: g[(task, length)].append(top[p, 0] == ids[p])
for key in g: print(f"  {key[0]} {key[1]:>5}  {np.mean(g[key]):6.1%} (n {len(g[key])})")
