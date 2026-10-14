"""Prequential interpolated Kneser-Ney (discount 0.75) on a u32 symbol stream; scores the second half.
Lower orders use continuation counts. usage: kn_orders.py file.syms vocab [maxorder]"""
import sys, math
import numpy as np
from collections import defaultdict
x = np.fromfile(sys.argv[1], dtype=np.uint32).tolist()
V = int(sys.argv[2]); M = int(sys.argv[3]) if len(sys.argv) > 3 else 3
D = 0.75
def run(order):
    c = [defaultdict(lambda: [0, defaultdict(int)]) for _ in range(order + 1)]
    types = set()
    tot = 0.0; h = len(x) // 2
    for i, s in enumerate(x):
        top = min(order, i)
        p = 1.0 / V
        for k in range(top + 1):
            e = c[k].get(tuple(x[i - k:i]))
            if e and e[0] > 0:
                p = (max(e[1].get(s, 0) - D, 0) + D * len(e[1]) * p) / e[0]
        if i >= h: tot -= math.log2(p)
        for k in range(top + 1):
            ctx = tuple(x[i - k:i])
            if k == order:
                inc = True
            else:
                g = tuple(x[i - k - 1:i]) + (s,) if i - k - 1 >= 0 else None
                inc = g is None or g not in types
            if inc:
                e = c[k][ctx]; e[0] += 1; e[1][s] += 1
        for k in range(1, top + 2):
            if i - k >= 0: types.add(tuple(x[i - k:i]) + (s,))
    return tot / (len(x) - h)
print(sys.argv[1], ' '.join(f'KN{o} {run(o):.3f}' for o in range(M + 1)), flush=True)
