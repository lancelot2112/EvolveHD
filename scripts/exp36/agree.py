"""With --sit-lookback and SIT_WDBG=1, the slots' and the window's answers at each question: how often they agree,
how right they are then, and who is right when they disagree. usage: agree.py IDS INFO.json LOG ..."""
import sys, json, collections
import numpy as np
ids = np.fromfile(sys.argv[1], dtype=np.uint32); n = len(ids); info = json.load(open(sys.argv[2]))
for log in sys.argv[3:]:
    W = {}
    for l in open(log):
        if l.startswith('WDBG'):
            p = l.split(); W[int(p[1])] = (int(p[3]), int(p[5]), int(p[9]))
    st = collections.defaultdict(collections.Counter)
    for p, k in info['answers']:
        if p < n // 2 or p not in W: continue
        s_, w_, u = W[p]; y = int(ids[p])
        for kk in (k, 'all'):
            c = st[kk]; c['n'] += 1; c['chosen'] += (w_ if u else s_) == y; c['slot'] += s_ == y; c['win'] += w_ == y
            if s_ == w_: c['agree'] += 1; c['agree_right'] += s_ == y
            else: c['dis_slot'] += s_ == y; c['dis_win'] += w_ == y; c['dis'] += 1
    print(log.split('/')[-1])
    for k in ('where-person', 'where-object', 'who-has', 'all'):
        c = st[k]
        print(f"  {k:<13} slots {c['slot'] / c['n']:5.1%}  window {c['win'] / c['n']:5.1%}  given {c['chosen'] / c['n']:5.1%}  |  agree {c['agree'] / c['n']:5.1%} of questions, right {c['agree_right'] / max(1, c['agree']):5.1%}  |  disagree: slots right {c['dis_slot'] / max(1, c['dis']):5.1%}, window right {c['dis_win'] / max(1, c['dis']):5.1%}")
