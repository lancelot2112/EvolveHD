"""Check every answer in a (probe) stream by simulating the world: people's places, who holds what, where dropped
things lie. Prints the share of answers the simulation agrees with, from answer index START on.
usage: check_answers.py DOC [START]"""
import sys, json
import numpy as np
doc = sys.argv[1]; start = int(sys.argv[2]) if len(sys.argv) > 2 else 0
ids = np.fromfile(doc + ".ids", dtype=np.uint32).tolist(); voc = open(doc + ".txt").read().split("\n")
info = json.load(open(doc + ".info.json")); w = [voc[i] for i in ids]
MOVE = {"went", "moved", "walked", "travelled", "journeyed", "wandered"}
TAKE = {"picked", "took", "grabbed", "got"}; DROP = {"dropped", "discarded", "left", "put"}; GIVE = {"gave", "handed"}
ans = {a[0]: i for i, a in enumerate(info["answers"])}
loc, held, lies = {}, {}, {}
ok = bad = 0; sent = []; bad_list = []
for p, t in enumerate(w):
    if p in ans:
        q = sent[:sent.index("?")]
        if q[:3] == ["in", "which", "room"]:
            q = ["where"] + q[3:]
        if q[:3] == ["who", "is", "holding"]:
            q = ["who", "has"] + q[3:]
        if q[0] == "what":  # what place is X in
            x = q[3]; exp = loc.get(x)
        elif q[:2] == ["who", "has"]:
            exp = held.get(q[-1])
        elif q[2] == "the":
            o = q[-1]; exp = loc.get(held[o]) if o in held else lies.get(o)
        else:
            exp = loc.get(q[-1])
        if ans[p] >= start:
            if exp == t: ok += 1
            else:
                bad += 1; bad_list.append((p, " ".join(q), t, exp))
        continue
    sent.append(t)
    if t != ".":
        continue
    s = sent; sent = []
    if s[:2] == ["new", "story"]:
        loc, held, lies = {}, {}, {}; continue
    if "?" in s or len(s) < 3:
        continue
    x, v = s[0], s[1]
    if v in MOVE:
        loc[x] = s[-2]
    elif v in TAKE:
        o = s[s.index("the") + 1]; held[o] = x; lies.pop(o, None)
    elif v in DROP:
        o = s[s.index("the") + 1]
        if held.get(o) == x:
            del held[o]
        lies[o] = loc.get(x)
    elif v in GIVE:
        o = s[s.index("the") + 1]; held[o] = s[-2]
print(f"{doc}: {ok} of {ok + bad} answers agree with the simulation; first disagreements: {bad_list[:3]}")
