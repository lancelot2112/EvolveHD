"""Per-file bits per byte on experiment 57's 16 files for several runs, each given as NAME=DIR[:OFFSET] (OFFSET:
where the test stream starts in that run's stream, e.g. 1021800 after pretraining). With WB4 and xz for reference.
usage: table.py NAME=DIR[:OFFSET] ..."""
import sys, json, lzma
import numpy as np
s = 'runs/exp57/bytes'; data = open(s + '.bin', 'rb').read(); meta = json.load(open(s + '.files.json'))['files']; wb = np.load(s + '.wb.npy')
runs = []
for a in sys.argv[1:]:
    nm, rest = a.split('=', 1); d, _, off = rest.partition(':')
    runs.append((nm, np.fromfile(d + '/level0.bits', dtype='<f4').astype(float), int(off or 0)))
xz = lambda x: 8 * len(lzma.compress(x, preset=9 | lzma.PRESET_EXTREME))
print(f"{'':<3}{'type':<6}{'WB4':>7}{'xz':>7} |" + ''.join(f"{nm:>9}" for nm, _, _ in runs))
X, W, R = [], [], {nm: [] for nm, _, _ in runs}
for m in meta:
    a, l = m['start'], m['len']; x = (xz(data[:a + l]) - (xz(data[:a]) if a else 0)) / l; X.append(x); W.append(wb[4, a:a + l].mean())
    for nm, b, o in runs: R[nm].append(b[o + a:o + a + l].mean())
    print(f"{m['round']:<3}{m['type']:<6}{W[-1]:7.3f}{x:7.3f} |" + ''.join(f"{R[nm][-1]:9.3f}" for nm, _, _ in runs))
print(f"{'all':<9}{np.mean(W):7.3f}{np.mean(X):7.3f} |" + ''.join(f"{np.mean(R[nm]):9.3f}" for nm, _, _ in runs))
for r in 'AB':
    ix = [i for i, m in enumerate(meta) if m['round'] == r and m['type'] != 'gzip']
    print(f"{r} w/o gzip{np.mean([W[i] for i in ix]):7.3f}{np.mean([X[i] for i in ix]):7.3f} |" + ''.join(f"{np.mean([R[nm][i] for i in ix]):9.3f}" for nm, _, _ in runs))
