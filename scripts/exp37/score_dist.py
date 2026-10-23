"""Score a run on a long document by question kind and distance (experiment 37).
Top answer (and recall@8: the answer among the 8 candidates); "unknown" (no candidate) counts as not right.
Two distances per long-range question:
  fact   words from the fact's latest statement to the answer (the generator's "dist")
  echo   the smaller of that and the words back to the last time the same question was asked with the same answer
         (a question followed by its answer restates the fact)
Grouped 8-127, 128-1k, 1k-8k, 8k-64k, 64k-512k, 512k+. Questions in the first 10% of the document are left out.
Also the newest-find baseline: for "what did X find in chapter N ?", answering with X's latest find whatever the chapter.
usage: score_dist.py PREFIX_OF_DOC RUN_PREFIX ..."""
import sys, json, numpy as np
doc = sys.argv[1]
ids = np.fromfile(doc + ".ids", dtype=np.uint32).astype(np.int64); n = len(ids)
V = open(doc + ".txt").read().split("\n"); ix = {w: i for i, w in enumerate(V)}
info = json.load(open(doc + ".info.json"))
groups = [(8, 128, "8-127"), (128, 1024, "128-1k"), (1024, 8192, "1k-8k"), (8192, 65536, "8k-64k"), (65536, 524288, "64k-512k"), (524288, 1 << 62, "512k+")]
dot, found = ix["."], ix["found"]
# echo distances and the newest-find baseline, from the text itself, in one pass
ans = {p: k for p, k in info["answers"]}
fdist = {p: d for (p, k), d in zip(info["answers"], info["dist"])}
last_q, newest_find, echo, base = {}, {}, {}, {}
start = 0
for i in range(n):
    w = ids[i]
    if w == found and i >= 1 and i + 2 < n:
        newest_find[ids[i - 1]] = ids[i + 2]           # "X found the Z ."
    if i in ans:
        q = tuple(ids[start:i])                          # the question: from the last "." to the "?"
        if fdist[i] > 0:
            prev = last_q.get((q, ids[i]))
            echo[i] = min(fdist[i], i - prev) if prev is not None else fdist[i]
            if ans[i] == "found":                        # "what did X find in chapter N ?": X follows "did"
                j = i - 1
                while V[ids[j]] != "did": j -= 1                # "what did X find in chapter M [of part N] ?"
                base[i] = newest_find.get(ids[j + 1], -1) == ids[i]
        last_q[(q, ids[i])] = i
    if w == dot:
        start = i + 1
def table(name, rows, right, by):
    out = []
    for kind in ("home", "owner", "found", "qual"):
        if not any(k == kind for _, k, _, _ in rows): continue
        cells = []
        for lo, hi, g in groups:
            ps = [p for p, k, d, e in rows if k == kind and lo <= (d if by == "fact" else e) < hi]
            cells.append(f"{g} {np.mean([right[p] for p in ps]):5.1%} ({len(ps)})" if ps else f"{g}   -  ")
        out.append(f"  {kind:<6} by {by:<5} " + "  ".join(cells))
    return out
rows0 = [(p, k, d, echo.get(p, d)) for (p, k), d in zip(info["answers"], info["dist"]) if p >= n // 10 and d > 0]
bl = {p: base[p] for p in base}
print("newest-find baseline (X's latest find, any chapter): " + "  ".join(
    f"{g} {np.mean([bl[p] for p, k, d, e in rows0 if k == 'found' and lo <= d < hi]):5.1%}" for lo, hi, g in groups
    if any(k == 'found' and lo <= d < hi for p, k, d, e in rows0)))
for pre in sys.argv[2:]:
    top = np.fromfile(pre + ".top", dtype=np.uint32).reshape(n, -1).astype(np.int64)
    print(pre.split("/")[-1])
    for kind in ("where-person", "who-has", "where-object"):
        ps = np.array([p for p, k in info["answers"] if k == kind and p >= n // 10])
        if len(ps): print(f"  {kind:<13} {np.mean(top[ps, 0] == ids[ps]):6.1%} (n {len(ps)})")
    r1 = {p: bool(top[p, 0] == ids[p]) for p, k, d, e in rows0}
    r8 = {p: bool((top[p] == ids[p]).any()) for p, k, d, e in rows0}
    for line in table("top-1", rows0, r1, "fact") + table("top-1", rows0, r1, "echo"): print(line)
    print("  recall@8:")
    for line in table("recall@8", rows0, r8, "fact"): print("  " + line)
