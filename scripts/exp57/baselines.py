"""Baselines on the byte stream, per file, in bits per byte:
  - Witten-Bell orders 0-4 (each order backs off to the one below; order 0 to uniform over 256), prequential over the
    whole stream, so each file is coded after everything before it;
  - xz -9e, bzip2 -9 and gzip -9 on the stream up to the end of each file minus the stream up to its start (the same
    "after everything before it" cost), and xz on the file alone.
Writes OUT.wb.npy (per byte bits, orders 0-4) and prints a table.
usage: baselines.py STREAM"""
import sys, json, subprocess, lzma, bz2, zlib
import numpy as np
from collections import defaultdict
s = sys.argv[1]
data = open(s + ".bin", "rb").read(); meta = json.load(open(s + ".files.json"))["files"]
n = len(data); K = 4
cnt = [defaultdict(lambda: defaultdict(int)) for _ in range(K + 1)]
bits = np.zeros((K + 1, n))
for i, b in enumerate(data):
    p = 1.0 / 256
    for k in range(K + 1):
        if i < k:
            bits[k, i] = -np.log2(p); continue
        ctx = data[i - k:i]
        d = cnt[k].get(ctx)
        if d:
            tot = sum(d.values()); u = len(d)
            p = (d.get(b, 0) + u * p) / (tot + u)
        bits[k, i] = -np.log2(p)
    for k in range(K + 1):
        if i >= k:
            cnt[k][data[i - k:i]][b] += 1
np.save(s + ".wb.npy", bits)
def comp(f, x): return 8 * len(f(x))
xz = lambda x: lzma.compress(x, preset=9 | lzma.PRESET_EXTREME)
bz = lambda x: bz2.compress(x, 9)
gz = lambda x: zlib.compress(x, 9)
print(f"{'file':<8}{'type':<6}" + "".join(f"WB{k:<5}" for k in range(K + 1)) + "xz-str bz2-str gz-str  xz-alone")
for m in meta:
    a, l = m["start"], m["len"]
    wb = [bits[k, a:a + l].mean() for k in range(K + 1)]
    cs = [(comp(f, data[:a + l]) - (comp(f, data[:a]) if a else 0)) / l for f in (xz, bz, gz)]
    al = comp(xz, data[a:a + l]) / l
    print(f"{m['round']:<8}{m['type']:<6}" + "".join(f"{x:<7.3f}" for x in wb) + "".join(f"{x:<7.3f}" for x in cs) + f" {al:.3f}")
