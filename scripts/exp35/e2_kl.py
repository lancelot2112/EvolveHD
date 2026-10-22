"""E2: depth (80% recall among 50 candidates) of a sparse trace by active elements K and levels L, at two sizes D.
Random keys, each used once, values from 1,000. Prediction: best K ~ 4 L^2, independent of D."""
import numpy as np, sys
from trace import Trace, depth
D = int(sys.argv[1]); LS = [int(x) for x in sys.argv[2].split(',')]
KS = [8, 12, 16, 24, 32, 48, 64, 96, 128, 192, 256, 384, 512, 768, 1024, 1536, 2048, 4096, 8192]
KS = [k for k in KS if k <= D // 2]
for L in LS:
    row = []
    for K in KS:
        r = np.random.default_rng(K * 31 + L)
        tr = Trace(D, L, seed=L)
        M = int(min(40000, max(3000, D * 0.35)))
        KI = r.integers(0, D, (M, K)); KP = r.integers(0, 4, (M, K)).astype(np.int8); VV = r.integers(0, 1000, M)
        for i in range(M): tr.write(KI[i], KP[i], VV[i])
        ages = np.unique(np.round(np.geomspace(1, M - 1, 45)).astype(int)); acc = []
        for a in ages:
            ii = M - 1 - (a - 1) - np.arange(0, 40) * 0  # the binding at that age, plus neighbours
            ii = np.clip(M - a - np.arange(40), 0, M - 1)
            c = np.concatenate([VV[ii][:, None], r.integers(0, 1000, (len(ii), 49))], 1)
            sc = tr.scores(KI[ii], KP[ii], c); acc.append(float(np.mean(sc.argmax(1) == 0)))
        row.append(depth(acc, ages))
    b = int(np.argmax(row))
    print(f"D={D} L={L:>2}: " + " ".join(f"{k}:{d}" for k, d in zip(KS, row)) + f"  | best K={KS[b]} depth={row[b]} (4L^2={4*L*L})", flush=True)
