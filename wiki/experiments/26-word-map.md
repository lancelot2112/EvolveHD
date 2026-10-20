# 26 · A growing word map and a compressed story code (2026-10-19)

*[All experiments](README.md)* · [← 25](25-kept-words.md) · [27 →](27-meaning-codes.md)

**Concepts:** [Sparse codes](../concepts/sparse-codes.md) · [Growth operators](../concepts/growth-operators.md#story-state) · [Stepping stones](../concepts/stepping-stones.md)

Question: [25](25-kept-words.md) kept every word as its own key, and the story state paid 0.30 bits per exact word. Could meaning be organised underneath it? The plan: a "cortex" that gives each word its own spot and lets related words gather by swapping positions, so a stretch of the map is a topic, and a compression of the lit map into a fixed-width code (like a hippocampal index) that carries the story's meaning, not just its words. Everything must grow and organise as it reads, in one pass.

Counts prototypes (`scripts/cortex_map.py`), one run per configuration, scored on exact-word bits as in [25](25-kept-words.md), beyond the counts system of 25 (order-2 counts over the 512 commonest words, and a decaying memory of recent words that names "other" words; `scripts/cortex_base.py`).

## The map

A line of spots that starts empty and grows:
- **Placement.** A content word (outside the 100 commonest) gets a spot on first sight, next to its target (below; for *links*, next to a recent story word drawn by salience).
- **Moves.** On each later occurrence, its spot moves part of the way toward its target, shifting the spots in between one place each: a run of swaps. The step shrinks as the word is seen more (`max(0.02, 0.5/√n)`).
- **Targets tried:**
  - *draw*: one recent story word drawn by salience;
  - *median*: the salience-weighted median of everything lit;
  - *links*: the weighted median of the word's 8 strongest learned links. The map counts, for each word, the content words among the 16 before each of its occurrences, keeps those seen twice or more with positive PMI, and uses each linked word's first spot. The target is recomputed on each of a word's first 64 occurrences, then every 8th.
- **Senses.** A word can get a second spot. For *draw* and *median*: once a word has 32 occurrences, at every 16th, its last 16 targets are split into two groups, and a spot is added if the group medians are more than 10% of the line apart. For *links*: if its link positions have a gap of more than 10% of the line, with at least 3 links on each side. An occurrence uses the spot nearest its context. The *median* runs added no second spots.
- **Control:** random placement, no moves, no second spots.

**Does it organise?** Locality: of each word's 10 strongest associates (PMI over the whole text; associates need 5 co-occurrences within 10 content words; words seen 50+ times with at least 10 such associates), the share with a spot within 1% of the line of one of the word's spots. The baseline is the same share for random words from the same pool.

| Map | Austen | Whole corpus |
|---|---|---|
| Random placement (control) | 0.023 (random 0.022) | 0.020 (0.020) |
| Draw | 0.091 (0.131) | 0.096 (0.104) |
| Median | 0.048 (0.064) | 0.041 (0.025) |
| Links | 0.075 (0.071) | 0.086 (0.068) |

Barely, if at all, and the comparison is muddied. The baselines of the organised maps are inflated: by second spots (the *draw* runs gave nearly every frequent word one), and because the maps sort words by frequency, so frequent words crowd into one stretch (in the Austen *median* map, 589 of the 683 frequent words sit between 30% and 60% of the line). Against its own baseline, the best ratio is *median* on the whole corpus (1.6 times), though in absolute terms it is the least local organised map. On Austen no map clearly beats its baseline. Read directly, the regions are word salad with faint pockets (a run of Bible names, a run of Shakespeare spellings), and the second spots go mostly to common words ("know", "than", "house") with unrelated neighbours.

**How much can an ordering hold?** The same locality for orderings computed offline from all the co-occurrences at once (`scripts/line_ceiling.py`, words seen 3+ times, one spot each), with the nearest 2% of words as the neighbourhood. These are heuristic orderings, not true upper bounds: on a line, barycentric sorting beats the spectral ordering (0.071 against 0.050), and sampling noise is about ±0.002.

| Dimensions | Whole corpus | Austen |
|---|---|---|
| 1 (line, barycentric sorting) | 0.071 (random 0.021) | 0.037 (0.027) |
| 2 (spectral) | 0.104 (0.029) | 0.052 (0.049) |
| 3 (spectral) | 0.081 (0.024) | |
| 5 (spectral) | 0.111 (0.027) | |
| 9 (spectral) | 0.153 (0.031) | |
| 16 (spectral) | 0.164 (0.032) | |
| 32 (spectral) | 0.224 (0.037) | |

Even computed offline, a line keeps about 7% of a word's strongest associates near it, nine dimensions 15% and 32 dimensions 22%. Meaning by position needs many dimensions; a low-dimensional map can hold only coarse topics, and Austen, one author in one register, has almost none to hold.

## The compressed story code

The story state lights spots (salience decaying with a half-life of 200 content words). D bits pool the line in two offset banks of windows (each spot feeds 2 bits). A spot's light is added to the bits for where it sits when it is lit, and decays there: the code is the k most lit bits, a decaying histogram of where lit words were. Two readouts predict the exact next word:
- **Rows over bits:** each code bit counts the words that follow while it is lit (learned).
- **Region readout:** each code bit spreads its share over the words whose spots it pools now (a snapshot of the map every 256 words), by frequency so far, so a lit region can name a word the story hasn't used yet.

Gains in bits per exact word beyond the counts system of 25 (scored part: the last 80% of the second half; base 8.672 on the whole corpus, 7.359 on Austen). Rare words have been seen fewer than 10 times; words new to the story are content words not in the previous 2,000 words.

| Whole corpus | All words | Rare words | Words new to the story |
|---|---|---|---|
| **Story edge keyed by word id ([25](25-kept-words.md))** | **+0.130** | **+0.171** | **+0.270** |
| Code rows, random placement, D 2,048, k 32 | +0.088 | +0.069 | +0.185 |
| Code rows, draw map | +0.082 | +0.040 | +0.178 |
| Code rows, median map | +0.075 | +0.024 | +0.158 |
| Code rows, links map | +0.072 | −0.007 | +0.139 |
| Code rows, draw map, D 256 / D 4,096 with k 64 | +0.071 / +0.079 | | |
| Region readout, draw map, D 256 | +0.057 | +0.041 | +0.192 |
| Region readout, links map, D 256 | +0.032 | +0.034 | +0.092 |
| Region readout, random placement, D 256 | +0.028 | +0.022 | +0.065 |
| Story edge + code rows, every map | +0.130 | +0.171 | +0.270 |
| Story edge + code rows + region readout, worst (random placement) | +0.123 | +0.134 | +0.217 |

On Austen (D 2,048 only): the story edge gains +0.055; code rows +0.049 to +0.051, random placement highest; the region readout +0.002 to +0.013, random placement highest overall (draw is better only on words new to the story); combinations +0.052 to +0.057.

- **The compressed code adds essentially nothing to the story edge.** On the whole corpus, with code rows the mixture gives the story edge exactly its own gain; adding the region readout gives slightly less overall and up to a fifth less on rare and new words, as the weights chosen on the calibration fifth fit the later part worse. On Austen the best combination is +0.002 above the story edge, and worse on rare and new words.
- **Organising the map doesn't help the code.** Random placement gives the highest code rows in both corpora, by small margins (0.002 on Austen, 0.006 on the whole corpus, single runs). The code carries which words were lit, compressed; the arrangement adds no meaning to it.
- **Code size changes little:** with the draw map, +0.071 at D 256, +0.082 at 2,048, +0.079 at 4,096 (with k 64, so size and sparsity both change).
- **The region readout is where an organised map shows at all:** with coarse regions (D 256), the draw map's lit regions name words new to the story about three times as well as random placement (+0.192 against +0.065), the links map's 1.4 times (+0.092). The median map was not run at D 256. It is still less than the story edge, and adds nothing beyond it.

## Findings

1. **A map that grows and organises as it reads barely organises.** Words placed and moved by swaps toward their contexts end up at best slightly closer to their associates than chance, and frequency banding muddies even that.
2. **Low-dimensional maps can't hold much meaning anyway.** Offline orderings keep about 7% of strongest associates nearby on a line, 10% on a sheet, 15% in nine dimensions and 22% in 32.
3. **The meaning that predicts lives in the links, not in position.** The story edge keyed by word id, a localist association memory, beats every compressed code, and nothing compressed adds to it beyond noise.
4. **A compressed code needs no organisation, but it loses detail.** A compression of the lit words keeps +0.088 of the story edge's +0.130 overall, organised or not, but only 40% of its gain on rare words. An index built this way would address stories coarsely; the cortex's links still have to supply the specifics.
5. **What it suggests (untested):** if similar words should share structure, the place for it is a high-dimensional distributed code built from the links (each word's code a bundle of its associates' codes), not a low-dimensional arrangement. The 2,048-bit space is such a space. Tested in [27](27-meaning-codes.md): words group there, ten times above chance.

## Code

- `scripts/cortex_base.py`: the shared counts system of [25](25-kept-words.md) (order 2, the memory naming "other" words, the story edge keyed by word id), per word.
- `scripts/cortex_map.py`: the growing map (placement, moves, senses; `--target draw|median|assoc`, where *assoc* is *links*; `--place random --move 0 --senses 1` for the control), the compressed code (`--D`, `--k`), its readouts, scoring and the map report.
- `scripts/line_ceiling.py`: offline orderings in 1 to 32 dimensions and their locality.
- Runs: `runs/exp26/` (`*.map.txt` show regions and second spots). The locality lines in the logs of the D 256 and 4,096 runs and of `a_d2048`, `a_mut`, `a_med_mut` come from an earlier version that counted a word as its own associate; their prediction numbers are unaffected.
