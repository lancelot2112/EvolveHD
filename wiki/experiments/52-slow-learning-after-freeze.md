# 52 · Slow learning after the freeze (2026-10-26)

*[All experiments](README.md)* · [← 51](51-why-seeds-differ.md) · [53 →](53-never-stop-learning.md)

**The question:** experiment 51 showed that a frozen route learner locks in its last training phase. The bad seed recovered within about 20 questions when learning stayed on, but learning on also cost the good seed some accuracy. Does slow learning after the freeze keep the recovery without the drift?

**Results (full qa2, three seeds, the merge stack of experiment 50):**
- **Learning at 1 word in 4 after the freeze closes the book-text gap and removes most of the seed spread:**
  - 4k: 62 / 69 / 69%, mean 66.7%, against 68 / 27 / 32% frozen (mean 42.3%);
  - 0k: 93 / 90 / 80%, mean 87.7%, against 96 / 88 / 60% frozen (mean 81.3%).
  - The 4k mean now matches the old hand-banned stack's single-seed 65%.
- **Fully on is about the same** (0k 88.7%, 4k 65.3%, spread 3 points at 4k). So 1 in 4 is not slower in any way that matters here.
- **1 in 16 is too slow for book text:** 0k 88.7%, but 4k 54 / 34 / 16%.
- **Every run is still scored fairly.** Each answer is scored before the reader learns from it (prequential); the test answers are never seen ahead.

## The rule (`--sit-slow-after N`, binary `sit-52a`)

- From `--sit-freeze-from` on, the route learner still learns, at 1 word in N: an integer rate, since weights step by 1, so learning less often is learning slower.
- 0 keeps the old behaviour (frozen).
- Everything else frozen at the freeze stays frozen: the act gate, the merge judge and the detector.

Stack: k90 + length-weighted judge + act path + relation merging, full qa2 with chains, the budget and the cap. Script `runs/exp52/run.sh`. Fully-on runs: seeds 0 and 2 from experiment 51 (binary `sit-51c`), seed 1 here.

| Router after the freeze | 0k, seeds 0 / 1 / 2 | Mean | 4k, seeds 0 / 1 / 2 | Mean |
|---|---|---|---|---|
| Frozen (exp 50) | 96 / 88 / 60% | 81.3% | 68 / 27 / 32% | 42.3% |
| 1 word in 16 | 95 / 90 / 81% | 88.7% | 54 / 34 / 16% | 34.7% |
| **1 word in 4** | **93 / 90 / 80%** | **87.7%** | **62 / 69 / 69%** | **66.7%** |
| Always (fully on) | 90 / 87 / 89% | 88.7% | 66 / 64 / 66% | 65.3% |

Training scores are identical across rows (the freeze is at the end of training).

## What this means

- **Most of the book-text gap was the frozen router, not memory or segmentation.** The 4k test comes after the plain 0k test. A router frozen at the end of training has to carry its last tuning through both. Learning on, it re-tunes to each, and 4k goes from 42% to 66%.
- **The seed lottery is mostly gone with learning on** (4k range 62–69% against 27–68%). The seeds still differ at 0k (80–93%), less than frozen (60–96%).
- **The project rule "skill learned once, then frozen" is what costs us.** The evidence favours a router that keeps learning, at a rate fast enough to follow a change in the material (1 in 4 works, 1 in 16 doesn't for 4k).
- **Not tested yet:** long BABILong, where answer-only reading means the router sees only answers after the freeze, so it has far fewer chances to learn; and the story set and pilot.

## Next

1. Learning on (1 in 4) on long BABILong and the pilot, three seeds.
2. The same with merging off, to see whether merging still matters once the router keeps learning.
3. Decide with the owner whether "skill keeps learning slowly" replaces "skill frozen" as the project rule.
