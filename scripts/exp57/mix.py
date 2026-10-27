"""A switching mixture (fixed share, Herbster and Warmuth) over coders' per-byte probabilities of the actual byte:
the level-0 byte brain, the word reader (its mixture with its count model), Witten-Bell order 4, and uniform (8 bits).
Prequential: each byte's weights come from the bytes before it; a share alpha of the weight moves to the others
each byte, so the mixture can switch coders when the file type changes. Prints per-file bits per byte.
usage: mix.py STREAM READER_RUN BRAIN_DIR [ALPHA]"""
import sys, json, subprocess
import numpy as np
s, rr, bd = sys.argv[1], sys.argv[2], sys.argv[3]; alpha = float(sys.argv[4]) if len(sys.argv) > 4 else 1e-3
meta = json.load(open(s + ".files.json"))["files"]
wb = np.load(s + ".wb.npy")
brain = np.fromfile(bd + "/level0.bits", dtype='<f4').astype(float)
# the word reader's mixture, recomputed as in scripts/exp57/score.py
out = subprocess.run(["python3", "-c", f"""
import numpy as np
d=np.fromfile('{rr}.bits',dtype=np.dtype([('c','<f4'),('p','<f4'),('t','<f4'),('f','u1')]))
pred=(d['f']==1)|(d['f']==3); acting=d['f']==3
A=np.array([0.0,0.0625,0.125,0.25,0.5,0.75,0.875,0.96875]); lw=np.zeros((16,len(A)))
pc,pr=2.0**-d['c'].astype(float),d['p'].astype(float); bk=np.minimum((d['t']*8).astype(int),7)+8*acting
mix=d['c'].astype(float).copy()
for i in np.nonzero(pred)[0]:
    pa=(1-A)*pc[i]+A*pr[i]; w=lw[bk[i]]; e=2.0**(w-w.max()); mix[i]=-np.log2((e*pa).sum()/e.sum()); lw[bk[i]]+=np.log2(pa)
np.save('{rr}.mix.npy',mix)"""], check=True)
reader = np.load(rr + ".mix.npy")
names = ["brain", "reader", "WB4", "uniform"]
E = np.stack([brain, reader, wb[4], np.full(len(brain), 8.0)])  # bits
P = 2.0 ** -E
n = P.shape[1]; k = P.shape[0]
w = np.full(k, 1.0 / k); mix = np.zeros(n); share = np.zeros((k, n))
for i in range(n):
    p = (w * P[:, i]).sum(); mix[i] = -np.log2(p); share[:, i] = w
    w = w * P[:, i] / p
    w = (1 - alpha) * w + alpha * (1 - w) / (k - 1)  # fixed share
print(f"{'':<3}{'type':<6}" + "".join(f"{x:>8}" for x in names) + f"{'switch':>8}  weight most on")
for m in meta:
    a, l = m["start"], m["len"]
    r = [E[j, a:a + l].mean() for j in range(k)]
    top = names[int(np.argmax(share[:, a:a + l].mean(1)))]
    print(f"{m['round']:<3}{m['type']:<6}" + "".join(f"{x:8.3f}" for x in r) + f"{mix[a:a + l].mean():8.3f}  {top}")
print(f"{'all':<9}" + "".join(f"{E[j].mean():8.3f}" for j in range(k)) + f"{mix.mean():8.3f}")
