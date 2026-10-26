# 49 · Learned relation merging: the lossy step, from the overwrite rule (2026-10-25)

*[All experiments](README.md)* · [← 48](48-goal-directed-acts.md)

**The owner's question:** the glue between facts ("went to the", "journeyed to the") and the forms of questions are meant to be abstracted a little; what happened to the lossy compression step? Build it, and make it agnostic to the kind of information processed: maybe it isn't text.

**What had happened.** Experiment 35 pooled paths into relation classes, but the pooling merged "went to the" with "got the", so one fact overwrote the other (object → holder → place: 9.9% merged, 95.1% separated by hand). The rule written down then: compression may only merge what storage may let overwrite each other. Instead of fixing the merge, long-term memory went exact (heads keyed by the exact path). Classes stayed only as route features and short-term slot addresses, and on book text the hashed classes became nearly one per path (about 126,000 on the pilot).

**Results (one seed unless stated):**
- **The overwrite rule, measured at answers, finds the right relations from nothing, with no knowledge of words:**
  - story set: [moved / walked / travelled to the] and [grabbed / took / picked up / got / dropped the];
  - small qa2: [picked up / took / grabbed / got / put down / discarded / dropped / left the];
  - pilot: [journeyed / travelled / moved / went to the] (in one run).
- **It does not improve answers.** Over three seeds on the report streams, merging gives small qa2 reports 92 / 90 / 70% against 90 / 92 / 88% without it, and the story set 75.2 / 71.7 / 69.0% against 73.1 / 78.6 / 77.2%. Standard sets are unchanged (small qa2 100%, story set 97.3%, pilot 100 / 100%).
- **In noisier streams it also merges junk** (on the small-qa2 report stream: [the apple / the milk / the / the football], and the two empty paths).
- **Two fixes were needed to stop it doing harm.** Merged keys are added as a feature rather than replacing the exact key, and merging stops when the route learner freezes. Before them, a merge late in the pilot run dropped 0k to 83% (the frozen router lost its features), and small qa2 reports fell to 64%.
- **Found on the way: the clause an act starts in (the request, "report the apple") is the act's cue and must not be stored as a fact,** like the question before an answer and the teacher's reply after an act. With it, small qa2 reports are 88–92% over three seeds.

## The rule (`--sit-rel-merge`, binaries `sit-49a`–`sit-49d`)

It is agnostic: it uses only symbols, their learned kinds, write times and whether answers come out right.

- **When it looks.** At each answer w (where the act gate or detector says one is), for each cue entity e (a member of a kind with 3 or more members), e's long-term heads are grouped by the kind of their value and ranked newest first (at most 6 per group).
- **Does a value lead to the answer?** It does when it is w, or when its own newest head of w's kind holds w (one hop: object → holder → place).
- **The pair test.** For each pair of keys in a group: a win when only the newer value leads to w, a loss when only the older one does, nothing otherwise.
- **The merge.** When wins are significant ((w − l)² > 9 (w + l)) and losses are at most an eighth of wins, the two keys merge (union-find).
- **What a merged key does.** Merged keys are counted together for recurrence (a head enters long-term memory once its relation recurs), and new heads are written under the merged key. The route learner sees the merged key as an added feature next to the exact key.
- **When it stops.** Merging stops when skill freezes.

Script `runs/exp49/a.sh` (standard sets) and `r.sh` (report streams). The stack is experiment 48's.

| Run | Small qa2 reports: answer (QA kept) | Story set reports: answer, all / where-object | Standard sets: small qa2 / story / pilot |
|---|---|---|---|
| No merging, request routed, seeds 0 / 1 / 2 | 90 (98) / 92 (100) / 88% (86%) | 73.1 / 38.2; 78.6 / 64.7; 77.2 / 58.8% | — |
| Merging, request routed, seeds 0 / 1 / 2 | 92 (96) / 90 (94) / 70% (78%) | 75.2 / 47.1; 71.7 / 58.8; 69.0 / 44.1% | 100% / 97.3% / 100 / 100% (seed 0) |
| Merging replacing the exact key, no freeze stop (sit-49a) | 64% (96%) | 67.6 / 38.2% | 100% / 96.8% / 83 / 100% |

## What this means

- **The lossy step can be learned without knowing the input is text.** The overwrite rule ("the newer of the two is the one that answers") is a local statistic like the others.
- **It doesn't pay here,** for two reasons:
  - the reader already ranks the newest value of a kind across all of an entity's keys, so separate keys cost little at reading;
  - the reports' weakness is cue position, not relation keys.
- **Where it should pay is where exact keys run out:** book text, where almost every path is new and a rare verb never recurs enough to be stored. There the merged recurrence count lets "trudged to the" enter memory once it has merged with "went to the". Not tested yet.
- **The same goes for question forms:** the next step for reports is route features that don't depend on cue position.

## Next

1. Route features by role, not position (the cue word's kind and which value kind is asked), so questions and reports share routes.
2. Merging on book text (full qa2, BABILong), where exact keys don't recur.
3. Merge only keys that each recur, to keep junk keys out.
