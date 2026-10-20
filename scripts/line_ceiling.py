"""Ceiling for a 1-D word map: order content words (id >= 100) offline from the whole text's co-occurrence
(within 10 content positions, PPMI) by a spectral ordering (Fiedler vector), or by iterated barycentric sorting
from it, then measure locality as cortex_map.py does (share of each word's 10 strongest associates within 1% of
the line; one spot per word). Also a 2-D version (two spectral coordinates, nearest 2% of words).
usage: line_ceiling.py IDS"""
import sys, random
import numpy as np, scipy.sparse as spm
from scipy.sparse.linalg import eigsh
ids = np.fromfile(sys.argv[1], dtype=np.uint32).astype(np.int64); SKIP = 100
cw = ids[ids >= SKIP]; NW = int(ids.max()) + 1
r_, c_ = [], []
for d in range(1, 11): r_.append(cw[:-d]); c_.append(cw[d:])
r_ = np.concatenate(r_); c_ = np.concatenate(c_)
M = spm.coo_matrix((np.ones(len(r_)), (r_, c_)), shape=(NW, NW)).tocsr(); M = M + M.T
cnt = np.bincount(cw, minlength=NW); words = np.nonzero(cnt >= 3)[0]
M = M[words][:, words].tocoo(); tot = M.sum(); rs = np.asarray(M.sum(1)).ravel()
pmi = np.log(M.data * tot / (rs[M.row] * rs[M.col])); keep = (pmi > 0) & (M.data >= 2)
P = spm.coo_matrix((pmi[keep], (M.row[keep], M.col[keep])), shape=M.shape).tocsr()
dg = np.asarray(P.sum(1)).ravel() + 1e-9; Dm = spm.diags(1 / np.sqrt(dg))
L = Dm @ P @ Dm
vals, vecs = eigsh(L, k=4, which='LA')
o = np.argsort(-vals); vecs = vecs[:, o] * (1 / np.sqrt(dg))[:, None]
nwd = len(words); rng = random.Random(1)
Mfull = (M.tocsr())
def locality(pos_fn, near_fn):
    loc, ch = [], []
    cand = [i for i in range(nwd) if cnt[words[i]] >= 50]
    for i in rng.sample(cand, min(2000, len(cand))):
        row = Mfull.getrow(i); js, vs = row.indices, row.data; ok = vs >= 5; js, vs = js[ok], vs[ok]
        if len(js) < 10: continue
        pm = np.log(vs * tot / (rs[i] * rs[js])); top = js[np.argsort(-pm)[:10]]
        loc.append(np.mean([near_fn(i, j) for j in top])); ch.append(np.mean([near_fn(i, j) for j in rng.sample(cand, 10)]))
    return np.mean(loc), np.mean(ch), len(loc)
rank = np.empty(nwd); rank[np.argsort(vecs[:, 1])] = np.arange(nwd)
print("1-D spectral order:", "locality %.3f (random %.3f; %d words)" % locality(None, lambda i, j: abs(rank[i] - rank[j]) <= 0.01 * nwd))
# barycentric refinement: each pass, every word moves to the PPMI-weighted mean rank of its associates, then re-rank
r = rank.copy()
for it in range(30):
    t = 0.5 * r + 0.5 * (P @ r) / dg; r = np.empty(nwd); r[np.argsort(t)] = np.arange(nwd)
print("1-D barycentric (30 passes):", "locality %.3f (random %.3f; %d words)" % locality(None, lambda i, j: abs(r[i] - r[j]) <= 0.01 * nwd))
X = vecs[:, 1:3]; X = (X - X.mean(0)) / X.std(0)
from scipy.spatial import cKDTree
tree = cKDTree(X); kn = max(1, int(0.02 * nwd))
nbr = {}
def near2(i, j):
    if i not in nbr: nbr[i] = set(tree.query(X[i], k=kn)[1])
    return j in nbr[i]
print("2-D spectral sheet (nearest 2%):", "locality %.3f (random %.3f; %d words)" % locality(None, near2))
