import os
CORPUS = os.environ.get('EVOLVEHD_CORPUS', os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'corpus'))

import json, os, sys, time
from multiprocessing import Pool
from text_sim import TextBrain
A = open(os.path.join(CORPUS, 'bookA.bin'), 'rb').read()
B = open(os.path.join(CORPUS, 'bookB.bin'), 'rb').read()
JOBS = [
    ('grown, real pools', 0, dict(real=True), 'AB'),
    ('grown, real pools', 1, dict(real=True), 'AB'),
    ('grown, symbolic nodes', 0, dict(real=False), 'AB'),
    ('grown, real pools, noisy input', 0, dict(real=True, noise=(0.25, 4)), 'AB'),
    ('hand-wired order 4, symbolic', 0, dict(real=False, grow=False, oracle=4), 'AB'),
    ('hand-wired order 4, real pools', 0, dict(real=True, grow=False, oracle=4), 'AB'),
    ('grown, real pools, fresh on book B', 0, dict(real=True), 'B'),
    ('hand-wired order 4, symbolic, fresh on book B', 0, dict(real=False, grow=False, oracle=4), 'B'),
]
def job(j):
    name, seed, kw, stream = j
    data = A + B if stream == 'AB' else B
    t0 = time.time()
    b = TextBrain(data, seed=seed, **kw)
    b.run()
    s = b.summary(1000)
    s['secs'] = time.time() - t0
    return dict(name=name, seed=seed, stream=stream, r=s)
if __name__ == '__main__':
    out = 'trun.jsonl'; done = set()
    if os.path.exists(out):
        for l in open(out):
            d = json.loads(l); done.add((d['name'], d['seed']))
    todo = [j for j in JOBS if (j[0], j[1]) not in done]
    with Pool(2) as p, open(out, 'a') as f:
        for d in p.imap_unordered(job, todo):
            f.write(json.dumps(d, default=lambda o: o.item() if hasattr(o, 'item') else str(o)) + '\n'); f.flush()
            print('done', d['name'], d['seed'], round(d['r']['secs']), flush=True)
