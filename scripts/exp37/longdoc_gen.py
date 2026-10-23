"""Long documents with dependencies at controlled distances (experiment 37): a test for a reader of arbitrary length.

A book is chapters of scenes. Scenes are bAbI-like ("mary went to the kitchen .", "john got the apple .") with local
questions, so local reading keeps being needed. Over the whole book run three kinds of long-range fact:
  home    "X lives in the Y ."   stated once per person, sometimes restated later with a new home (newest wins)
  owner   "X owns the Z ."       one owner per object, sometimes passed to someone else later (newest wins)
  found   "X found the Z ."      inside chapter N; asked later as "what did X find in chapter N ?" (addressed by
                                 the book's structure: the reader must find chapter N, then the event in it)
Long-range questions: "where does X live ?", "who owns the Z ?", "what did X find in chapter N ?". Each picks its
fact so that the distance (words from the fact's latest statement to the answer) is spread evenly on a log scale:
a kind is drawn uniformly, then a log2 bucket uniformly among those that have a fact of that kind, then a fact.
Local questions: "where is X ?" (the person's place in this scene), "who has the Z ?", "where is the Z ?".

Output: PREFIX.ids (u32 little-endian word ids), PREFIX.txt (vocabulary, one word per line), PREFIX.info.json with
"answers": [[position of the answer word, kind], ...] (kinds: where-person, who-has, where-object, home, owner,
found) and "dist": the distance in words for each answer (-1 for local questions), plus the parameters.
Control (5th argument "shuffle"): each chapter's heading prints a permuted chapter number while the questions keep
the true one, so a reader that answers by finding the heading should fall to about chance on found; the rest of
the book is the same as without it (the permutation has its own random generator).
Nested structure ("parts=P", with "chapters=C", default 20): the book is P parts ("part N ."), each of C chapters
numbered from one again ("chapter M ."), and found is asked as "what did X find in chapter M of part N ?": the
chapter heading recurs in every part, so only the part's unit tells them apart (structure of structure).
Read, then ask ("noq", with "qend=N"): no questions inside the book at all, then N long-range questions at its end
(kinds and log-distance buckets drawn as above, each fact asked at most once), so no answer is in the text before
it is asked (no echoes) and a reader can be scored on questions it has never seen answered.
A new vocabulary ("fresh"): the same structure with new words for every person, place and object and for the
heading words ("chapter" becomes "section", "part" becomes "volume"). The new words are appended to the
vocabulary, so the old ids are unchanged and a book in the old words can be read before it.
A ladder toward natural text, one factor at a time (the story itself is unchanged: these choices draw on a random
generator of their own):
  pron=P  a statement whose subject is the previous statement's subject in the same scene says "he" or "she"
          instead, with probability P (coreference)
  mod=P   a modifier after the subject ("mary finally found the lamp"), with probability P
  syn=P   the statement's verb said with a synonym the questions never use ("discovered", "possesses", "resides"),
          with probability P (paraphrase between question and text)
usage: longdoc_gen.py WORDS OUT_PREFIX [seed] [shuffle] [parts=P] [chapters=C] [noq] [qend=N] [fresh]
       [pron=P] [mod=P] [syn=P]"""
import sys, json, math, random

n_words = int(sys.argv[1]); out = sys.argv[2]; rng = random.Random(int(sys.argv[3]) if len(sys.argv) > 3 else 1)
extra = sys.argv[4:]
shuffle = "shuffle" in extra
n_parts = next((int(x.split("=")[1]) for x in extra if x.startswith("parts=")), 0)
per_part = next((int(x.split("=")[1]) for x in extra if x.startswith("chapters=")), 20)
noq = "noq" in extra
qend = next((int(x.split("=")[1]) for x in extra if x.startswith("qend=")), 0)
names = ("mary john sandra daniel emma fred bill julie anna peter lucy oscar nina victor clara hugo "
         "rosa felix irene marco tess omar ella sam").split()
places = "kitchen garden office bathroom hallway bedroom cellar attic library garage porch studio".split()
objects = "apple football milk book key pen hat cup lamp ring coin map".split()
units = "one two three four five six seven eight nine".split()
teens = "ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen".split()
tens = "twenty thirty forty fifty sixty seventy eighty ninety".split()
func = ("chapter scene . , ? to the where is who has what place in went moved picked up got dropped gave lives "
        "owns found does live own did find").split()
vocab = func + names + places + objects + units + teens + tens + ["part", "of"]   # new words last: old ids unchanged
H_CH, H_PT = "chapter", "part"
fx = lambda k: next((float(x.split("=")[1]) for x in extra if x.startswith(k + "=")), 0.0)
p_pron, p_mod, p_syn = fx("pron"), fx("mod"), fx("syn")
lrng = random.Random(9173)                       # the ladder's own choices; the story's generator is untouched
SHE = set("mary sandra emma julie anna lucy nina clara rosa irene tess ella".split())
MODS = "finally quickly then later also".split()
SYN = {"found": ["discovered"], "owns": ["possesses"], "lives": ["resides"]}
if "fresh" in extra:
    names = ("alba bruno cora dante elsa fabio greta hans ines jonas kira lars mila nico olga pavel "
             "quinn rita stan tara ugo vera walt yara").split()
    places = "pantry orchard workshop laundry corridor nursery vault loft gallery shed balcony forge".split()
    objects = "pear kite tea scarf whistle brush cap bowl candle chain badge globe".split()
    H_CH, H_PT = "section", "volume"
    vocab = vocab + names + places + objects + [H_CH, H_PT]
    SHE = set("alba cora elsa greta ines kira mila olga quinn rita tara vera yara".split())
extra_words = (["he", "she"] if p_pron else []) + (MODS if p_mod else []) + (sum(SYN.values(), []) if p_syn else [])
vocab = vocab + extra_words
ix = {w: i for i, w in enumerate(vocab)}
assert len(ix) == len(vocab), "duplicate word in the vocabulary"

def number_words(k):
    """1..99 as words: 'seven', 'fourteen', 'thirty two'"""
    if k < 10: return [units[k - 1]]
    if k < 20: return [teens[k - 10]]
    t, u = divmod(k, 10)
    return [tens[t - 2]] + ([units[u - 1]] if u else [])

n_chapters = 99 if n_parts == 0 else n_parts * per_part
assert n_parts <= 99 and per_part <= 99
perm = list(range(1, (n_chapters if n_parts == 0 else per_part) + 1))
if shuffle:
    prng = random.Random(4242)
    while True:                                   # a permutation that moves every chapter
        prng.shuffle(perm)
        if all(perm[i] != i + 1 for i in range(len(perm))): break
chapter_words = max(400, n_words // n_chapters)
toks, answers, dist = [], [], []
def say(*ws): toks.extend(ix[w] for w in ws)
last_subj = [None]
def st(p, verb, *rest):
    """a statement by p: the subject (or a pronoun for it), maybe a modifier, the verb (or a synonym), the rest;
    returns the position of the subject word"""
    at = len(toks)
    subj = p
    if p_pron and p == last_subj[0] and lrng.random() < p_pron:
        subj = "she" if p in SHE else "he"
    words = [subj]
    if p_mod and lrng.random() < p_mod:
        words.append(lrng.choice(MODS))
    if p_syn and verb in SYN and lrng.random() < p_syn:
        verb = lrng.choice(SYN[verb])
    say(*words, verb, *rest)
    last_subj[0] = p
    return at
def ask(kind, fact_pos, *ws_and_answer):
    """the question words, then the answer word (recorded with its distance from the fact, -1 for local)"""
    *q, a = ws_and_answer
    say(*q); answers.append([len(toks), kind]); dist.append(len(toks) - fact_pos if fact_pos >= 0 else -1); say(a)

home = {}      # person -> (place, position of the latest statement)
owner = {}     # object -> (person, position)
found = {}     # (person, chapter) -> (object, position)
# when each person's home and each object's owner is first stated: spread over the first 60% of the book
first_home = {p: rng.randrange(0, int(0.6 * n_words)) for p in names}
first_owner = {o: rng.randrange(0, int(0.6 * n_words)) for o in objects}

asked = set()   # with qend: the facts already asked at the end
def long_question(once=False):
    """a long-range question whose fact's distance is spread evenly over log2 buckets (once: each fact at most once)"""
    pos = len(toks)
    cands = [("home", p, pl, pos - t) for p, (pl, t) in home.items()]
    cands += [("owner", o, pr, pos - t) for o, (pr, t) in owner.items()]
    cands += [("found", pc, ob, pos - t) for pc, (ob, t) in found.items()]
    cands = [c for c in cands if c[3] >= 8 and not (once and (c[0], c[1]) in asked)]
    if not cands: return
    kinds = sorted({c[0] for c in cands})
    kd = rng.choice(kinds)                      # a kind first, so no kind crowds out the others
    buckets = {}
    for c in cands:
        if c[0] == kd: buckets.setdefault(int(math.log2(c[3])), []).append(c)
    kind, key, ans, _ = rng.choice(buckets[rng.choice(sorted(buckets))])
    asked.add((kind, key))
    if kind == "home":
        ask("home", home[key][1], "where", "does", key, "live", "?", ans)
    elif kind == "owner":
        ask("owner", owner[key][1], "who", "owns", "the", key, "?", ans)
    else:
        p, pt, ch = key
        if pt == 0:
            ask("found", found[key][1], "what", "did", p, "find", "in", H_CH, *number_words(ch), "?", ans)
        else:
            ask("found", found[key][1], "what", "did", p, "find", "in", H_CH, *number_words(ch), "of", H_PT,
                *number_words(pt), "?", ans)
    say(".")

units_list = [(0, c) for c in range(1, n_chapters + 1)] if n_parts == 0 else \
    [(pt, c) for pt in range(1, n_parts + 1) for c in range(1, per_part + 1)]
for pt, ch in units_list:
    if pt and ch == 1:
        say(H_PT, *number_words(pt), ".")
    say(H_CH, *number_words(perm[ch - 1]), ".")
    start = len(toks)
    while len(toks) - start < chapter_words:
        say("scene", "."); last_subj[0] = None
        people = rng.sample(names, 4); things = rng.sample(objects, 3)
        loc, held = {}, {}
        for _step in range(rng.randint(8, 14)):
            r = rng.random(); p = rng.choice(people)
            if r < 0.45 or p not in loc:
                pl = rng.choice([x for x in places if x != loc.get(p)])
                st(p, rng.choice(["went", "moved"]), "to", "the", pl, "."); loc[p] = pl
            elif r < 0.62:
                free = [o for o in things if o not in held]
                if free:
                    o = rng.choice(free); st(p, *rng.choice([["picked", "up"], ["got"]]), "the", o, "."); held[o] = p
            elif r < 0.72:
                mine = [o for o, h in held.items() if h == p]
                if mine:
                    o = rng.choice(mine); st(p, "dropped", "the", o, "."); del held[o]
            elif r < 0.80:
                mine = [o for o, h in held.items() if h == p]
                others = [q for q in people if q != p and loc.get(q) == loc[p]]
                if mine and others:
                    o = rng.choice(mine); q = rng.choice(others); st(p, "gave", "the", o, "to", q, "."); held[o] = q
            elif r < 0.84 and (p, pt, ch) not in found:
                o = rng.choice(objects); st(p, "found", "the", o, "."); found[(p, pt, ch)] = (o, len(toks) - 2)
            # long-range facts: first statements when due; rare restatements with a new value
            pos = len(toks)
            for q in people:
                if q not in home and pos >= first_home[q]:
                    pl = rng.choice(places); st(q, "lives", "in", "the", pl, "."); home[q] = (pl, len(toks) - 2)
                elif q in home and rng.random() < 0.0008:
                    pl = rng.choice([x for x in places if x != home[q][0]])
                    st(q, "lives", "in", "the", pl, "."); home[q] = (pl, len(toks) - 2)
            for o in things:
                if o not in owner and pos >= first_owner[o]:
                    pr = rng.choice(names); owner[o] = (pr, st(pr, "owns", "the", o, "."))
                elif o in owner and rng.random() < 0.0008:
                    pr = rng.choice([x for x in names if x != owner[o][0]])
                    owner[o] = (pr, st(pr, "owns", "the", o, "."))
            # questions: local ones as in the stories; a long-range one now and then
            u = rng.random()
            if noq:
                pass
            elif u < 0.25 and loc:
                q = rng.choice(list(loc)); ask("where-person", -1, "where", "is", q, "?", loc[q]); say(".")
            elif u < 0.32 and held:
                o = rng.choice(list(held)); ask("who-has", -1, "who", "has", "the", o, "?", held[o]); say(".")
            elif u < 0.36 and held:
                o = rng.choice(list(held))
                if held[o] in loc:
                    ask("where-object", -1, "where", "is", "the", o, "?", loc[held[o]]); say(".")
            elif u < 0.46:
                long_question()

book_end = len(toks)
for _ in range(qend):
    long_question(once=True)
import numpy as np
np.array(toks, dtype=np.uint32).tofile(out + ".ids")
open(out + ".txt", "w").write("\n".join(vocab))
json.dump({"answers": answers, "dist": dist, "words": len(toks), "chapters": n_chapters, "chapter_words": chapter_words,
           "shuffled_headings": shuffle, "parts": n_parts, "chapters_per_part": per_part,
           "no_questions_in_text": noq, "questions_at_end": qend, "book_end": book_end, "headings": [H_CH, H_PT],
           "names": names, "places": places, "objects": objects}, open(out + ".info.json", "w"))
from collections import Counter
c = Counter(k for _, k in answers)
print(f"{len(toks)} words, {n_chapters} chapters of ~{chapter_words}, vocabulary {len(vocab)}; questions: {dict(c)}")
ld = [d for d in dist if d > 0]
print("long-range distances: " + " ".join(f"2^{b}:{n}" for b, n in sorted(Counter(int(math.log2(d)) for d in ld).items())))
