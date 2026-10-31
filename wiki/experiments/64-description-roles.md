# 64 · What a description is: an oracle, and two learned substitutes (2026-10-31)

*[All experiments](README.md)* · [← 63](63-regime-forms-wide-steps-desc-in-stack.md)

**The question:** in experiment 63 the description expert, moved into the stack, kept only about half of the prototype's gain. Is that because it can't tell a description from the segment before any other segment, or because of the port's other differences?

**Results** (experiment 60's progressive formats stream; second half, first files of never-seen formats; one run each, same brain in every run):
- **Told which segments are descriptions, the stack's expert matches the prototype.**
  - Magic bytes, expert alone: 4.94 bits per byte with the right description, 9.04 with a wrong one, 8.00 with none. The prototype gave 5.04 / 9.02 / 8.00.
  - Mixed: 6.49 / 9.22 / 9.31 (the prototype's mixture 6.30 / 9.11 / 8.43).
  - All file bytes with the right description: 3.324 (prototype 3.300; without the oracle 3.431).
  - **So the gap was the role:** the port's small rule differences and the base brain account for little.
- **Learning its trust per regime recovers a little.**
  - The expert's mixture context also holds the regime the bytes before the segment were in (`--desc-ctx-regime`). Magic bytes, mixed: 8.39 with the right description against 8.83 without.
  - The expert alone is unchanged (6.78): only its trust is learned per regime, not what it learns.
- **Learning in proportion to its trust (credit from the mixture above) trades the notation for the rest.** `--desc-learn-trust`: the expert learns from each byte in proportion to the mixture's weight on it.
  - **Whole files improve:** all file bytes with the right description 3.431 → 3.387; first files 3.766 → 3.701; constants 8.50 → 8.09.
  - **Magic bytes get worse:** mixed 8.39 → 9.43; expert alone 6.78 → 8.24.
  - The mixture trusts the expert only moderately on average, so it slows all the expert's learning, including the hex notation that only pays on a few bytes.

## The magic bytes of never-seen formats

Bits per byte, second half, first file of each new format.

| Run | Right description | Wrong description | None |
|---|---|---|---|
| Prototype (exp 60), expert alone | 5.04 | 9.02 | 8.00 |
| Prototype, mixed | 6.30 | 9.11 | 8.43 |
| No expert (`fmt_base`) | 11.09 | 11.08 | 9.67 |
| Expert, guessing (exp 63), expert alone | 6.78 | 9.18 | 8.36 |
| Expert, guessing, mixed | 8.83 | 9.84 | 9.05 |
| **Oracle, expert alone** | **4.94** | 9.04 | 8.00 |
| **Oracle, mixed** | **6.49** | 9.22 | 9.31 |
| Trust per regime, expert alone | 6.78 | 9.18 | 8.36 |
| Trust per regime, mixed | 8.39 | 9.54 | 9.52 |
| + learning by trust, expert alone | 8.24 | 8.67 | 8.35 |
| + learning by trust, mixed | 9.43 | 9.38 | 9.31 |
| Learning by trust without the regime, mixed | 9.74 | 9.71 | 9.11 |

## All file bytes

| Run | Right | Wrong | None | First files, right |
|---|---|---|---|---|
| No expert | 3.494 | 3.586 | 3.658 | 3.874 |
| Expert, guessing | 3.431 | 3.521 | 3.624 | 3.770 |
| Oracle | **3.324** | 3.465 | 3.671 | **3.588** |
| No expert, regime context | 3.496 | 3.581 | 3.673 | 3.882 |
| Expert, guessing, regime context | 3.433 | 3.519 | 3.634 | 3.777 |
| Trust per regime | 3.431 | 3.516 | 3.642 | 3.766 |
| + learning by trust | 3.387 | 3.476 | 3.644 | 3.701 |
| Learning by trust, no regime | 3.413 | 3.492 | 3.635 | 3.738 |

- **Description bytes:** 0.578 (no expert, and the oracle, which leaves them alone) against 0.590–0.603 for the guessing runs.
- **With no description, the oracle is slightly worse** (3.671 against 3.624): with no description it says nothing, while the guessing expert reads something from the previous file.

## The rules (binaries `sit-64a`, `sit-64b`)

- **`--desc-oracle FILE`:** the start offsets of the description segments.
  - The expert leaves description segments to the earlier stages.
  - A file reads the segment before it only if that segment is a description.
  - This is the prototype's setting.
- **`--desc-ctx-regime`:** the expert's mixture context (offset × confidence) also holds the regime at the segment's start, i.e. the regime of the bytes just before it (needs `--regime`; here `--regime 128,16 --regime-no-node`).
- **`--desc-learn-trust`:** every learning step of the expert (emissions, start, jumps, background, shares) is scaled by the mixture's weight on the expert in the previous byte's context: three times the weight, at most 1. Weights start at a third, so learning starts at the full rate.
- **Runs:** `runs/exp63/desc2.sh` and the two learning-by-trust runs. Position sense on, stride act off. The brain is the same in every run (identical symbols).
- **Reproduction:** `sit-64a` reproduces `sit-63c`'s expert run exactly.

## What this means

- **Knowing what a description is was worth most of the gain.** With the role given, the stack reads the magic of a new format from its description as well as the prototype did.
- **Neither learned substitute finds the role.**
  - Trust per regime tunes how much to listen, not what to learn.
  - Learning by trust slows the learning that matters most and pays least often (the notation).
  - A role has to be read off the segment itself: "this is text that talks about the next thing". That is what the word reader does with questions and reports, so the role should come from it: the word reader marks a segment as a description of the next one, and the expert reads only what it marks.

## Next

1. **The word reader as the judge of roles:** run it over the byte stream's segments (as words) and let its act gate learn when a segment is followed by something it describes (the expert's gain on the next segment is the teacher).
2. Keep the oracle as the upper bound for that test.
