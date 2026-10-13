"""Per-book bits per byte for a full-corpus run against the Witten-Bell baselines.

usage: python3 scripts/full_report.py runs/full_real.json runs/full_wb3.json runs/full_wb4.json corpus/gutenberg_all.idx
Each JSON holds "bpb": mean bits per byte per 1,000-byte block.
"""
import json
import sys


def blocks(path):
    return json.load(open(path))['bpb']


def mean(b, lo, hi):
    lo, hi = lo // 1000, max(lo // 1000 + 1, hi // 1000)
    seg = b[lo:hi]
    return sum(seg) / len(seg)


def main():
    run, wb3, wb4, idx = sys.argv[1:5]
    g, w3, w4 = blocks(run), blocks(wb3), blocks(wb4)
    books = [(n, int(o), int(l)) for n, o, l in (line.split('\t') for line in open(idx) if line.strip())]
    print(f"{'book':<22}{'bytes':>10}{'grown':>8}{'WB3':>8}{'WB4':>8}{'grown-WB4':>11}")
    for n, o, l in books:
        a, b3, b4 = mean(g, o, o + l), mean(w3, o, o + l), mean(w4, o, o + l)
        print(f"{n:<22}{l:>10}{a:>8.3f}{b3:>8.3f}{b4:>8.3f}{a - b4:>11.3f}")
    end = len(g) * 1000
    a, b3, b4 = mean(g, 0, end), mean(w3, 0, end), mean(w4, 0, end)
    print(f"{'whole':<22}{end:>10}{a:>8.3f}{b3:>8.3f}{b4:>8.3f}{a - b4:>11.3f}")
    d = json.load(open(run))
    print('\nkept at the end:', {k: (v['beta'], v['gain_at_keep']) for k, v in d['kept'].items()})
    ev = d['events']
    first = {}
    for e in ev:
        if e[1] == 'keep' and e[3] not in first:
            first[e[3]] = e[0]
    print('first keeps:', sorted(first.items(), key=lambda x: x[1])[:20])
    for kind in ('screen-prevword', 'screen-prev-given-chunk'):
        v = [(e[0] // 1000, e[-1]) for e in ev if e[1] == kind]
        print(kind, v[:: max(1, len(v) // 12)])
    counts = {}
    for e in ev:
        counts[e[1]] = counts.get(e[1], 0) + 1
    print('event counts:', counts)
    print('nodes:', [(n['name'], n['kind'], n.get('cells'), n.get('evictions')) for n in d['nodes']])
    print('info:', d['info'], 'secs:', d['secs'])


if __name__ == '__main__':
    main()
