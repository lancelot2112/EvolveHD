"""Synthetic reading-comprehension stories (bAbI-style) as a word stream, for experiment 30.

People move between places and pick up, drop and hand over objects; questions are asked along the way and the
answer is the word right after the question mark, so a next-word predictor answers by predicting it.
Question types:
  where-person   "where is mary ? kitchen ."        the person's last place (one supporting fact, with updates)
  where-object   "where is the apple ? office ."    where its holder is now, or where it was dropped (two facts)
  who-has        "who has the apple ? john ."       its holder after hand-overs
Each story starts with "new story ." and resets the world. Function words get ids 0..F-1 and content words
(names, places, objects) ids F..; F is written to OUT.info.json with the answer positions and types.
usage: babi_gen.py STORIES OUT_PREFIX [seed]"""
import sys, json, random
n_st = int(sys.argv[1]); out = sys.argv[2]; rng = random.Random(int(sys.argv[3]) if len(sys.argv) > 3 else 1)
names = "mary john sandra daniel emma fred bill julie".split()
places = "kitchen garden office bathroom hallway bedroom cellar attic".split()
objects = "apple football milk book key pen hat cup".split()
func = "new story . went moved journeyed to the picked up got dropped gave where is who has ?".split()
vocab = func + names + places + objects
ix = {w: i for i, w in enumerate(vocab)}
F = len(func)
toks, answers = [], []
def say(*ws): toks.extend(ix[w] for w in ws)
for _ in range(n_st):
    say("new", "story", ".")
    people = rng.sample(names, 4); things = rng.sample(objects, 3)
    loc = {}; held = {}; obj_loc = {}
    for _step in range(rng.randint(8, 14)):
        r = rng.random(); p = rng.choice(people)
        if r < 0.5 or p not in loc:
            pl = rng.choice([x for x in places if x != loc.get(p)])
            say(p, rng.choice(["went", "moved", "journeyed"]), "to", "the", pl, "."); loc[p] = pl
            for o, h in held.items():
                if h == p: obj_loc[o] = pl
        elif r < 0.7:
            free = [o for o in things if o not in held]
            if free:
                o = rng.choice(free); say(p, *rng.choice([["picked", "up"], ["got"]]), "the", o, ".")
                held[o] = p; obj_loc[o] = loc[p]
        elif r < 0.82:
            mine = [o for o, h in held.items() if h == p]
            if mine:
                o = rng.choice(mine); say(p, "dropped", "the", o, "."); del held[o]; obj_loc[o] = loc[p]
        else:
            mine = [o for o, h in held.items() if h == p]
            others = [q for q in people if q != p and loc.get(q) == loc[p]]
            if mine and others:
                o = rng.choice(mine); q = rng.choice(others)
                say(p, "gave", "the", o, "to", q, "."); held[o] = q
        # ask a question now and then
        if rng.random() < 0.35 and loc:
            kinds = ["where-person"] + (["where-object"] if obj_loc else []) + (["who-has"] if held else [])
            k = rng.choice(kinds)
            if k == "where-person":
                q = rng.choice(list(loc)); say("where", "is", q, "?"); a = loc[q]
            elif k == "where-object":
                o = rng.choice(list(obj_loc)); say("where", "is", "the", o, "?")
                a = loc[held[o]] if o in held else obj_loc[o]
            else:
                o = rng.choice(list(held)); say("who", "has", "the", o, "?"); a = held[o]
            answers.append((len(toks), k)); say(a, ".")
import array
array.array('I', toks).tofile(open(out + ".ids", "wb"))
open(out + ".txt", "w").write("\n".join(vocab) + "\n")
json.dump({"F": F, "answers": answers, "names": [ix[w] for w in names], "places": [ix[w] for w in places],
           "objects": [ix[w] for w in objects]}, open(out + ".info.json", "w"))
print(f"{n_st} stories, {len(toks)} words, {len(answers)} questions, vocab {len(vocab)} (function words {F})")
