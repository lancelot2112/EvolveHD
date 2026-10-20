"""Score the heteroassociative store alone (evolvehd assoc --out PREFIX) at the answer positions of the second
half: per hop, how often the answer is the first candidate, among the candidates at all, and the first
candidate of the answer's kind (places for where-questions, names for who-has: the kind the word level
predicts anyway). usage: assoc_score.py IDS INFO.json HOPS K PREFIX ..."""
import sys, json
import numpy as np
ids = np.fromfile(sys.argv[1], dtype=np.uint32); n = len(ids)
info = json.load(open(sys.argv[2])); hops = int(sys.argv[3]); K = int(sys.argv[4])
ans = [(p, k) for p, k in info['answers'] if p >= n // 2]
places, names = set(info['places']), set(info['names'])
for pre in sys.argv[5:]:
    r = np.fromfile(pre + ".rank", dtype=np.uint8).reshape(n, hops)
    top = np.fromfile(pre + ".top", dtype=np.uint32).reshape(n, hops, K)
    row = [f"{pre.split('/')[-1]:<10}"]
    for k in ['where-person', 'where-object', 'who-has', 'all']:
        ps = [p for p, kk in ans if k == 'all' or kk == k]
        parts = []
        for h in range(hops):
            kind = lambda p: names if dict(ans)[p] == 'who-has' else places
            first = [next((int(x) for x in top[p, h] if int(x) in kind(p)), None) for p in ps]
            kt = np.mean([f == ids[p] for f, p in zip(first, ps)])
            parts.append(f"h{h+1} {np.mean(r[ps, h] == 0):.0%}/{np.mean(r[ps, h] < 255):.0%}/kind {kt:.0%}")
        row.append(f"{k}: " + " ".join(parts))
    print("  ".join(row))
