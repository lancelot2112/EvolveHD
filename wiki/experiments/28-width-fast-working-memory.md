# 28 · Wider codes, fast mapping and a working memory (2026-10-20)

*[All experiments](README.md)* · [← 27](27-meaning-codes.md)

**Concepts:** [Sparse codes](../concepts/sparse-codes.md) · [Shift-bind](../concepts/shift-bind.md) · [Growth operators](../concepts/growth-operators.md#story-state)

Question: [27](27-meaning-codes.md) learned meaning codes in 2,048 bits; words grouped, but nothing beat the story edge of [25](25-kept-words.md), and rare words, whose codes are learned from few contexts, were the weak spot. Three additions, each following the architecture the experiments are converging on (a slow cortex of meaning codes and links, a fast hippocampus for what is new, a compressed readout):
1. **Width:** does a wider space help?
2. **Fast mapping (hippocampal):** a word met only a few times takes the story's lit bits as its provisional meaning, refined later by its own contexts.
3. **Working memory (layer 3):** a short recursive state over every word.

Counts prototype (`scripts/meaning_codes.py`), one pass, prequential, one run per configuration, scored on exact-word bits beyond the counts system of [25](25-kept-words.md) as in 26 and 27 (whole corpus: base 8.672). Meaning codes use distinctive bits, 64 per word; the story code is the 64 most lit bits.

## 1. Width

Meaning codes keep 64 bits per word at every width, so wider spaces are sparser.

| Width | Locality (random) | Code rows: all | Rare words | New to the story | Readout + story edge |
|---|---|---|---|---|---|
| 1,024 | 0.291 (0.032) | +0.091 | +0.081 | +0.204 | +0.126 |
| 2,048 | 0.297 (0.031) | +0.101 | +0.117 | +0.226 | +0.127 |
| 4,096 | 0.293 (0.028) | +0.112 | +0.158 | +0.252 | +0.129 |
| 8,192 | 0.285 (0.028) | +0.123 | +0.203 | +0.277 | +0.132 |
| **14,336** | not measured | **+0.147** | **+0.277** | **+0.365** | +0.134 |
| *Random codes, 2,048 / 8,192 / 14,336* | | *+0.080 / +0.095 / +0.103* | *+0.038 / +0.076 / +0.090* | *+0.165 / +0.189 / +0.195* | |
| *Story edge ([25](25-kept-words.md))* | | *+0.130* | *+0.171* | *+0.270* | |

- **Wider is better for the compressed readout, and the trend steepens:** each doubling from 1,024 to 8,192 bits adds about 0.01 bits per word; going to 14,336 adds 0.024. At 14,336 bits the code rows alone beat the story edge (+0.147 against +0.130), on rare words by 0.11. With the story edge and the meaning readout they reach +0.150.
- **Meaning's margin over random codes grows with width** (+0.021 at 2,048, +0.028 at 8,192, +0.044 at 14,336).
- **Grouping doesn't change with width:** locality stays at 0.29 to 0.30. At 8,192 bits the neighbour lists are more specific ("church": jesus, paul, grace, saints, gospel; "emma": emma's, elton, weston, palmer, cole, hartfield, perry, knightley).

## 2. Fast mapping

On each of a word's first 8 occurrences, the story code lit at that moment is added to its meaning counters at a fixed weight per bit (`--fast` 5 or 20; for comparison, each context word adds 2 to 14 per bit, by its rarity). Width 2,048:

| Fast weight | Locality (random) | Readout: all | Rare words | New to the story | Readout + story edge: all / rare / new |
|---|---|---|---|---|---|
| 0 | 0.297 (0.031) | +0.070 | +0.007 | +0.215 | +0.127 / +0.140 / +0.276 |
| 5 | 0.297 (0.028) | +0.078 | +0.079 | +0.258 | +0.130 / +0.164 / +0.291 |
| **20** | 0.241 (0.020) | **+0.092** | **+0.376** | **+0.384** | **+0.138** / +0.291 / +0.348 |

- **A new word takes the meaning of the story it appeared in, and is predicted when that kind of story returns.** On rare words the readout gains +0.376, more than twice the story edge (+0.171). With the story edge it is +0.138, the first combination above the story edge alone (by 0.008; one run).
- **The cost is a little blur:** provisional bits lower locality from 0.297 to 0.241.

## 3. Working memory

A recursive state over every word, function words included: state ← fade × rotate(state) + identity code, read through its 64 most active bits by learned rows (`--wm-half`; rotation by one bit position per word, so a word's bits sit in a different place for each lag). The control does the same without rotation (an orderless short-term bag). Width 2,048:

| | Whole corpus: all / rare / new | Austen: all / rare / new |
|---|---|---|
| Story edge | +0.130 / +0.171 / +0.270 | +0.055 / +0.027 / +0.111 |
| Working memory, half-life 2 words | +0.088 / +0.039 / +0.205 | **+0.084** / +0.147 / +0.304 |
| Working memory, half-life 4 | +0.086 / +0.035 / +0.199 | +0.079 / +0.135 / +0.290 |
| Working memory, half-life 8 | +0.077 / +0.015 / +0.170 | +0.063 / +0.071 / +0.197 |
| Half-life 4, no rotation (orderless) | +0.089 / +0.042 / +0.206 | +0.079 / +0.142 / +0.293 |
| Story edge + working memory (half-life 4) | +0.130 / +0.171 / +0.270 | +0.075 / +0.129 / +0.259 |

- **On Austen the working memory alone beats the story edge** (+0.084 against +0.055), mostly on rare and new words.
- **On the whole corpus it adds nothing to the story edge** at 2,048 bits.
- **Order adds nothing through this readout:** the orderless state does as well. What the state carries that helps is the last few words as a bag; rows keyed by single bits, each a mixture of (word, lag) pairs, don't resolve order.

## 4. All together

Width 8,192, fast mapping 20, working memory half-life 4, whole corpus:

| | All words | Rare words | New to the story |
|---|---|---|---|
| Story edge | +0.130 | +0.171 | +0.270 |
| Working memory alone | +0.129 | +0.083 | +0.358 |
| Story edge + working memory | +0.142 | +0.161 | +0.363 |
| Story edge + meaning readout | +0.147 | +0.457 | +0.400 |
| **Story edge + meaning readout + working memory** | **+0.170** | **+0.467** | **+0.538** |

**Together they add +0.040 bits per word beyond the story edge** (8.672 → 8.502 bits per exact word, against 8.542 with the story edge alone). At this width the working memory alone matches the story edge (+0.129). The pieces now complement each other: the story edge for what the story keeps saying, fast-mapped meaning for rare words, working memory for words new to the story.

Caveats: one run per configuration; mixture weights are chosen on a coarse grid on the first fifth of the second half. Readout counts are 16-bit, saturating, in the 8,192-bit combined run and the 14,336-bit runs (32-bit elsewhere); in the 14,336-bit meaning run 8 entries hit the cap. Saturation could only lower the probability given to the commonest words, so it can only understate those readouts. The 14,336-bit runs did not measure locality (no report). The combined run was not rerun at 2,048 bits, so how much of its gain is width and how much the combination is not separated.

## Findings

1. **Wider codes help the compressed readout, increasingly:** about +0.01 bits per word per doubling up to 8,192 bits, then +0.024 to 14,336, where the code rows alone beat the story edge (+0.147 against +0.130). Meaning's lead over random codes widens with width; grouping quality stays the same. Where it levels off is not yet known.
2. **Fast mapping works:** binding a new word to the story it appeared in more than doubles the gain on rare words, at a small cost in grouping.
3. **A short working memory helps, but as a bag, not a sequence,** through this readout.
4. **Combined, at 8,192 bits, they add 0.040 bits per word beyond the story edge,** the first clear gain beyond it.

## Code

`scripts/meaning_codes.py`: `--D` (width), `--fast`, `--fast-n` (fast mapping), `--wm-half`, `--wm-k`, `--wm-rot` (working memory), `--memmap` (disk-backed counters for wide codes). Runs: `runs/exp27/` (`all_d*`, `all_fast*`, `all_wm*`, `a_wm*`, `all_d8192_fast20_wm4`).
