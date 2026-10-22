"""E5: a sense as an address or as a gate. 10 words, each with 2 senses; a sense's code is 64 bits in 2,048, the
two senses of a word share half (32 bits: the word's core). An example of word w in sense s keeps each of the
sense's bits with probability 0.5 and adds 64 noise bits; the sense is known from the context at learning time.
All memories are 4-bit up/down counters per bit (local). Learned code of (w, s) = its top 64 bits under:
  plain    one counter set per word, senses mixed
  address  one counter set per (word, sense), each sees only its own sense's examples (evidence split)
  gate     one shared set per word (all examples) plus per-sense sets; the sense's code is the shared top bits
           masked to those where the sense's counter is at least the other sense's (a read-time gate)
  shrink   the top 64 of (shared + 2 x sense) counters: the sense borrows the word's evidence
Scored on 400 new examples (w, s): nearest learned code among all 20 is the right (w, s) (sense accuracy), and the
fraction of the true sense bits the learned code holds (bit recall)."""
import numpy as np
D, W, k = 2048, 10, 64
def run(mode, N, seed=1, keep=0.5, noise=64):
    r = np.random.default_rng(seed)
    P = np.zeros((W, 2, k), int)
    for w in range(W):
        core = r.choice(D, 32, replace=False); rest = r.choice(np.setdiff1d(np.arange(D), core), 64, replace=False)
        P[w, 0] = np.concatenate([core, rest[:32]]); P[w, 1] = np.concatenate([core, rest[32:]])
    def ex(w, s):
        x = np.zeros(D, bool); x[P[w, s][r.random(k) < keep]] = True; x[r.integers(0, D, noise)] = True; return x
    sh = np.zeros((W, D), np.int64); se = np.zeros((W, 2, D), np.int64)
    def ud(m, x): return np.clip(m + np.where(x, 1, -1), 0, 15)
    for i in range(N):                       # N examples per word, senses alternate at random
        for w in range(W):
            s = int(r.integers(0, 2)); x = ex(w, s); sh[w] = ud(sh[w], x); se[w, s] = ud(se[w, s], x)
    codes = np.zeros((W, 2, D), bool)
    for w in range(W):
        for s in range(2):
            if mode == 'plain': v = sh[w]
            elif mode == 'address': v = se[w, s]
            elif mode == 'gate': v = np.where(se[w, s] >= se[w, 1 - s], sh[w], -1)
            else: v = sh[w] + 2 * se[w, s]
            codes[w, s, np.argsort(-(v + r.random(D) * 0.01))[:k]] = True
    flat = codes.reshape(W * 2, D).astype(int); ok = 0; rec = 0
    for _ in range(400):
        w, s = int(r.integers(0, W)), int(r.integers(0, 2)); x = ex(w, s)
        ok += int(np.argmax(flat @ x)) == w * 2 + s
        rec += codes[w, s, P[w, s]].mean()
    return ok / 400, rec / 400
print("sense accuracy / bit recall, by examples per word (each word's two senses share half their bits)")
for mode in ('plain', 'address', 'gate', 'shrink'):
    print(f"{mode:>8}: " + "   ".join("N=%d %3.0f%% %3.0f%%" % ((N,) + tuple(100 * v for v in run(mode, N))) for N in (4, 8, 16, 32, 64, 256)), flush=True)
