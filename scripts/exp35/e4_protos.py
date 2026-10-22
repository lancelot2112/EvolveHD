"""E4: online pooling with bit-local prototype memories. 16 latent classes of 64 bits in 2,048, in pairs sharing half
their bits; items keep each class bit with probability 0.6 and add 32 noise bits. Each item goes to the prototype it
overlaps most if the overlap (shared bits / item bits) is at least 0.3, else it starts a new prototype; the chosen
prototype's per-bit memory is updated with the item only (local). Prototype memories:
  or       a bit once set stays set; read = the set bits
  or+re    the same, rebuilt every 2,000 items from the last 2,000 (the situation model's reclustering)
  up4      4-bit up-only counters; read = top 64 bits
  ud4      4-bit up/down counters (down on the prototype's bits absent from its item); read = top 64
  full     32-bit counts; read = top 64
  full-w   32-bit counts; read = the counts themselves (overlap weighted by count / max), no binarizing
Scored at checkpoints on 400 new items: each prototype is labelled with its members' majority class; accuracy =
the new item's nearest prototype carries its class. Also: prototypes made, and purity of the assignments."""
import numpy as np, sys
D, C, k = 2048, 16, 64
def world(seed):
    r = np.random.default_rng(seed); P = []
    for c in range(0, C, 2):
        a = r.choice(D, k, replace=False); b = a.copy(); m = k // 2
        b[:m] = r.choice(np.setdiff1d(np.arange(D), a), m, replace=False); P += [a, b]
    return r, np.array(P)
def item(r, P, c):
    x = np.zeros(D, bool); x[P[c][r.random(k) < 0.6]] = True; x[r.integers(0, D, 32)] = True; return x
def run(mode, N, seed=1, thr=0.3):
    r, P = world(seed)
    mem = []; lab = []; recent = []
    def read(m):
        if mode in ('or', 'or+re'): return m.astype(float)
        if mode == 'full-w': return m / max(1, m.max())
        v = np.zeros(D); v[np.argsort(-(m + r.random(D) * 0.01))[:k]] = 1; v[m == 0] = 0; return v
    reads = []
    def assign(x, learn=True):
        if reads:
            R = np.array(reads); ov = (R @ x) / x.sum()
            if mode in ('or', 'or+re'): ov = ov - 0.25 * R.sum(1) / D * 0     # plain coverage of the item
            j = int(np.argmax(ov))
            if ov[j] >= thr or not learn: return j
        if not learn: return -1
        mem.append(np.zeros(D, np.int64)); reads.append(np.zeros(D)); lab.append([]); return len(mem) - 1
    def update(j, x):
        m = mem[j]
        if mode in ('or', 'or+re'): m |= x
        elif mode == 'up4': np.clip(m + x, 0, 15, out=m)
        elif mode == 'ud4':
            m += x; m[(~x) & (reads[j] > 0)] -= 1; np.clip(m, 0, 15, out=m)
        else: m += x
        reads[j] = read(m)
    out = {}
    for t in range(N):
        c = int(r.integers(0, C)); x = item(r, P, c); j = assign(x); update(j, x); lab[j].append(c)
        recent.append((x, c))
        if mode == 'or+re' and t % 2000 == 1999:
            mem.clear(); reads.clear(); lab.clear()
            for xx, cc in recent[-2000:]:
                jj = assign(xx); update(jj, xx); lab[jj].append(cc)
    maj = [np.bincount(l, minlength=C).argmax() if l else -1 for l in lab]
    pur = sum(np.bincount(l, minlength=C).max() for l in lab if l) / sum(len(l) for l in lab)
    ok = 0
    for _ in range(400):
        c = int(r.integers(0, C)); j = assign(item(r, P, c), learn=False); ok += j >= 0 and maj[j] == c
    return ok / 400, len(mem), pur
NN = (250, 1000, 4000, 16000)
print("accuracy / prototypes made / purity, by items seen (16 latent classes)")
for mode in ('or', 'or+re', 'up4', 'ud4', 'full', 'full-w'):
    print(f"{mode:>7}: " + "   ".join("N=%d %3.0f%% %3d %3.0f%%" % ((N,) + tuple(v * 100 if i != 1 else v for i, v in enumerate(run(mode, N)))) for N in NN), flush=True)
