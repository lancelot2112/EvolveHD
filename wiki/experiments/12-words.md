# 12 · Words: chunking by surprise (2026-10-13)

*[All experiments](README.md)* · [← 11](11-real-text.md)

**Concepts:** [Time: delays and holds](../concepts/time-delays-and-holds.md) · [Screens](../concepts/screens.md) · [Growth operators](../concepts/growth-operators.md#chunk) · [Broadcast](../concepts/broadcast.md) · [Stepping stones](../concepts/stepping-stones.md)

Question: [experiment 11](11-real-text.md) grew byte contexts up to three bytes, but nothing at word level. Can the grammar find word-like units by itself, if it is offered one new move: cut the stream wherever a byte is surprising, and carry "everything since the last cut" as a context? And does a "previous word" context help after that?

## Setup

Same data, readout, trials and growth as [11](11-real-text.md): book A (300k bytes) then book B (100k), one seed, real pools and exact (symbolic) nodes as separate runs. Two moves are added.

| Move | Definition |
|---|---|
| Surprise | A [broadcast](../concepts/broadcast.md)-style statistic: running mean μ and sd σ of the byte target's bits (EMA over 2,000 ticks) |
| Boundary | Event at tick t when bits_t > μ_t + z·σ_t |
| Chunk | Recurrent bind: chunk_t = bind(chunk_{t−1}, byte_t), reset to bind(START, byte_t) at a boundary. Real: matcher pool, capacity 4,096, 48-bit masks, LRU. Exact: a trie |
| Previous word | A hold: the completed chunk latched at each boundary |

**Screens.** Both moves are screened virtually from stored history, like delays and holds. The chunk is screened at z ∈ {0.5, 1.0, 1.5} and the best z is built; the previous-word hold is screened once a chunk exists. The shift-null MI screen fails here. Chunks have thousands of states and contain the byte itself, and it scored them at −0.5 bits. So both moves use a **held-out predictive gain**: counts on the first half of the 30k window, Witten-Bell interpolation, log-loss gain on the second half ([screens](../concepts/screens.md)).

Calibration on 60k bytes, before the run:

| z | 0 | 0.5 | 1.0 | 1.5 | 2.0 |
|---|---|---|---|---|---|
| Boundary rate | 0.39 | 0.23 | 0.13 | 0.07 | 0.05 |
| Held-out gain (bits) | 0.35 | 0.37 | 0.31 | 0.26 | 0.21 |

## Results

**Bits per byte**, against the same seed of [11](11-real-text.md), which had no chunk move:

| System | Book A, last 100k | Book B, first 50k after A | Book B, 50–100k after A | Book A whole |
|---|---|---|---|---|
| Grown, real pools (11) | 2.625 | 2.911 | 2.826 | 2.832 |
| **Grown, real pools + chunk** | **2.546** | **2.898** | **2.799** | **2.753** |
| Grown, exact nodes (11) | 2.329 | 2.688 | 2.544 | 2.637 |
| **Grown, exact nodes + chunk** | **2.284** | **2.676** | **2.539** | **2.577** |
| Witten-Bell order 4 (reference) | 1.958 | 2.586 | 2.385 | 2.153 |

**What was grown and kept** (both runs alike):
- At 10k ticks the screens chose z = 0.5 (0.25 bits real, 0.18 exact).
- The chunk was kept at 22k ticks with a trial gain of 0.20 bits, the largest of any edge, ahead of byte@2 (0.12–0.15).
- The usual byte-pair and skip-pair binds followed.
- At the end the chunk edge had the largest learned gain β (3.0–3.2) apart from one exact 3-byte bind.

**Boundaries against real word starts** (book A 200k–300k; word starts are 18% of bytes):

| | Real pools | Exact nodes |
|---|---|---|
| Boundaries per byte | 0.23 | 0.24 |
| Precision (boundary is a word start) | 0.42 | 0.43 |
| Recall (word start is a boundary) | 0.54 | 0.58 |
| Chunks that are exactly one word | 15% | 15% |
| Mean chunk length (bytes) | 4.3 | 4.2 |
| Book B: precision / recall | 0.37 / 0.50 | 0.38 / 0.55 |

Sample chunks (exact nodes, book A): `upplied` · `, to ` · `Henrietta, ` · `still t` · `ried` · `inter` · `val` · `s` · `com` · `fort ` · `to the ` · `ried to ` · `quiet ` · `Mary, to ` · `an` · `i` · `mate ` · `the ` · `feelings of ` · `Captain Wentworth`.

**Previous word.** Its held-out screen was negative at every check, from −0.10 to +0.003 bits over 39 checks, so it was never built.

## Findings

1. **A surprise-reset context pays.** The chunk was the most valuable single edge on text. It cut 0.08 bits per byte with real pools and 0.05 with exact nodes on book A, and it carried over to book B. The real-pool gain is about twice the spread between seeds 0 and 1 in [11](11-real-text.md) (0.03). This is still one seed per arm.
2. **The cuts are morph-like, not words.** Boundaries land on word starts 2.4 times as often as chance (0.42 against 0.18), but only 15% of chunks are one whole word.
   - Predictable runs merge: `to the `, `feelings of `, `Captain Wentworth`.
   - Rare words split at their surprising letters: `an|i|mate`, `inter|val|s`, `com|fort`.

   That is what a cut at surprise should do: a chunk is "a run the model could follow", which is close to the segmentation of a PPM-style model. Word identity is a different variable.
3. **Previous chunk is invisible at this window.** With thousands of chunk values and 15k training ticks per screen, almost every (previous chunk, byte) context in the held-out half is new. The screen can't tell "no information" from "not enough data". Word-to-word statistics need either far more data per screen or a small variable to condition on, such as a word class.
4. **The capacity comparison is not quite fair.** The real chunk pool had 4,096 cells against 1,024 for binds. Its evictions were not logged.
5. **Chunks show what Compress should target.** [Distill](../concepts/growth-operators.md#distill) of successor profiles never paid ([11](11-real-text.md)). A chunk is a variable that later levels need: it has a natural reset, it is the right length for one-step context, and it is kept on its own merits. Clustering chunks by the contexts they occur in is the obvious next [stepping stone](../concepts/stepping-stones.md) toward word classes.

## Spec changes this supports

- Add **chunk** as a growth move: a recurrent bind reset by a surprise event, with z chosen by the screen.
- Use **held-out predictive gain** to screen high-cardinality contexts; keep the shift-null MI screen for small ones.
- Make surprise (bits above a running mean) a standard per-target broadcast signal, available as an event to Gate moves.
- Leave previous-word holds out until there is a compressed chunk variable to latch.

## Next

- Distill chunks by their left and right contexts into 32–256 classes, then screen previous-class holds.
- Repeat with seeds 1–3, and log chunk-pool evictions at 1,024 and 4,096 cells.
- Run on millions of bytes once the [Rust port](../reference-code.md) can do it.

## Code

[`reference/python/word_sim.py`](../../reference/python/word_sim.py) and [`wrun.py`](../../reference/python/wrun.py); see [reference code](../reference-code.md).
