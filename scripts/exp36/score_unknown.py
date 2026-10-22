"""Score runs that may answer "unknown" (no candidate: u32::MAX in OUT.top), on the scored half's questions.
Per question kind: right, wrong and unknown as shares of the questions, and the accuracy of the answers given.
usage: score_unknown.py IDS INFO.json PREFIX ..."""
import sys, json, numpy as np
ids = np.fromfile(sys.argv[1], dtype=np.uint32).astype(np.int64); n = len(ids)
info = json.load(open(sys.argv[2]))
ans = [(p, k) for p, k in info['answers'] if p >= n // 2]
for pre in sys.argv[3:]:
    top = np.fromfile(pre + ".top", dtype=np.uint32).reshape(n, -1)[:, 0].astype(np.int64)
    parts = []
    for kind in ('where-person', 'where-object', 'who-has', 'all'):
        ps = np.array([p for p, k in ans if kind == 'all' or k == kind])
        unk = top[ps] == 0xFFFFFFFF
        right = (top[ps] == ids[ps]) & ~unk
        wrong = ~right & ~unk
        given = (~unk).sum()
        parts.append(f"{kind} right {right.mean():.1%} wrong {wrong.mean():.1%} unknown {unk.mean():.1%} (answers {right.sum() / max(1, given):.1%} right)")
    print(pre.split('/')[-1]); print('  ' + '\n  '.join(parts))
