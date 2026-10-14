# 16 · A better readout, and what the system can do (2026-10-13)

*[All experiments](README.md)* · [← 15](15-growing-pools.md) · [17 →](17-level-stack.md)

**Concepts:** [Writer-mask readout](../concepts/writer-mask-readout.md#readout-v2) · [Matcher pools](../concepts/matcher-pools.md#growing-pools) · [Drive and prime](../concepts/drive-and-prime.md)

Questions:
- Where do the 0.2 bits per byte go that the readout loses to count models?
- Can rows be made cheaper?
- What can the trained system do beyond scoring the next byte?

## 1. Where the readout gap was

The test bed is the hand-wired order-4 chain on two books, with exact nodes: 2.182 bits per byte on book A's last 100k, against 1.958 for order-4 counts.

| Change | Book A, last 100k |
|---|---|
| Python's readout: 8 levels × 640 bits, 8-bit writes | 2.182 |
| Template rows (new rows copy one random template) | 2.182 |
| Fixed confidence scale n / (n + T) (an earlier form of `--confidence`, since replaced) | 2.193 |
| *Rows below use template rows* | |
| 640 bits, **32-bit writes** | 2.126 |
| 1,024 bits, 8-bit writes | 2.128 |
| **1,024 bits, 16-bit writes** | **2.103** |
| 1,024 bits, 32-bit writes | 2.116 |
| 1,400 bits, 32-bit writes | 2.105 |
| 300 bits, 32-bit writes | 2.240 |
| 1,024 bits, 16-bit writes + a learned confidence gain | **2.099** |

- **About a third of the gap was mask capacity.** A level of 640 bits holds about 20 target codes. Every write replaces only 8 of the target's 32 code bits, so frequent successors crowd out rarer ones before their counts settle. Wider masks (1,024 bits) and wider writes (16 bits) recover 0.08 of the 0.22 bits per byte. Memory is unchanged (masks are 2,048-bit fields either way), and a clean timing of the conjoin run showed no extra time.
- **Confidence barely matters.** Scaling each row by its Witten-Bell confidence hurt (2.193). A second per-edge gain on confidence-scaled scores, learned like the first, gained 0.004.
- **Template rows cost nothing in accuracy on the hand-wired chain,** and at 1,024 bits they helped slightly (2.111 without, 2.103 with). On grown runs this is within seed spread and not shown. They save only 5–10% of run time, so rows were not the main cost after all.

`--readout2` sets 1,024-bit masks, 16-bit writes, template rows and the learned confidence gain. The defaults stay at the Python values, so every earlier result still reproduces.

**Grown systems with `--readout2`** (two books, seed 0):

| System | Book A, last 100k | Book B, first 50k after A | Book B, 50–100k after A | Book A whole |
|---|---|---|---|---|
| Grown, exact nodes: old readout → readout2 | 2.321 → **2.183** | 2.674 → **2.543** | 2.538 → **2.391** | 2.641 → **2.544** |
| Grown, fixed real pools | 2.621 → **2.518** | 2.914 → **2.867** | 2.836 → **2.801** | 2.831 → **2.746** |
| Growing pools + chunks + conjoin | 2.206 → **2.150** | 2.634 → **2.582** | 2.503 → **2.427** | 2.535 → **2.476** |
| Witten-Bell order 4 | 1.958 | 2.586 | 2.385 | 2.153 |

On book B's first 50k after A, grown exact nodes now beat order-4 counts (2.543 against 2.586); on its second 50k they nearly match (2.391 against 2.385).

**Three Austen novels (2M bytes), growing pools + chunks + conjoin:**

| System | *Emma* | *Persuasion* | *Sense and Sensibility* | Last 500k | Whole |
|---|---|---|---|---|---|
| Witten-Bell order 4 | 2.008 | 1.904 | 1.887 | 1.878 | 1.944 |
| Exact nodes, old readout ([13](13-word-pairs.md)) | 2.281 | 2.054 | 1.989 | 1.970 | 2.132 |
| Growing pools, 400k horizon, old readout ([15](15-growing-pools.md)) | 2.258 | 2.035 | 1.990 | 1.977 | 2.118 |
| Growing pools, 50k horizon, readout2 | 2.208 | 2.014 | 1.993 | 1.982 | 2.092 |
| **Growing pools, 400k horizon, readout2** | **2.157** | **1.947** | **1.925** | **1.913** | **2.032** |

The best system is now 0.035 bits per byte behind order-4 counts on the last 500k bytes. It took 39 minutes and held 413,000 cells at the end. The word-pair conjoin was kept at 66k ticks.

## 2. What the system can do

Besides scoring the next byte, the trained system can **continue text.** Learning is frozen; it reads a prompt; then each next byte is sampled from its own prediction (temperature 0.7) and fed back in as input. In the engine's terms, Prime becomes Drive: the system imagines instead of sensing ([drive and prime](../concepts/drive-and-prime.md)). An order-4 Witten-Bell model trained on the same text samples the same way, as the reference.

After training on the three Austen novels (growing pools, 400k horizon, readout2), with prompt `"I do not know," said`:

> **Grown:** "I do not know," said he, sand poor my dear indeed new he had all, and for that, sure to setting only world, picture was before she expected at last night his to be consideration farther, with her exceeding.

> **Witten-Bell order 4:** "I do not know," said she was not one most underness which engagement.  I have whose to me, now forgetting the cannot as for I can best for thing before the should not felt it will do not to seemed the

| Samples (5 prompts × 300 bytes) | Words that occur in the training text |
|---|---|
| Grown, 50k horizon, readout2 | 256 / 294 (87.1%) |
| **Grown, 400k horizon, readout2** | **271 / 295 (91.9%)** |
| Witten-Bell order 4 | 276 / 300 (92.0%) |

Both write locally plausible English that loses track within a few words: names and phrases from all three novels mixed together, no sentence-level structure. That is what a model of contexts up to two word-pieces long should produce. The grown system's samples are as wordlike as order-4 counts' (the same rate of words found in the training text; the count model's figure rests on one set of samples), and its prediction is close to theirs. It built its context model itself, from raw bytes, with no list of contexts given to it.

## Findings

1. **About a third of the readout gap was mask capacity.** Wider masks and writes recovered 0.08 of the 0.22 bits per byte on hand-wired order 4; the full readout2 recovered 0.14 of the 0.36 on grown exact nodes. A 0.14-bit gap remains between log-linear writer masks and Witten-Bell backoff with the same contexts.
2. **On text, the grown system is now close to an order-4 count model:** 0.035 bits per byte behind on the last 500k of Austen (growing pools), and ahead on book B's first 50k after book A (exact nodes).
3. **It can generate.** Its continuations are as wordlike as order-4 counts' (92% of words occur in the training text, for both). Like any byte model with short contexts, it has no plan beyond the next few words.

## Code

`edge.rs`: `Row::confidence`, `Template`, `contribution`, settings `kw` and `swaps`. `brain.rs`: `predict`, `generate`, `frozen`. `baseline.rs`: `WbModel`. `scripts/gen_report.py`. Flags: `--readout2`, `--mask-bits`, `--write-bits`, `--row-template`, `--confidence`, `--generate "prompt|prompt" --gen-len 300 --temp 0.7 --gen-out file`.
