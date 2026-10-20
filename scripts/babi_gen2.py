"""Comprehension stories with held-out phrasings (experiment 31): as babi_gen.py, but the first half of the stories
uses one set of verbs and question forms and the second half adds synonyms never seen before:
  move     first half: went, moved            second half also: walked, travelled
  take     first half: picked up, got         second half also: grabbed, took
  drop     first half: dropped                second half also: put down
  give     first half: gave X to Y            second half also: handed X to Y
  where-person question  first half: "where is X ?"   second half also: "what place is X in ?"
Each answer records whether its question used a new form and whether a fact it depends on used a new verb, so
answers can be scored separately on new phrasings.
usage: babi_gen2.py STORIES OUT_PREFIX [seed]"""
import sys, json, random
n_st = int(sys.argv[1]); out = sys.argv[2]; rng = random.Random(int(sys.argv[3]) if len(sys.argv) > 3 else 1)
names = "mary john sandra daniel emma fred bill julie".split()
places = "kitchen garden office bathroom hallway bedroom cellar attic".split()
objects = "apple football milk book key pen hat cup".split()
func = ("new story . to the ? where is who has what place in went moved walked travelled picked up got grabbed took "
        "dropped put down gave handed").split()
vocab = func + names + places + objects
ix = {w: i for i, w in enumerate(vocab)}
F = len(func)
MOVE = (["went"], ["moved"]), (["walked"], ["travelled"])
TAKE = (["picked", "up"], ["got"]), (["grabbed"], ["took"])
DROP = (["dropped"],), (["put", "down"],)
GIVE = (["gave"],), (["handed"],)
toks, answers = [], []
def say(*ws): toks.extend(ix[w] for w in ws)
for st in range(n_st):
    late = st >= n_st // 2
    def pick(forms):
        old, new = forms
        if late and rng.random() < 0.5: return rng.choice(new), True
        return rng.choice(old), False
    say("new", "story", ".")
    people = rng.sample(names, 4); things = rng.sample(objects, 3)
    loc = {}; held = {}; obj_loc = {}
    newp = {}; newo = {}          # whether the latest fact about a person / object used a new verb
    for _step in range(rng.randint(8, 14)):
        r = rng.random(); p = rng.choice(people)
        if r < 0.5 or p not in loc:
            pl = rng.choice([x for x in places if x != loc.get(p)])
            v, nv = pick(MOVE); say(p, *v, "to", "the", pl, "."); loc[p] = pl; newp[p] = nv
            for o, h in held.items():
                if h == p: obj_loc[o] = pl; newo[o] = newo.get(o, False) or nv
        elif r < 0.7:
            free = [o for o in things if o not in held]
            if free:
                o = rng.choice(free); v, nv = pick(TAKE); say(p, *v, "the", o, ".")
                held[o] = p; obj_loc[o] = loc[p]; newo[o] = nv
        elif r < 0.82:
            mine = [o for o, h in held.items() if h == p]
            if mine:
                o = rng.choice(mine); v, nv = pick(DROP); say(p, *v, "the", o, "."); del held[o]
                obj_loc[o] = loc[p]; newo[o] = nv
        else:
            mine = [o for o, h in held.items() if h == p]
            others = [q for q in people if q != p and loc.get(q) == loc[p]]
            if mine and others:
                o = rng.choice(mine); q = rng.choice(others); v, nv = pick(GIVE)
                say(p, *v, "the", o, "to", q, "."); held[o] = q; newo[o] = nv
        if rng.random() < 0.35 and loc:
            kinds = ["where-person"] + (["where-object"] if obj_loc else []) + (["who-has"] if held else [])
            k = rng.choice(kinds); newq = False
            if k == "where-person":
                q = rng.choice(list(loc)); a = loc[q]; nf = newp.get(q, False)
                if late and rng.random() < 0.5: say("what", "place", "is", q, "in", "?"); newq = True
                else: say("where", "is", q, "?")
            elif k == "where-object":
                o = rng.choice(list(obj_loc)); say("where", "is", "the", o, "?")
                a = loc[held[o]] if o in held else obj_loc[o]; nf = newo.get(o, False)
            else:
                o = rng.choice(list(held)); say("who", "has", "the", o, "?"); a = held[o]; nf = newo.get(o, False)
            answers.append((len(toks), k, newq, nf)); say(a, ".")
import array
array.array('I', toks).tofile(open(out + ".ids", "wb"))
open(out + ".txt", "w").write("\n".join(vocab) + "\n")
json.dump({"F": F, "answers": [(p, k) for p, k, _, _ in answers], "newq": [p for p, _, q, _ in answers if q],
           "newfact": [p for p, _, _, f in answers if f], "names": [ix[w] for w in names],
           "places": [ix[w] for w in places], "objects": [ix[w] for w in objects]}, open(out + ".info.json", "w"))
print(f"{n_st} stories, {len(toks)} words, {len(answers)} questions ({sum(1 for a in answers if a[2])} with a new "
      f"question form, {sum(1 for a in answers if a[3])} whose fact used a new verb), vocab {len(vocab)} (function words {F})")
