# 11 · Real text with real matcher pools (2026-10-13)

*[All experiments](README.md)* · [← 10](10-full-primitives.md) · [12 →](12-words.md)

**Concepts:** [Writer-mask readout](../concepts/writer-mask-readout.md) · [Matcher pools](../concepts/matcher-pools.md) · [Capacity and budget](../concepts/capacity-and-budget.md) · [Sleep and consolidation](../concepts/sleep-and-consolidation.md) · [Screens](../concepts/screens.md)

Question: does the growth grammar find real structure in raw text, where nobody designed the stepping stones, and does it survive real matcher pools (capacity, recognition noise) instead of clean symbolic nodes?

## Setup

**Data.** NLTK Gutenberg corpus. Book A: Austen, *Persuasion*, 300,000 bytes. Book B: Chesterton, *The Man Who Was Thursday*, 100,000 bytes. The stream is read once, A then B. Every byte is scored before it is learned from (prequential bits per byte).

**One target**, the byte node: a matcher pool over fixed random 32-of-2048 byte codes (24-bit masks).

**Grown nodes:**

| Node | Definition |
|---|---|
| Delay taps | Byte node d ticks back, d ∈ {2, 3, 4, 5, 6, 8, 12, 16} |
| Binds, real | Matcher pool, capacity 1024, LRU eviction, 48-bit masks over the shift-bind (X code OR Y code rotated 977) |
| Binds, symbolic | Exact unlimited pair table |
| Holds | Latch X whenever the byte node shows a value seen on 0.5–20% of ticks |
| Distill | k-means (k = 32) of a kept bind's source cells by predicted-successor profile |

**Readout.** Writer-mask Prime edges with 8 levels × 640 bits; level k is written with probability 2^−k, 8 bits per write. Edges are mixed log-linearly: p(cell) ∝ exp(Σ β_e · score_e), with β learned online.

**Trials.** Shadow trials scored by paired log-loss: burn-in 4k, evaluation 8k, keep at ≥ 0.01 bits and z ≥ 2.5. Prune every 10k ticks if gain < 0.003 bits or z < 1.

**Screens.** Conditional MI with a circular-shift null over 30k ticks. The bind screen measures gain over the best part, and a pair must not exceed W/4 distinct values (or 2 × capacity for real pools).

## Readout calibration first

The toy sims' single 160-bit writer mask could not represent wide successor distributions. Order-1 byte model on 80k bytes, bits per byte:

| Readout | bits/byte |
|---|---|
| 1 level × 160 bits | 4.11 |
| 1 level × 640 bits | 4.06 |
| 8 levels × 640 bits, 1-bit writes | 3.69 |
| 8 levels × 640 bits, 8-bit writes | 3.67 |
| Witten-Bell exact counts | 3.49 |

With exact order-3 contexts, the 8-level, 8-bit-write readout reached 2.60 at 80k bytes. Witten-Bell order 3 reached 2.33 and order 2 reached 2.80.

## Main results (bits per byte)

| System | Book A, last 100k | Book B, first 50k after A | Book B, 50–100k after A | Book B from scratch, first 50k | Book A whole |
|---|---|---|---|---|---|
| Witten-Bell order 2 | 2.712 | 3.086 | 2.961 | 3.072 | 2.769 |
| Witten-Bell order 3 | 2.151 | 2.677 | 2.525 | 2.828 | 2.287 |
| Witten-Bell order 4 | 1.958 | 2.586 | 2.385 | 2.862 | 2.153 |
| Hand-wired order 4, exact nodes | 2.177 | 2.614 | 2.484 | 3.133 | 2.398 |
| Hand-wired order 4, real pools | 2.735 | 2.988 | 2.920 | – | 2.843 |
| Grown, exact nodes | 2.329 | 2.688 | 2.544 | – | 2.637 |
| Grown, real pools, seed 0 | 2.625 | 2.911 | 2.826 | 3.620 | 2.832 |
| Grown, real pools, seed 1 | 2.593 | 2.943 | 2.853 | – | 2.819 |
| Grown, real pools, noisy input, recruit < 16 | 13.197 | 13.258 | 13.226 | – | 12.938 |
| Grown, real pools, noisy input, recruit < 10 | 4.923 | 5.100 | 4.962 | – | 4.849 |
| Grown, real pools, noisy input, recruit < 10 + merge | 3.181 | 3.260 | – | – | 3.267 |

Notes on the table:
- The from-scratch grown run on book B reached 2.939 over bytes 50–100k. Witten-Bell order 4 from scratch reached 2.445 there.
- Compressors on the whole of book A: gzip −9 2.925, xz 2.522, bzip2 2.198. On book B: gzip 3.191, xz 2.934, bzip2 2.682.
- Noisy input means 25% of code bits dropped and 4 spurious bits added per byte.
- "Merge" means cells whose masks overlap by ≥ 8 of 24 bits are merged every 1,000 ticks (older cell kept), and Hebbian swaps are cut to 2% of partial matches.

## What it grew

**Real pools, both seeds.** In order:
1. Delay tap byte@2 (10k).
2. Bind (byte × byte@2), kept at +0.25 bits (27k).
3. Bind (byte@3 × byte@2), +0.05.
4. Bind (byte × byte@3), +0.02.
5. Then (byte@4 × byte@2), (byte@4 × byte@3) or (byte@5 × byte@3) at about +0.01 each.
6. Holds latching the byte before the last space or 'a', kept at about +0.01 with negative learned gain.

The bind (byte × byte@3) evicted about 4,000 cells over the run, because its capacity is far below the number of distinct pairs.

**Exact nodes.** Same start, then 3-byte contexts: (byte × (byte@3 × byte@2)) at +0.056, plus skip contexts ((byte@3 × byte@5) × byte@2), ((byte@3 × byte@2) × byte@4) and (byte × (byte@2 × byte@4)). Bind tables grew to 8–12k cells.

**Distill.** Distill nodes were built at 27k and 37k in every run. Their own edges were rejected (gain at most 0.0023 bits), and binds over them were rejected (gain at most 0.0032), so cleanup removed them.

## Findings

1. **The grammar grows its own context model from raw bytes.** Nothing was listed: taps, pair contexts, 3-byte contexts (exact) and skip contexts were all found by the screens and kept by shadow trials. With exact nodes it came within 0.15 bits per byte of hand-wiring order 4 on book A (2.33 vs 2.18), and within 0.08 on book B (2.69 vs 2.61).
2. **Growth adapts to resource limits.** Real 1024-cell bind pools cannot hold 3-byte contexts. The data rule refused those binds, and growth built factored skip-pair binds instead. With real pools it beat the hand-wired order-4 chain on the same pools (2.61 vs 2.74 on A; 2.93 vs 2.99 on B). Capacity still costs 0.28 bits against exact nodes.
3. **The readout costs about 0.2 bits.** Hand-wired exact order-4 contexts with writer masks reached 2.18, against Witten-Bell order 4's 1.96. Count-based backoff between orders is something the log-linear mix only approximates.
4. **Against compressors:** real pools averaged 2.83 over the whole of book A, beating gzip (2.92) but behind xz (2.52) and bzip2 (2.20). Exact nodes averaged 2.64.
5. **Structure transfers across authors.** After book A, the grown system scored 2.93 on book B's first 50k, against 3.62 from scratch. Most of that is the time a fresh system spends growing: over B's 50–100k it was 2.84 vs 2.94. Exact counts transfer too (order 4: 2.59 vs 2.86), so this shows learned structure reused rather than memorised strings, but it is not unique to the grammar.
6. **Noise breaks naive recruitment, and merging fixes most of it.**
   - With 25% dropout and 4 spurious bits, a recruit bar of 16 of 24 bits recruited 55,000 cells and destroyed recognition (normalised MI between cell and byte 0.36; 13 bits per byte).
   - A bar of 10 restored one byte per cell, but each byte was spread over duplicate cells (1.26 bits of cell entropy per byte; 4.9 bits per byte).
   - Two causes: rare low-overlap inputs recruit duplicates, and 1-bit Hebbian swaps on every partial match let masks drift toward spurious bits.
   - Merging cells with ≥ 8 overlapping mask bits every 1,000 ticks, plus Hebbian on only 2% of partial matches, cut the duplicate entropy to 0.23 bits and gave 3.18 bits per byte, with the same structure grown.
7. **What did not pay.** Distilled clusters of predicted successors added nothing, either as a source or as a bind part. Event holds keyed on spaces or letters scraped ~0.01 bits. No word-level structure appeared in 400k bytes. The toy-world Compress operator does not yet give text a reusable higher-level variable.

## Spec changes this supports

- Use a graded readout: stochastic multi-level writer masks with multi-bit writes, and learned per-edge gains in a log-linear mix.
- Use log-loss shadow trials for probabilistic targets.
- In the bind screen, measure gain over the best part, and add a data-sufficiency rule (distinct pairs ≤ W/4, and ≤ 2 × capacity for real pools).
- Set input pool recruit bars far above chance overlap but well below the expected noisy overlap (10 of 24 here), and make merging duplicates a standing sleep-phase operation.
- Compress for text is open: the next attempt should target a variable that later levels actually need (word identity or word class), probably through intermediate targets.

## Code

[`reference/python/text_sim.py`](../../reference/python/text_sim.py); see [reference code](../reference-code.md) for how to run it.
