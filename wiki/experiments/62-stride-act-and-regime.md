# 62 · Value operators (a stride act) and recall by kind of material (a regime sense) (2026-10-29)

*[All experiments](README.md)* · [← 61](61-position-sense-and-primitives.md)

**The question:** experiment 61 left two untested explanations for why pretraining on primitive encodings didn't transfer:
1. the byte brain can't represent values (differences, repeats at a stride);
2. its single economy of kept edges can't hold many kinds of material at once.

Here we build a piece for each and test them.

**Results:**
- **The stride act is a clean gain where values repeat or step, at almost no cost elsewhere.** On experiment 57's 16 files, whole stream 3.079 → 3.034 bits per byte.
  - The first ELF file falls from 2.83 to 2.41, now below xz (2.43).
  - The second ELF file falls from 3.05 to 2.78 (xz 2.49).
  - The bitmaps gain about 0.01. Every text, JSON, csv, Rust and xml file gets 0.001–0.002 worse, on all three streams: a small, consistent cost of the extra mixture.
  - The comparison is clean: the act sits after the brain, so the runs with and without it have the same kept edges byte for byte.
- **The regime sense helps the brain hold many kinds of material, and costs on some files.** It works only as a node the brain builds on: as context for the output mixtures alone it changes nothing (3.033 against 3.034).
  - **After primitives:** on this build the brain alone collapses while reading them (kept edges 9 → 2, a single learned edge left), and reads the test files at 3.31 bits per byte (with the stride act). With the regime node it keeps 5–9 edges throughout, and reads them at 3.05.
  - **With nothing before:** the brain builds 5 of its 7 learned edges on the regime. The second book (+0.15) and the second JSON file (+0.10) get worse. Whole stream 3.045 against 3.034 without it.
  - **A confound, found by the check:** adding the regime node draws a seed from the brain's shared random generator, so every later bind and chunk node gets different random codes. The regime runs are therefore different brains for a second reason, and primitive runs swing a lot by themselves (3.09–3.37 between builds). At the end of the default seed's primitive run with the regime node, no kept edge involves the regime (in the seed study's other two seeds most do).
- **The seed study settles part of it** (three seeds each, stride act on, with and without the regime node):
  - **After primitives the regime node makes the brain robust:** 3.047 / 3.019 / 3.034 bits per byte (mean 3.033), against 3.314 / 3.223 / 3.058 without (mean 3.198). Without it, one seed collapses (2 kept edges at the end), one partly (4) and one doesn't (5). With it, all three end with 6.
  - **With nothing before it makes no difference overall:** mean 3.043 against 3.041. It costs the second book in two of three seeds (+0.13 to +0.15) and not in the third.
  - **Primitives against nothing,** with the regime node: 3.033 against 3.043. They are about equal; the seeds overlap.
- **Primitive pretraining still doesn't transfer:** the best run after primitives (3.047) equals the test stream alone (3.034–3.045). The stride act needs no pretraining: its predictors score themselves in a few bytes, so primitives have nothing to teach it.
- **The best so far, with English that excludes the test books, is English first with both pieces:** 2.919 bits per byte on the test stream (xz in the stream 2.894; order-4 counts 3.658). Experiment 61's first English control, which held test-book passages, reached 2.893 with harm-only pruning.
- **On the made-up formats both pieces do little.** Files without a description: 3.658 → 3.651; with the right one 3.494 → 3.458, with a wrong one 3.586 → 3.547. Magic bytes gain (0.26 with the right description, 0.48 with a wrong one); count and version without a description get worse (0.17 and 0.16). Values are random numbers under 1,000 and counts random 3–12, so there is little for strides to predict.

## The rules (binaries `sit-62a`, `sit-62b`; `src/acts.rs`)

**`--stride-act`: value operators as an act.**
- **The predictors:** for each stride k (1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 128, 192, 256, 384, 512, 600, 768, 1024), two predictors of the next byte:
  - *repeat:* the byte k back;
  - *step:* the byte k back plus its change from the byte 2k back (mod 256).
- **Scores per lane:** each predictor keeps a recent score per lane (the byte's offset mod 8 from its file's start, or t mod 8 without file starts). The score is s ← s − s/8, plus 32 when right.
- **The offer:** the act offers the byte of the best-scoring predictor in this lane (if its score is at least 32; ties go to the first, the shortest stride, repeat before step).
- **Trust:** counted per context (score level 1–7, the top merged into 7, × repeat or step, plus a no-offer context; × regime), prequential, halved past 4,096.

**`--regime W,K`: the regime sense.**
- **The window:** the byte counts of the last W bytes (here 128), matched every 16 bytes to learned centroids by cosine similarity.
- **Clusters:** a window less than 0.6 similar to all of them starts a new centroid (up to K, here 16); otherwise the best one moves 1/50 of the way toward it and is renormalised.
- **The regime** is the best centroid's index (in the mixtures, K + 1 values: 0 means none yet).
- **How it is used:**
  - offered as a node (a sense like the position nodes, never collected);
  - part of the context of every output mixture: the floor (per regime), the copy act's contexts (× regime) and the stride act's contexts (× regime).
- **`--regime-no-node`:** the mixture contexts only.

**The code:** the brain's floored code, then the copy act's mixture (experiment 59), then a second switching mixture (fixed share 1/1000) over that code, the stride act and a uniform guess, one per stride context. The stride mixture runs only with the copy act on.

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
- **The regime sense** used all 16 regimes, but 5 of them hold 99% of the windows (23,789 of 23,993); the other 11 hold 2–92 each.
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

## The seed study (stride act on; `--seed 1`, `--seed 2` and the default)

`runs/exp61/seeds62.sh`, binary `sit-62b`.

| | Seeds (default / 1 / 2) | Mean |
|---|---|---|
| Nothing before, no regime | 3.034 / 3.051 / 3.038 | 3.041 |
| Nothing before, regime node | 3.045 / 3.040 / 3.045 | 3.043 |
| Primitives before, no regime | 3.314 / 3.223 / 3.058 | 3.198 |
| Primitives before, regime node | 3.047 / 3.019 / 3.034 | 3.033 |

- **Kept edges at the end:** primitives without the regime 2 / 4 / 5; with it 6 / 6 / 6. Nothing before: 6 / 8 / 5 without, 6 / 5 / 5 with.
- **Second book, nothing before:** 3.414 / 3.350 / 3.430 without the regime, 3.564 / 3.437 / 3.562 with it.
- **Second JSON file:** 1.497 / 1.515 / 1.524 without the regime, 1.595 / 1.504 / 1.543 with it.

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

- **Value operators belong in the reader as acts.** "The byte k back" and "the byte k back plus its last change" close the first ELF file's gap to xz and about half of the second's. Their scores reach the offer threshold after one right guess, so they need little pretraining. Their trust counts and mixture weights do carry over, and the act gained slightly more after primitives (0.054) than with nothing before (0.045), but on a collapsed brain. What primitives could still teach is what the act can't do: carries across bytes, a value seen in two forms, numbers that are not near the last one.
- **Recall by kind helps the brain survive mixed material.** Over three seeds, the regime node takes the run after primitives from a lottery (3.06–3.31) to a steady 3.02–3.05. How is not clear: at the end of the default seed's run no kept edge involves the regime, while in seeds 1 and 2 four or five of the five learned edges do. So it may protect structure while the material changes, carry it, or both. It does not make primitives teach anything. Two refinements to try:
  - fewer, steadier regimes (a slower or merged clustering);
  - the regime as a gate on edges (an edge used only in the regimes where it has paid) instead of a source edges bind with.
- **The default byte stack is now** floor, guard, two maps, copy act, stride act, position sense. Add the regime sense for mixed material.

## Next

1. **Regime refinements:** merge clusters that are rarely used, and try the regime as a per-edge gate.
2. **Carries and wider values for the stride act:** treat 2-, 4- and 8-byte little- and big-endian lanes as numbers (step the whole number, not each byte), for counters that cross byte boundaries.
3. **Back to the descriptions (experiment 60):** with position, strides and regimes in place, try the description expert inside the stack.
