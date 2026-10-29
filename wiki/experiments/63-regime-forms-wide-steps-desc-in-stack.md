# 63 · Steadier regimes, whole-number steps, and the description expert inside the stack (2026-10-29)

*[All experiments](README.md)* · [← 62](62-stride-act-and-regime.md)

Experiment 62's three next steps, in order.

**Results:**
- **Regimes: fewer is worse; a gate helps with nothing before the files; node and gate together are the best balance** (three seeds each, stride act on):

  | Regime | Nothing before | After primitives |
  |---|---|---|
  | none | 3.041 (3.034–3.051) | 3.198 (3.058–3.314) |
  | 16, as a node (exp 62) | 3.043 (3.040–3.045) | **3.033** (3.019–3.047) |
  | 4, as a node | 3.041 (3.034–3.048) | 3.178 (3.033–3.304) |
  | 16, as a gate | 3.030 (3.027–3.032) | 3.159 (3.097–3.201) |
  | 4, as a gate | 3.031 (3.027–3.034) | 3.193 (3.077–3.396) |
  | **16, as node and gate** | **3.027** (3.025–3.030) | 3.070 (3.045–3.097) |

  - **Fewer regimes lose the node's robustness.** With 4 regimes, the run after primitives is a lottery again.
  - **The gate helps with nothing before the files,** on every seed. It is the first regime form to do so, mostly on the second book (3.41 → 3.29 on seed 0).
  - **After primitives the gate alone is not robust.** It keeps 16–19 edges alive, each because it "pays somewhere", but the brain it leaves reads the test files worse than the node's.
  - **Node and gate together:** best with nothing before, no collapse after primitives (0.04 behind the node alone there), and the node's cost on the second book is gone (3.39–3.41 against 3.44–3.56).
- **Whole-number steps add almost nothing.** Test stream 3.034 → 3.035. Integer primitives 1.515 → 1.516. The one gain is floats: 5.42 → 5.02 over the float primitives (a float ramp falls from 2.13 to 0.94). Byte-by-byte step and repeat already predict nearly every byte of a counter; carries are rare in these files.
- **The description expert works inside the stack, but less well than the prototype.** On experiment 60's formats stream (same brain with and without it), all file bytes:
  - with the right description 3.494 → 3.431;
  - with a wrong one 3.586 → 3.521;
  - with none 3.658 → 3.624.
  - **Magic bytes of never-seen formats** (second half): mixed, 8.83 with the right description against 9.84 with a wrong one and 9.05 with none. The right description is worth 1.0 bit per byte over a wrong one; the prototype's mixture gave 6.30 / 9.11 / 8.43.
  - The expert alone: 6.78 / 9.18 / 8.36, against the prototype's 5.04 / 9.02 / 8.00.
  - **Why it is weaker:** inside the stack it has no way to know which segment is a description. It treats the segment before every file as that file's description: the previous file before a description, and the previous file before a file with none. So it learns from segments that describe nothing, and its mixture is shared with them. The prototype was told which bytes were files and which descriptions. The description bytes themselves now cost a little more (0.578 → 0.603).

## Regimes as a gate (`--regime-gate`, binary `sit-63a`)

- **Measured per regime:** each kept edge's gain on each byte (leave-one-out: the cost without it against with it; for an edge left out, what it would have added) is averaged per regime, over about 256 bytes (`--regime W,K` gives the regimes).
- **Gated:** an edge is left out of the prediction in a regime once it has been measured there 256 times and its average gain there is below zero. It is still measured, so it can come back.
- **Pruning:** an edge is not pruned while its gain in some regime is at least the prune bar over 1,000 or more bytes ("kept for its kind"). It is judged again in the next window.
- **Two forms:** `--regime-no-node` gives the gate alone; without it the regime is also a node.
- **Fewer regimes:** `--regime 128,4` (and `--regime-thr X` for the similarity below which a window starts a new regime, default 0.6; not varied here).
- **Cost:** the gated runs take about twice as long (every kept edge is measured on every byte).

Seeds 0, 1, 2 (`--seed`); scripts `runs/exp63/regimes.sh`, `runs/exp63/both.sh`; stride act on, floor, guard, two maps, copy act, position sense; test stream alone (Z) and after experiment 61's primitives (P).

**Kept edges after primitives** (every 200,000 bytes; counts include the byte's own edge):

| Regime form | Seed 0 | Seed 1 | Seed 2 |
|---|---|---|---|
| 16, gate | 14, 19, 19, 18, 17, 19, 19 | 14, 19, 16, 18, 16, 16, 16 | 12, 19, 18, 18, 17, 18, 18 |
| 16, node and gate | 11, 16, 16, 17, 17, 18, 18 | 11, 15, 16, 20, 19, 20, 20 | 11, 16, 19, 18, 15, 19, 20 |
| 4, node | 6, 7, 8, 5, 8, 5, 5 | 8, 10, 7, 5, 5, 5, 4 | 7, 9, 5, 3, 3, 3, 3 |

**Second book (nothing before):**

| Regime form | Seeds 0 / 1 / 2 |
|---|---|
| none | 3.414 / 3.350 / 3.430 |
| 16, node | 3.564 / 3.437 / 3.562 |
| 16, gate | 3.286 / 3.271 / 3.357 |
| 16, node and gate | 3.386 / 3.399 / 3.414 |

## Whole-number steps (`--stride-wide`, binary `sit-63b`)

- **The predictors:** besides repeat and step at each stride, the stride act gets number predictors for widths 2, 4 and 8 (little and big endian) at every stride that is a multiple of the width.
- **The prediction:** the w-byte number k back plus its change from the number 2k back, as a whole number with carries. The byte offered is the current byte's place in that number (its lane modulo w gives the place).
- **Choice and trust:** number predictors compete per lane with the others, with their own trust contexts.
- **A clean comparison:** the act sits after the brain, so the runs with and without it have the same brain (identical kept edges).

| | Byte steps | + whole-number steps |
|---|---|---|
| Test stream alone | 3.034 | 3.035 |
| After primitives | 3.314 | 3.314 |
| Primitives during pretraining: int | 1.515 | 1.516 |
| Primitives during pretraining: float | 5.418 | 5.022 |

Offers on the test stream: 345,038 (24.8% right) against 349,217 (24.5% right).

## The description expert in the stack (`--desc-expert`, binary `sit-63c`)

- **The rule:** experiment 60's expert, rewritten in Rust as a last mixture stage after the copy and stride acts.
- **What it reads:** the segment from the previous file start to this one is read as this file's description, with the same tokens, alignment, background and learning rules as the prototype.
- **The mixture:** fixed share 1/1000 over the code so far, the expert and a uniform guess, per context: offset in the segment (0–7, 8+) × the expert's confidence (quarters).
- **The test:** runs with and without it on experiment 60's progressive formats stream (`fmt2`), position sense on, stride act off. The brain is the same in both (identical symbols). Script `runs/exp63/desc.sh`.

| File bytes | Without | With the expert | The prototype (exp 60, mixed) |
|---|---|---|---|
| right description | 3.494 | 3.431 | 3.300 |
| wrong description | 3.586 | 3.521 | 3.426 |
| no description | 3.658 | 3.624 | 3.608 |
| first files, right | 3.874 | 3.770 | 3.574 |
| first files, wrong | 3.810 | 3.721 | 3.591 |
| first files, none | 3.766 | 3.717 | 3.711 |

The prototype mixed with experiment 60's brain run (binary `sit-60a`, no position sense), so its column is not the same brain.

**Stated fields of never-seen formats** (second half; without / with the expert / the expert alone):

| Field | Right description | Wrong description | None |
|---|---|---|---|
| magic | 11.09 / 8.83 / 6.78 | 11.08 / 9.84 / 9.18 | 9.67 / 9.05 / 8.36 |
| version | 7.34 / 7.34 / 6.75 | 7.16 / 7.16 / 6.40 | 7.18 / 7.19 / 7.53 |
| constant | 8.57 / 8.26 / 8.00 | 8.27 / 7.94 / 8.07 | 8.22 / 7.79 / 6.44 |

## What this means

- **Recall by kind takes both uses of the regime.**
  - The node lets the brain build structure that survives a change of material.
  - The gate keeps structure from acting where it doesn't pay.
  - Together they are steady over seeds and help in both settings.
  - Fewer regimes are worse: the kinds of material here need more than 4.
- **The stride act's gains came from byte-level repeat and step; whole numbers don't add.** Carries matter for counters that cross byte boundaries often, which these files don't have.
- **A description expert needs to know what a description is.** In the stack it guesses (the segment before every file) and loses about half of the prototype's gain on magic bytes. That is a role, and roles are what the word reader learns (its act path and question boundaries). So the next step for descriptions is the word reader: let it read the description, and have its facts answer the expert's question "what does the description say about here?".

## Next

1. **Default stack for mixed material:** floor, guard, two maps, copy act, stride act, position sense, regime as node and gate.
2. **The description expert with a learned notion of "description":** only segments the word reader reads as text are taken as descriptions, or the expert's trust is conditioned on the previous segment's regime.
3. **Larger mixed corpora** to see whether the regime's benefit grows with more kinds of material.
