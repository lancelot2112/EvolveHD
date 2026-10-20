"""Inspect the store's top candidates at answer positions: assoc_top.py IDS INFO.json PREFIX"""
import sys, json, collections
import numpy as np
ids = np.fromfile(sys.argv[1], dtype=np.uint32); n = len(ids); voc = open(sys.argv[1].replace('.ids', '.txt')).read().split()
info = json.load(open(sys.argv[2])); top = np.fromfile(sys.argv[3] + ".top", dtype=np.uint32); K = int(sys.argv[4]) if len(sys.argv) > 4 else 8; h = len(top) // n // K
top = top.reshape(n, h, K)[:, :, 0]; ans = [p for p, k in info['answers'] if p >= n // 2]
V = lambda t: voc[t] if t < len(voc) else '-'
for j in range(h):
    print(f"hop {j+1} top-1 at answers:", collections.Counter(V(t) for t in top[ans, j]).most_common(8))
for p in ans[:8]:
    print('  ', ' '.join(voc[x] for x in ids[p-8:p]), '->', voc[ids[p]], '| top', [V(t) for t in top[p]])
