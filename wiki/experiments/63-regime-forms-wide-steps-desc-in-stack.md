# 63 · Steadier regimes, whole-number steps, and the description expert inside the stack (2026-10-29)

*[All experiments](README.md)* · [← 62](62-stride-act-and-regime.md) · [64 →](64-description-roles.md)

Experiment 62's three next steps, in order.

**Results:**
- **Regimes: fewer is worse after primitives; a gate helps a little with nothing before the files; node and gate together are a good balance** (three seeds each, stride act on):

  | Regime | Nothing before | After primitives |
  |---|---|---|
  | none | 3.041 (3.034–3.051) | 3.198 (3.058–3.314) |
  | 16, as a node (exp 62) | 3.043 (3.040–3.045) | **3.033** (3.019–3.047) |
  | 4, as a node | 3.041 (3.034–3.048) | 3.178 (3.033–3.304) |
  | 16, as a gate | 3.030 (3.027–3.032) | 3.159 (3.097–3.201) |
  | 4, as a gate | 3.031 (3.027–3.034) | 3.193 (3.077–3.396) |
  | **16, as node and gate** | **3.027** (3.025–3.030) | 3.070 (3.045–3.097) |

  - **Fewer regimes lose the node's robustness after primitives.** With 4 regimes, the run after primitives is a lottery again. With nothing before, 4 and 16 regimes are the same. Only 4 and 16 were tried.
  - **The gate helps a little with nothing before the files.** With 16 regimes it is better on every seed (by 0.002, 0.023 and 0.007; mean 0.011), mostly on the second book (3.41 → 3.29 on seed 0). The no-regime seeds themselves spread over 0.016, so this is a small effect.
  - **After primitives the gate alone is not robust.** It keeps 16–19 edges alive, each because it "pays somewhere", but the brain it leaves reads the test files worse than the node's.
  - **Node and gate together:**
    - With nothing before: 3.027, as good as the gate alone (their seed ranges overlap).
    - After primitives: no collapse, though the node alone is better on every seed (by 0.02–0.08) and steadier.
    - The node's cost on the second book is gone on average (3.400 against 3.398 with no regime), though seed 1 is still worse (3.399 against 3.350).
- **Whole-number steps add almost nothing** (one run; clean, since the brain is the same). Test stream 3.034 → 3.035. Integer primitives 1.515 → 1.516. The one gain is floats: 5.42 → 5.02 over the float primitives (a float ramp falls from 2.13 to 0.94). Byte-by-byte step and repeat already predict nearly every byte of a counter (the counter primitives cost 0.07–0.13 bits per byte with byte steps alone). How often carries occur was not measured.
- **The description expert works inside the stack, but less well than the prototype.** On experiment 60's formats stream (same brain with and without it), all file bytes:
  - with the right description 3.494 → 3.431;
  - with a wrong one 3.586 → 3.521;
  - with none 3.658 → 3.624.
  - **Magic bytes of never-seen formats** (second half): mixed, 8.83 with the right description against 9.84 with a wrong one and 9.05 with none. The right description is worth 1.0 bit per byte over a wrong one; the prototype's mixture gave 6.30 / 9.11 / 8.43 (2.8 bits).
  - So the stack keeps 48% of the prototype's gain on magic bytes against its own base (2.26 against 4.75 bits) and 36% of its right-versus-wrong gap.
  - The expert alone: 6.78 / 9.18 / 8.36, against the prototype's 5.04 / 9.02 / 8.00.
  - **Why it is weaker is not tested.** One run, no ablation. Candidates:
    - **It doesn't know which segment is a description.** It treats the segment before every segment as its description. The stream marks description segments as well as files, so before a description that is the previous file, and before a file with no description it is also the previous file. The prototype was told which bytes were files and which descriptions.
    - **Its rules differ slightly.** It drops tokens over 32 bytes; it takes the jump from the pure posterior, not the r-mixed belief; its background and shares learn on every segment, descriptions included; its mixture runs on all bytes. The last is why the description bytes cost a little more (0.578 → 0.603).
    - **The base brain differs.** The prototype's base coded file bytes better (3.438 against 3.494 with the right description). On magic bytes the two bases are close (11.05 against 11.09).
    - **Against the first explanation:** with no description the previous file is not uninformative. The expert alone gets 6.44 on constants there, and mixing still helps magic (9.67 → 9.05) and constants (8.22 → 7.79).

## Regimes as a gate (`--regime-gate`, binary `sit-63a`)

- **Measured per regime:** each kept edge's gain on each byte (leave-one-out: the cost without it against with it; for an edge left out, what it would have added) is averaged per regime, over about 256 bytes (`--regime W,K` gives the regimes).
- **Gated:** an edge is left out of the prediction in a regime once it has been measured there 256 times and its average gain there is below zero. It is still measured, so it can come back.
- **Pruning:** an edge is not pruned while, in some regime, it has been measured 1,000 times or more and its recent average gain there (the same average over about 256 bytes) is at least the prune bar (0.003 bits). It is judged again in the next window. The report's "prune judgments skipped" counts these with the guard's skips.
- **Two forms:** `--regime-no-node` gives the gate without the node; without it the regime is also a node. In both, the regime is also part of the floor's, copy act's and stride act's mixture contexts (experiment 62), so the "gate" rows are gate plus regime-conditioned mixtures.
- **Fewer regimes:** `--regime 128,4` (and `--regime-thr X` for the similarity below which a window starts a new regime, default 0.6; not varied here).
- **Cost:** the gated runs take 1.1–1.5 times as long as the matching node runs (after primitives, seed 0: 16 gate 3,350 s, 16 node 2,173 s, node and gate 2,398 s, no regime 1,055 s). Every kept edge is measured on every byte in all runs; the gate's runs keep more edges (16–20 against 4–10).

Seeds 0, 1, 2 (`--seed`); scripts `runs/exp63/regimes.sh` (binary `sit-63a`) and `runs/exp63/both.sh` (node and gate, `sit-63b`); stride act on, floor, guard, two maps, copy act, position sense; test stream alone (Z) and after experiment 61's primitives (P).

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
- **What it reads:** the segment from the previous segment start to this one is read as this segment's description, with the prototype's tokens, alignment, background and learning rules, except for the differences listed in the results.
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

- **The two uses of the regime do different jobs.**
  - The node lets the brain build structure that survives a change of material: it is the best form after primitives.
  - The gate keeps structure from acting where it doesn't pay: it helps a little with nothing before.
  - Together they are a compromise: as good as the gate with nothing before, robust after primitives but not as good as the node alone.
  - 4 regimes are worse than 16 after primitives; other numbers were not tried.
- **The stride act's gains came from byte-level repeat and step; whole numbers don't add here.** They would matter for numbers whose carries cross byte boundaries often.
- **The description expert works in the stack, at about half strength, for reasons not yet separated.** The likeliest is that it doesn't know what a description is: it takes the segment before every segment. That is a role, and roles are what the word reader learns (its act path and question boundaries). So a natural next step is to let the word reader read the description and have its facts answer the expert's question "what does the description say about here?". Before that, an oracle run (told which segments are descriptions) would show how much of the gap that explains.

## Next

1. **Default stack for mixed material:** floor, guard, two maps, copy act, stride act, position sense, and the regime as a node (best after mixed material) or as node and gate (best with a single kind).
2. **The description expert:** an oracle run (told which segments are descriptions) to measure the gap; then a learned notion of "description" (segments the word reader reads as text, or trust conditioned on the previous segment's regime).
3. **Larger mixed corpora** to see whether the regime's benefit grows with more kinds of material.
