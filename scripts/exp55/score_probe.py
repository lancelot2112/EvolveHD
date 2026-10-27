"""Score a run on a probe stream (or on the unchanged stream, using a probe stream's info for which answers count):
top-1 over the answers in probed stories, by kind, in blocks of 25 in order (how fast it catches on), and the
prequential bits at those answers and over the probed words (count model, reader, mixture; see score_bits.py).
usage: score_probe.py DOC RUN [PROBE_INFO]"""
import sys, json
import numpy as np
doc, run = sys.argv[1], sys.argv[2]
ids = np.fromfile(doc + ".ids", dtype=np.uint32); info = json.load(open(doc + ".info.json"))
pinfo = json.load(open(sys.argv[3])) if len(sys.argv) > 3 else info
f0 = pinfo["probe_first_answer"]
top = np.fromfile(run + ".top", dtype=np.uint32).reshape(len(ids), -1)
A = info["answers"][f0:]
kind = [a[1] if isinstance(a[1], str) and a[1].startswith(("where", "who")) else "all" for a in A]
right = np.array([top[a[0], 0] == ids[a[0]] for a in A])
row = [f"all {right.mean():6.1%} (n {len(A)})"]
for k in sorted(set(kind) - {"all"}):
    m = np.array([x == k for x in kind]); row.append(f"{k} {right[m].mean():5.1%}")
print("  top-1: " + "  ".join(row))
print("  in blocks of 25: " + " ".join(f"{right[i:i + 25].mean() * 100:3.0f}" for i in range(0, len(right), 25)))
d = np.fromfile(run + ".bits", dtype=np.dtype([('c', '<f4'), ('p', '<f4'), ('t', '<f4'), ('f', 'u1')]))
pred = (d['f'] == 1) | (d['f'] == 3)
Aw = np.array([0.0, 0.0625, 0.125, 0.25, 0.5, 0.75, 0.875, 0.96875]); lw = np.zeros((16, len(Aw)))
pc, pr = 2.0 ** -d['c'].astype(float), d['p'].astype(float); bk = np.minimum((d['t'] * 8).astype(int), 7) + 8 * (d['f'] == 3)
mix = d['c'].astype(float).copy()
for i in np.nonzero(pred)[0]:
    pa = (1 - Aw) * pc[i] + Aw * pr[i]; w = lw[bk[i]]; e = 2.0 ** (w - w.max())
    mix[i] = -np.log2((e * pa).sum() / e.sum()); lw[bk[i]] += np.log2(pa)
# the probed words: from the first probed story on (the unchanged stream: the same place, since the names probe moves
# no words)
ws = np.zeros(len(d), bool); ws[info.get("probe_from", pinfo["probe_from"]):] = True
am = np.zeros(len(d), bool); am[[a[0] for a in A]] = True
rd = -np.log2(np.maximum(pr, 1e-12))
for nm, m in (("answers", am & pred), ("probed words", ws & pred)):
    if m.sum(): print(f"  bits per word, {nm} (n {m.sum()}): counts {d['c'][m].mean():.3f}  reader {rd[m].mean():.3f}  mix {mix[m].mean():.3f}")
