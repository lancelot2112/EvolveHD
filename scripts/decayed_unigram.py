"""Recency-weighted (exponentially decayed) unigram baseline on a u32 symbol stream; second half scored.
A level that only tracks drifting symbol frequencies matches this; sequential learning has to beat it.
usage: decayed_unigram.py FILE VOCAB [halflives...]   (a huge halflife reproduces a static add-0.5 unigram)"""
import numpy as np, math, sys
def run(path, V, hl):
    x = np.fromfile(path, dtype=np.uint32); n = len(x); h = n // 2
    g = 0.5 ** (1 / hl); inc = 1.0; w = np.zeros(max(V, int(x.max()) + 1) + 2); tot = 0.0; s = 0.0
    for i, c in enumerate(x.tolist()):
        p = (w[c] + 0.5 * inc / V) / (tot + 0.5 * inc)
        if i >= h: s -= math.log2(p)
        inc /= g; w[c] += inc; tot += inc
        if inc > 1e100: w /= inc; tot /= inc; inc = 1.0
    return s / (n - h)
path, V = sys.argv[1], int(sys.argv[2])
hls = [float(a) for a in sys.argv[3:]] or [300, 1000, 3000, 10000, 1e12]
r = {hl: run(path, V, hl) for hl in hls}
best = min(r, key=r.get)
print(path, ' '.join(f'hl{hl:g}:{v:.3f}' for hl, v in r.items()), f'| best {r[best]:.3f} at {best:g}')
