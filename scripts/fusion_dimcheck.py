"""Dimensionality check for fusing role-bound sparse codes: OR, OR + thinning, spot regions, XOR (experiment 22)."""
import numpy as np
rng = np.random.default_rng(0)
D, K, V = 2048, 32, 512                      # bits, active bits per code, fillers per role
codes = np.zeros((V, D), bool)
for v in range(V):
    codes[v, rng.choice(D, K, replace=False)] = True
shift = lambda c, r: np.roll(c, 977 * (r + 1) % D)   # role binding by rotation (as shift-bind does)

def fuse(parts, scheme, n):
    bound = [shift(codes[p], r) for r, p in enumerate(parts)]
    if scheme == 'or':                                   # plain superposition, density grows with n
        return np.any(bound, axis=0)
    if scheme == 'or+thin':                              # superpose, then thin back to K bits
        u = np.any(bound, axis=0)                        # (context-dependent thinning: keep bits of u that are
        perm = np.roll(u, 501) | np.roll(u, 1303) | np.roll(u, 1777)   # also set in permuted copies of u)
        t = u & perm
        idx = np.flatnonzero(t)
        if len(idx) < K:                                 # top up deterministically if thinning removed too much
            extra = np.flatnonzero(u & ~t)
            idx = np.concatenate([idx, extra[:K - len(idx)]])
        out = np.zeros(D, bool); out[np.sort(idx)[:K] if len(idx) > K else idx] = True
        return out
    if scheme == 'spot':                                 # each role owns a fixed 1/n of the positions
        out = np.zeros(D, bool)
        for r, b in enumerate(bound):
            spot = (np.arange(D) * n // D) == r          # role r's region
            out |= b & spot
        return out
    if scheme == 'xor':
        out = np.zeros(D, bool)
        for b in bound: out ^= b
        return out

def trial(scheme, n, m, reps=400):
    ov = []
    for _ in range(reps):
        a = rng.integers(0, V, n); b = a.copy()
        diff = rng.choice(n, n - m, replace=False)       # b shares exactly m (role, filler) pairs with a
        for i in diff:
            x = rng.integers(0, V)
            while x == a[i]: x = rng.integers(0, V)
            b[i] = x
        fa, fb = fuse(a, scheme, n), fuse(b, scheme, n)
        ov.append((fa & fb).sum())
    ov = np.array(ov); return ov.mean(), ov.std(), fa.sum()

for n in [2, 4, 8]:
    print(f"\n== {n} parts")
    for scheme in ['or', 'or+thin', 'spot', 'xor']:
        rows = [trial(scheme, n, m) for m in range(n + 1)]
        bits = rows[0][2]
        # separability of 'shares m' vs 'shares m-1': d' = gap / pooled sd
        dp = [(rows[m][0] - rows[m-1][0]) / max(1e-9, np.sqrt((rows[m][1]**2 + rows[m-1][1]**2) / 2)) for m in range(1, n + 1)]
        print(f"  {scheme:8s} active bits {bits:4d} | overlap by shared parts: " +
              ' '.join(f"{m}:{r[0]:.1f}±{r[1]:.1f}" for m, r in enumerate(rows)) +
              f" | d' per extra shared part: min {min(dp):.1f}")
# XOR on sparse codes: is it a binding (dissimilar to its inputs)?
x, y = codes[1], codes[2]
print("\nXOR of two sparse codes: bits", (x ^ y).sum(), "| overlap with x", ((x ^ y) & x).sum(), "of", K,
      "| OR overlap with x", ((x | y) & x).sum())
xs = shift(x, 0)
print("rotation as binding: overlap of rotated x with x", (xs & x).sum(), "(chance", K * K / D, ")")
# dense (50%) codes: XOR is a real binding there
xd, yd = rng.random(D) < .5, rng.random(D) < .5
print("dense codes: XOR overlap with x", ((xd ^ yd) & xd).sum(), "of", xd.sum(), "(chance half)")
