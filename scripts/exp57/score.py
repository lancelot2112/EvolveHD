"""Per-file bits per byte on the byte stream: Witten-Bell orders 2 and 4 and xz (in the stream, from baselines.py),
the word reader (its count model, the reader alone, their mixture; from R.bits) and the level-0 byte brain (from
level0.json, blocks of 1,000 bytes; files are multiples of 1,000 bytes).
usage: score.py STREAM READER_RUN BRAIN_DIR"""
import sys, json, lzma, os
import numpy as np
s, rr, bd = sys.argv[1], sys.argv[2], sys.argv[3]
data = open(s + ".bin", "rb").read(); meta = json.load(open(s + ".files.json"))["files"]
wb = np.load(s + ".wb.npy")
d = np.fromfile(rr + ".bits", dtype=np.dtype([('c', '<f4'), ('p', '<f4'), ('t', '<f4'), ('f', 'u1')])) if os.path.exists(rr + ".bits") else None
if d is not None:
    pred = (d['f'] == 1) | (d['f'] == 3); acting = d['f'] == 3
    A = np.array([0.0, 0.0625, 0.125, 0.25, 0.5, 0.75, 0.875, 0.96875]); lw = np.zeros((16, len(A)))
    pc, pr = 2.0 ** -d['c'].astype(float), d['p'].astype(float)
    bk = np.minimum((d['t'] * 8).astype(int), 7) + 8 * acting
    mix = d['c'].astype(float).copy()
    for i in np.nonzero(pred)[0]:
        pa = (1 - A) * pc[i] + A * pr[i]; w = lw[bk[i]]; e = 2.0 ** (w - w.max())
        mix[i] = -np.log2((e * pa).sum() / e.sum()); lw[bk[i]] += np.log2(pa)
    rd = -np.log2(np.maximum(pr, 1e-12))
bb = None
if os.path.exists(bd + "/level0.json"):
    bb = np.array(json.load(open(bd + "/level0.json"))["bpb"])
xz = lambda x: 8 * len(lzma.compress(x, preset=9 | lzma.PRESET_EXTREME))
print(f"{'':<3}{'type':<6}{'WB2':>7}{'WB4':>7}{'xz':>7} |{'counts':>7}{'reader':>7}{'mix':>7} {'acting':>7} |{'brain':>7}")
tot = {}
for m in meta:
    a, l = m["start"], m["len"]
    row = [wb[2, a:a + l].mean(), wb[4, a:a + l].mean(), (xz(data[:a + l]) - (xz(data[:a]) if a else 0)) / l]
    if d is not None:
        row += [d['c'][a:a + l].mean(), rd[a:a + l].mean(), mix[a:a + l].mean(), acting[a:a + l].mean()]
    else:
        row += [np.nan] * 4
    row += [bb[a // 1000:(a + l) // 1000].mean() if bb is not None and len(bb) >= (a + l) // 1000 else np.nan]
    for i, x in enumerate(row):
        tot.setdefault(i, []).append(x)
    print(f"{m['round']:<3}{m['type']:<6}" + "".join(f"{x:7.3f}" for x in row[:3]) + " |" + "".join(f"{x:7.3f}" for x in row[3:6]) + f" {row[6]:7.1%} |{row[7]:7.3f}")
print(f"{'all':<9}" + "".join(f"{np.mean(tot[i]):7.3f}" for i in range(3)) + " |" + "".join(f"{np.mean(tot[i]):7.3f}" for i in range(3, 6)) + f" {np.mean(tot[6]):7.1%} |{np.mean(tot[7]):7.3f}")
