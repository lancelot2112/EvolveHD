# 56 · Instant kinds, and route features by kind (2026-10-27)

*[All experiments](README.md)* · [← 55](55-generalization-probes.md)

**The owner's request:** make the router's features per group, and add instant grouping. Experiment 55 found that new names fail to join the person kind when their company is new too (new names with a new verb: small qa2 70.3%). On the story set, new names cost "who has" answers 29 points.

**Results (three seeds, the default never-frozen stack):**
- **Instant grouping closes most of the generalization gap.** A young word takes the kind of the words that filled the same places before.

  | Probe | Before (exp 55) | Instant kinds |
  |---|---|---|
  | Small qa2, new names + new verb | 70.3% | 95.7% |
  | Small qa2, all four changes | 75.7% | 97.0% |
  | Story set, new names | 89.8% (who-has 65%) | 96.5% (who-has 94%) |
  | Story set, new names + new verb | 87.6% (who-has 60%) | 96.2% (who-has 92%) |
  | Story set, all four changes | 91.0% (who-has 70%) | 95.5% (who-has 88%) |

  - On the first 25 probed questions of small qa2: 64–83% before, 97–99% now.
- **Route features by kind add little on top.** The router already worked mostly through kinds, and the gap was that new words never got into the right kind.
  - Naming the question's words by kind, with instant kinds, scored within about a point of instant kinds alone on most probes. It was 3 points better on small qa2's new verb and 1.3 points worse on the story set's all-four probe.
  - Alone (without instant kinds) it did nothing: small qa2 names + verb 71% on seed 0.
  - With only kinds in the query (no frame codes) the unchanged story set fell to 91.5% (seed 0).
- **No cost on the unchanged streams, and none on full qa2:**
  - small qa2 99.0% (was 97.0%); story set 96.8% (was 97.2%);
  - full qa2 0k / 4k 91.7 / 68.0% (was 88.7 / 67.7%).
- **Long BABILong and the pilot are mixed:**
  - long 16k 88.9% (was 94.4%: 80 against 85 of 90 questions over three seeds); 64k 95.8% (was 87.5%: 23 against 21 of 24);
  - pilot 0k / 4k 98.7 / 86.0% (was 99.3 / 86.7%), with the seeds trading places (4k 74 / 94 / 90% against 100 / 100 / 60%).

## The rules (binary `sit-56a`)

**`--sit-kind-instant`:**
- **The places:** at each word, the word two back (x) has two neighbours on each side. Five places are pairs of them:
  - (before, after);
  - (before, two after);
  - (two before, after);
  - (after, two after);
  - (two before, before).
- **Learning the places:** an established word (32 occurrences or more) teaches its places its kind. Each place keeps counts per kind, halved together past 1,024.
- **The vote:** an occurrence of x votes for a kind when the kind holds 3/4 or more of at least 8 counts in each of two or more of x's places.
  - One place is not enough. "kitchen ." is followed by names and by "where", so the place (two before, before) alone would make the new question word "in" a name.
- **At once:** a young word (under 32 occurrences) with no kind yet, or in a group of fewer than 3, takes the kind voted for, if that kind has 3 or more members.
- **After each reclustering:** a word in a group of fewer than 3 takes a clear vote (3/4 of at least 2 votes) for a content kind (3 or more members).
- **`--sit-kind-instant-move`:** after reclustering, a word with a clear vote also leaves a group of 3 or more for a larger one it is voted into.
  - Without it, two new names stayed in a group of three with "where" (small qa2 names + verb, seed 0: 88% instead of 96%).

**`--sit-q-kind N`:** the route learner's query code names each cue word by its kind.
- 1: its 4 identity bits and the question-form bits (the conjunctions of the cue words 1 and 3, and 1 and 4, back) are hashed from its kind.
- 2: its frame code is replaced by 8 kind bits too, so the query holds kinds only.
- Either way, the question form used elsewhere (`qform`) is computed from kinds.

The places are counted on word identities. A young word's kind only needs its neighbours to be familiar, not itself: "alice wandered to the" votes for the person kind through (before, two after) = (".", "to"), even when "wandered" is new too.

## Probes (three seeds)

Script `runs/exp56/p.sh` (tag, extra flags), grid `runs/exp56/grid.sh`, summary `scripts/exp56/summ.py`. Probe streams from experiment 55, plus the story set's names + verb probe. Columns: before / instant kinds (`--sit-kind-instant --sit-kind-instant-move`) / instant kinds + `--sit-q-kind 1`.

**Small qa2** (100 probed questions):

| Probe | Before | Instant | Instant + kind query | Instant, seeds 0 / 1 / 2 |
|---|---|---|---|---|
| unchanged | 97.0% | 99.0% | 98.7% | 99 / 100 / 98% |
| names | 93.7% | 98.0% | 99.0% | 99 / 98 / 97% |
| verb | 98.3% | 96.0% | 99.3% | 99 / 94 / 95% |
| names + verb | 70.3% | 95.7% | 96.7% | 96 / 98 / 93% |
| question | 99.7% | 97.3% | 99.0% | 98 / 97 / 97% |
| longer | 96.0% | 96.3% | 99.0% | 96 / 96 / 97% |
| all | 75.7% | 97.0% | 95.7% | 94 / 100 / 97% |

**Story set** (590 probed questions; who-has in brackets):

| Probe | Before | Instant | Instant + kind query |
|---|---|---|---|
| unchanged | 97.2% (94%) | 96.8% (92%) | 96.2% (92%) |
| names | 89.8% (65%) | 96.5% (94%) | 96.7% (92%) |
| verb | 95.5% (86%) | 95.7% (89%) | 96.4% (86%) |
| names + verb | 87.6% (60%) | 96.2% (92%) | 97.0% (95%) |
| question | 95.9% (89%) | 95.7% (89%) | 96.7% (91%) |
| longer | 96.6% (94%) | 96.5% (92%) | 96.6% (93%) |
| all | 91.0% (70%) | 95.5% (88%) | 94.2% (84%) |

**Bits per answer** (mixture, three seeds), before → instant:
- small qa2: names 0.50 → 0.23; names + verb 1.77 → 0.37; all 1.49 → 0.28;
- story set: names 0.87 → 0.39; names + verb 0.99 → 0.43; all 0.81 → 0.44.
- Bits over all probed words barely change (for example small qa2 names + verb 2.12 → 2.08). The kinds change the answers, not the reader's text prediction.

**The kinds at the end** (seed 0, instant kinds):
- **Small qa2, names + verb:** "wandered" joins the verbs; all four new names join mary, daniel, sandra, john (and "where", which is in that kind in the unchanged run too).
- **Small qa2, all four changes:** the same, and the new question word "in" lands in the person kind as well.
- **Story set, all four changes:** "wandered" joins the verbs. The new names form a kind of their own at the end, though the score holds (95.5%).

Instant takes per run: 1–17 at once, 35–141 after reclusterings (more where the probes bring new words).

**Seed 0, other variants** (small qa2 names + verb / all; story set names / all, who-has in brackets):

| Variant | Small qa2 | Story set |
|---|---|---|
| kind query 1 alone | 71 / 77% | 91.0 (66) / 90.3% (71) |
| kind query 2 alone | 71 / 76% | 89.0 (62) / 89.0% (72); unchanged 91.5% |
| instant, no move | 95 / 92% | 95.3 (87) / 89.8% (68) |
| instant + move + kind query 2 | 96 / 92% | 97.1 (96) / 92.4% (82); unchanged 91.5% |

## The big benchmarks (instant kinds + move, never frozen)

Script `runs/exp56/big.sh`. Before: experiment 53's final stack.

| Test | Seeds 0 / 1 / 2 | Mean | Before (exp 53) |
|---|---|---|---|
| Full qa2 0k | 93 / 89 / 93% | 91.7% | 88.7% |
| Full qa2 4k | 68 / 61 / 75% | 68.0% | 67.7% |
| Long 16k (n = 30) | 90.0 / 90.0 / 86.7% | 88.9% | 94.4% |
| Long 64k (n = 8) | 7 / 8 / 8 of 8 | 95.8% | 87.5% |
| Pilot 0k | 97 / 100 / 99% | 98.7% | 99.3% |
| Pilot 4k (n = 50) | 74 / 94 / 90% | 86.0% | 86.7% |

Training scores change too (kinds now settle differently): pilot training 1k 68.5 / 62.0 / 84.0%, against 70.5 / 88.0 / 59.5%.

## What this means

- **A new word is placed by the company it keeps, at once.** Counting a word's own contexts until it clusters takes many occurrences, and fails when its company is new as well. Voting by the places it fills, as the established words filled them, needs one or two occurrences and only half-familiar company. This is the "role from a familiar frame" experiment 55 asked for, and it is agnostic: places are pairs of neighbours, whatever the tokens are.
- **Kinds do the generalizing; the router already reads them.** Naming the router's query by kind adds little once the kinds are right. The route features were already per kind (cue kind, value kind, hop classes); the exact identities left were a small part of the query code.
- **The cost shows where places mislead:** the new question word "in" joins the person kind, because the only familiar company it has (". before it, the sentence start) is a person's. Two agreeing places keep most such mistakes out, not all.
- **Long BABILong 16k dropped 5 of 90 questions**, within the spread seen between stacks before (experiments 50–54: 88.9–94.4%), and 64k rose by 2 of 24. Not explained yet.

## Next

1. Keep `--sit-kind-instant --sit-kind-instant-move` in the default stack; `--sit-q-kind 1` optional.
2. Look at long BABILong 16k's lost questions under instant kinds (book text has many young words, and each can take a kind now).
3. Places on kinds as well as words (a place's neighbours named by their kinds), so a word can be placed when its neighbours are new but of known kinds.
