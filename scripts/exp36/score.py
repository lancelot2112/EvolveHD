"""Score `evolvehd assoc --sit --out PREFIX` runs at the answer positions of the second half: the top candidate
(no kind filter) and, for comparison, the first candidate of the answer's kind.
usage: score.py IDS INFO.json PREFIX ..."""
import sys, json
import numpy as np
ids = np.fromfile(sys.argv[1], dtype=np.uint32); n = len(ids)
info = json.load(open(sys.argv[2])); K = 8
ans = [(p, k) for p, k in info['answers'] if n // 2 <= p < n]
places, names = set(info['places']), set(info['names'])
for pre in sys.argv[3:]:
    top = np.fromfile(pre + ".top", dtype=np.uint32).reshape(n, 1, K)[:, 0]
    row = [f"{pre.split('/')[-1]:<18}"]
    for k in ('where-person', 'where-object', 'who-has', 'all'):
        ps = [p for p, kk in ans if k == 'all' or kk == k]
        t1 = np.mean([top[p, 0] == ids[p] for p in ps])
        kf = np.mean([next((int(x) for x in top[p] if int(x) in (names if kk == 'who-has' else places)), -1) == ids[p] for p, kk in ans if (k == 'all' or kk == k)])
        row.append(f"{k} {t1:5.1%} ({kf:5.1%})")
    print("  ".join(row))
