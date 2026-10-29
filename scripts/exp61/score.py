"""Exp 61: the test stream (experiment 57's 16 files) after pretraining on primitives (P), after plain text of the
same length (C), and with nothing before it (0: experiment 57's stream alone, same flags with the position sense);
and, during pretraining, each primitive kind's first and last files.
usage: score.py P_DIR C_DIR ZERO_DIR"""
import sys, json
import numpy as np
pd, cd, zd = sys.argv[1:4]
tm = json.load(open('runs/exp57/bytes.files.json'))['files']
P = np.fromfile(pd + '/level0.bits', dtype='<f4').astype(float); C = np.fromfile(cd + '/level0.bits', dtype='<f4').astype(float)
Z = np.fromfile(zd + '/level0.bits', dtype='<f4').astype(float)
po = json.load(open('runs/exp61/P.files.json'))['test_from']; co = json.load(open('runs/exp61/C.files.json'))['test_from']
print(f"{'':<3}{'type':<6}{'none':>8}{'text':>8}{'prims':>8}   prims - text")
t = {'0': [], 'C': [], 'P': []}
for m in tm:
    a, l = m['start'], m['len']
    z, c, p = Z[a:a + l].mean(), C[co + a:co + a + l].mean(), P[po + a:po + a + l].mean()
    t['0'].append(z); t['C'].append(c); t['P'].append(p)
    print(f"{m['round']:<3}{m['type']:<6}{z:8.3f}{c:8.3f}{p:8.3f}   {p - c:+.3f}")
print(f"{'all':<9}{np.mean(t['0']):8.3f}{np.mean(t['C']):8.3f}{np.mean(t['P']):8.3f}   {np.mean(t['P']) - np.mean(t['C']):+.3f}")
for r in 'AB':
    ix = [i for i, m in enumerate(tm) if m['round'] == r and m['type'] != 'gzip']
    print(f"round {r} without gzip: none {np.mean([t['0'][i] for i in ix]):.3f} text {np.mean([t['C'][i] for i in ix]):.3f} prims {np.mean([t['P'][i] for i in ix]):.3f}")
pm = [m for m in json.load(open('runs/exp61/P.files.json'))['files'] if m['start'] < po]
print("pretraining, per kind: first file / last file / all (bits per byte)")
for k in sorted(set(m['kind'] for m in pm)):
    fs = [P[m['start']:m['start'] + m['len']].mean() for m in pm if m['kind'] == k]
    print(f"  {k:<8} {fs[0]:6.3f} {fs[-1]:6.3f} {np.mean(fs):6.3f}  (n {len(fs)})")
