"""E6a: order from the state's own dynamics vs from edges, same bits (a ternary vector of D elements).
Stream of items (5,000-word vocabulary, K = 64 active (element, phase) pairs each).
  state  every step the whole vector shifts by one element (S <- roll(S, 1)), then the new item is added at its
         elements (clipped to -1..1). The item seen n steps ago sits at its elements + n. No pairs are bound.
  edge   every step binds the previous item (key) to the new one (value) in a ternary trace (heteroassociation).
Queries about an item X seen n steps ago (its most recent occurrence):
  after    what came right after X?  state: find X's lag by scanning lags 0..W, then read lag - 1.
           edge: read key X.
  after2   what came two after X?    state: lag - 2.  edge: two chained reads.
  recency  which of X and Y (seen n and n + 5 steps ago) is more recent?  state: compare lags.  edge: no way.
Clean-up against the whole vocabulary. Accuracy by lag n."""
import numpy as np
from trace import UNIT, DRE, DIM
D, K, NV, W = 16384, 64, 5000, 1024
r = np.random.default_rng(5)
KI = r.integers(0, D, (NV, K)); KP = r.integers(0, 4, (NV, K)); VPH = r.integers(0, 4, (NV, D)).astype(np.int8)
T = 6000; stream = r.integers(0, NV, T)
# --- state with shift dynamics ---
sre = np.zeros(D, np.int16); sim = np.zeros(D, np.int16)
# --- edge trace ---
ere = np.zeros(D, np.int16); eim = np.zeros(D, np.int16)
def add(re, im, idx, p):
    re[idx] = np.clip(re[idx] + DRE[p], -1, 1); im[idx] = np.clip(im[idx] + DIM[p], -1, 1)
for t in range(T):
    sre = np.roll(sre, 1); sim = np.roll(sim, 1)
    x = stream[t]; add(sre, sim, KI[x], KP[x])
    if t: k = stream[t - 1]; add(ere, eim, KI[k], (KP[k] + VPH[x, KI[k]]) % 4)
def s_score(lag, items):
    """state: how strongly each item sits at this lag"""
    idx = (KI[items] + lag) % D; z = sre[idx] + 1j * sim[idx]
    return np.real((z * np.conj(UNIT[KP[items]])).sum(1))
def s_read(lag):
    return int(np.argmax(s_score(lag, np.arange(NV))))
def s_lag(x):
    lags = np.arange(W); idx = (KI[x][None, :] + lags[:, None]) % D
    z = sre[idx] + 1j * sim[idx]; return int(np.argmax(np.real((z * np.conj(UNIT[KP[x]])[None, :]).sum(1))))
def e_read(x):
    z = (ere[KI[x]] + 1j * eim[KI[x]]) * UNIT[(-KP[x]) % 4]
    return int(np.argmax(np.real(np.conj(UNIT[VPH[:, KI[x]]]) @ z)))
last = {}
for t, x in enumerate(stream): last[x] = t
print(f"D={D} ternary, K={K}, {T} steps; accuracy by lag n (up to 40 queries per lag, distinct positions)")
print(f"{'lag':>5} {'state after':>12} {'edge after':>11} {'state after2':>13} {'edge after2':>12} {'state recency':>14} {'state lag found':>16}")
for n in (2, 8, 32, 128, 256, 384, 512, 768, 1000, 2000):
    res = np.zeros(6); m = 0
    for q in range(40):
        t = T - 1 - n - q
        x = stream[t]
        if last[x] != t or t + 2 >= T: continue
        lag = T - 1 - t; m += 1
        L = s_lag(x); res[5] += L == lag
        res[0] += s_read(L - 1) == stream[t + 1]; res[1] += e_read(x) == stream[t + 1]
        res[2] += s_read(L - 2) == stream[t + 2]
        res[3] += e_read(e_read(x)) == stream[t + 2]
        y = stream[t - 5]
        if last[y] == t - 5: res[4] += s_lag(x) < s_lag(y)
        else: res[4] += 1
    res /= max(m, 1); res = list(res) + [m]
    print(f"{n:>5} " + " ".join(f"{v:>12.0%}" for v in (res[0], res[1], res[2], res[3], res[4], res[5])), flush=True)
