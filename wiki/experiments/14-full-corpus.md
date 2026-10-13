# 14 · The whole corpus (2026-10-13)

*[All experiments](README.md)* · [← 13](13-word-pairs.md) · [15 →](15-growing-pools.md)

**Concepts:** [Capacity and budget](../concepts/capacity-and-budget.md) · [Growth operators](../concepts/growth-operators.md#distill) · [Screens](../concepts/screens.md#running-count-screen) · [Matcher pools](../concepts/matcher-pools.md)

Question: does the grown system keep improving when it reads 30 times more text, across 18 books by 12 authors, with real pools? Does word-level structure appear at scale?

## Setup

- **Data:** all 18 texts of the NLTK Gutenberg corpus as one stream, 11,744,888 bytes, in alphabetical order. `scripts/fetch_corpus.sh` writes it, with each book's offset.
- **System:** the Rust crate, real pools, seed 0. Chunk moves, running screens over 200k ticks, word-pair conjoin and trial extensions all on (`--words --run-window 200000 --conjoin-words --long-trials 5`).
- **Baselines:** online Witten-Bell order 3 and order 4 over the same stream.
- **Cost:** 44 minutes on one core, sharing the machine with other runs; peak memory 1.1 GB.

## Results

Bits per byte, each byte scored before it is learned from:

| Book | Bytes | Grown | WB order 3 | WB order 4 | Grown − WB4 |
|---|---|---|---|---|---|
| Austen, *Emma* | 887,071 | 2.596 | 2.188 | 2.008 | 0.59 |
| Austen, *Persuasion* | 466,292 | 2.500 | 2.173 | 1.904 | 0.60 |
| Austen, *Sense and Sensibility* | 673,022 | 2.503 | 2.159 | 1.887 | 0.62 |
| King James Bible | 4,332,554 | 2.286 | 2.165 | 1.900 | 0.39 |
| Blake, poems | 38,153 | 2.968 | 2.910 | 2.800 | 0.17 |
| Bryant, *Stories to Tell to Children* | 243,901 | 2.592 | 2.516 | 2.292 | 0.30 |
| Burgess, *The Adventures of Buster Bear* | 82,992 | **2.417** | 2.566 | 2.289 | 0.13 |
| Carroll, *Alice* | 144,395 | 2.630 | 2.569 | 2.353 | 0.28 |
| Chesterton, *The Ball and the Cross* | 457,450 | 2.717 | 2.544 | 2.303 | 0.41 |
| Chesterton, *The Wisdom of Father Brown* | 406,629 | 2.715 | 2.513 | 2.247 | 0.47 |
| Chesterton, *Thursday* | 320,525 | 2.700 | 2.483 | 2.198 | 0.50 |
| Edgeworth, *The Parent's Assistant* | 916,861 | 2.650 | 2.413 | 2.129 | 0.52 |
| Melville, *Moby Dick* | 1,220,066 | 2.890 | 2.595 | 2.342 | 0.55 |
| Milton, *Paradise Lost* | 468,220 | 2.872 | 2.670 | 2.464 | 0.41 |
| Shakespeare, *Julius Caesar* | 112,310 | **2.946** | 2.990 | 2.848 | 0.10 |
| Shakespeare, *Hamlet* | 162,881 | 2.874 | 2.864 | 2.720 | 0.15 |
| Shakespeare, *Macbeth* | 100,351 | 2.870 | 2.859 | 2.690 | 0.18 |
| Whitman, *Leaves of Grass* | 711,215 | 2.850 | 2.632 | 2.408 | 0.44 |
| **Whole stream** | 11,744,888 | **2.555** | 2.356 | 2.107 | 0.45 |

What was grown, over the whole run:
- 855 binds, 74 delay taps and 701 holds were built.
- There were 2,248 trials: 90 keeps, 2,157 rejections and 80 prunes. Cleanup removed 1,612 nodes.
- At the end, 10 edges were kept: the chunk, pair and skip-pair binds, one hold, a bind with a hold as a part, and one bind over a distill.

## Findings

1. **More data does not close the gap with real pools.** The grown system sits 0.4–0.6 bits per byte behind order-4 counts on long books, from the first million bytes to the last. Counts keep using everything they have seen; fixed pools of 1,024 cells keep forgetting. The bind pools were evicted up to 966,000 times, and the chunk pool 3.9 million times: about once every 3 bytes.
2. **It doesn't adapt faster than counts after a change of author.** Over whole books it beat order-3 counts on two short ones, *Buster Bear* (2.42 against 2.57) and *Julius Caesar* (2.95 against 2.99), and roughly tied on *Hamlet* and *Macbeth*. But over the first 20k bytes after most author switches it trails order 3 by 0.07–0.34. *Buster Bear* is the only clear early win (0.08 better).
3. **Distill paid on text for the first time, in the Bible.** Its verse numbering is regular, and a distill of (byte, 6 back) into 32 clusters became the part of a kept bind, `((byte@3 × Z[byte × byte@6]) × byte@2)`. That bind was kept at 5.1M bytes with a trial gain of 0.144 bits and was still kept at the end. Compress works when the data has a small hidden variable to find; English prose so far hasn't offered one.
4. **The word-pair conjoin fails with real pools at scale too.** It was tried 71 times, all normal trials (its 1,024 cells are under the extension threshold): 69 rejected, 2 kept briefly and then pruned. The previous-word screen measured beyond the chunk peaked at 0.49 bits in the Bible (2.56M bytes) and was still 0.36 at 5.15M, then fell to about 0.02 by the end. Chunk cells recycle so fast that a chunk id no longer means one word.
5. **The engine holds up over a long run.** 11.7M bytes, 44 minutes, bounded memory, no manual intervention.

## What this says to do next

*Done in [15](15-growing-pools.md): with growing pools the whole-stream gap to order-4 counts fell from 0.45 to 0.02 bits per byte.*


Capacity, not the grammar, is now the limit. With exact nodes, the same moves reach word pairs ([13](13-word-pairs.md)). With real pools, every word-level structure is evicted before it can be learned. The next experiment should let pools grow on demand: a parallel pool when evictions of recently used cells pass a threshold. That is the [capacity question](../open-questions.md) at the top of the list.

## Code

`scripts/fetch_corpus.sh` (corpus), `scripts/full_report.py` (this table); the run: `evolvehd text --data corpus/gutenberg_all.bin --words --run-window 200000 --conjoin-words --long-trials 5 --out runs/full_real.json`; baselines: `evolvehd ngram --data corpus/gutenberg_all.bin --order 4 --out runs/full_wb4.json`.
