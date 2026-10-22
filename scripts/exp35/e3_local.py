"""E3: one fact per cell vs locally vs fully distributed, at the same bit budget, as cues get noisy.
A key is a code of K (element, phase) pairs (K = 64); a noisy cue has a fraction f of its pairs redrawn.
 - hash: a table of value ids (10 bits each), addressed by a hash of the whole cue; newest write wins.
 - CAM-8: a table of entries, each 8 of the key's elements (14 bits each) + a value id; read = the entry sharing
   the most elements with the cue; oldest entry replaced.
 - sparse ternary trace, K = 64; sparse 1-bit trace, K = 16 (its own key, same noise fraction);
 - dense trace, 16 levels, every element active (fewer elements for the same bits).
Budget: a ternary trace of 16,384 elements = 51,937 bits. Depth: writes back at which recall of the exact value among all 1,000 is 80%."""
import numpy as np
from trace import Trace, depth, best_of_all
BITS = int(16384 * 2 * np.log2(3)); NV = 1000
def run(kind, f, M=6000, seed=0):
    r = np.random.default_rng(seed)
    if kind in ('tern', 'bit', 'dense'):
        L, K, D = {'tern': (3, 64, 16384), 'bit': (2, 16, int(BITS / 2)), 'dense': (16, None, int(BITS / 8))}[kind]
        K = K or D
        tr = Trace(D, L, NV, seed)
        if kind == 'dense':
            KI = np.tile(np.arange(D), (M, 1)); KP = r.integers(0, 4, (M, D)).astype(np.int8)
        else:
            KI = r.integers(0, D, (M, K)); KP = r.integers(0, 4, (M, K)).astype(np.int8)
        VV = r.integers(0, NV, M)
        for i in range(M): tr.write(KI[i], KP[i], VV[i])
        def recall(ii):
            ci, cp = KI[ii].copy(), KP[ii].copy(); n, k = ci.shape
            mask = r.random((n, k)) < f
            if kind != 'dense': ci[mask] = r.integers(0, D, mask.sum())
            cp[mask] = r.integers(0, 4, mask.sum())
            return best_of_all(tr, ci, cp, VV[ii])
    elif kind == 'hash':
        S = BITS // 10; table = -np.ones(S, int); VV = r.integers(0, NV, M); keyh = r.integers(0, 2**62, M)
        for i in range(M): table[keyh[i] % S] = VV[i]
        def recall(ii):
            hit = r.random(len(ii)) >= f ** 0 * (1 - (1 - f) ** 64)   # any redrawn pair changes the hash
            got = np.where(hit, table[keyh[ii] % S], r.integers(0, NV, len(ii)))
            # a miss is a random lookup: right only by chance among 50
            return np.where(hit, got == VV[ii], r.random(len(ii)) < 1 / NV)
    elif kind == 'cam':
        E = BITS // (8 * 14 + 10); D = 16384
        KI = r.integers(0, D, (M, 64)); VV = r.integers(0, NV, M)
        def recall(ii):
            out = []
            for i in ii:
                age = M - 1 - i
                if age >= E: out.append(r.random() < 1 / NV); continue
                cue = KI[i].copy(); m = r.random(64) < f; cue[m] = r.integers(0, D, m.sum())
                lo = M - E; stored = KI[lo:, :8]
                ov = np.isin(stored, cue).sum(1)
                out.append(VV[lo + int(np.argmax(ov + r.random(E) * 0.1))] == VV[i] if ov.max() > 0 else r.random() < 1 / NV)
            return np.array(out)
    ages = np.unique(np.round(np.geomspace(1, M - 1, 40)).astype(int)); acc = []
    for a in ages:
        ii = np.clip(M - a - np.arange(NQ), 0, M - 1); acc.append(float(np.mean(recall(ii))))
    return depth(acc, ages)
FS = (0.0, 0.1, 0.25, 0.5, 0.75)
import sys
KINDS = sys.argv[1].split(",") if len(sys.argv) > 1 else ("hash", "cam", "bit", "tern", "dense")
if KINDS != ["none"]: print("depth (writes back at 80% recall) by cue noise f; same bits for every store")
print(f"{'store':>8} " + " ".join(f"f={x:<5}" for x in FS))
import sys
KINDS = sys.argv[1].split(',') if len(sys.argv) > 1 else ('hash', 'cam', 'bit', 'tern', 'dense')
NQ = 80 if len(sys.argv) < 3 else int(sys.argv[2])
for kind in KINDS:
    print(f"{kind:>8} " + " ".join(f"{run(kind, x):>7}" for x in FS), flush=True)
# graded broadcast: a contextified ternary trace whose K context slots are partly redrawn at each boundary
print("\ngraded reset: recall of facts written b boundaries ago (ternary, K = 64, D = 16384, 20 facts per story)")
D, K = 16384, 64
for B in (8, 16, 32, 64):
    r = np.random.default_rng(B); tr = Trace(D, 3, NV, 1)
    off = r.integers(0, D, K); ph = r.integers(0, 4, K); hist = []
    base = r.integers(0, D, (400, K)); bph = r.integers(0, 4, (400, K)); VV = r.integers(0, NV, 400)
    offs = []
    for s in range(20):
        offs.append((off.copy(), ph.copy()))
        for j in range(20):
            i = s * 20 + j; tr.write((base[i] + off) % D, (bph[i] + ph) % 4, VV[i])
        sl = r.choice(K, B, replace=False); off[sl] = r.integers(0, D, B); ph[sl] = r.integers(0, 4, B)
    res = []
    for b in (0, 1, 2, 4, 8):
        s = 19 - b; ii = np.arange(s * 20, s * 20 + 20)
        ci = (base[ii] + off) % D; cp = (bph[ii] + ph) % 4      # cued in the *current* context
        res.append(float(np.mean(best_of_all(tr, ci, cp, VV[ii]))))
    print(f"  {B:>2} of 64 slots redrawn per boundary: " + "  ".join(f"b={b}:{x:4.0%}" for b, x in zip((0, 1, 2, 4, 8), res)))
