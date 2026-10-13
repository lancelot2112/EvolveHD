"""Online (prequential) byte n-gram baseline with Witten-Bell interpolation; bits per byte."""
import os
CORPUS = os.environ.get('EVOLVEHD_CORPUS', os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'corpus'))
import math, sys
from collections import defaultdict

def wb_bpb(data, order, block=25000):
    counts = [defaultdict(lambda: defaultdict(int)) for _ in range(order + 1)]
    tot = [defaultdict(int) for _ in range(order + 1)]
    bits, out, blk = 0.0, [], 0.0
    for i, c in enumerate(data):
        p = 1.0 / 256
        for k in range(0, order + 1):
            if i < k:
                break
            ctx = data[i - k:i]
            n = tot[k][ctx]
            if n == 0:
                continue
            d = counts[k][ctx]
            T = len(d)
            p = (d.get(c, 0) + T * p) / (n + T)
        b = -math.log2(p)
        bits += b; blk += b
        for k in range(0, order + 1):
            if i < k:
                break
            ctx = data[i - k:i]
            counts[k][ctx][c] += 1
            tot[k][ctx] += 1
        if (i + 1) % block == 0:
            out.append(blk / block); blk = 0.0
    return bits / len(data), out

if __name__ == '__main__':
    import json
    A = open(os.path.join(CORPUS, 'bookA.bin'), 'rb').read()
    B = open(os.path.join(CORPUS, 'bookB.bin'), 'rb').read()
    res = {}
    for order in (0, 1, 2, 3, 4, 5):
        tot_ab, blocks_ab = wb_bpb(A + B, order)
        tot_b, blocks_b = wb_bpb(B, order)
        res[order] = dict(AB=tot_ab, blocks_AB=blocks_ab, B_fresh=tot_b, blocks_B=blocks_b)
        lastA = sum(blocks_ab[8:12]) / 4
        firstB_cont = sum(blocks_ab[12:14]) / 2
        firstB_fresh = sum(blocks_b[0:2]) / 2
        print(f'order {order}: A+B {tot_ab:.3f} | A last 100k {lastA:.3f} | B first 50k after A {firstB_cont:.3f} | B first 50k fresh {firstB_fresh:.3f}', flush=True)
    json.dump(res, open('ngram.json', 'w'))
