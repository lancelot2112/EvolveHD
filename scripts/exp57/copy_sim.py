"""Offline copy act (the same rule as --copy-act: 5-byte context, checked back to 32, read on while right) mixed
with a brain run's per-byte code (level0.bits), to study the mixing. Prints per-file bits for: the brain alone, the
act mixed per length bucket (as in the brain), and variants. usage: copy_sim.py BRAIN_TAG"""
import sys, json
import numpy as np
s = 'runs/exp57/bytes'; data = np.frombuffer(open(s + '.bin', 'rb').read(), dtype=np.uint8); meta = json.load(open(s + '.files.json'))['files']
br = np.fromfile(f'runs/exp57/brain_{sys.argv[1]}/level0.bits', dtype='<f4').astype(float)
n = len(data); K = 5
tab = {}; ptr = None; ln = 0
offer = np.full(n, -1); olen = np.zeros(n, int)
for t in range(n):
    if ptr is not None and ptr < t:
        offer[t] = data[ptr]; olen[t] = ln
        if data[ptr] == data[t]:
            ptr += 1; ln += 1
        else:
            ptr = None; ln = 0
    if t + 1 >= K:
        h = bytes(data[t + 1 - K:t + 1])
        if ptr is None and h in tab:
            q = tab[h]; l = 0
            while l < 32 and l < q and data[q - 1 - l] == data[t - l]:
                l += 1
            if l >= K:
                ptr = q; ln = l
        tab[h] = t + 1
hit = offer == data
print(f"offers {np.mean(offer >= 0):.1%} of bytes, right {hit[offer >= 0].mean():.1%}")
def run(ctx_fn, nb, sh=1e-3):
    cal = np.zeros((nb, 2)); w = np.full((nb, 3), 1 / 3); out = np.zeros(n)
    for t in range(n):
        b = ctx_fn(t)
        pl = 2.0 ** -br[t]
        if offer[t] >= 0:
            c = (cal[b, 0] + 0.5) / (cal[b, 1] + 1)
            pc = c if hit[t] else (1 - c) / 255
        else:
            pc = 1 / 256
        pk = np.array([pl, pc, 1 / 256]); p = (w[b] * pk).sum(); out[t] = -np.log2(p)
        x = w[b] * pk / p; w[b] = (1 - sh) * x + sh * (1 - x) / 2
        if offer[t] >= 0:
            cal[b, 1] += 1; cal[b, 0] += hit[t]
    return out
base = run(lambda t: 0 if offer[t] < 0 else 1 + min(olen[t], 32) // 2, 18)
# context also by the brain's own cost on the byte before (is the brain doing well here?)
prev = np.concatenate([[8.0], br[:-1]])
bb = lambda t: min(int(prev[t]), 7)
both = run(lambda t: (0 if offer[t] < 0 else 1 + min(olen[t], 32) // 2) * 8 + bb(t), 18 * 8)
fast = run(lambda t: 0 if offer[t] < 0 else 1 + min(olen[t], 32) // 2, 18, sh=1e-2)
print(f"{'':<3}{'type':<6}{'brain':>8}{'act/len':>8}{'+brain':>8}{'share.01':>9}  offers right")
for m in meta:
    a, l = m['start'], m['len']; o = offer[a:a + l] >= 0
    print(f"{m['round']:<3}{m['type']:<6}{br[a:a+l].mean():8.3f}{base[a:a+l].mean():8.3f}{both[a:a+l].mean():8.3f}{fast[a:a+l].mean():9.3f}  {o.mean():5.1%} {hit[a:a+l][o].mean():5.1%}")
print(f"{'all':<9}{br.mean():8.3f}{base.mean():8.3f}{both.mean():8.3f}{fast.mean():9.3f}")
