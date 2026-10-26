# 51 · Why some seeds settle badly, and credit fan-out (2026-10-26)

*[All experiments](README.md)* · [← 50](50-merging-on-book-text.md)

**The question:** with relation merging on, full qa2's plain-text test (0k) was 96% on seed 0 and 60% on seed 2, with normal training scores in both. Why? And the owner's idea: compression plus fan-out could be part of the teaching signal, to help relate concepts.

**Results:**
- **The right answer is always among the candidates** (100% at 0k and 4k, both seeds). Seed 2 fails at choosing among them, not at storing or finding the facts.
- **Both seeds answer almost every question through a two-hop chain** (object → holder → place). Seed 2 picks the wrong chain option 39% of the time at 0k (seed 0: 3%).
- **In training, seed 2's wrong chains mostly pick the wrong time** for the holder's place: "now" (where the holder is) against "then" (where they were when they dropped the object). That happens in 74 of 79 wrong chains, with the same hop-1 relation key in 68 of them. In the test, the wrong chains mostly agree with the right one on time and hop-1 recency and differ in the hop-2 option.
- **The merges differ by seed, and so do the scores:**
  - seed 0 kept the drop verbs apart ([picked up / took / grabbed / discarded / put down the] and [left / dropped the]): 96%;
  - seed 1 merged "left" with the taking verbs: 88%;
  - seed 2 merged them all, taking and dropping verbs together: 60%.
  - **Likely cause:** "got the apple" and "dropped the apple" overwrite each other as the newest event, so the overwrite rule merges them. But the reader needs exactly that difference to choose "now" or "then". Compression erased a distinction the answer depends on.
  - **Not yet confirmed:** the same seed with merging off scored 81% on an earlier binary (experiment 48), and no run varies the merge alone.
- **Credit fan-out, as built, hurts.** Crediting every candidate route that held the answer (up to 4), instead of only the best-ranked one, dropped seed 2's training from 83.6% to 67.9%, with the test about the same (63 / 41%). In a world with a handful of places, many wrong routes hold the right answer by chance, and fan-out credits them all.

## Diagnostics

**Answer-source diagnostic** (`SIT_ANSDBG`, binaries `sit-51a`/`sit-51c`). At each answer it logs the top candidate's source (slot, head, range, chain, record, replay), its hops, cue position, hop-1 recency, now/then and hop-1 key, and the same for the best-ranked candidate holding the answer. Stack: k90 + judge + act path + `--sit-rel-merge`, full qa2. Script `runs/exp51/a.sh`.

| | Seed 0 (0k 96%, 4k 68%) | Seed 2 (0k 60%, 4k 32%) |
|---|---|---|
| Answer among candidates (0k / 4k) | 100 / 100% | 100 / 100% |
| Top source at 0k | chain 93 (98% right), slot 7 (71%) | chain 99 (61% right), slot 1 |
| Top source at 4k | chain 96 (69%), slot 4 (50%) | chain 86 (37%), range 10 (0%), slot 4 (0%) |
| Training 0k: top source | chain 581 (94%), slot 290 (91%) | chain 793 (90%), slot 12 (83%) |

Seed 0 also answers a third of training questions from working-memory slots; seed 2 almost never does. The two seeds settle on different routes early and keep them.

**Credit fan-out** (`--sit-credit-fan N`, binary `sit-51c`): on a mistake, up to N distinct routes that held the answer are each credited once, and the wrong top route is blamed once.

| Seed 2 | Training 0k / 1k | Test 0k / 4k |
|---|---|---|
| Best-ranked route credited (default) | 83.6 / 88.0% | 60 / 32% |
| Up to 4 routes credited | 67.9 / 53.0% | 63 / 41% |

## What this means

- **The seed lottery is in the router's choice among stored facts.** Memory is fine; the chain options (which holder event, now or then, which hop-2 option) are ranked by weights that settle differently per seed.
- **Compression has to be judged by what reads it, not only by how facts overwrite.** The merge rule asks whether the newer value answers, using the holder's place now. For dropped objects the answer is the place then, so the rule can't see that "dropped" differs from "got".
- **The owner's direction stands, but the credit signal has to separate signal from coincidence.** Fan-out must reach the compressed features that right routes share, not every route that happens to hold the right word. In a small world, half the routes hold the right place by chance.

## Next

1. **Judge merges through the reading they feed:** a merge stays only if, at answers, the route through the merged key does as well as the exact keys (a shadow comparison, as forum round seven designed for boundaries).
2. **Fan out credit by specificity:** credit a right route in proportion to how few other candidates held the same answer, so chance holders get little.
3. **Confirm the merge cause:** seed 2 with merging off on this binary, and with the drop and take verbs kept apart.
