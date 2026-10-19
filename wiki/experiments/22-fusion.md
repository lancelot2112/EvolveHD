# 22 · Fusing units into codes instead of slots (2026-10-18)

*[All experiments](README.md)* · [← 21](21-word-units.md)

**Concepts:** [Shift-bind](../concepts/shift-bind.md) · [Sparse codes](../concepts/sparse-codes.md) · [Matcher pools](../concepts/matcher-pools.md) · [Stepping stones](../concepts/stepping-stones.md)

Question: in [21](21-word-units.md) a stack started from whole words climbed to a phrase level, but the phrase level learned nothing. Each phrase got one arbitrary slot, so two phrases sharing most of their words were as unrelated as two random ones, and most phrases got no slot at all ("other"). If a unit is instead handed up as a code built from its parts, so similar units overlap, does the phrase level learn?

All runs are single-seed, with experiment 20's fixes and the upper-level readout settings of [21](21-word-units.md).

## The dimensionality check

Before building anything, a simulation of the candidate fusion schemes at the engine's sizes: 2,048 bits, 32-bit part codes, 512 fillers per role, each role bound by its own rotation. For two fused codes sharing m of n (role, part) pairs, the table gives how cleanly each extra shared part separates (d′; above about 3 is clean):

| Scheme | Active bits, 4 / 8 parts | d′ per shared part, 4 / 8 parts |
|---|---|---|
| **Rotate by role, then OR** | 127 / 243 | **12 / 5.7** |
| Rotate, OR, then thin back to 32 bits | 32 / 32 | 2.8 / 1.0 |
| Spot regions (each role owns 1/n of the bits) | 28 / 36 | 1.7 / 0.8 |
| Rotate, then XOR | 122 / 236 | 10 / 4.2 |

- **No sparsification before or after fusion.** Thinning to 32 bits costs most of the resolution. The fused code goes into the level above's matcher pool, whose winning cell emits its own 32-bit code, so k-WTA is the sparsification.
- **Fusion is a superposition of bound pairs:** rotation binds (a rotated code overlaps its original by 0 bits), OR bundles. It is shift-bind generalised from two parts to n roles.
- **No XOR.** On 32-of-2,048 codes XOR keeps 31 of an input's 32 bits: it is almost exactly OR, not a binding. XOR binds only dense (≈50%) codes.
- **2,048 bits holds about 8 role-bound parts cleanly.** At 8 parts the code is 12% dense; unrelated codes share about 28 bits by chance, and each shared part adds about 26 (collisions take a few).
- **Nesting would need non-commuting roles.** Rotations add, so if fused codes were fused again, role A inside role B would alias with role B inside role A. Fixed random permutations avoid that; one level up, rotations are fine.

## The op

`--fuse`: when a level hands up a unit, each of its last `--fuse-parts` (8) inputs is taken as its input cell's code, rotated by its role (role 0 = the unit's last part) and ORed. The result goes to the level above as a raw input to its matcher pool (`Brain::push_code`), which matches it to a cell or recruits one. The recruit bar is the similarity radius: `--fuse-bar` is the share of a cell's mask bits an input must overlap to match it (default 10 of 24; the bar is rounded to whole bits). The pool holds 640 cells. `--fuse-rel` adds relational roles: the input just before the unit, and the states of the level's kept holds. In these runs no hold was kept directly at the word level, so `--fuse-rel` added only the previous word.

Two caveats, both found in checking:
- **The pool doesn't realise the fusion's resolution.** A cell's mask is 24 bits sampled from the input that recruited it, so matching works through a random 24-bit thinning: d′ per shared part is about 2.6–3.4 at 4 parts and 1.1–1.5 at 8, no better than thinning the fused code to 32 bits.
- **The fused level's bits include a cell-to-symbol cost** that count models don't pay: every cell maps to one dummy symbol, at 0.07–0.10 bits per cell on Austen and 0.016 on the whole corpus.

The fused level's symbols are the cells its inputs matched, so its bits per symbol are measured on its own cell stream, against count models on that same stream.

## 1. Austen, starting from whole words

| Phrase level | Cells used | Bits per cell, second half | WB orders 0 / 1 / 2 on the cells |
|---|---|---|---|
| Slots ([21](21-word-units.md)) | – | 3.962 | 3.837 / 3.894 / 4.129 |
| Fused, positional roles, bar 10/24 | 569 | 6.597 | 6.502 / 6.401 / 6.826 |
| Fused, positional, bar 14/24 | 640 (682 recruits) | 6.679 | 6.564 / 6.464 / 6.890 |
| Fused, positional, bar 18/24 | 640 (977 recruits) | 6.849 | 6.718 / 6.619 / 7.063 |
| Fused, relational roles, bar 10/24 | 640 (683) | 4.427 | 5.662 / 4.215 / 4.406 |
| Fused, relational, bar 14/24 | 640 (1,761) | 4.653 | 5.778 / 4.409 / 4.630 |

Bits are not comparable across rows: each row's stream is different.

- **Positional fusion gives the phrase stream a little order-1 structure.** Order-1 counts gain 0.10 bits over unigram counts on the cell stream, where on the slot stream they lost 0.06. The phrase level's readout trails unigram counts by about 0.1 bits, which is almost all the cell-to-symbol cost (net 6.506 against 6.502), and it keeps only its bias edge.
- **The radius barely matters here,** but the test is capped by capacity: at 14/24 and 18/24 the 640-cell pool fills and churns.
- **Relational roles look dramatic, but the gain is likely partly circular.** A phrase's code now contains the word just before it, which is the last word of the previous phrase. So the next cell is partly a copy of what the level already saw: unigram 5.66 against order-1 4.22. No control (for example, ablating that role) was run.
- **Neither helps the word level.** The phrase level's predictions are offered to the word level as a top-down candidate. In every run it got one trial, gaining −0.0007 to 0.0000.
- **No chunk at the phrase level, so no level 2.** Chunk screens peaked at −0.0006 to −0.0011.

## 2. The whole corpus, starting from words

Level 0 reads the whole corpus as words (top 512), with growing real pools; the phrase level spawned at word 66k in both runs. 85 and 79 minutes.

| Phrase level | Cells used | Bits per cell, second half | WB orders 0 / 1 / 2 on its own stream | Kept |
|---|---|---|---|---|
| Slots ([21](21-word-units.md)), 660k phrases | – | 4.089 | 3.918 / 3.920 / 4.107 | bias |
| Fused, positional roles, 749k phrases | 550 | 5.704 | 5.943 / 5.775 / 6.046 | bias |
| Fused, relational roles, 752k phrases | 640 (985 recruits) | 3.732 | 4.911 / 3.654 / 3.759 | bias |

The streams differ (the fused runs cut about 13% more units), so bits are not comparable across rows.

- **The positional phrase level beats unigram counts by 0.24 bits and order-1 counts by 0.07, but that is not sequential learning.** A unigram with exponentially decaying counts (`scripts/decayed_unigram.py`, best half-life 3k cells) gets 5.718, as good as the level (5.704; 5.688 net of the cell-to-symbol cost). The fused cell stream drifts: each imperfect match swaps a mask bit toward the input, so cell meanings change, and decay gains 0.23 bits on this stream against 0.065 on the slot stream. The phrase level kept only its bias edge, which tracks recent frequencies.
- **The cells are coarse.** 550 cells hold 749k phrases; the 640-cell pool never filled. One- and two-word phrases match a cell when they share their last word; longer phrases need three or four shared parts.
- **The word level is unchanged:** 4.703 bits per word (positional) and 4.722 (relational), against 4.710 with slots.
- **The phrase level helps the word level only briefly.** The top-down node itself got 11 trials in each run (−0.0007 to +0.0033, none kept). Binds containing it were kept twice in each run (+0.012 to +0.025) and pruned within 30k–100k words. The slot run of [21](21-word-units.md) likewise kept the top-down node once, briefly, so fusion and slots can't be told apart here.
- **No chunk at the phrase level, so no level 2.** Its chunk screens peaked at +0.0023 (positional) and +0.0003 (relational), under the 0.005 threshold. The positional run trialled one hold and one bind on it, both rejected.

## 3. Finer cells without drift

The direct fix for both caveats: wider masks (96 bits, `--fuse-k 96`) so matching sees more of the fused code, and no mask drift (`--hebb 0`). Austen, positional roles:

| Pool | Recruits (118k phrases) | Bits per cell | WB0 / WB1 on its own stream | Decayed unigram, best |
|---|---|---|---|---|
| 24-bit masks, bar 10/24, drift on | 569 | 6.597 | 6.502 / 6.401 | 6.456 |
| 96-bit masks, bar 48/96, no drift | 67,783 | 13.710 | 9.071 / 8.816 | 9.022 |
| 96-bit masks, bar 67/96, no drift | 102,731 | 16.954 | 9.324 / 6.994 | 9.325 |

With masks that resolve the fused code, almost every phrase is new: the 640-cell pool recruits on most ticks, cells mean nothing, and the readout collapses. The coarse masks gave a few hundred cells only by lumping phrases together.

That is the trade-off underneath this experiment. A single-winner pool whose cells the level above must predict one by one is a slot vocabulary in disguise. Fusion changes which phrases share a cell, not the bottleneck: either few cells, which lump phrases, or many, which the readout can't predict. To use the overlap between fused codes, the level above probably has to predict the code's parts (for example, each role's part separately, a factored readout) instead of a cell.

## Findings

1. **Fusion is a superposition of role-bound parts; it needs no thinning and no XOR.** On 2,048 bits it holds about 8 parts cleanly. But a pool whose cells sample 24 bits of their input acts as a random thinning, so that resolution isn't realised in matching.
2. **Fused codes did not make the phrase level learn sequence.** On the whole corpus it beats static unigram and order-1 counts on its own stream, but a decaying unigram matches it: the cells drift, and the level tracks their recent frequencies. On Austen it ties unigram counts.
3. **Its knowledge reaches the word level only briefly** (binds with the top-down node kept twice, then pruned, as with slots), and it forms no chunk, so no third level appears.
4. **Single-winner cells are slots in disguise.** Masks that resolve the fused code make almost every phrase a new cell; masks that don't, lump phrases. Using the overlap likely needs a readout that predicts the parts of the next unit's code rather than its cell.
5. **Relational roles need care.** Putting the previous unit's content into the next unit's code likely makes cell prediction partly circular (untested); it inflated the phrase level's score without helping the word level.

## Code

- `brain.rs`: `Brain::push_code`, `input_code`, `hold_codes`, the `pending` fused input in `observe`; `Cfg::fuse`, `fuse_parts`, `fuse_rel`, `fuse_k`, `fuse_bar`, `force_real`.
- `stack.rs`: `Stack::fuse_unit`, `cell_texts`.
- `scripts/fusion_dimcheck.py` (the dimensionality check); `scripts/decayed_unigram.py` (the drifting-frequency baseline).
- Flags: `--fuse`, `--fuse-parts`, `--fuse-rel`, `--fuse-k`, `--fuse-bar`, `--real`.
