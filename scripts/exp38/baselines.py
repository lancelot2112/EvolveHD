"""Baselines for long-range questions (experiment 38), scored like score_dist.py (top-1 by kind and fact distance).
  prior      the question alone: per (kind, entity) the answer given most often to that question earlier in the
             stream (the question-only prior), else the kind's most frequent answer
  newest     the newest value: X's latest find / X's latest home / the latest owner of Z (recency, no structure)
  bm25       retrieval without learning: BM25 over 128-word chunks of the text before the question, query = the
             question's words minus "what did in of the where does who ?"; from the top chunk, the word of the
             answer's kind nearest to a mention of the entity (within 8 words); none -> wrong
  oracle     the structure read by rule: the latest heading "part N", then "chapter M" inside it, then "X found the
             Z" inside that chapter (found); the latest statement (home, owner). The ceiling.
usage: baselines.py DOC [max_questions_per_kind]"""
import sys, json, math, collections
import numpy as np
doc = sys.argv[1]; cap = int(sys.argv[2]) if len(sys.argv) > 2 else 10 ** 9
ids = np.fromfile(doc + ".ids", dtype=np.uint32); V = open(doc + ".txt").read().split("\n"); n = len(ids)
w = [V[i] for i in ids]
info = json.load(open(doc + ".info.json"))
names, places, objects = set(info["names"]), set(info["places"]), set(info["objects"])
kind_set = {"found": objects, "home": places, "owner": names}
H_CH, H_PT = info.get("headings", ["chapter", "part"])
groups = [(8, 128, "8-127"), (128, 1024, "128-1k"), (1024, 8192, "1k-8k"), (8192, 65536, "8k-64k"), (65536, 524288, "64k-512k"), (524288, 1 << 62, "512k+")]
qs = [(p, k, d) for (p, k), d in zip(info["answers"], info["dist"]) if k in kind_set and d > 0 and p >= n // 10]
by = collections.defaultdict(list)
for q in qs: by[q[1]].append(q)
rng = np.random.default_rng(0)
qs = sorted(sum([list(v) if len(v) <= cap else [v[i] for i in sorted(rng.choice(len(v), cap, replace=False))] for v in by.values()], []))
# one pass over the text: headings, statements, earlier answers per question
part = ch = None; newest = {"found": {}, "home": {}, "owner": {}}; addr = {}   # (X, part, chapter) -> object
heads = []   # (pos, "part"/"chapter", label)
prior = collections.defaultdict(collections.Counter); kprior = collections.defaultdict(collections.Counter)
qpos = {p: (k, d) for p, k, d in qs}; allq = {p: k for p, k in info["answers"]}
res = {m: {} for m in ("prior", "newest", "bm25", "oracle")}
CH = 128; chunks = []; df = collections.Counter(); start = 0; post = collections.defaultdict(list)
STOP = set("what did in of the where does who ? .".split())
def question_at(p):
    j = p - 1
    while j > 0 and w[j - 1] != ".": j -= 1
    return w[j:p]
def bm25_answer(p, kind, qw, ent):
    nch = len(chunks)
    if nch == 0: return None
    terms = [t for t in qw if t not in STOP]
    sc = collections.defaultdict(float)   # every chunk is CH words long, so the length norm is 1
    for t in set(terms):
        idf = math.log(1 + (nch - df[t] + 0.5) / (df[t] + 0.5))
        for ci, f in post.get(t, ()):
            sc[ci] += idf * terms.count(t) * f * 2.2 / (f + 1.2)
    if not sc: return None
    bi = max(sc, key=lambda c: (sc[c], c))
    cw = chunks[bi][1]
    # the word of the answer's kind nearest to a mention of the entity (within 8 words; ties: the later one)
    best = None
    for i, t in enumerate(cw):
        if t == ent:
            for j in range(max(0, i - 8), min(len(cw), i + 9)):
                if j != i and cw[j] in kind_set[kind] and cw[j] != ent:
                    key = (abs(j - i), -j)
                    if best is None or key < best[0]: best = (key, cw[j])
    return best[1] if best else None
for i in range(n):
    t = w[i]
    if i in qpos:
        kind, d = qpos[i]; qw = question_at(i)
        ent = qw[2] if kind != "owner" else qw[3]
        if kind == "found":
            X = qw[2]; rest = " ".join(qw[qw.index(H_CH) + 1:-1]).split(f" of {H_PT} ")
            M, N = rest[0], (rest[1] if len(rest) > 1 else None)
            # oracle: the latest part N heading, then chapter M after it, then X's find in that chapter
            pa = max([h[0] for h in heads if h[1] == H_PT and h[2] == N], default=-1) if N else -1
            cm = [h[0] for h in heads if h[1] == H_CH and h[2] == M and h[0] > pa]
            res["oracle"][i] = addr.get((X, N, M, cm[0] if cm else -1)) == w[i]
            res["newest"][i] = newest["found"].get(X) == w[i]
        else:
            key = qw[2] if kind == "home" else qw[3]
            res["oracle"][i] = res["newest"][i] = newest[kind].get(key) == w[i]
        pk = (kind, ent); guess = prior[pk].most_common(1)[0][0] if prior[pk] else (kprior[kind].most_common(1)[0][0] if kprior[kind] else None)
        res["prior"][i] = guess == w[i]
        res["bm25"][i] = bm25_answer(i, kind, qw, ent) == w[i]
    if i in allq and allq[i] in kind_set:
        qw = question_at(i); kind = allq[i]; ent = qw[2] if kind != "owner" else qw[3]
        prior[(kind, ent)][t] += 1; kprior[kind][t] += 1
    # the text as it stands before word i+1
    if t in (H_PT, H_CH) and (i == 0 or w[i - 1] == "."):
        j = i + 1; lab = []
        while w[j] != ".": lab.append(w[j]); j += 1
        lab = " ".join(lab); heads.append((i, t, lab))
        if t == H_PT: part = lab
        else: ch = (lab, i)
    if t == "found" and w[i + 1] == "the" and ch:
        newest["found"][w[i - 1]] = w[i + 2]; addr[(w[i - 1], part, ch[0], ch[1])] = w[i + 2]
    if t == "lives" and w[i + 1] == "in" and w[i - 1] in names: newest["home"][w[i - 1]] = w[i + 3]
    if t == "owns" and w[i - 1] in names: newest["owner"][w[i + 2]] = w[i - 1]   # a statement, not "who owns"
    if i + 1 - start == CH:
        cw = w[start:i + 1]; tf = collections.Counter(cw); chunks.append((start, cw, tf)); df.update(tf.keys()); start = i + 1
        for t2, f in tf.items(): post[t2].append((len(chunks) - 1, f))
print(f"{doc}: {len(qs)} questions scored (at most {cap} per kind)")
for m in res:
    print(m)
    for kind in ("home", "owner", "found"):
        cells = []
        for lo, hi, g in groups:
            ps = [p for p, k, d in qs if k == kind and lo <= d < hi]
            if ps: cells.append(f"{g} {np.mean([res[m][p] for p in ps]):6.1%} ({len(ps)})")
            else: cells.append(f"{g}   -  ")
        print(f"  {kind:<6} " + "  ".join(cells))
