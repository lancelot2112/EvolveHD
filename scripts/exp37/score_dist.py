"""Score a run on a long document by question kind and by distance (experiment 37).
Top answer only; "unknown" (no candidate) counts as not right. Distances are words from the fact's latest
statement to the answer, grouped as 8-127, 128-1k, 1k-8k, 8k-64k, 64k+. Questions in the first 10% of the
document are left out (the reader is still learning the forms).
usage: score_dist.py PREFIX_OF_DOC RUN_PREFIX ..."""
import sys, json, numpy as np
doc = sys.argv[1]
ids = np.fromfile(doc + ".ids", dtype=np.uint32).astype(np.int64); n = len(ids)
info = json.load(open(doc + ".info.json"))
groups = [(8, 128, "8-127"), (128, 1024, "128-1k"), (1024, 8192, "1k-8k"), (8192, 65536, "8k-64k"), (65536, 1 << 62, "64k+")]
for pre in sys.argv[2:]:
    top = np.fromfile(pre + ".top", dtype=np.uint32).reshape(n, -1)[:, 0].astype(np.int64)
    rows = [(p, k, d) for (p, k), d in zip(info["answers"], info["dist"]) if p >= n // 10]
    print(pre.split("/")[-1])
    local = [(p, k) for p, k, d in rows if d < 0]
    for kind in ("where-person", "who-has", "where-object"):
        ps = np.array([p for p, k in local if k == kind])
        if len(ps): print(f"  {kind:<13} {np.mean(top[ps] == ids[ps]):6.1%} (n {len(ps)})")
    for kind in ("home", "owner", "found"):
        cells = []
        for lo, hi, name in groups:
            ps = np.array([p for p, k, d in rows if k == kind and lo <= d < hi])
            cells.append(f"{name} {np.mean(top[ps] == ids[ps]):5.1%} ({len(ps)})" if len(ps) else f"{name}   -  ")
        print(f"  {kind:<13} " + "  ".join(cells))
