import os
CORPUS = os.environ.get('EVOLVEHD_CORPUS', os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'corpus'))

import json, time, sys
from multiprocessing import Pool
from word_sim import WordBrain
A = open(os.path.join(CORPUS, 'bookA.bin'), 'rb').read()
B = open(os.path.join(CORPUS, 'bookB.bin'), 'rb').read()
JOBS = [('words, real pools', 0, dict(real=True)), ('words, symbolic nodes', 0, dict(real=False))]
def job(j):
    name, seed, kw = j
    t0 = time.time()
    b = WordBrain(A + B, seed=seed, **kw)
    b.run()
    s = b.summary(1000); s['secs'] = time.time() - t0
    s['words_A'] = b.word_report(200000, 300000)
    s['words_B'] = b.word_report(300000, 400000)
    return dict(name=name, seed=seed, stream='AB', r=s)
if __name__ == '__main__':
    with Pool(2) as p, open('wrun.jsonl', 'a') as f:
        for d in p.imap_unordered(job, JOBS):
            f.write(json.dumps(d, default=lambda o: o.item() if hasattr(o, 'item') else str(o)) + '\n'); f.flush()
            print('done', d['name'], round(d['r']['secs']), flush=True)
