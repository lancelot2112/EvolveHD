# 50 · Relation merging on book text (2026-10-26)

*[All experiments](README.md)* · [← 49](49-learned-relation-merging.md)

**The question:** experiment 49's learned relation merging (condensed cards next to exact facts) did nothing on plain bAbI, where every relation recurs anyway. Does it pay on book text, where exact keys run out?

**Result: no.**
- **Full qa2** (three seeds): with merging, 0k / 4k means 81.3 / 42.3%, against 86.3 / 44.0% without (experiment 48).
- **Long BABILong:** with merging, 16k 91.1% and 64k 95.8% on average, against 93.3% / 87.5% without (experiment 46's check).
- Both are within the seed spread.
- **The idea behind the test was wrong for these benchmarks.**
  - Merges are learned only from evidence at answers.
  - The facts asked about are bAbI sentences; the book text is distractor.
  - So the only relations that ever merge are the bAbI ones (the movement verbs, the object-handling verbs), never book phrases.
  - The 4k gap is not about unknown relations. It is about the distractor text getting in the way.

## Runs

Binary `sit-49e`; stack k90 + length-weighted judge + act path (`--sit-act`), with `--sit-rel-merge`; full qa2 with chains, the budget and the cap; long BABILong with the budget and the cap. Script `runs/exp50/run.sh`. The baselines without merging are experiment 48's (full qa2, binary `sit-47f`) and experiment 46's check (long, `sit-46a`). Seed 0 of the long baseline, rerun on `sit-49e` with merging off, reproduces it exactly (100% / 100%).

| Test | Seed 0 | Seed 1 | Seed 2 | Mean | Without merging (mean) |
|---|---|---|---|---|---|
| Full qa2 0k | 96% | 88% | 60% | 81.3% | 86.3% |
| Full qa2 4k | 68% | 27% | 32% | 42.3% | 44.0% |
| Long 16k (n = 30) | 86.7% | 90.0% | 96.7% | 91.1% | 93.3% |
| Long 64k (n = 8) | 8 of 8 | 8 of 8 | 7 of 8 | 95.8% | 87.5% |

**What merged:**
- full qa2: the object-handling verbs ([picked up / took / grabbed / got / left / dropped / discarded / put down the], one or two groups);
- long BABILong: the movement verbs ([journeyed / travelled / moved / went to the]), on every seed.

- **No book phrase ever merged.** Book text takes no part in answers, so it gives the merge rule no evidence.
- **Seed 2's 60% at 0k on full qa2 is the lowest plain-text score of any stack so far.** Training was normal (0k 83.6%, 1k 88.0%), so the router settled badly in that seed. This is the instability the next step should look at.

## What this means

- **Learned merging stays as an option.** It finds the right relations from nothing, does no harm when it sits next to the exact facts, and gains nothing on these benchmarks.
- **The book-text gap needs a different explanation.**
  - The hand-banned stack's 97 / 65% (experiment 41) was one seed. The candidates are what distractor text does to bindings, to the judge's stops (it crossed "and" and "of" in book text, experiment 42) and to the router.
  - A three-seed hand-ban run would show whether that gap is real.
- **The seed instability is now the main obstacle to telling anything apart:** 0k on full qa2 ranges from 60% to 96% for one stack.

## Next

1. Three seeds of the hand-banned stack on full qa2, to see whether the 65% at 4k is real.
2. Why the router settles badly in some seeds: compare seed 0 and seed 2's route weights on the same answers.
