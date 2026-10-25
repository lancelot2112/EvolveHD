"""Score an echo-stream run: QA top-1 at answer positions (story set by kind; qa2 by task/length) and echo accuracy
(each self slot's top-1 against the teacher; whole trials exact), split at the training/test boundary.
usage: score_echo.py DOC RUN SPLIT_KEY  (SPLIT_KEY: a key of info['shift'], e.g. 12329 or 102303)"""
import sys, json, collections
import numpy as np
doc, run, key = sys.argv[1], sys.argv[2], sys.argv[3]
ids = np.fromfile(doc + ".ids", dtype=np.uint32); info = json.load(open(doc + ".info.json"))
n = len(ids); top = np.fromfile(run + ".top", dtype=np.uint32).reshape(n, -1)[:, 0]
truth = ids & 0x7FFFFFFF
split = info["shift"][key]
rows = []
if "names" in info:  # story set
    g = collections.defaultdict(list)
    for p, k in info["answers"]:
        if p >= split:
            g[k].append(top[p] == truth[p]); g["all"].append(top[p] == truth[p])
    rows.append("QA test: " + "  ".join(f"{k} {np.mean(v):5.1%}" for k, v in g.items()))
else:
    g = collections.defaultdict(list)
    for a in info["answers"]:
        g[(a[1], a[2])].append(top[a[0]] == truth[a[0]])
    rows.append("QA: " + "  ".join(f"{t} {l} {np.mean(v):5.1%} (n {len(v)})" for (t, l), v in g.items()))
for name, sel in (("train", lambda s: s < split), ("test", lambda s: s >= split)):
    tr = [(s, l) for s, l in info["echo"] if sel(s)]
    words = [top[s + i] == truth[s + i] for s, l in tr for i in range(l)]
    first = [top[s] == truth[s] for s, l in tr]
    rest = [top[s + i] == truth[s + i] for s, l in tr for i in range(1, l)]
    exact = [all(top[s + i] == truth[s + i] for i in range(l)) for s, l in tr]
    rows.append(f"echo {name}: words {np.mean(words):5.1%}  first word {np.mean(first):5.1%}  later words {np.mean(rest):5.1%}  whole sentences {np.mean(exact):5.1%} (n {len(tr)})")
print("\n".join(rows))
