# 15 · Pools that grow on demand (2026-10-13)

*[All experiments](README.md)* · [← 14](14-full-corpus.md) · [16 →](16-readout-and-generation.md)

**Concepts:** [Matcher pools](../concepts/matcher-pools.md#growing-pools) · [Capacity and budget](../concepts/capacity-and-budget.md) · [Sleep and consolidation](../concepts/sleep-and-consolidation.md) · [Growth operators](../concepts/growth-operators.md#conjoin)

Question: [14](14-full-corpus.md) showed that fixed real pools are the limit. They stay 0.45 bits per byte behind order-4 counts over the whole stream (0.4–0.6 on long books), and the word-pair conjoin of [13](13-word-pairs.md) works only with exact nodes. If pools may grow when they need to, do real pools reach exact-node quality?

## The rule

- **Pressure:** each eviction records how long ago the evicted cell was last used. Every 10k ticks, if a pool has made at least 100 evictions and more than half removed cells used within the **horizon** (50k ticks by default), its working set does not fit, and it doubles.
- **Limits:** at most 131,072 cells per pool, and a total budget across pools (1M cells; 512k in the full-corpus run).
- **Sized from the evidence:** a new bind or chunk starts at the next power of two above 1.5 times the distinct inputs in its replay window, instead of always starting at 1,024 or 4,096. Without this, a conjoin rebuilt after each failed trial started small again, and was kept only at 267k ticks instead of 79k.
- **Cheap at size:** matching uses an inverted index (bit → cells), and eviction an ordered set of (last use, cell). Winners are identical to a linear scan. The earlier runs I repeated were unchanged (fixed pools 2.621, exact nodes 2.321, fixed-pool words 2.543), and the real-pool word run became about 30% faster (51 s to 35–37 s).

Flags: `--grow-pools`, `--young-age` (the horizon), `--max-pool`, `--pool-budget`.

## Results

**Two books, book A last 100k (bits per byte):**

| System | Fixed real pools | Growing real pools | Exact nodes |
|---|---|---|---|
| Plain growth | 2.621 | **2.331** | 2.321 |
| + chunks (running screens) | 2.543 | **2.286** | 2.252 / 2.287 (seeds 0 / 1) |
| + chunks + word-pair conjoin, seeds 0 / 1 | 2.550 | **2.206 / 2.243** | 2.200 / 2.200 |

With growth, the conjoin was kept at 79k–80k ticks on both seeds; its pool grew to 65,536 cells. Book B gains the same way: 2.634 / 2.655 on its first 50k against 2.615 / 2.650 for exact nodes.

**Three Austen novels, 2M bytes, chunks + conjoin:**

| System | *Emma* | *Persuasion* | *Sense and Sensibility* | Last 500k | Whole |
|---|---|---|---|---|---|
| Witten-Bell order 4 | 2.008 | 1.904 | 1.887 | 1.878 | 1.944 |
| Exact nodes | 2.281 | 2.054 | 1.989 | 1.970 | 2.132 |
| Growing real pools, 50k horizon | 2.303 | 2.105 | 2.080 | 2.066 | 2.183 |
| **Growing real pools, 400k horizon, max 262,144** | **2.258** | **2.035** | **1.990** | **1.977** | **2.118** |

With the 50k horizon, the word-pair bind stopped at 65,536 cells and was evicted 1.16 million times. Most evicted cells were older than 50k ticks, so they never counted as pressure. With a 400k horizon, it grew to 262,144 cells and the chunk pool to 65,536, and real pools matched exact nodes: 0.007 behind on the last 500k, and 0.014 ahead over the whole stream. The run took 42 minutes instead of 21.

**The whole corpus**: 11.7M bytes, 18 books. The settings are the same as [14](14-full-corpus.md) plus growing pools, with a 50k horizon and a budget of 512k cells. It took 2 h 14 min, with about 2.6 GB in use at 1 h 24 min.

| Book | Fixed pools ([14](14-full-corpus.md)) | Growing pools | WB order 3 | WB order 4 | Growing − WB4 |
|---|---|---|---|---|---|
| Austen, *Emma* | 2.596 | 2.303 | 2.188 | 2.008 | +0.30 |
| Austen, *Persuasion* | 2.500 | 2.105 | 2.173 | 1.904 | +0.20 |
| Austen, *Sense and Sensibility* | 2.503 | 2.080 | 2.159 | 1.887 | +0.19 |
| King James Bible | 2.286 | **1.847** | 2.165 | 1.900 | −0.05 |
| Blake, poems | 2.968 | **2.652** | 2.910 | 2.800 | −0.15 |
| Bryant, *Stories to Tell to Children* | 2.592 | **2.263** | 2.516 | 2.292 | −0.03 |
| Burgess, *The Adventures of Buster Bear* | 2.417 | **2.112** | 2.566 | 2.289 | −0.18 |
| Carroll, *Alice* | 2.630 | **2.272** | 2.569 | 2.353 | −0.08 |
| Chesterton, *The Ball and the Cross* | 2.717 | 2.326 | 2.544 | 2.303 | +0.02 |
| Chesterton, *The Wisdom of Father Brown* | 2.715 | 2.273 | 2.513 | 2.247 | +0.03 |
| Chesterton, *Thursday* | 2.700 | 2.227 | 2.483 | 2.198 | +0.03 |
| Edgeworth, *The Parent's Assistant* | 2.650 | 2.188 | 2.413 | 2.129 | +0.06 |
| Melville, *Moby Dick* | 2.890 | 2.385 | 2.595 | 2.342 | +0.04 |
| Milton, *Paradise Lost* | 2.872 | **2.396** | 2.670 | 2.464 | −0.07 |
| Shakespeare, *Julius Caesar* | 2.946 | **2.531** | 2.990 | 2.848 | −0.32 |
| Shakespeare, *Hamlet* | 2.874 | **2.454** | 2.864 | 2.720 | −0.27 |
| Shakespeare, *Macbeth* | 2.870 | **2.446** | 2.859 | 2.690 | −0.24 |
| Whitman, *Leaves of Grass* | 2.850 | **2.386** | 2.632 | 2.408 | −0.02 |
| **Whole stream** | 2.555 | **2.125** | 2.356 | 2.107 | +0.02 |

On the whole file, `bzip2 -9` takes 2.135 bits per byte, `xz -9e` 2.212 and `gzip -9` 2.857. Those compressors see the whole file at once; the grown system and the Witten-Bell models predict each byte before seeing it.

- The word-pair conjoin was kept at 78k ticks and stayed kept for the rest of the 11.7M bytes.
- There were 15 growth events in 13 pools. The largest is the word-pair bind, at 65,536 cells (7.2M evictions over the run, at the 50k horizon). Next come the chunk pool and one 4-byte bind, at 32,768 each.
- At every 500k-tick checkpoint the total was under 213k cells, and growth never ran out of room in the 512k budget.

## Findings

1. **Capacity was the cost of real pools.** Growing on demand removes the 0.3-bit gap on two books and lets the word-pair conjoin work with real pools. Over the whole corpus it cut the gap to order-4 counts from 0.45 to 0.02 bits per byte. It now beats them on 10 of 18 books: the Bible, all three Shakespeare plays, Blake, Bryant, Burgess, Carroll, Milton and Whitman.
2. **The horizon is the memory span.** A pool grows only to hold contexts that recur within the horizon. On 2M bytes, a 50k horizon left word pairs that recur less often to be forgotten; 400k matched exact nodes. This is a real choice of what to remember, not just a tuning knob.
3. **Start structures at the size their evidence says.** A bind that must grow from 1,024 cells under trial loses its trial.
4. **Size costs time.** Every new source cell needs its own writer-mask rows. The full corpus took 2 h 14 min against 44 min with fixed pools, and on Austen the 400k horizon doubled the run time. Rows are the next thing to make cheap.
5. **Counts still win on long prose.** Order-4 counts lead on Austen, Chesterton, Edgeworth and Melville (*Emma* by 0.30). The horizon explains only a little of that: on Austen even exact nodes trail order-4 counts by 0.19 bits per byte overall, and the 400k horizon moves *Emma* only from +0.30 to +0.25. The rest is the readout itself: writer masks against exact counts.

## Code

`pool.rs` (index, ordered eviction, `grow`, `take_pressure`); `brain.rs`: `grow_pools`, `sized_cap`.
