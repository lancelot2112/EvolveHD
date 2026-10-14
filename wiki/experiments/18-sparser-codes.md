# 18 · Sparser codes, and counting with every bit (2026-10-13)

*[All experiments](README.md)* · [← 17](17-level-stack.md) · [19 →](19-whole-corpus-stack.md)

**Concepts:** [Writer-mask readout](../concepts/writer-mask-readout.md#readout-v3) · [Sparse codes](../concepts/sparse-codes.md#readout-codes) · [Stepping stones](../concepts/stepping-stones.md)

Question: [17](17-level-stack.md) found that the readout's error grows with the number of outcomes a row must hold, so a word-piece level could not learn its stream. If the readout uses fewer bits per symbol, do rows hold hundreds of outcomes, does level 1 learn, and does a third level appear?

## The change

`--code-bits k`: the readout uses only the first k of each target code's 32 bits, a random k-subset. Scores are normalised by k × levels instead of 32 × levels. Matcher pools keep all 32 bits, so matching and its noise tolerance are unchanged. Masks and writes are set separately (`--mask-bits`, `--write-bits`).

All runs are single-seed. The test beds are those of [17](17-level-stack.md): one brain alone (`evolvehd syms`) with growth off, so only the order-1 edge and a bias edge (the order-0 symbol frequencies) learn. They run on synthetic streams and on the saved level-1 stream from Austen.

## 1. Sparser codes alone help a little

A grid over code bits (32, 16, 8, 4), codes held per level (mask bits = code bits × 20 to 160), write width and row style, on Zipf unigrams of 200k symbols. The table gives the gap to unigram counts, in bits per symbol:

| Distinct symbols | Old default (32-bit codes) | Best with 32-bit codes | Best with sparser codes |
|---|---|---|---|
| 16 | 0.40 | 0.11 | 0.10 (16 bits) |
| 64 | 0.12 | 0.13 | 0.09 (8 bits) |
| 256 | 0.25 | 0.28 | 0.21 (8 bits) |
| 512 | 0.45 | 0.49 | 0.32 (8 bits, 320-bit masks) |

The grid wrote half or all of a code's bits per write; the old default writes 8 of 32, probably why it beats the grid's 32-bit settings at 64 symbols and above. The 256-symbol stream was run with a vocabulary of 257 (one unused symbol), working around a defect fixed in this experiment (see Code); with 256 the new binary gives gaps 0.01–0.02 smaller, and nothing changes.

On the level-1 stream (unigram counts 4.696, order-1 counts 4.253), the old readout with a bias edge gets 4.750. With sparser codes alone it gets 4.675 (8-bit codes, 320-bit masks, 8-bit writes): just below unigram counts, and far from order-1 counts.

## 2. The bigger cost was noisy counting

A synthetic order-1 stream shows it: 512 symbols, each followed by one of 8 fixed successors with Zipf weights (true entropy 2.62 bits). A row holds 8 codes, 256 bits at most, well under a 640-bit level, so nothing crowds. Still, over 300k symbols the readout gets 3.071 against 2.653 for order-1 counts: 0.42 bits behind. With 4× more data the gap halves (2.840 against 2.630). So rows learn slowly.

The cause is in how levels count. Each write reaches level k with probability 2^−k, and it does so **as a whole**: one coin decides whether all of the written bits go into that level. A successor seen a few times therefore sits at a random height, give or take a level. That is a factor of two in its probability.

**Per-bit levels** (`--bitwise-levels`): each bit of the code flips its own coin for each level. The fraction of a code's bits at level k becomes a smooth function of how often it was written, averaged over its bits. On the 8-successor stream this takes 3.071 to 2.771. With rows that start empty (`--sparse-rows`, from [17](17-level-stack.md)) it reaches 2.756.

## 3. Together

All runs below use a bias edge. The new settings are per-bit levels, empty rows, 1,024-bit masks, writes as wide as the code, and aging every 1,024 writes. Gaps are to the count model that has the stream's structure: unigram counts for Zipf streams, order-1 counts for the Markov streams.

| Stream | Count model | Old readout | New, 32-bit codes | New, 8-bit codes |
|---|---|---|---|---|
| Zipf, 16 symbols | 3.406 | 3.810 (+0.40) | 3.439 (**+0.03**) | 3.466 (+0.06) |
| Zipf, 32 | 4.156 | 4.318 (+0.16) | 4.195 (**+0.04**) | 4.222 (+0.07) |
| Zipf, 64 | 4.866 | 4.983 (+0.12) | 4.932 (**+0.07**) | 4.946 (+0.08) |
| Zipf, 128 | 5.561 | 5.703 (+0.14) | 5.653 (**+0.09**) | 5.665 (+0.10) |
| Zipf, 256 | 6.210 | 6.458 (+0.25) | 6.430 (+0.22) | 6.373 (**+0.16**) |
| Zipf, 512 | 6.880 | 7.325 (+0.45) | 7.311 (+0.43) | 7.121 (**+0.24**) |
| Order 1, 8 successors | 2.653 | 3.071 (+0.42) | 2.754 (**+0.10**) | 2.792 (+0.14) |
| Order 1, 64 successors | 5.178 | 6.110 (+0.93) | 5.518 (+0.34) | 5.385 (**+0.21**) |

"Old readout" is the default (640-bit masks, 8-bit writes). [17](17-level-stack.md)'s capacity table used the best of several old settings, so its gaps are smaller (0.06 at 16 symbols, 0.44 at 512).

- **The new settings remove most of the error with few outcomes,** and per-bit levels are the main part of it. The same settings without per-bit levels give +0.24 at 16 symbols and +0.45 on the 8-successor stream (against +0.03 and +0.10 with them).
- **Sparser codes help where rows fill up.** At 256 and 512 symbols, and with 64 successors, 8-bit codes beat 32-bit codes; with up to 128 symbols, 32 bits are slightly better (by 0.01–0.03, within what one seed can show). What seems to matter is how many bits a row must hold (outcomes × code bits) against its mask, not the number of outcomes alone.

**The level-1 stream** (unigram counts 4.696, order-1 counts 4.253):

| Readout (all with a bias edge) | Bits per symbol |
|---|---|
| Old | 4.750 |
| Sparser codes only (8-bit codes, 320-bit masks, 8-bit writes) | 4.675 |
| Per-bit levels only | 4.636 |
| Per-bit levels, 8-bit codes, 320-bit masks | 4.604 |
| + empty rows | 4.594 |
| Per-bit levels, empty rows, aging every 1,024 writes, 1,024-bit masks, 32-bit codes | 4.553 |
| The same with **8-bit codes** | **4.442** |

The readout now uses nearly 60% of what order-1 context offers (0.25 of 0.44 bits), up from none.

## 4. Readout v3 at level 0

`--readout3` sets per-bit levels, empty rows, 1,024-bit masks, 32-bit writes and the learned confidence gain. It keeps 32-bit codes: at level 0, with per-bit levels and empty rows, 8-, 16- and 32-bit codes are within 0.013 of each other on one seed (grown exact nodes: 2.092, 2.084, 2.079), so there is no case for changing them. At level 0 the two parts need each other: per-bit levels alone gave 2.179, and empty rows alone cost a little in [17](17-level-stack.md) (2.194), against 2.183 for readout2. (On the level-1 stream, per-bit levels help on their own: 4.750 → 4.636.)

**Two books, bits per byte** (book A last 100k / book B first 50k after A / book B 50–100k / book A whole):

| System | Readout v2 | Readout v3 |
|---|---|---|
| Hand-wired order 4, exact | 2.099 / 2.549 / 2.435 / 2.297 | **1.974 / 2.453 / 2.342 / 2.158** |
| Grown, exact nodes | 2.183 / 2.543 / 2.391 / 2.544 | **2.079 / 2.471 / 2.353 / 2.405** |
| Grown, fixed real pools | 2.518 / 2.867 / 2.801 / 2.746 | **2.422 / 2.849 / 2.784 / 2.628** |
| Growing pools + chunks + conjoin | 2.150 / 2.582 / 2.427 / 2.476 | **2.027 / 2.509 / 2.376 / 2.333** |
| Witten-Bell order 4 | 1.958 / 2.586 / 2.385 / 2.153 | |

With the same contexts, the readout is now 0.016 bits per byte behind order-4 counts on book A's last 100k. The gap was 0.22 with the Python readout and 0.14 with readout v2. On book B, grown exact nodes and growing pools with readout v3 beat order-4 counts in both 50k windows; fixed real pools do not.

**Three Austen novels (2M bytes), exact nodes, chunks + conjoin** (level 0 of the stack runs below):

| System | *Emma* | *Persuasion* | *Sense and Sensibility* | Last 500k | Whole |
|---|---|---|---|---|---|
| Witten-Bell order 4 | 2.008 | 1.904 | 1.887 | 1.878 | 1.944 |
| Readout v2 | 2.210 | 1.955 | 1.910 | 1.899 | 2.051 |
| **Readout v3** | 2.073 | **1.876** | **1.852** | **1.840** | 1.955 |

**Three Austen novels, growing real pools, 400k horizon, chunks + conjoin** (the best system of [16](16-readout-and-generation.md)):

| System | *Emma* | *Persuasion* | *Sense and Sensibility* | Last 500k | Whole |
|---|---|---|---|---|---|
| Witten-Bell order 4 | 2.008 | 1.904 | 1.887 | 1.878 | 1.944 |
| Readout v2 ([16](16-readout-and-generation.md)) | 2.157 | 1.947 | 1.925 | 1.913 | 2.032 |
| **Readout v3** | 2.060 | **1.868** | **1.855** | **1.843** | 1.948 |

With real pools too, the grown system now beats order-4 counts on the last two novels and the last 500k bytes, and is level with them over the whole stream (1.948 against 1.944). Only on *Emma*, the first book, do counts still lead. The word-pair conjoin was kept at 61k ticks. The run took 28 minutes, against 39 for readout v2.

## 5. The stack again

The run used readout v3 at every level, plus `--upper "--code-bits 8 --write-bits 8 --age-every 1024 --bias-edge"` for levels above 0. Exact nodes, `--words --run-window 200000 --conjoin-words --long-trials 5`, three Austen novels: 190 s, about 3.4 GB peak memory.

| Run | Level 1 bits per symbol | Witten-Bell orders 0 / 1 / 2 on the same symbols | Level 2 |
|---|---|---|---|
| Readout v2 ([17](17-level-stack.md)) | 4.710 | 4.696 / 4.253 / 4.290 | No |
| Readout v3, sparse upper codes | **4.501** | 4.816 / 4.329 / 4.349 | No |

(Level 0 cuts differently with the new readout, so level 1's stream differs.)

- **Level 1 learns its stream.** It is 0.31 bits below unigram counts and 0.17 above order-1 counts, so it uses about 65% of what order-1 context offers.
- **Still no level 2.**
  - Level 1's chunk screens stayed negative (−0.29 to −0.02 bits).
  - 48 of its 49 delay and hold trials were rejected, with gains from −0.0009 to +0.0005; the last was still running when the stream ended.
- **Witten-Bell counts do worse with two symbols of context than with one** (order 2 4.349, order 1 4.329). *Correction ([19](19-whole-corpus-stack.md)): that is Witten-Bell's penalty on sparse contexts. Kneser-Ney order 2 gains 0.034 bits here, and a mixture 0.055.* The shared "other" symbol is 46% of the stream.
- **The top-down node is unchanged: slightly positive, far too small to keep.** Its 12 trials at level 0 gained −0.0002 to +0.0023 bits, against a 0.01 bar (in [17](17-level-stack.md): 13 trials, −0.0003 to +0.0015). Its screens beyond the chunk stayed negative (−0.12 to −0.02).

## Findings

1. **Sparser codes help where rows must hold many codes.** With 8 instead of 32 bits per code, the gap on a 512-symbol unigram falls from 0.43 to 0.24 bits, and level 1 goes from 4.553 to 4.442. At bytes they make no difference.
2. **The readout's main error was noisy counting, not capacity.** One coin per level per write puts each successor at a random height. Giving every code bit its own coin, together with empty rows, wider masks, full writes and aging, cuts the error from 0.40 to 0.03 bits on a 16-symbol unigram, and from 0.42 to 0.10 on an order-1 stream with 8 successors; without per-bit levels those settings reach only 0.24 and 0.45.
3. **The readout gap at level 0 is nearly closed.**
   - With identical contexts it is 0.016 bits per byte behind order-4 counts, down from 0.22.
   - On Austen, the grown system beats order-4 counts on the last 500k bytes with exact nodes (1.840 against 1.878) and with growing real pools (1.843), and with real pools matches them over all three novels (1.948 against 1.944).
4. **Level 1 learns, but a third level still doesn't appear.** Witten-Bell order-2 counts do worse than order 1 on the word-piece stream. *Correction ([19](19-whole-corpus-stack.md)): order-2 information is there (Kneser-Ney +0.034 bits, a mixture +0.055); Witten-Bell backoff hides it, and the engine's screens use the same kind of estimator.*

## Code

- `edge.rs`: `TargetIndex::rk` (readout code bits), `Edge::bitwise`.
- `brain.rs`: `Cfg::code_bits`, `bitwise_levels`; `Brain::bytes` replaces the old `vocab == 256` test, so a symbol stream with 256 symbols is no longer treated as bytes.
- `stack.rs`: `upper_cfg`.
- Flags: `--code-bits`, `--bitwise-levels`, `--readout3`, and `stack --upper "flags"` for the levels above 0.
