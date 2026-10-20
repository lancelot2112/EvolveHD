"""Score the readouts written by `evolvehd meaning` (or any per-tick probability files), as meaning_codes.py does:
exact-word bits beyond the experiment-25 counts system (cortex_base.py), as mixtures whose weights are chosen on
the first fifth of the second half and reported on the rest; with the gains on rare words (seen fewer than 10 times
before) and on content words new to the story (not in the previous 2,000 words).
usage: score_meaning.py IDS BASE.npz PREFIX [names...]   (names default: q_bits q_mean, plus q_wm if present)"""
import sys, os, itertools
import numpy as np
ids = np.fromfile(sys.argv[1], dtype=np.uint32).astype(np.int64); n = len(ids)
B = np.load(sys.argv[2]); p_base, q_story = B['p_base'], B['q_story']
pre = sys.argv[3]
names = sys.argv[4:] or [x for x in ('q_bits', 'q_mean', 'q_wm') if os.path.exists(f"{pre}.{x}")]
Q = {x: np.fromfile(f"{pre}.{x}", dtype='<f8') for x in names}
SKIP = 100; h = n // 2; cal = h + (n - h) // 5; sc = slice(cal, n); ca = slice(h, cal)
LAMS = [0.0, 0.02, 0.05, 0.1, 0.15, 0.2, 0.3, 0.45]
def fit(qs):
    best = None
    for ls in itertools.product(LAMS, repeat=len(qs)):
        if sum(ls) >= 0.95: continue
        p = (1 - sum(ls)) * p_base[ca] + sum(l * q[ca] for l, q in zip(ls, qs))
        v = np.log2(p).mean()
        if best is None or v > best[0]: best = (v, ls)
    ls = best[1]
    return -np.log2((1 - sum(ls)) * p_base + sum(l * q for l, q in zip(ls, qs))), ls
base_bits = -np.log2(p_base)
fc = {}; prior = np.zeros(n, dtype=np.int64); last = {}; storynew = np.zeros(n, dtype=bool)
for i, w in enumerate(ids.tolist()):
    prior[i] = fc.get(w, 0); fc[w] = prior[i] + 1
    storynew[i] = w >= SKIP and (w not in last or i - last[w] > 2000); last[w] = i
rare = prior < 10
print(f"{pre}: base {base_bits[sc].mean():.4f} bits per exact word (scored part)")
combos = [('story', ['story'])] + [(x, [x]) for x in names] + [('story+' + x, ['story', x]) for x in names]
if len(names) > 1: combos.append(('story+' + '+'.join(names), ['story'] + names))
QQ = dict(Q, story=q_story)
for name, parts in combos:
    bb, ls = fit([QQ[p] for p in parts]); d = base_bits - bb
    m = lambda mask: d[sc][mask[sc]].mean()
    print(f"  {name:<28} gain {d[sc].mean():+.4f}   on rare words {m(rare):+.4f}   on words new to the story {m(storynew):+.4f}   weights {ls}", flush=True)
