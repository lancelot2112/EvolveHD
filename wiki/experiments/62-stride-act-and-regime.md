# 62 · Value operators (a stride act) and recall by kind of material (a regime sense) (2026-10-29)

*[All experiments](README.md)* · [← 61](61-position-sense-and-primitives.md)

**The question:** experiment 61 left two untested explanations for why pretraining on primitive encodings didn't transfer:
1. the byte brain can't represent values (differences, repeats at a stride);
2. its single economy of kept edges can't hold many kinds of material at once.

Here we build a piece for each and test them.

**Results:**
- **The stride act is a clean gain where values repeat or step, at no cost elsewhere.** On experiment 57's 16 files, whole stream 3.079 → 3.034 bits per byte.
  - The first ELF file falls from 2.83 to 2.41, now below xz (2.43).
  - The second ELF file falls from 3.05 to 2.78 (xz 2.49).
  - The bitmaps gain about 0.01; nothing else moves by more than 0.002.
- **The regime sense helps the brain hold many kinds of material, and costs on some files.** It works only as a node the brain builds on: as context for the output mixtures alone it changes nothing (3.033 against 3.034).
  - **After primitives:** on this build the brain alone collapses while reading them (kept edges 9 → 2, a single learned edge left), and reads the test files at 3.31 bits per byte (with the stride act). With the regime node it keeps 6–9 edges throughout, and reads them at 3.05.
  - **With nothing before:** the brain builds about half its kept edges on the regime. The second book (+0.15) and the second JSON file (+0.10) get worse. Whole stream 3.045 against 3.034 without it.
- **Primitive pretraining still doesn't transfer:** the best run after primitives (3.047) equals the test stream alone (3.034–3.045). The stride act needs no pretraining: its predictors score themselves in a few bytes, so primitives have nothing to teach it.
- **The best so far is English first, with both pieces:** 2.919 bits per byte on the test stream (xz in the stream 2.894; order-4 counts 3.658).
- **On the made-up formats both pieces do almost nothing** (whole files with no description 3.658 → 3.651). Their values are random numbers under 1,000 and their counts random: there is nothing for strides to predict.

## The rules (binaries `sit-62a`, `sit-62b`; `src/acts.rs`)

**`--stride-act`: value operators as an act.**
- **The predictors:** for each stride k (1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 128, 192, 256, 384, 512, 600, 768, 1024), two predictors of the next byte:
  - *repeat:* the byte k back;
  - *step:* the byte k back plus its change from the byte 2k back (mod 256).
- **Scores per lane:** each predictor keeps a recent score per lane (the byte's offset mod 8 from its file's start, or t mod 8 without file starts). The score is s ← s − s/8 + 32 when right.
- **The offer:** the act offers the byte of the best-scoring predictor in this lane (if its score is at least 32).
- **Trust:** counted per context (score level in eighths × repeat or step × regime), prequential, halved past 4,096.

**`--regime W,K`: the regime sense.**
- **The window:** the byte counts of the last W bytes (here 128), matched every 16 bytes to learned centroids by cosine similarity.
- **Clusters:** a window less than 0.6 similar to all of them starts a new centroid (up to K, here 16); otherwise the best one moves 1/50 of the way toward it.
- **The regime** is the best centroid's index.
- **How it is used:**
  - offered as a node (a sense like the position nodes, never collected);
  - part of the context of every output mixture: the floor (per regime), the copy act's contexts (× regime) and the stride act's contexts (× regime).
- **`--regime-no-node`:** the mixture contexts only.

**The code:** the brain's floored code, then the copy act's mixture (experiment 59), then a second switching mixture (fixed share 1/1000) over that code, the stride act and a uniform guess, one per stride context.

**Settings for every run:** floor, guard, two maps (127), copy act, position sense (file starts given). Binary `sit-61c` (the collection fix of experiment 61) for the baselines.

## The test stream alone (bits per byte)

| | Type | WB4 | xz | Base | + stride | + regime | + both | + stride, regime as context only |
|---|---|---|---|---|---|---|---|---|
| A | text | 2.813 | 3.157 | 3.666 | 3.668 | 3.661 | 3.663 | 3.668 |
| A | json | 0.709 | 0.408 | 0.570 | 0.572 | 0.567 | 0.568 | 0.572 |
| A | csv | 3.172 | 2.900 | 2.774 | 2.776 | 2.739 | 2.742 | 2.777 |
| A | rust | 2.695 | 2.337 | 2.411 | 2.412 | 2.375 | 2.377 | 2.412 |
| A | xml | 1.250 | 0.913 | 0.952 | 0.954 | 0.950 | 0.952 | 0.954 |
| A | elf | 4.080 | 2.431 | 2.833 | 2.405 | 2.786 | 2.397 | 2.386 |
| A | bmp | 1.517 | 1.049 | 1.247 | 1.234 | 1.240 | 1.228 | 1.230 |
| B | text | 3.031 | 3.168 | 3.413 | 3.414 | 3.562 | 3.564 | 3.414 |
| B | json | 1.754 | 1.273 | 1.496 | 1.497 | 1.593 | 1.595 | 1.495 |
| B | csv | 3.559 | 3.064 | 3.233 | 3.233 | 3.257 | 3.257 | 3.235 |
| B | rust | 2.719 | 2.712 | 2.751 | 2.753 | 2.771 | 2.773 | 2.753 |
| B | xml | 2.355 | 1.655 | 1.725 | 1.727 | 1.724 | 1.725 | 1.725 |
| B | elf | 3.539 | 2.487 | 3.054 | 2.775 | 2.979 | 2.746 | 2.768 |
| B | bmp | 3.447 | 2.567 | 3.147 | 3.134 | 3.137 | 3.127 | 3.135 |
| All | | 3.658 | 2.894 | 3.079 | 3.034 | 3.084 | 3.045 | 3.033 |
| A without gzip | | 2.319 | 1.885 | 2.065 | 2.003 | 2.045 | 1.989 | 2.000 |
| B without gzip | | 2.915 | 2.418 | 2.688 | 2.647 | 2.718 | 2.684 | 2.646 |

The gzip files are 8.00 in every run.

- **The stride act** made 345,038 offers on the test stream alone, 24.8% right.
- **The regime sense** used all 16 regimes; several hold only a few windows (2–92 of 24,000).
- **Kept with the regime node** (test stream alone): (regime × byte), regime, (regime × chunk), and binds of regime with recent bytes.

## After pretraining (test stream, bits per byte)

Experiment 61's streams: 1,021,800 bytes of primitives (P), or of English from books other than the test books (C2), then the test stream.

| Before the test | Base | + stride | + regime | + both | + stride, regime as context only |
|---|---|---|---|---|---|
| Nothing | 3.079 | 3.034 | 3.084 | 3.045 | 3.033 |
| Primitives | 3.368 | 3.314 | 3.094 | 3.047 | 3.311 |
| English | 2.990 | 2.945 | 2.959 | **2.919** | 2.943 |

**Kept edges while reading primitives and then the test** (every 200,000 bytes; counts include the byte's own edge):
- base: 9, 7, 4, 2, 2, 2, 2;
- + stride: 9, 7, 4, 2, 2, 2, 2;
- + both: 8, 8, 8, 9, 7, 5, 6.

The base and stride runs end with a single learned edge (byte@2).

- **The primitive runs vary a lot between builds,** with the same streams and flags but small code changes: 3.336 (`sit-61a`), 3.088 (`sit-61b`), 3.368 (`sit-61c`). Two of the three collapsed. Single runs of this test should be read with that in mind.
- **After English, the regime sense helps the second book** (3.52 → 3.36) and the second JSON file (1.58 → 1.48), and costs the first book a little (2.11 → 2.14). It was the other way round with nothing before.

## The made-up formats (experiment 60's `fmt2`, brain alone, position sense)

| File bytes | Base (`sit-62a`) | + both |
|---|---|---|
| with the right description (R) | 3.494 | 3.458 |
| with a wrong one (W) | 3.586 | 3.547 |
| with none (N) | 3.658 | 3.651 |

Fields without a description (base → both):

| Field | Base | + both |
|---|---|---|
| count | 5.198 | 5.370 |
| version | 5.969 | 6.131 |
| constants | 7.385 | 7.241 |
| kind | 6.208 | 6.137 |

The stride act made 574,857 offers, 13.2% right.

## What this means

- **Value operators belong in the reader as acts, and they need no curriculum.** "The byte k back" and "the byte k back plus its last change" catch most of what made executables hard (repeated instruction patterns with a changed field, tables of offsets). They are right within a few bytes of seeing a stride, so pretraining has nothing to add. What primitives could still teach is what the act can't do: carries across bytes, a value seen in two forms, numbers that are not near the last one.
- **Recall by kind is real, but needs a better form.** As a sense the brain can bind with, the regime keeps the brain from collapsing on mixed material. That is the explanation experiment 61 gave for the failed pretraining, now with evidence. But the brain then builds its main edges on the regime, and where the regime clusters are noisy (many small ones) that costs. Two refinements to try:
  - fewer, steadier regimes (a slower or merged clustering);
  - the regime as a gate on edges (an edge used only in the regimes where it has paid) instead of a source edges bind with.
- **The default byte stack is now** floor, guard, two maps, copy act, stride act, position sense. Add the regime sense for mixed material.

## Next

1. **Regime refinements:** merge clusters that are rarely used, and try the regime as a per-edge gate.
2. **Carries and wider values for the stride act:** treat 2-, 4- and 8-byte little- and big-endian lanes as numbers (step the whole number, not each byte), for counters that cross byte boundaries.
3. **Back to the descriptions (experiment 60):** with position, strides and regimes in place, try the description expert inside the stack.
