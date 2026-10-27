"""Systematic generalization probes: change the test part of a stream (from word FROM on) in one controlled way the
reader never saw in training, keep training as it is, and keep every answer right.
Probes (comma-separated, applied together):
  names     every person in the test gets a new name, never seen in training (one fixed mapping)
The probes start at the first whole story at or after FROM; info "probe_first_answer" is the index of the first
answer in a probed story (the same index in the unchanged stream, for a baseline on the same questions).
  verb      every movement verb in the test (went, moved, walked, travelled, journeyed) becomes "wandered"
  question  "where is X ?" becomes "in which room is X ?"; "who has the X ?" becomes "who is holding the X ?"
  longer    each test story gets as many extra sentences again: moves by people whose moves cannot change any answer
            (story set: people absent from the story; bAbI: people who never touch the object asked about)
Writes DST.ids, DST.txt (the old vocabulary plus the new words at the end) and DST.info.json (the source's info with
answer positions moved; "from" is the first test word, "probe" the probes, "new_words" the new words' ids).
usage: probe_stream.py SRC DST FROM PROBES [SEED]"""
import sys, json, random
import numpy as np
src, dst, frm, probes = sys.argv[1], sys.argv[2], int(sys.argv[3]), set(sys.argv[4].split(","))
rng = random.Random(int(sys.argv[5]) if len(sys.argv) > 5 else 0)
ids = np.fromfile(src + ".ids", dtype=np.uint32).tolist()
voc = open(src + ".txt").read().split("\n")
while voc and voc[-1] == "":
    voc.pop()
nvoc0 = len(voc)
def wid(w):
    if w not in voc:
        voc.append(w)
    return voc.index(w)
info = json.load(open(src + ".info.json"))
story = "names" in info
V = {w: i for i, w in enumerate(voc)}
DOT, Q = V["."], V["?"]
MOVES = {V[w] for w in ("went", "moved", "walked", "travelled", "journeyed") if w in V}
HANDLE = {V[w] for w in ("picked", "took", "grabbed", "got", "left", "dropped", "discarded", "put", "gave", "handed") if w in V}
if story:
    PEOPLE = list(info["names"]); PLACES = list(info["places"])
else:
    PEOPLE = [V[w] for w in ("mary", "john", "sandra", "daniel")]
    PLACES = [V[w] for w in ("kitchen", "garden", "office", "bathroom", "hallway", "bedroom")]
ans_pos = {a[0] for a in info["answers"]}

# the probes start at the first story that begins at or after FROM (story set: "new story"; bAbI: after a question's
# answer sentence), so no story is half changed
p0 = frm
if story:
    while p0 + 1 < len(ids) and not (ids[p0] == V["new"] and ids[p0 + 1] == V["story"]):
        p0 += 1
else:
    while p0 > 0 and not (ids[p0 - 1] == DOT and (p0 - 2) in ans_pos):
        p0 += 1
# sentences of the probed part, as lists of (token, original position or None); a sentence ends at "."
head = [(t, p) for p, t in enumerate(ids[:p0])]
sents, cur = [], []
for p in range(p0, len(ids)):
    cur.append((ids[p], p))
    if ids[p] == DOT:
        sents.append(cur); cur = []
if cur:
    sents.append(cur)
# stories: story set, split at "new story ."; bAbI, a story ends with its question sentence
docs, d = [], []
for s in sents:
    toks = [t for t, _ in s]
    if story and toks[:2] == [V["new"], V["story"]]:
        if d:
            docs.append(d)
        d = [s]; continue
    d.append(s)
    if not story and Q in toks:
        docs.append(d); d = []
if d:
    docs.append(d)

n_ins, n_skip = 0, 0
if "longer" in probes:
    for d in docs:
        toks = [t for s in d for t, _ in s]
        if story:
            ok = [x for x in PEOPLE if x not in toks]
        else:
            asked = [s[[t for t, _ in s].index(Q) - 1][0] for s in d if Q in [t for t, _ in s]]
            touch = {s[0][0] for s in d if len(s) > 1 and s[1][0] in HANDLE and any(t in asked for t, _ in s)}
            ok = [x for x in PEOPLE if x not in touch]
        if not ok:
            n_skip += 1; continue
        k0 = 1 if story and [t for t, _ in d[0]][:2] == [V["new"], V["story"]] else 0
        last = max([i for i, s in enumerate(d) if Q in [t for t, _ in s]], default=len(d))
        slots = list(range(k0, last + 1)) if not story else list(range(k0, len(d) + 1))
        extra = []
        for _ in range(len(d) - k0):
            x, pl, vb = rng.choice(ok), rng.choice(PLACES), rng.choice(sorted(MOVES))
            extra.append((rng.choice(slots), [(x, None), (vb, None), (V["to"], None), (V["the"], None), (pl, None), (DOT, None)]))
        for at, s in sorted(extra, key=lambda e: -e[0]):
            d.insert(at, s); n_ins += 1

# token-level probes
newname = {}
if "names" in probes:
    pool = ["alice", "peter", "nora", "oscar", "ruth", "sam", "tom", "vera"]
    newname = {x: wid(pool[i]) for i, x in enumerate(PEOPLE)}
WAND = wid("wandered") if "verb" in probes else None
out, pos = [t for t, _ in head], {}
for t, p in head:
    pos[p] = p
for dd in docs:
    for s in dd:
        toks = [t for t, _ in s]
        if "question" in probes and Q in toks:
            qi = toks.index(Q)
            if toks[:2] == [V["where"], V["is"]]:
                s = [(wid("in"), None), (wid("which"), None), (wid("room"), None), (V["is"], s[1][1])] + s[2:]
            elif toks[:2] == [V["who"], V["has"]]:
                s = [s[0], (V["is"], None), (wid("holding"), s[1][1])] + s[2:]
        for t, p in s:
            if t in newname:
                t = newname[t]
            if WAND is not None and t in MOVES:
                t = WAND
            if p is not None:
                pos[p] = len(out)
            out.append(t)
info2 = dict(info)
info2["answers"] = [[pos[a[0]]] + list(a[1:]) for a in info["answers"] if a[0] in pos]
for k in ("newq", "newfact"):
    if k in info:
        info2[k] = [pos[p] for p in info[k] if p in pos]
if "words" in info:
    info2["words"] = len(out)
if story and newname:
    info2["names"] = sorted(set(info["names"]) | set(newname.values()))
first_ans = sum(1 for a in info["answers"] if a[0] < p0)
info2.update({"from": frm, "probe_from": pos.get(p0, len(out)), "probe_first_answer": first_ans, "probe": sorted(probes), "new_words": list(range(nvoc0, len(voc))), "inserted": n_ins, "docs_not_lengthened": n_skip})
np.array(out, dtype=np.uint32).tofile(dst + ".ids")
open(dst + ".txt", "w").write("\n".join(voc) + "\n")
json.dump(info2, open(dst + ".info.json", "w"))
print(f"{dst}: {len(ids)} -> {len(out)} words, {len(info2['answers'])} answers, new words {[voc[i] for i in info2['new_words']]}, "
      f"{n_ins} sentences inserted, {n_skip} stories not lengthened; probes from word {p0} (answer {first_ans})", file=sys.stderr)
