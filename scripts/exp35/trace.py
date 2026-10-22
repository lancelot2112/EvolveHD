"""Shared: a sparse 4-phase trace with L levels per part (re, im). Integer only. Keys are K (element, phase) pairs."""
import numpy as np
UNIT = np.array([1, 1j, -1, -1j]); DRE = np.array([1, 0, -1, 0]); DIM = np.array([0, 1, 0, -1])
class Trace:
    def __init__(s, D, L, NV=1000, seed=0):
        r = np.random.default_rng(seed)
        s.D, s.L = D, L
        s.VPH = r.integers(0, 4, (NV, D)).astype(np.int8)
        if L == 2:   # 1 bit per part: the sign of the last write
            s.re = r.choice([-1, 1], D).astype(np.int16); s.im = r.choice([-1, 1], D).astype(np.int16)
        else:
            s.re = np.zeros(D, np.int16); s.im = np.zeros(D, np.int16)
        s.lo = -(L // 2); s.hi = L // 2 if L % 2 else L // 2 - 1
    def write(s, idx, kp, v, m=1):
        p = (kp + s.VPH[v, idx]) % 4; dre = DRE[p]; dim = DIM[p]
        if s.L == 2:
            s.re[idx] = np.where(dre != 0, dre, s.re[idx]); s.im[idx] = np.where(dim != 0, dim, s.im[idx])
        else:
            s.re[idx] = np.clip(s.re[idx] + m * dre, s.lo, s.hi); s.im[idx] = np.clip(s.im[idx] + m * dim, s.lo, s.hi)
    def scores(s, idx, kp, cands):
        """idx, kp: (n, K); cands: (n, C) value ids -> (n, C) scores"""
        z = (s.re[idx] + 1j * s.im[idx]) * UNIT[(-kp) % 4]                # n x K
        ph = s.VPH[cands[:, :, None], idx[:, None, :]]                     # n x C x K
        return np.real((z[:, None, :] * np.conj(UNIT[ph])).sum(2))
def depth(acc_by_age, ages, thr=0.8):
    """largest age before accuracy first drops below thr (linear interpolation)"""
    acc_by_age = np.convolve(np.pad(acc_by_age, 1, mode='edge'), np.ones(3) / 3, 'valid')   # smooth over 3 ages
    for i, (a, x) in enumerate(zip(ages, acc_by_age)):
        if x < thr:
            if i == 0: return 0
            a0, x0 = ages[i - 1], acc_by_age[i - 1]
            return int(a0 + (a - a0) * (x0 - thr) / (x0 - x + 1e-9))
    return int(ages[-1])

def best_of_all(tr, idx, kp, true):
    """is the true value the best of every value (full clean-up)? idx, kp: (n, K)"""
    z = (tr.re[idx] + 1j * tr.im[idx]) * UNIT[(-kp) % 4]
    out = []
    for j in range(len(true)):
        sc = np.real(np.conj(UNIT[tr.VPH[:, idx[j]]]) @ z[j])
        out.append(int(np.argmax(sc)) == true[j])
    return np.array(out)
