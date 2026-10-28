"""A description expert for the formats stream, learned from nothing and mixed with the byte brain's code.

It reads the description just before a file (the bytes since the last file ended) as tokens (runs of non-space
bytes, trailing "." "," ":" dropped) and, while the file is read, keeps a belief over which token the current byte
"comes from" (an alignment, as in word alignment for translation, IBM model / HMM style):
  - start: a learned distribution over token positions for the file's first byte;
  - move: from one byte to the next the alignment jumps by d tokens (-16..+8), with a learned distribution of d;
  - emit: each token w has a learned byte distribution t(b | w);
  - not from the description: a byte may instead come from a background distribution (learned, over all file
    bytes), with a learned share per file offset bucket. This keeps the tokens from soaking up bytes they don't
    explain (names, values, counts).
Before each byte it predicts p(b) = l bg(b) + (1 - l) sum_i belief(i) t(b | w_i); after it, the share of the
description in explaining the byte (r) weighs the updates: tokens and alignment learn from r times the posterior,
the background from 1 - r, and the belief moves to the posterior by r (online EM). Everything is prequential: the prediction for
a byte uses only what came before.

What it is given beyond the bytes: where each file starts (the position sense: a reader of files knows when a file
is opened) and the tokenization rule above. With no description (condition N) it says nothing (uniform).

The code: a switching mixture (fixed share 1/1000) over the brain's per-byte code (level0.bits), this expert and a
uniform guess, one mixture per context: the file offset (0..7, 8+) by the expert's confidence (its top byte's
probability in quarters).
usage: desc_expert.py STREAM BRAIN_BITS OUT_BITS"""
import sys, json, re
import numpy as np
s, bb, outp = sys.argv[1], sys.argv[2], sys.argv[3]
data = open(s + ".bin", "rb").read(); meta = json.load(open(s + ".items.json"))["items"]
brain = np.fromfile(bb, dtype="<f4").astype(float)
n = len(data)
JMIN, JMAX = -16, 8
J = np.ones(JMAX - JMIN + 1); J[-JMIN + 1] += 4.0  # a mild prior for reading on by one token
S0 = np.ones(64)
T = {}
def tvec(w):
    v = T.get(w)
    if v is None:
        v = np.full(256, 0.05); T[w] = v
    return v
desc_bits = np.full(n, 8.0); conf = np.zeros(n)
BG = np.ones(256)
LAM = {}  # per offset bucket: [counts from description, counts from background]
for it in meta:
    d = data[it["desc_start"]:it["desc_start"] + it["desc_len"]].decode("latin1")
    toks = [re.sub(r"[.,:]+$", "", x) for x in d.split()]
    toks = [x for x in toks if x][:64]
    a, l = it["start"], it["len"]
    if not toks:
        continue
    m = len(toks)
    E = np.stack([tvec(w) for w in toks])  # m x 256 (views into counts: refreshed below)
    belief = S0[:m] / S0[:m].sum()
    for k in range(l):
        if k > 0:
            # move: belief convolved with the jump distribution (truncated at the ends)
            jn = J / J.sum()
            nb = np.zeros(m)
            for di, pj in enumerate(jn):
                dd = di + JMIN
                if dd >= 0:
                    nb[dd:] += pj * belief[:m - dd] if dd < m else 0
                else:
                    nb[:m + dd] += pj * belief[-dd:]
            belief = nb / nb.sum() if nb.sum() > 0 else np.full(m, 1.0 / m)
        y = data[a + k]
        P = E / E.sum(1, keepdims=True)
        pd = belief @ P
        lc = LAM.setdefault(min(k, 8), [1.0, 1.0]); lam = lc[1] / (lc[0] + lc[1])
        bg = BG / BG.sum()
        pb = (1 - lam) * pd + lam * bg
        desc_bits[a + k] = -np.log2(pb[y]); conf[a + k] = ((1 - lam) * pd).max()
        r = (1 - lam) * pd[y] / pb[y]
        lc[0] += r; lc[1] += 1 - r; BG[y] += 1 - r
        post = belief * P[:, y]; post /= post.sum()
        # learn: emissions, start, jumps (expected jump from the previous belief to this posterior, approximated)
        for i in np.nonzero(post * r > 1e-3)[0]:
            T[toks[i]][y] += post[i] * r
        E = np.stack([T[w] for w in toks])
        if k == 0:
            S0[:m] += post * r
        elif r > 0.5:
            ei = int(np.argmax(prev_post)); ej = int(np.argmax(post))
            dd = ej - ei
            if JMIN <= dd <= JMAX:
                J[dd - JMIN] += r
        post = r * post + (1 - r) * belief
        prev_post = post
        belief = post
# the mixture
A = 3; SH = 1e-3
W = {}
mix = brain.copy()
fileoff = np.full(n, -1)
for it in meta:
    fileoff[it["start"]:it["start"] + it["len"]] = np.arange(it["len"])
for t in range(n):
    if fileoff[t] < 0:
        continue
    ctx = (min(fileoff[t], 8), min(int(conf[t] * 4), 3))
    w = W.get(ctx)
    if w is None:
        w = np.full(A, 1.0 / A)
    pk = np.array([2.0 ** -brain[t], 2.0 ** -desc_bits[t], 1.0 / 256])
    p = (w * pk).sum(); mix[t] = -np.log2(p)
    x = w * pk / p; W[ctx] = (1 - SH) * x + SH * (1 - x) / (A - 1)
mix.astype("<f4").tofile(outp)
desc_bits.astype("<f4").tofile(outp + ".expert")
big = sorted(T, key=lambda w: -T[w].sum())
learned = [(w, int(np.argmax(T[w])), round(float(T[w].max() / T[w].sum()), 2)) for w in big if re.fullmatch(r"[0-9a-f]{2}", w)][:12]
hexs = [w for w in T if re.fullmatch(r"[0-9a-f]{2}", w) and T[w].sum() > 2]
okh = sum(int(np.argmax(T[w])) == int(w, 16) for w in hexs)
print(f"hex tokens whose likeliest byte is their own value: {okh} of {len(hexs)}; background share by offset: " + " ".join(f"{k}:{v[1] / sum(v):.2f}" for k, v in sorted(LAM.items())), file=sys.stderr)
print("tokens learned (hex-looking, commonest): " + " ".join(f"{w}->{b:02x}({p})" for w, b, p in learned), file=sys.stderr)
print("jumps: " + " ".join(f"{d}:{c / J.sum():.2f}" for d, c in zip(range(JMIN, JMAX + 1), J) if c / J.sum() > 0.02), file=sys.stderr)
