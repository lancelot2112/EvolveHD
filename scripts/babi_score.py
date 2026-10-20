"""Score answers on the comprehension benchmark (babi_gen.py): accuracy (the top prediction is the answer) and
bits paid for the answer word, by question type, over the second half of the stream. With PREFIX given, reads a
run's per-word dump (wordslots --dump-bits PREFIX, or any PREFIX.bits f64 + PREFIX.hits u8); without, scores the
reference models:
  chance among the 8 candidates of the right kind (places or names)
  order-2 Witten-Bell counts (prequential)
  tracking rule (hand-written, for reference): follows each person's place and each object's holder and place
           through the story, and answers from that state
usage: babi_score.py IDS INFO.json [PREFIX ...]"""
import sys, json, math
import numpy as np
from collections import Counter, defaultdict
ids = np.fromfile(sys.argv[1], dtype=np.uint32).astype(np.int64); n = len(ids)
info = json.load(open(sys.argv[2])); ans = [(p, k) for p, k in info['answers'] if p >= n // 2]
kinds = ['where-person', 'where-object', 'who-has']
kind_of = dict(ans)
def report(name, bits_at, hit_at):
    row = [f"{name:<34}"]
    for k in kinds + ['all']:
        ps = [p for p, kk in ans if k == 'all' or kk == k]
        acc = np.mean([hit_at(p) for p in ps]); b = np.mean([bits_at(p) for p in ps])
        row.append(f"{k} {acc:5.1%} {b:5.2f}b")
    print("  ".join(row), flush=True)
print(f"{len(ans)} questions scored (second half); accuracy and bits per answer")
if len(sys.argv) > 3:
    for pre in sys.argv[3:]:
        bits = np.fromfile(pre + ".bits", dtype='<f8'); hits = np.fromfile(pre + ".hits", dtype=np.uint8)
        report(pre.split('/')[-1], lambda p: bits[p], lambda p: hits[p])
    sys.exit()
report("chance (8 places; 4 people per story)", lambda p: 2.0 if kind_of[p] == "who-has" else 3.0, lambda p: 0.25 if kind_of[p] == "who-has" else 1 / 8)
# order-2 WB, prequential
V = int(ids.max()) + 1
def wb(c, tot, y, base):
    if not tot: return base
    return (c.get(y, 0) + len(c) * base) / (tot + len(c))
uni = Counter(); un = 0; big = defaultdict(Counter); bt = Counter(); tri = defaultdict(Counter); tt = Counter()
pbits = np.zeros(n); phit = np.zeros(n, dtype=bool)
need = set(p for p, _ in ans)
for i in range(n):
    y = int(ids[i]); a = int(ids[i-1]) if i else -1; b = (int(ids[i-2]), a) if i > 1 else None
    if i in need:
        def prob(z):
            p0 = (uni[z] + 1) / (un + V); p1 = wb(big[a], bt[a], z, p0)
            return wb(tri[b], tt[b], z, p1) if b else p1
        pbits[i] = -math.log2(prob(y)); phit[i] = max(range(V), key=prob) == y
    uni[y] += 1; un += 1
    if i: big[a][y] += 1; bt[a] += 1
    if b: tri[b][y] += 1; tt[b] += 1
report("order-2 counts", lambda p: pbits[p], lambda p: phit[p])
# recency rule
vocab = open(sys.argv[1].replace('.ids', '.txt')).read().split()
W = {w: i for i, w in enumerate(vocab)}
names, places, objs = set(info['names']), set(info['places']), set(info['objects'])
rhit = {}
last_place = {}; holder = {}; obj_place = {}
sent = []; last_q = []
for i in range(n):
    if i in need:
        q = last_q  # the question just asked: "where is mary ?", "where is the apple ?", "who has the apple ?"
        y = int(ids[i]); qn = [w for w in q if w in names]; qo = [w for w in q if w in objs]
        if W['who'] in q: guess = holder.get(qo[0]) if qo else None
        elif qo: guess = obj_place.get(qo[0])
        else: guess = last_place.get(qn[0]) if qn else None
        rhit[i] = guess == y
    w = int(ids[i]); sent.append(w)
    if w in (W['.'], W['?']):
        s = sent; sent = []
        if w == W['?']:
            last_q = s; continue
        if W['new'] in s: last_place, holder, obj_place = {}, {}, {}
        ns = [x for x in s if x in names]; ps = [x for x in s if x in places]; os_ = [x for x in s if x in objs]
        if ns and ps: last_place[ns[0]] = ps[0]
        for o in os_:
            if ns: holder[o] = ns[-1]
            if ns and ns[0] in last_place: obj_place[o] = last_place[ns[0]]
        if W['dropped'] in s and os_: holder.pop(os_[0], None)
        # an object moves with its holder
        if ns and ps:
            for o, h in holder.items():
                if h == ns[0]: obj_place[o] = ps[0]
report("tracking rule (hand-written)", lambda p: 0.0, lambda p: rhit[p])
