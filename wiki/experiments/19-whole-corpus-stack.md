# 19 · The stack on the whole corpus (2026-10-14)

*[All experiments](README.md)* · [← 18](18-sparser-codes.md)

**Concepts:** [Stepping stones](../concepts/stepping-stones.md) · [Screens](../concepts/screens.md#estimator-bias) · [Capacity and budget](../concepts/capacity-and-budget.md) · [Writer-mask readout](../concepts/writer-mask-readout.md#readout-v3)

Question: on 2M bytes of Austen, the word-piece level learned its stream but kept no chunk, so no third level appeared ([18](18-sparser-codes.md)). Given six times more data, does level 1 find a chunk worth keeping?

## Fitting it in memory

Writer-mask rows are almost all of a run's memory. A new progress line breaks memory down by part. On the 2M-byte Austen stack with exact nodes, level 0 held 806,000 rows, 2.4 GB, growing with the stream; on the whole corpus that would be well over 10 GB.
- **Level 0 uses growing real pools** (as in [15](15-growing-pools.md): 50k horizon, a 512k-cell budget). Rows are keyed by cell, so the budget bounds them: 99k–196k rows (240–470 MB) through the run.
- **Row counters are 16-bit.** They never exceed 32 bits × 8 levels; two-book reruns were identical.

The run: `stack --data corpus/gutenberg_all.bin --words --run-window 200000 --conjoin-words --long-trials 5 --grow-pools --pool-budget 524288 --readout3 --levels 3 --vocab 512 --upper "--code-bits 8 --write-bits 8 --age-every 1024 --bias-edge"`. It took 94 minutes. The process used 2.2 GB at the last check (6.5M bytes); the progress lines' own estimate peaked at 1.9 GB. One seed, as everywhere on this page.

## Level 0: the best result so far

| | Whole stream (11.7M bytes) | Books where it beats order-4 counts |
|---|---|---|
| Fixed real pools ([14](14-full-corpus.md)) | 2.555 | 0 of 18 |
| Growing pools, old readout ([15](15-growing-pools.md)) | 2.125 | 10 of 18 |
| **Growing pools, readout v3 (level 0 of this run)** | **1.976** | **15 of 18** |
| Witten-Bell order 4 | 2.107 | |
| `bzip2 -9` / `xz -9e` (whole file at once) | 2.135 / 2.212 | |

| Book | This run | WB order 4 | Difference |
|---|---|---|---|
| Austen, *Emma* | 2.095 | 2.008 | +0.09 |
| Austen, *Persuasion* | 1.918 | 1.904 | +0.01 |
| Austen, *Sense and Sensibility* | 1.899 | 1.887 | +0.01 |
| King James Bible | 1.708 | 1.900 | −0.19 |
| Blake, poems | 2.457 | 2.817 | −0.36 |
| Bryant, *Stories to Tell to Children* | 2.098 | 2.293 | −0.19 |
| Burgess, *Buster Bear* | 1.952 | 2.287 | −0.33 |
| Carroll, *Alice* | 2.124 | 2.354 | −0.23 |
| Chesterton, *The Ball and the Cross* | 2.195 | 2.304 | −0.11 |
| Chesterton, *The Wisdom of Father Brown* | 2.128 | 2.246 | −0.12 |
| Chesterton, *Thursday* | 2.094 | 2.198 | −0.10 |
| Edgeworth, *The Parent's Assistant* | 2.055 | 2.130 | −0.08 |
| Melville, *Moby Dick* | 2.251 | 2.342 | −0.09 |
| Milton, *Paradise Lost* | 2.267 | 2.465 | −0.20 |
| Shakespeare, *Julius Caesar* | 2.421 | 2.849 | −0.43 |
| Shakespeare, *Hamlet* | 2.322 | 2.719 | −0.40 |
| Shakespeare, *Macbeth* | 2.281 | 2.689 | −0.41 |
| Whitman, *Leaves of Grass* | 2.236 | 2.408 | −0.17 |

Both columns are computed the same way, from means over 1,000-byte blocks weighted by how much of each block falls in the book; [15](15-growing-pools.md)'s Witten-Bell column differs from this one by up to 0.02 for that reason. Only the three Austen novels, which open the corpus, still favour counts.

## Level 1: learns, but keeps nothing

| | Bits per symbol, second half |
|---|---|
| Level 1 (2.96M word-pieces) | **3.877** |
| Witten-Bell orders 0 / 1 / 2 / 3 | 4.069 / 3.801 / 3.793 / 3.946 |
| Kneser-Ney orders 0 / 1 / 2 / 3 | 4.069 / 3.802 / 3.772 / 3.842 |

- **Level 1 uses about 70% of what order-1 context offers** (0.19 of 0.27 bits).
- **No level 2.**
  - All 885 of level 1's chunk screens were negative (−0.29 to −0.02 bits).
  - 289 of its 290 delay and hold trials were rejected, the best at +0.0016 bits; the last was still running when the stream ended.
  - It kept only its bias edge.
- **The top-down node is the same as before.** Its 88 trials at level 0 were mostly slightly positive (60 of 88, up to +0.0061 bits), and all fell under the 0.01 bar.
- **The vocabulary is worse here.** The 512 slots go to the first word-pieces seen twice, and all are taken within about the first million bytes, inside *Emma*: no later book adds a symbol. The shared "other" symbol is 54% of the stream (46% on Austen alone). With real pools a slot is tied to a chunk cell and its generation, so once that cell is recycled, its word-piece can no longer appear: only 471 symbols ever occur in this stream, against 486 in the exact-node Austen run. That likely accounts for some of the difference.

## A correction: order 2 does carry information

[18](18-sparser-codes.md) concluded that on 2M bytes even exact counts find nothing beyond order 1 in the word-piece stream, because order-2 Witten-Bell did worse than order 1. That was the estimator. Witten-Bell backoff pays heavily for sparse contexts: a context seen once costs about a bit whenever it is wrong. On Austen it turns order-2 information into a loss; on the whole corpus it shows only a fraction of it. Two fairer measures both find order-2 information:
- **Kneser-Ney** (discount 0.75, continuation counts; `scripts/kn_orders.py`).
- **A mixture:** blend the order-1 prediction with one from the other context, with the weight chosen on the first fifth of the scored half (`scripts/context_mix.py`). A weight of 0 is among the choices, so the gain can't be much below zero (the lowest seen was −0.0004). It is not like-for-like with counts: the weight is tuned on held-out data, and the candidate model backs off to unigram counts.

| Gain over each method's own order-1 model, bits per symbol | Witten-Bell backoff | Kneser-Ney | Mixture |
|---|---|---|---|
| Level-1 stream, Austen | −0.020 | +0.034 | +0.055 |
| Level-1 stream, whole corpus | +0.008 | +0.030 | +0.047 |
| Whole words, Austen (top 4,096) | | | +0.17 |
| Whole words, whole corpus (top 4,096) | | | +0.22 |

The engine's screens estimate gains with the same Witten-Bell joint-context counts, backing off to the baseline. For the two previous symbols on the Austen level-1 stream, the half-split held-out screen gives −0.075 bits. A running-screen analogue (the kind this run's chunk screens used: prequential, counts halved every 1M ticks, mean of the last 200k) gives −0.013 on Austen and −0.007 on the whole corpus, where the mixture finds +0.055 and +0.047. So the screens likely understate many-state contexts. A chunk's context is not the same as the two previous symbols (it is the run of symbols since the last cut), so this is evidence about the estimator, not a measurement of the chunk; it is untested inside the engine.

## Is the useful context a graph, not a sequence?

If the context that matters above word-pieces is a relation, it would be invisible to chunks of adjacent symbols. Examples: a word several positions back, what was mentioned recently, the topic. The same mixture test measures each such context beyond order-1 counts, in bits per symbol:

| Context added to the previous symbol | Level-1 stream, Austen | Level-1 stream, whole corpus | Whole words, Austen | Whole words, whole corpus |
|---|---|---|---|---|
| Two previous symbols (adjacent order 2) | +0.055 | +0.047 | +0.170 | **+0.216** |
| The symbol 2 back alone | +0.001 | +0.003 | +0.052 | +0.070 |
| 3 back | 0.000 | +0.001 | +0.009 | +0.029 |
| 5 back | 0.000 | +0.001 | +0.001 | +0.020 |
| 10 back | 0.000 | +0.001 | +0.001 | +0.011 |
| Recency cache, last 100 content symbols | 0.000 | +0.005 | +0.015 | +0.101 |
| Recency cache, last 1,000 content symbols | 0.000 | +0.003 | +0.020 | **+0.128** |

("Content" here means not "other". Words are lower-cased words and punctuation marks, the 4,096 commonest kept, the rest "other": 3% on Austen, 8% on the corpus.)

- **On whole words, non-adjacent context is real.** What was mentioned in the last 1,000 words is worth 0.13 bits per word on the corpus. That is the second-largest context measured, after adjacent order 2 (0.22), and it is larger on the bigger, more varied corpus than on Austen alone (0.02).
- **On the level-1 stream it is almost gone** (at most 0.005). The likely reason is the vocabulary: 512 first-come word-pieces from the start of *Emma* can hold few of the corpus's content words, which carry topic, names and who is speaking, so most of them fall into "other" (54% of the stream). The vocabulary's make-up was not inspected directly.

## Findings

1. **Level 0 is now well ahead of order-4 counts on the whole corpus:** 1.976 against 2.107 bits per byte (one seed), beating them on 15 of 18 books and beating `bzip2`. Against [15](15-growing-pools.md) the readout, the stack's top-down offers and 16-bit counts all changed; the readout is the likely cause, but the step from 2.125 was not isolated. Growing pools bound memory, so the whole corpus runs in 94 minutes in about 2 GB.
2. **Six times more data did not give level 1 a chunk.** It learns its stream (70% of its order-1 information) but kept nothing, and no level 2 appeared.
3. **Three likely reasons, each testable:**
   - **The screens' estimator understates sparse contexts.** Witten-Bell backoff understates order-2 information, and on Austen reverses it; Kneser-Ney and mixtures find it.
   - **The level-1 vocabulary likely discards content.** A capped, first-come vocabulary makes "other" 54% of the stream, and almost all non-adjacent context is gone from it.
   - **The grammar's moves above word-pieces are sequential.** On whole words, recency (a cache of what was recently mentioned) is the second-largest context. No move builds a node like that yet.
4. **Correction to [18](18-sparser-codes.md):** order-2 information does exist in the word-piece stream. Witten-Bell order-2 counts hid it.

## Code

- `brain.rs`: `Brain::mem_mb` (memory by part, shown on stack progress lines); `edge.rs`: 16-bit row counts.
- `scripts/kn_orders.py` (Kneser-Ney orders on a symbol stream); `scripts/context_mix.py` (mixture gains of skip, recency and order-2 contexts, on a symbol stream or on whole words).
