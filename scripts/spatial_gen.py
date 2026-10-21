"""Spatial stories (experiment 33): each story places 9 rooms (of 12 names) on a 3 x 3 grid at random and states
adjacencies one at a time ("the kitchen is north of the garden ."), either way round, in random order. Now and
then a question asks what lies one step in a direction from a room ("what is north of the garden ?"); the answer
may never have been stated, only implied by facts chained through other rooms, or not determined yet at all.
Each question records how many facts came before it in the story, whether its answer was stated directly, and
whether the facts so far determine it.
usage: spatial_gen.py STORIES OUT_PREFIX [seed]"""
import sys, json, random
from collections import deque
n_st = int(sys.argv[1]); out = sys.argv[2]; rng = random.Random(int(sys.argv[3]) if len(sys.argv) > 3 else 1)
rooms = "kitchen garden office bathroom hallway bedroom cellar attic study library pantry porch".split()
dirs = {"north": (0, 1), "south": (0, -1), "east": (1, 0), "west": (-1, 0)}
func = "new story . the is of what ? north south east west".split()
vocab = func + rooms; ix = {w: i for i, w in enumerate(vocab)}
toks, qs = [], []
def say(*ws): toks.extend(ix[w] for w in ws)
for st in range(n_st):
    say("new", "story", ".")
    names = rng.sample(rooms, 9); cell = {names[i]: (i % 3, i // 3) for i in range(9)}
    at = {v: k for k, v in cell.items()}
    edges = []
    for a in names:
        for d, (dx, dy) in dirs.items():
            b = at.get((cell[a][0] - dx, cell[a][1] - dy))   # a is d of b
            if b and (b, a) not in [(e[1], e[0]) for e in edges]: edges.append((a, d, b))
    rng.shuffle(edges); told = []
    for (a, d, b) in edges[: rng.randint(7, 12)]:
        if rng.random() < 0.5:   # state it the other way round
            inv = {"north": "south", "south": "north", "east": "west", "west": "east"}[d]; a, d, b = b, inv, a
        say("the", a, "is", d, "of", "the", b, "."); told.append((a, d, b))
        for _ in range(2 if rng.random() < 0.6 else 0):
            y = rng.choice(names); d = rng.choice(list(dirs)); dx, dy = dirs[d]
            x = at.get((cell[y][0] + dx, cell[y][1] + dy))
            if not x: continue
            # determined? the facts so far must connect x and y (any path; offsets compose)
            g = {}
            for (p, dd, q) in told:
                ox, oy = dirs[dd]; g.setdefault(q, []).append((p, (ox, oy))); g.setdefault(p, []).append((q, (-ox, -oy)))
            seen = {y: (0, 0)}; dq = deque([y])
            while dq:
                u = dq.popleft()
                for v, (ox, oy) in g.get(u, []):
                    if v not in seen: seen[v] = (seen[u][0] + ox, seen[u][1] + oy); dq.append(v)
            det = x in seen
            direct = any((p, dd, q) == (x, d, y) for p, dd, q in told) or any((q, dd2, p) == (x, d, y) for p, dd2, q in [])
            inv = {"north": "south", "south": "north", "east": "west", "west": "east"}[d]
            direct = direct or any((p, dd, q) == (y, inv, x) for p, dd, q in told)
            say("what", "is", d, "of", "the", y, "?"); qs.append((len(toks), len(told), direct, det)); say(x, ".")
import array
array.array('I', toks).tofile(open(out + ".ids", "wb"))
open(out + ".txt", "w").write("\n".join(vocab) + "\n")
json.dump({"questions": qs, "rooms": [ix[r] for r in rooms], "dirs": [ix[d] for d in dirs]}, open(out + ".info.json", "w"))
nd = sum(1 for q in qs if q[3]); ndir = sum(1 for q in qs if q[2])
print(f"{n_st} stories, {len(toks)} words, {len(qs)} questions ({ndir} stated directly, {nd - ndir} implied only, {len(qs) - nd} not determined yet)")
