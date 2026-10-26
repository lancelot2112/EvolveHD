# 48 · The act path, stage 4: goal-directed acts (report requests), and full qa2 over seeds (2026-10-25)

*[All experiments](README.md)* · [← 47](47-hearing-itself.md) · [49 →](49-learned-relation-merging.md)

Two things, in the order proposed after experiment 47:
1. three seeds of k90 + the act path on full qa2 (the long test was very seed-sensitive in experiment 46's check);
2. stage 4 of the act path: a goal-directed act that combines a sentence frame, a word copied from the request, and a fact retrieved from memory.

**Results:**
- **Full qa2 over three seeds: the act path neither helps nor hurts.** k90 + act path averages 86.3 / 44.0% (0k / 4k), against 84.0 / 47.0% for k90 + judge. Across seeds the scores range over up to 10 points at 0k and up to 20 at 4k, more than either difference. Both stay well below the hand ban's 97 / 65% (experiment 41, one seed).
- **The reader learns to answer a request with a sentence:**
  - The world says "report emma"; the reader says "emma is in the garden .", word by word, hearing itself.
  - The answer word inside the act is right 82–90% of the time on small qa2 and 80–84% on the story set (where-person), over three seeds.
  - Whole sentences: 82–88% and 77–78%.
- **The retrieval inside the act depends on hearing itself.** Deaf to its own words, the answer word is right 0% of the time: the cue that finds the fact is the reader's own "emma is in the".
- **The teacher's sentence after an act must be routed to the act path, not stored as a fact.** Stored as a fact, "the apple is in the bedroom" becomes a stale shortcut and QA on the remaining questions falls (small qa2 86 / 86 / 94% over three seeds). Routed to the act path, QA is 100 / 100 / 100%.

## Report requests (`scripts/exp48/report_stream.py`)

**The stream.** Every other "where is NP ? ANS ." becomes:
1. "report NP" (the world's request);
2. "NP is in the ANS ." as self slots (the reader says it and hears its own words; each slot is credited against the teacher's word);
3. "NP is in the ANS ." said by the world (the teacher).

"report" is a new word (and "in", on small qa2). The other questions stay questions.
- Story set: 24,659 → 28,071 words, 406 reports, 791 questions kept.
- Small qa2: 112,790 → 118,290 words, 550 reports, 550 questions kept.

**What the act needs:**
- the frame "is in the", learned from the teacher's sentences (replay and the bigram);
- NP, copied from the request (replay);
- ANS, a fact retrieved from memory, with the reader's own words as the cue.

The answer is the only word that neither the bigram nor replay can give.

**The rule added, `--sit-teacher-route`** (binary `sit-48a`): the world's sentence right after an act (the teacher saying what the reader should have said) is feedback for the act path. Its clause is not committed to fact memory. The stack is experiment 47's (k90 + length-weighted judge + act path + replay). Script `runs/exp48/r.sh`, scorer `scripts/exp48/score_report.py`.

**Test half (small qa2: the test stories; story set: the second half):**

| Run | Small qa2: answer in act / whole sentence / QA kept | Story set where-person: answer / whole | Story set where-object: answer / whole | Story set QA kept (where-person) |
|---|---|---|---|---|
| Teacher stored as fact, seed 0 | 66 / 66 / 86% | 85.6 / 83.8% | 70.6 / 58.8% | 98.1% |
| … seeds 1, 2 | 98 / 98 / 86%; 80 / 78 / 94% | — | — | — |
| **Teacher routed to the act path, seed 0** | **90 / 88 / 100%** | 83.8 / 76.6% | 52.9 / 44.1% | 97.8% |
| … seed 1 | 82 / 82 / 100% | 80.2 / 77.5% | 47.1 / 35.3% | 95.1% |
| … seed 2 | 88 / 88 / 100% | 82.9 / 77.5% | 58.8 / 47.1% | 96.6% |
| Deaf to itself (teacher stored), seed 0 | — | 0 / 0% | 0 / 0% | 97.2% |

n = 50 reports on small qa2, 111 (where-person) and 34 (where-object) on the story set.

- **The frame is learned almost perfectly on small qa2** (99.0–99.3% of the other words). The misses are the answer: memory proposes another place for the person or object.
- **Where-object reports are hard** (47–59% with routing). The fact needs a chain (object → holder → place), and the act's cue order ("the milk is in the") differs from a question's, so the routes learned on questions transfer only partly. There are 48 training reports for where-object.
- **On the story set, routing the teacher's sentence costs the reports some accuracy** (seed 0 where-object 70.6% → 52.9%) and leaves where-person QA unchanged (98.1% → 97.8%; where-object 85.1% → 78.7%, who-has 90.0% → 85.0%, small n). On small qa2 it is what keeps QA at 100%. On the story set, the stored teacher sentence may be giving the next report about the same object its answer directly.

**Examples of misses** (story set, teacher stored, seed 0):

| Truth | Said |
|---|---|
| emma is in the bathroom . | emma is in the bedroom . |
| the hat is in the office . | the hat is in ? office . |
| emma is in the bathroom . | in the milk . julie is |

## Full qa2 over three seeds

Same binary (`sit-47f`), same stacks as experiment 42's full qa2 runs with the cap; script `runs/exp48/seeds.sh`. Test 0k / 4k (n = 100 each):

| Stack | Seed 0 | Seed 1 | Seed 2 | Mean |
|---|---|---|---|---|
| k90 + judge (mood detector, question drop) | 87 / 34% | 79 / 53% | 86 / 54% | 84.0 / 47.0% |
| **k90 + judge + act path** | 87 / 54% | 91 / 38% | 81 / 40% | **86.3 / 44.0%** |

- **No difference beyond the seed spread.** The act path gains 2.3 points at 0k and loses 3.0 at 4k.
- **Training scores match** (0k 83.5–89.8%, 1k 85.0–89.0%).
- **The 4k book-text test is the weak spot for both,** and experiment 42's single-seed 87 / 34% was a low draw for 4k.
- **The act path is kept:** equal on full qa2, better at 16k on long BABILong (experiment 46), and the only stack that doesn't need "?" in L.

## What this means

- **Stage 4's first act works:** a request is answered with a composed sentence whose content comes from memory, cued by the reader's own words.
- **Routing applies on both sides of an act.** The clause before it is the act's cue (experiment 46), and the teacher's reply after it is the act's feedback. Neither is a fact about the world.
- **The weak part is retrieval with an unfamiliar cue order** (where-object reports). The route learner keys routes by cue position, so a fact asked in a new wording needs its own training.

## Next

1. Make route features less tied to cue position (the cue word's kind and role rather than its index), so questions and reports share routes.
2. Reports on BABILong (qa1 with book text), with the act gate deciding where requests are.
3. Continuations as acts: after "and then", say the next event.
