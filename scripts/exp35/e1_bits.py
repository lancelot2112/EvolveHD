"""E1: bits as a time constant. Every cell is a saturating counter with S states, stepped up when its bit is on
and down when it is off (local to the cell). S = 2 is 1 bit (the last observation), S = 3 is ternary.
(a) Tracking: a variable holds one of V values and changes to a new one every T events on average; each event
    shows the true value with probability q, otherwise a random one. Read: the value whose cell is highest.
(b) Fixed classes from noisy examples (as learn_counters.py), with up-only and up/down counters of S states, out to
    many examples, to see when up-only counters saturate."""
import numpy as np, sys
rng = np.random.default_rng(1)
V = 8
SS = [2, 3, 4, 8, 16, 64, 256, 2**16, 2**31]
lab = {2: '1b', 3: 'tern', 4: '2b', 8: '3b', 16: '4b', 64: '6b', 256: '8b', 2**16: '16b', 2**31: '32b'}
def track(T, q, S, R=400, L=None):
    L = L or max(4000, 40 * min(T, 2000))
    c = np.zeros((R, V), np.int64); val = rng.integers(0, V, R); ok = 0; n = 0
    for t in range(L):
        if T < 10**9:
            ch = rng.random(R) < 1 / T; val = np.where(ch, rng.integers(0, V, R), val)
        pred = np.argmax(c + rng.random((R, V)) * 0.5, 1)
        if t > L // 4: ok += (pred == val).sum(); n += R
        o = np.where(rng.random(R) < q, val, rng.integers(0, V, R))
        on = np.zeros((R, V), bool); on[np.arange(R), o] = True
        c = np.clip(c + np.where(on, 1, -1), 0, S - 1)
    return ok / n
if 'a' in sys.argv[1:]:
    print("(a) tracking accuracy; rows: mean events between changes T, obs reliability q; columns: counter states")
    print(f"{'T':>6} {'q':>5} " + " ".join(f"{lab[s]:>6}" for s in SS) + "   best")
    for q in (1.0, 0.6, 0.35, 0.2):
        for T in (2, 8, 32, 128, 512, 10**10):
            r = [track(T, q, s) for s in SS]
            b = SS[int(np.argmax(r))]
            print(f"{('inf' if T > 10**9 else T):>6} {q:>5} " + " ".join(f"{x:6.1%}" for x in r) + f"   {lab[b]}", flush=True)
D, C, k = 2048, 20, 64
def classes(mode, S, N, share=0.75, keep=0.5, noise=64, seed=1):
    r = np.random.default_rng(seed)
    protos = []
    for c in range(0, C, 2):
        a = r.choice(D, k, replace=False); b = a.copy(); m = int(k * (1 - share))
        b[:m] = r.choice(np.setdiff1d(np.arange(D), a), m, replace=False); protos += [a, b]
    protos = np.array(protos)
    def ex(c):
        return np.unique(np.concatenate([protos[c][r.random(k) < keep], r.integers(0, D, noise)]))
    mem = np.zeros((C, D), np.int64)
    for i in range(N):
        for c in range(C):
            x = np.zeros(D, bool); x[ex(c)] = True
            mem[c] = np.clip(mem[c] + (x if mode == 'up' else np.where(x, 1, -1)), 0, S - 1)
    learned = [set(np.argsort(-(mem[c] + r.random(D) * 0.01))[:k]) for c in range(C)]
    ok = 0
    for _ in range(20):
        for c in range(C):
            x = set(ex(c)); ok += int(np.argmax([len(x & l) for l in learned]) == c)
    return ok / 400
if 'b' in sys.argv[1:]:
    print("\n(b) fixed classes (pairs share 75%, keep 0.5, 64 noise bits); accuracy by examples per class")
    NN = (4, 16, 64, 256, 1024)
    for mode in ('up', 'updown'):
        for S in (2, 3, 4, 8, 16, 64, 256, 2**31):
            print(f"{mode:>7} {lab[S]:>5}: " + "  ".join(f"N={N}:{classes(mode, S, N):5.0%}" for N in NN), flush=True)
