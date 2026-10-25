"""Score a report-stream run: QA top-1 on the questions kept, and the reports: the answer word inside the act, the
other words, whole sentences; test half (from info['shift'][KEY]) and training.
usage: score_report.py DOC RUN KEY"""
import sys, json, collections
import numpy as np
doc, run, key = sys.argv[1], sys.argv[2], sys.argv[3]
ids = np.fromfile(doc + ".ids", dtype=np.uint32); info = json.load(open(doc + ".info.json"))
n = len(ids); top = np.fromfile(run + ".top", dtype=np.uint32).reshape(n, -1)[:, 0]
truth = ids & 0x7FFFFFFF; split = info["shift"][key]
story = "names" in info
q = collections.defaultdict(list)
for a in info["answers"]:
    if story:
        if a[0] >= split:
            q[a[1]].append(top[a[0]] == truth[a[0]])
    else:
        q[(a[1], a[2])].append(top[a[0]] == truth[a[0]])
print("QA (questions kept" + (", test half" if story else "") + "): " + "  ".join(f"{k} {np.mean(v):5.1%} (n {len(v)})" for k, v in q.items()))
for name, sel in (("train", lambda s: s < split), ("test", lambda s: s >= split)):
    by = collections.defaultdict(lambda: [[], [], []])
    for s, l, off, kind in info["report"]:
        if not sel(s):
            continue
        ok = [top[s + i] == truth[s + i] for i in range(l)]
        for k in (kind, "all"):
            by[k][0].append(ok[off]); by[k][1].extend(ok[:off] + ok[off + 1:]); by[k][2].append(all(ok))
    print(f"reports {name}: " + "  |  ".join(f"{k}: answer {np.mean(v[0]):5.1%}, other words {np.mean(v[1]):5.1%}, whole {np.mean(v[2]):5.1%} (n {len(v[0])})" for k, v in by.items()))
