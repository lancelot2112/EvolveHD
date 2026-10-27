"""Table of probe results: rows = configuration tags, columns = set/probe, mean top-1 over the seeds found.
Tag "off" reads experiment 55's runs. usage: summ.py TAG ..."""
import sys, re, glob, os
import numpy as np
cols = [(S, P) for S in ("qs", "st") for P in ("base", "names", "verb", "names_verb", "question", "longer", "all")]
def acc(f):
    t = open(f).read(); m = re.search(r'all\s+([\d.]+)%', t)
    k = dict(re.findall(r'(who-has|where-object) ([\d.]+)%', t))
    return float(m.group(1)), k
print("tag".ljust(8) + "".join(f"{S}:{P}".rjust(15) for S, P in cols))
for tag in sys.argv[1:]:
    row = tag.ljust(8)
    for S, P in cols:
        pat = f"runs/exp55/R_{S}_{P}_s?.score" if tag == "off" else f"runs/exp56/R_{tag}_{S}_{P}_s?.score"
        fs = sorted(glob.glob(pat))
        if not fs:
            row += "-".rjust(15); continue
        a = [acc(f) for f in fs]
        v = np.mean([x[0] for x in a])
        wh = np.mean([float(x[1].get("who-has", "nan")) for x in a]) if S == "st" else None
        row += (f"{v:.1f}" + (f"/{wh:.0f}" if wh is not None else "") + f"({len(fs)})").rjust(15)
    print(row)
