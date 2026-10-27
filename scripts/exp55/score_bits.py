"""Prequential bits per word from a run's OUT.bits (counts bits, reader probability, reader top probability, flag: 1 predicted, 3 predicted while acting).
Reports the count model, the reader alone (calibrated by rank counts) and their mixture, over all words and, when the
document has answer positions (.info.json "answers"), at answers and elsewhere, split train/test at FROM.
The mixture here is the per-bucket Bayesian mixture over fixed weights, recomputed online (prequential).
usage: score_bits.py DOC RUN [FROM]"""
import sys, json, os
import numpy as np
doc, run = sys.argv[1], sys.argv[2]; frm = int(sys.argv[3]) if len(sys.argv) > 3 else None
d = np.fromfile(run + ".bits", dtype=np.dtype([('c', '<f4'), ('p', '<f4'), ('t', '<f4'), ('f', 'u1')]))
A = np.array([0.0, 0.0625, 0.125, 0.25, 0.5, 0.75, 0.875, 0.96875])
lw = np.zeros((16, len(A))); mix = np.zeros(len(d))
pc_all = 2.0 ** -d['c'].astype(float); pr_all = d['p'].astype(float)
pred = (d['f'] == 1) | (d['f'] == 3); acting = d['f'] == 3
bk = np.minimum((d['t'] * 8).astype(int), 7) + 8 * acting
for i in range(len(d)):
    if not pred[i]:
        mix[i] = d['c'][i]; continue
    pa = (1 - A) * pc_all[i] + A * pr_all[i]; w = lw[bk[i]]; e = 2.0 ** (w - w.max())
    mix[i] = -np.log2((e * pa).sum() / e.sum()); lw[bk[i]] += np.log2(pa)
rd = -np.log2(np.maximum(pr_all, 1e-12))
ans = np.zeros(len(d), bool)
if os.path.exists(doc + ".info.json"):
    for a in json.load(open(doc + ".info.json")).get("answers", []): ans[a[0]] = True
def row(name, m):
    m = m & pred
    if m.sum(): print(f"  {name:<22} n {m.sum():>8}  counts {d['c'][m].mean():6.3f}  reader {rd[m].mean():6.3f}  mix {mix[m].mean():6.3f}")
idx = np.arange(len(d))
parts = [("all", idx >= 0)] if frm is None else [("train", idx < frm), ("test", idx >= frm)]
for nm, m in parts:
    row(nm, m)
    row(nm + " acting", m & acting)
    if ans.any(): row(nm + " answers", m & ans); row(nm + " other words", m & ~ans)
