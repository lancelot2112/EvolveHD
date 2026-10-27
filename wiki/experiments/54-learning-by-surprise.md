# 54 · The route learner's rate by surprise (2026-10-27)

*[All experiments](README.md)* · [← 53](53-never-stop-learning.md)

**The owner's rule:** learn fast to get seeded, then slow down, never stop. Experiment 53 showed that slowing by a feature's age hurts when the material changes mid-training. So here the rate follows surprise: full rate while the router is being wrong more than usual, slow otherwise.

**Result: no better than learning at full rate for ever.**
- Over three seeds, against experiment 53's full-rate stack (everything else the same):
  - full qa2: 88.7 / 58.3%, against 88.7 / 67.7%;
  - long BABILong: 90.0% / 91.7%, against 94.4% / 87.5%;
  - small qa2: 97.0%, against 97.0%;
  - story set: 95.0%, against 97.1%;
  - pilot 4k: 86.7%, against 86.7%.
- **The slow phase costs more than it saves:**
  - the router is slow 52–64% of the time;
  - the book-padded tests (full qa2 4k, long 16k) lose 4–9 points;
  - 64k gains one question in one seed.
  - In these streams there is no long stable stretch where drift does harm.
- **Surprise has to be measured at answers, not at every word.** Measured at every word, book text makes nearly every word a surprise. The long-run rate then adapts within a couple of thousand words, and the surprise is gone before the answers need it: pilot 4k 60% on two seeds of three. Measured at answers (the level above's teacher), the pilot matches full rate.

## The rule (`--sit-learn-surprise seed,floor`, binaries `sit-54a`, `sit-54b`)

- **Seeding:** a route feature learns at full rate for its first 2^seed updates.
- **After that:** full rate while the router is surprised, one update in 2^floor otherwise.
- **Surprised:** its recent hit rate at answers (an average over about 8) is more than an eighth below its long-run rate (about 128).
- Here seed 6 and floor 2: 64 updates, then 1 in 4.
- `sit-54a` measured the hit rate at every prediction; `sit-54b` at answers only.

Stack: the length-weighted judge, the act path with the hysteresis gate, relation merging, nothing frozen (`FREEZE=999999999`). Script `runs/exp54/run.sh`.

| Test | Seed 0 | Seed 1 | Seed 2 | Mean | Full rate, never stop (exp 53) |
|---|---|---|---|---|---|
| Full qa2 0k | 92% | 87% | 87% | 88.7% | 88.7% |
| Full qa2 4k | 61% | 51% | 63% | 58.3% | **67.7%** |
| Long 16k (n = 30) | 93.3% | 90.0% | 86.7% | 90.0% | **94.4%** |
| Long 64k (n = 8) | 8 of 8 | 7 of 8 | 7 of 8 | 91.7% | 87.5% |
| Small qa2 | 97% | 96% | 98% | 97.0% | 97.0% |
| Story set | 93.6% | 97.0% | 94.5% | 95.0% | 97.1% |
| Pilot 0k / 4k (answers) | 100 / 98% | 100 / 94% | 97 / 68% | 99.0 / 86.7% | 99.3 / 86.7% |
| Pilot 0k / 4k (every word, `sit-54a`) | 99 / 100% | 98 / 60% | 96 / 60% | 97.7 / 73.3% | — |

Share of updates made while surprised: full qa2 42–48%, long BABILong 36–38%, pilot 38–41% (answers) or 24% (every word).

## What this means

- **Full-rate learning that never stops is the best rule so far** on every benchmark we have.
  - "Slow down" only pays where a learner is stable for a long time and drift does harm. These streams change too often for that: plain stories, then book-padded ones, then tests of each.
  - The one sign of harmful drift was seed 0's plain-story score falling from 96% (frozen) to 90% with learning on (experiment 51). The surprise rule didn't recover it either (92%).
- **Surprise belongs at the level that teaches,** which here means answers. It matches the principle the project already uses for credit: a level is paid for what the level below missed.
- **Seeding is not the issue.** Every feature reaches its 64 seeding updates long before the tests.

## Next

- Keep full-rate, never-frozen learning as the default stack.
- Revisit slowing when there is a long stationary stream where drift can be measured, for example the same task for millions of words.
- Next direction: the raw-bytes reader (uncompressed files first).
