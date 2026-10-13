# 13 · Word pairs: more data, then conjoin (2026-10-13)

*[All experiments](README.md)* · [← 12](12-words.md) · [14 →](14-full-corpus.md)

**Concepts:** [Screens](../concepts/screens.md#running-count-screen) · [Shadow trials](../concepts/shadow-trials.md#extensions-for-many-state-sources) · [Growth operators](../concepts/growth-operators.md#conjoin) · [Capacity and budget](../concepts/capacity-and-budget.md) · [Stepping stones](../concepts/stepping-stones.md)

Question: in [12](12-words.md) a previous-word context screened negative at every check, and it was never built. Was there no information in it, or too little data in a 30k-tick window to see it? And if it is there, what move lets the grammar use it?

Every run here is in the Rust crate: two books (400k bytes), or the three Austen novels (2M bytes); flags as in [reference code](../reference-code.md#the-rust-crate).

## 1. More data makes the previous word visible

The stored-window screen splits 30k ticks into halves: 15k ticks of counts for contexts with thousands of states. A **running count screen** keeps Witten-Bell counts for a candidate from the moment it is proposed. It scores each byte before learning from it, and reports the mean gain over the most recent 200k ticks ([screens](../concepts/screens.md#running-count-screen)).

| Screen of the previous chunk (exact nodes, two books) | 20k ticks | 100k | 200k | 400k |
|---|---|---|---|---|
| Stored window, beyond the current byte ([12](12-words.md)) | −0.02 | −0.03 | −0.03 | −0.06 |
| Running, beyond the current byte | −0.04 | +0.01 | +0.06 | +0.13 |
| Running, beyond the current chunk | +0.05 | +0.12 | +0.16 | +0.21 |

On the three Austen novels the last row keeps climbing: 0.07 at 20k ticks, 0.17 at 220k, 0.27 at 820k and about 0.30 from 1.7M on. So there was information. The stored window just couldn't see it. The last row is the important one: what the previous word tells you depends on the word now being read.

Running screens choose the chunk exactly as the stored window did (z = 0.5 at 10k ticks), so the two screens agree where both can see.

## 2. Three ways to use it

| Move | What it builds | Trial gain (bits) | Kept? |
|---|---|---|---|
| Previous-word hold, as an edge | The chunk completed at the last cut | −0.0001 to +0.0018 (7–9 trials per two-book run, about 60 on Austen) | No |
| Compress: 64 classes of completed chunks, grouped by the next chunk's first two bytes | A class variable, then a class edge | −0.0001 to 0.005 (exact nodes); up to 0.009 (real pools) | No |
| **Conjoin: bind(previous chunk, chunk)** | One cell per (previous word, current word-so-far) | 0.0004 to 0.0045 in normal trials; **0.010 with an extension** | **Yes** |

- **The additive edge fails** because the information is conditional on the current chunk. One writer mask per previous word cannot express "after *Captain*, expect *Wentworth*".
- **The classes fail** because grouping by the next chunk's first bytes throws away what matters. A screen of class-given-chunk sat between −0.08 and 0.00 bits, while the previous chunk itself gave +0.2. This is the same lesson as [distill on text](11-real-text.md): compressing by what a context predicts keeps the wrong variable.
- **The conjoin works, but only with time to learn.** The bind has about 31,000 cells by 40k ticks. A 12k-tick shadow trial gives each cell's writer mask a handful of writes, so its gain looks like 0.004 bits. A fixed 5-times-longer trial for every source with more than 4,096 states let the conjoin through (kept at 0.034 bits), but slowed everything else: book A, last 100k, got worse, 2.421 against 2.252. So instead, trials of such sources are now **extended**: if the gain is positive but under the bar, the trial continues, up to 5 times its normal length ([shadow trials](../concepts/shadow-trials.md#extensions-for-many-state-sources)). With that, the conjoin was kept at 72k–76k ticks in every exact run, at a gain of 0.010 bits.

## 3. Results

**Two books (exact nodes), bits per byte:**

| System | Book A, last 100k | Book B, first 50k after A | Book B, 50–100k after A | Book A whole |
|---|---|---|---|---|
| Chunks, running screens, seed 0 / seed 1 | 2.252 / 2.287 | 2.643 / 2.700 | 2.516 / 2.542 | 2.569 / 2.581 |
| + trial extensions only | 2.284 / 2.287 | 2.692 / 2.710 | 2.536 / 2.587 | 2.579 / 2.580 |
| **+ trial extensions + word-pair conjoin** | **2.200 / 2.200** | **2.615 / 2.650** | **2.480 / 2.521** | **2.536 / 2.533** |
| Witten-Bell order 4 | 1.958 | 2.586 | 2.385 | 2.153 |

**Three Austen novels, 2M bytes (exact nodes):**

| System | *Emma* | *Persuasion* | *Sense and Sensibility* | Last 500k |
|---|---|---|---|---|
| Witten-Bell order 4 | 2.008 | 1.904 | 1.887 | 1.878 |
| Chunks | 2.311 | 2.075 | 2.022 | 2.006 |
| **Chunks + word-pair conjoin** | **2.281** | **2.054** | **1.989** | **1.970** |

The gap to the order-4 count model on the last 500k shrinks from 0.13 to 0.09 bits per byte.

Extensions alone are not free: without the conjoin they cost about 0.02 bits per byte on average (worse in 6 of 8 cells, better in none), probably because long trials hold slots other candidates would have used. The conjoin's gain is measured against chunks without extensions, so it is net of that cost.

**Real pools gain nothing, and the test is incomplete.** With 4,096 cells, the chunk pool was evicted 101,000 times in 400k bytes, and the conjoin bind (1,024 cells) another 37,000 over its rebuilds. Real pools, chunks: 2.543; chunks + conjoin: 2.550. But at 1,024 cells the real conjoin is under the 4,096-state threshold, so it only ever had normal trials. Lowering the threshold to 1,000 extended every ordinary bind too; they filled the trial slots, and the conjoin was never tried (book A, last 100k: 2.537). An eviction resets a cell's writer mask, so capacity is the likely cause, but it is untested.

## Findings

1. **The previous word carries information, and it grows with data.** It was invisible to a 15k-tick count window, but a running screen sees about 0.30 bits beyond the current chunk on 2M bytes of Austen.
2. **The information is a conjunction.** Neither an additive edge nor 64 successor classes captured it; a bind of the two chunks did. The grammar already had the move for this (conjoin), triggered by a screen that measures beyond the chunk.
3. **Trials must allow for learning speed.** Counts learn in one visit; writer masks need many. A trial short enough to judge a 1,000-state bind cannot judge a 30,000-state one. Extending positive-but-short trials of many-state sources let the conjoin through; on their own they cost about 0.02 bits per byte.
4. **Capacity is the likely limit for real pools.** The chunk pool is far too small for English word prefixes, so its states are recycled before anything can bind to them. A real conjoin was never given an extended trial, though, so this is not yet shown. It is the [capacity question](../open-questions.md) again, now at the word level.
5. **The successor-class Compress failed for the same reason as distill.** What a higher level needs from a word is not what follows it in the next two bytes.

## Spec changes this supports

- Screen high-cardinality candidates with **running count screens**, beyond both the target's state and the strongest kept context.
- Extend trials for **many-state sources** whose gain is positive but below the bar.
- Offer **conjoin** when a source informs only beyond another kept node, not only beyond the target's own state.

## Code

`brain.rs`: `RunScreen` and `Cand` (running screens), `word_moves_running` (chunk, previous word, classes, conjoin), `trial_mult` (extensions), `ChunkClass` (the class Compress). Flags: `--words --run-window 200000 --conjoin-words --long-trials 5`, and `--classes` for the class move.
