# 26 · A growing word map and a compressed story code (2026-10-20)

*[All experiments](README.md)* · [← 25](25-kept-words.md)

**Concepts:** [Sparse codes](../concepts/sparse-codes.md) · [Growth operators](../concepts/growth-operators.md#story-state) · [Stepping stones](../concepts/stepping-stones.md)

Question: [25](25-kept-words.md) kept every word as its own key, and the story state paid 0.30 bits per exact word. Could meaning be organised underneath it? The plan: a "cortex" that gives each word its own spot and lets related words gather by swapping positions, so a stretch of the map is a topic, and a compression of the lit map into a fixed-width code (like a hippocampal index) that carries the story's meaning, not just its words. Everything must grow and organise as it reads, in one pass.

Counts prototypes (`scripts/cortex_map.py`), single runs, scored on exact-word bits as in [25](25-kept-words.md), beyond the counts system of 25 (order-2 counts over the 512 commonest words, and a decaying memory of recent words that names "other" words; `scripts/cortex_base.py`).

## The map

A line of spots that starts empty and grows:
- **Placement.** A content word (outside the 100 commonest) gets a spot on first sight, next to a recent story word drawn by salience.
- **Moves.** On each later occurrence, its spot moves part of the way toward a target, shifting the spots in between one place each: a run of swaps. The step shrinks as the word is seen more (`max(0.02, 0.5/√n)`).
- **Targets tried:** one recent story word drawn by salience (*draw*); the salience-weighted median of everything lit (*median*); the weighted median of the word's 8 strongest learned links (*links*: the map keeps counts of which words occur within 16 content words of each other, ranked by PMI).
- **Senses.** A word whose targets split into two far-apart groups gets a second spot there; an occurrence uses the spot nearest its context.
- **Control:** random placement, no moves.

**Does it organise?** Locality: of each word's 10 strongest associates (PMI over the whole text, words seen 50+ times), the share with a spot within 1% of the line of one of the word's spots, against random words. Second spots raise the random baseline, so compare each row with its own baseline.

| Map | Austen | Whole corpus |
|---|---|---|
| Random placement (control) | 0.023 (random 0.022) | 0.020 (0.020) |
| Draw | 0.091 (0.131) | 0.096 (0.104) |
| Median | 0.048 (0.064) | 0.041 (0.025) |
| Links | 0.075 (0.071) | 0.086 (0.068) |

Barely. The best online map on the whole corpus, *median*, puts associates nearby 1.6 times as often as chance; on Austen none beats chance. The regions read as word salad with faint pockets (a run of Bible names, a run of Shakespeare spellings), and the second spots go mostly to common words ("know", "than", "house") with noise for neighbours.

**What could a map hold at best?** The same locality for orderings computed offline from all the co-occurrences at once (`scripts/line_ceiling.py`: spectral coordinates, or iterated barycentric sorting on a line), with the nearest 2% of words as the neighbourhood:

| Dimensions | Whole corpus | Austen |
|---|---|---|
| 1 (line, barycentric) | 0.071 (random 0.021) | 0.037 (0.027) |
| 2 (sheet) | 0.102 (0.028) | 0.052 (0.049) |
| 3 | 0.081 (0.024) | |
| 5 | 0.111 (0.027) | |
| 9 | 0.153 (0.031) | |
| 16 | 0.164 (0.032) | |
| 32 | 0.224 (0.037) | |

Even computed offline, a line keeps 7% of a word's strongest associates near it, and nine dimensions 15%. Meaning by position needs many dimensions; a low-dimensional map can hold only coarse topics, and Austen, one author in one register, has almost none to hold.

## The compressed story code

The story state lights spots (salience decaying with a half-life of 200 content words). D bits pool the map in two offset banks of windows (each spot feeds 2 bits), and the code is the k most lit bits. Two readouts predict the exact next word:
- **Rows over bits:** each code bit counts the words that follow while it is lit (learned).
- **Region readout:** each code bit spreads its share over the words whose spots it pools, by frequency so far, so a lit region can name a word the story hasn't used yet.

Gains in bits per exact word beyond the counts system of 25 (scored part: last 80% of the second half; base 8.672 on the whole corpus, 7.359 on Austen):

| Whole corpus | All words | Rare words (seen < 10 times) | Words new to the story (not in the last 2,000) |
|---|---|---|---|
| **Story edge keyed by word id ([25](25-kept-words.md))** | **+0.130** | **+0.171** | **+0.270** |
| Code rows, random placement, D 2,048, k 32 | +0.088 | +0.069 | +0.185 |
| Code rows, draw map | +0.082 | +0.040 | +0.178 |
| Code rows, median map | +0.075 | +0.024 | +0.158 |
| Code rows, links map | +0.072 | −0.007 | +0.139 |
| Code rows, draw map, D 256 / D 4,096 (k 64) | +0.071 / +0.079 | | |
| Region readout, best (draw map, D 256) | +0.057 | +0.041 | +0.192 |
| Region readout, random placement, D 256 | +0.028 | +0.022 | +0.065 |
| Story edge + code rows (+ region readout) | +0.130 | +0.171 | +0.270 |

On Austen the story edge gains +0.055, code rows +0.049 to +0.051 (random placement best), the region readout +0.002 to +0.013, and combinations at most +0.057.

- **The compressed code adds nothing to the story edge.** Every combination gives the story edge's gain or, where the weights chosen on the calibration fifth don't carry over to later books, a little less (the worst +0.123).
- **Organising the map doesn't help the code.** Random placement gives the best code rows in both corpora. The code carries which words were lit, compressed; the map's arrangement adds no meaning to it.
- **Code size barely matters** (draw map: +0.071 at D 256, +0.082 at 2,048, +0.079 at 4,096 with k 64).
- **The region readout is the only place the map shows:** with coarse regions (D 256), the draw map's lit regions name words new to the story three times as well as randomly placed ones (+0.192 against +0.065). But that is still less than the story edge, and nothing beyond it.

## Findings

1. **A map that grows and organises as it reads barely organises.** Words placed and moved by swaps toward their contexts end up only slightly closer to their associates than chance (1.6 times at best, on the whole corpus; not at all on Austen).
2. **Low-dimensional maps can't hold meaning anyway.** Even an offline ordering keeps 7% of strongest associates nearby on a line, 10% on a sheet, 15% in nine dimensions and 22% in 32.
3. **The meaning that predicts lives in the links, not in position.** The story edge keyed by word id, a localist association memory, beats every compressed code, and nothing compressed adds to it.
4. **For an index, compression is fine without organisation.** A compressed code of the lit words keeps most of what the story edge knows (+0.088 of +0.130) whether or not the map is organised. So a hippocampal index can be a random compression of the cortex's lit words.
5. **What it suggests (untested):** if similar words should share structure, the place for it is a high-dimensional distributed code built from the links (each word's code a bundle of its associates' codes), not a low-dimensional arrangement. The 2,048-bit space is such a space.

Caveats: one run per configuration; mixture weights are chosen on the first fifth of the second half. The locality baselines of runs with second spots are inflated, since random words with two spots are near more places.

## Code

- `scripts/cortex_base.py`: the shared counts system of [25](25-kept-words.md) (order 2, the memory naming "other" words, the story edge keyed by word id), per word.
- `scripts/cortex_map.py`: the growing map (placement, moves, senses; `--target draw|median|assoc`, `--place random --move 0` for the control), the compressed code (`--D`, `--k`), its readouts, scoring and the map report.
- `scripts/line_ceiling.py`: offline orderings in 1 to 32 dimensions and their locality.
- Runs: `runs/exp26/` (`*.map.txt` show regions and second spots).
