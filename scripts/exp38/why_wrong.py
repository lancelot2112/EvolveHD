"""Why far found answers are wrong: classify each wrong top-1 answer to "what did X find in chapter M [of part N]
?" by which of X's earlier finds it equals (same chapter of another part in this book or in an earlier one, the
same address in an earlier book, X's newest find, another find), or none. Books are cut where "part one ." (or the
heading word given) starts again. usage: why_wrong.py DOC RUN [min_dist]"""
import sys, json, collections
import numpy as np
doc, run = sys.argv[1], sys.argv[2]; mind = int(sys.argv[3]) if len(sys.argv) > 3 else 65536
ids = np.fromfile(doc + ".ids", dtype=np.uint32); V = open(doc + ".txt").read().split("\n"); w = [V[i] for i in ids]
info = json.load(open(doc + ".info.json")); top = np.fromfile(run + ".top", dtype=np.uint32).reshape(len(ids), -1)
heads_words = {"part", "volume"}; ch_words = {"chapter", "section"}
finds = collections.defaultdict(list); book = 0; part = ch = None
qs = {p: d for (p, k), d in zip(info["answers"], info["dist"]) if k == "found" and d >= mind}
cat = collections.Counter()
for i, t in enumerate(w):
    if i in qs:
        j = i - 1
        while w[j] != "did": j -= 1
        X = w[j + 1]; q = w[j + 1:i]
        k = next(n for n, u in enumerate(q) if u in ch_words)
        rest = " ".join(q[k + 1:-1]).split(" of ")
        M = rest[0]; N = rest[1].split(" ", 1)[1] if len(rest) > 1 else None
        pred = V[top[i, 0]] if top[i, 0] < len(V) else None
        if pred == w[i]: cat["right"] += 1; continue
        if pred is None: cat["no answer"] += 1; continue
        past = finds[X]; m = [f for f in past if f[4] == pred]
        if not m: cat["not one of X's finds"] += 1; continue
        tags = set()
        for (b, p, c, pos, o) in m:
            if c == M and p == N: tags.add("same address, earlier book" if b < book else "same address (?)")
            elif c == M: tags.add("same chapter, other part" + (", this book" if b == book else ", earlier book"))
        if past and past[-1][4] == pred: tags.add("X's newest find")
        cat[sorted(tags)[0] if tags else "another of X's finds"] += 1
    if (i == 0 or w[i - 1] == ".") and t in heads_words:
        j = i + 1; lab = []
        while w[j] != ".": lab.append(w[j]); j += 1
        if " ".join(lab) == "one" and part is not None: book += 1
        part = " ".join(lab)
    if (i == 0 or w[i - 1] == ".") and t in ch_words:
        j = i + 1; lab = []
        while w[j] != ".": lab.append(w[j]); j += 1
        ch = " ".join(lab)
    if t == "found" and i + 2 < len(w) and w[i + 1] == "the": finds[w[i - 1]].append((book, part, ch, i, w[i + 2]))
tot = sum(cat.values()); print(f"{run.split('/')[-1]}: far found (dist >= {mind}) {tot}")
for k, v in cat.most_common(): print(f"  {k:<40} {v:5d}  {v / tot:6.1%}")
