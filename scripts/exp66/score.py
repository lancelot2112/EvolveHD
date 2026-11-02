"""Bits per byte on experiment 66's real-files streams, pooled over the four rotations, by the condition before each
file (Rm: its magic entry, Rn: a note in hex notation, W: another format's magic entry, N: nothing), for the bytes of
the magic test, the first 32 bytes, and all file bytes; the first file of each format apart from later ones.
Also a paired view: for first files, each format's mean over its four conditions (one per rotation).
usage: score.py ITEMS_PATTERN NAME=BITS_PATTERN ...   (patterns contain {s}; bits are float32 per byte)"""
import sys, json
import numpy as np
items_pat = sys.argv[1]
runs = [(a.split("=")[0], a.split("=")[1]) for a in sys.argv[2:]]
C = ["Rm", "Rn", "W", "N"]
acc = {}
pair = {}
desc = {nm: [] for nm, _ in runs}
for s in range(4):
    meta = json.load(open(items_pat.format(s=s)))["items"]
    bits = {nm: np.fromfile(p.format(s=s), dtype="<f4").astype(float) for nm, p in runs}
    for it in meta:
        first = "first" if it["k"] == 0 else "later"
        a = it["start"]
        parts = {"all": (a, a + it["len"]), "head32": (a, a + 32)}
        if it["magic"]:
            parts["magic"] = (a + it["magic"][0], a + it["magic"][0] + it["magic"][1])
        for nm, b in bits.items():
            for part, (x, y) in parts.items():
                acc.setdefault((part, first, it["cond"], nm), []).extend(b[x:y])
                if first == "first":
                    pair.setdefault((part, nm, it["format"]), {})[it["cond"]] = b[x:y].mean()
            if it["desc_len"]:
                desc[nm].extend(b[it["desc_start"]:it["desc_start"] + it["desc_len"]])
for part in ("magic", "head32", "all"):
    for fl in ("first", "later"):
        row = f"{part:<7}{fl:<6}"
        for c in C:
            n = len(acc.get((part, fl, c, runs[0][0]), []))
            row += f"| {c} n {n:>6} " + " ".join(f"{nm} {np.mean(acc.get((part, fl, c, nm), [np.nan])):5.2f}" for nm, _ in runs) + " "
        print(row)
print("first files, per format, condition minus N (mean over formats with all four; bits per byte):")
for part in ("magic", "head32", "all"):
    for nm, _ in runs:
        d = [v for (p, n, f), v in pair.items() if p == part and n == nm and len(v) == 4]
        row = f"  {part:<7}{nm:<10} formats {len(d):>3}"
        for c in ("Rm", "Rn", "W"):
            x = np.array([v[c] - v["N"] for v in d])
            row += f" | {c}-N {x.mean():+6.2f} (better in {np.sum(x < 0):>2})"
        print(row)
print("description bytes:" + "".join(f"  {nm} {np.mean(v):6.3f}" for nm, v in desc.items()))
