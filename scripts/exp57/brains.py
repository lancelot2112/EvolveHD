"""Per-file bits per byte for several byte-brain runs (runs/exp57/brain_TAG/level0.bits), with WB4 and xz.
usage: brains.py TAG ..."""
import sys, json, lzma
import numpy as np
s = 'runs/exp57/bytes'; data = open(s + '.bin', 'rb').read(); meta = json.load(open(s + '.files.json'))['files']; wb = np.load(s + '.wb.npy')
B = {t: np.fromfile(f'runs/exp57/brain_{t}/level0.bits', dtype='<f4').astype(float) for t in sys.argv[1:]}
M = {t: np.array(json.load(open(f'runs/exp57/brain_{t}/level0.json'))['bpb_map']) for t in sys.argv[1:]}
xz = lambda x: 8 * len(lzma.compress(x, preset=9 | lzma.PRESET_EXTREME))
print(f"{'':<3}{'type':<6}{'WB4':>7}{'xz':>7} |" + ''.join(f"{t:>8}" for t in B) + " | map bits " + ' '.join(B))
X, W = [], []
for m in meta:
    a, l = m['start'], m['len']; x = (xz(data[:a + l]) - (xz(data[:a]) if a else 0)) / l; X.append(x); W.append(wb[4, a:a + l].mean())
    print(f"{m['round']:<3}{m['type']:<6}{W[-1]:7.3f}{x:7.3f} |" + ''.join(f"{B[t][a:a + l].mean():8.3f}" for t in B) + " | " + ' '.join(f"{M[t][a // 1000:(a + l) // 1000].mean():.2f}" for t in B))
print(f"{'all':<9}{np.mean(W):7.3f}{np.mean(X):7.3f} |" + ''.join(f"{B[t].mean():8.3f}" for t in B))
for r in 'AB':
    print(f"round {r} without gzip: WB4 {np.mean([w for w, m in zip(W, meta) if m['round'] == r and m['type'] != 'gzip']):.3f} xz {np.mean([w for w, m in zip(X, meta) if m['round'] == r and m['type'] != 'gzip']):.3f} " +
          ' '.join(f"{t} {np.mean([B[t][m['start']:m['start'] + m['len']].mean() for m in meta if m['round'] == r and m['type'] != 'gzip']):.3f}" for t in B))
