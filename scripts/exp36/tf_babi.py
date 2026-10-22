"""A transformer baseline on the comprehension stories (experiment 36): a small causal transformer language model
over the same word stream, scored like the situation model.

Score: the top prediction at the answer positions of the second half (the word after "?"), by question kind. The
second half uses verbs and a question form never seen in the first.

Two protocols:
  online   one pass, learning while reading, as the situation model does. The stream is cut into chunks of 64
           words. Each chunk's answers are predicted first, from a context window of the 128 words before each
           answer; then one Adam step is taken on the chunk's next-word loss (each position sees 128 words back).
  offline  train on the first half only, for many epochs (batches of random 128-word windows), then test on the
           second half; accuracy and wall time are reported after each epoch.
Model: token and learned position embeddings, `--layers` pre-norm blocks of `--heads`-head causal attention and a
4x MLP, width `--d`.
usage: tf_babi.py IDS INFO.json [--mode online|offline] [--d 128] [--layers 2] [--heads 4] [--epochs 20]"""
import sys, json, time, argparse
import numpy as np, torch, torch.nn as nn, torch.nn.functional as F
ap = argparse.ArgumentParser(); ap.add_argument('ids'); ap.add_argument('info')
ap.add_argument('--mode', default='online'); ap.add_argument('--d', type=int, default=128)
ap.add_argument('--layers', type=int, default=2); ap.add_argument('--heads', type=int, default=4)
ap.add_argument('--ctx', type=int, default=128); ap.add_argument('--chunk', type=int, default=64)
ap.add_argument('--epochs', type=int, default=20); ap.add_argument('--steps', type=int, default=400)
ap.add_argument('--lr', type=float, default=3e-3); ap.add_argument('--seed', type=int, default=0)
ap.add_argument('--threads', type=int, default=2)
a = ap.parse_args()
torch.manual_seed(a.seed); np.random.seed(a.seed); torch.set_num_threads(a.threads)
ids = np.fromfile(a.ids, dtype=np.uint32).astype(np.int64); n = len(ids)
info = json.load(open(a.info)); V = int(ids.max()) + 1; L = a.ctx
ans = [(p, k) for p, k in info['answers'] if n // 2 <= p < n]
class Block(nn.Module):
    def __init__(s, d, h):
        super().__init__(); s.n1 = nn.LayerNorm(d); s.att = nn.MultiheadAttention(d, h, batch_first=True)
        s.n2 = nn.LayerNorm(d); s.mlp = nn.Sequential(nn.Linear(d, 4 * d), nn.GELU(), nn.Linear(4 * d, d))
    def forward(s, x, mask):
        h = s.n1(x); x = x + s.att(h, h, h, attn_mask=mask, need_weights=False)[0]
        return x + s.mlp(s.n2(x))
class LM(nn.Module):
    def __init__(s):
        super().__init__(); s.tok = nn.Embedding(V, a.d); s.pos = nn.Embedding(L, a.d)
        s.blocks = nn.ModuleList([Block(a.d, a.heads) for _ in range(a.layers)]); s.nf = nn.LayerNorm(a.d)
        s.out = nn.Linear(a.d, V)
    def forward(s, x):
        T = x.shape[1]; mask = torch.triu(torch.ones(T, T, dtype=torch.bool), 1)
        h = s.tok(x) + s.pos(torch.arange(T))
        for b in s.blocks: h = b(h, mask)
        return s.out(s.nf(h))
model = LM(); opt = torch.optim.Adam(model.parameters(), lr=a.lr)
nparam = sum(p.numel() for p in model.parameters())
def predict(ps):
    """the top prediction at each position p, from the L words before it"""
    model.eval(); out = []
    with torch.no_grad():
        for i in range(0, len(ps), 256):
            batch = ps[i:i + 256]
            x = torch.tensor(np.stack([np.pad(ids[max(0, p - L):p], (L - min(L, p), 0)) for p in batch]))
            out += model(x)[:, -1].argmax(-1).tolist()
    return out
def score(ps_k):
    ps = [p for p, _ in ps_k]; pr = predict(ps); res = {}
    for (p, k), y in zip(ps_k, pr): res.setdefault(k, []).append(y == ids[p])
    allv = [v for vs in res.values() for v in vs]
    return {k: float(np.mean(v)) for k, v in res.items()}, float(np.mean(allv))
def fmt(r, tot): return "  ".join(f"{k} {r.get(k, 0):5.1%}" for k in ('where-person', 'where-object', 'who-has')) + f"  all {tot:5.1%}"
print(f"{n} words, vocab {V}, {len(ans)} questions scored; transformer {a.layers} layers x {a.heads} heads, d {a.d}, "
      f"context {L}: {nparam:,} parameters ({nparam * 4 / 1024:.0f} KB at 32 bits); mode {a.mode}", flush=True)
t0 = time.time()
if a.mode == 'online':
    ansd = dict(ans); results = []; marks = [len(ans) // 4, len(ans) // 2, 3 * len(ans) // 4]
    for c0 in range(0, n, a.chunk):
        c1 = min(n, c0 + a.chunk)
        qs = [(p, ansd[p]) for p in range(c0, c1) if p in ansd]
        if qs:
            pr = predict([p for p, _ in qs]); results += [(k, y == ids[p]) for (p, k), y in zip(qs, pr)]
        # one step on the chunk: every position predicts its next word from up to L words back
        model.train(); s0 = max(1, c0)
        xs = [np.pad(ids[max(0, p - L):p], (L - min(L, p), 0)) for p in range(s0, c1)]
        x = torch.tensor(np.stack(xs)); y = torch.tensor(ids[s0:c1])
        loss = F.cross_entropy(model(x)[:, -1], y); opt.zero_grad(); loss.backward(); opt.step()
        if len(results) in marks or c1 == n:
            r = {}
            for k, v in results: r.setdefault(k, []).append(v)
            tot = np.mean([v for _, v in results])
            print(f"  {c1:>7} words read, {time.time() - t0:7.1f} s: " + fmt({k: np.mean(v) for k, v in r.items()}, tot), flush=True)
    # accuracy over the last quarter of the scored questions (after most of the learning)
    q = results[3 * len(results) // 4:]; r = {}
    for k, v in q: r.setdefault(k, []).append(v)
    print(f"  last quarter of the questions: " + fmt({k: np.mean(v) for k, v in r.items()}, np.mean([v for _, v in q])))
else:
    half = n // 2; starts = np.arange(L, half)
    for ep in range(a.epochs):
        model.train()
        for st in range(a.steps):
            b = np.random.choice(starts, 32)
            x = torch.tensor(np.stack([ids[s - L:s] for s in b])); y = torch.tensor(np.stack([ids[s - L + 1:s + 1] for s in b]))
            loss = F.cross_entropy(model(x).reshape(-1, V), y.reshape(-1)); opt.zero_grad(); loss.backward(); opt.step()
        r, tot = score(ans)
        print(f"  epoch {ep + 1:>3} ({(ep + 1) * a.steps * 32 * L / 1e6:6.1f} M training positions, {time.time() - t0:7.1f} s): loss {loss.item():.3f}  " + fmt(r, tot), flush=True)
t1 = time.time(); predict([p for p, _ in ans[:256]])
print(f"  answer latency: {(time.time() - t1) / 256 * 1e3:.2f} ms per question (batched 256, CPU)")
