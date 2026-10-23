"""Learning curve: top-1 on one question kind in windows of the stream, for several runs (e.g. far found questions,
fact at least 64k words back, per 50k words). usage: curve.py DOC KIND MIN_DIST WINDOW RUN..."""
import sys, json
import numpy as np
doc, kind, mind, win = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
ids = np.fromfile(doc + ".ids", dtype=np.uint32); info = json.load(open(doc + ".info.json"))
qs = [(p, d) for (p, k), d in zip(info["answers"], info["dist"]) if k == kind and d >= mind]
lo = range(0, len(ids), win)
print("window start " + " ".join(f"{a // 1000:>6}k" for a in lo))
for r in sys.argv[5:]:
    top = np.fromfile(r + ".top", dtype=np.uint32).reshape(len(ids), -1)
    cells = []
    for a in lo:
        ps = [p for p, d in qs if a <= p < a + win]
        cells.append(f"{100 * np.mean([top[p, 0] == ids[p] for p in ps]):6.1f}%" if ps else "     - ")
    print(f"{r.split('/')[-1]:<12} " + " ".join(cells))
