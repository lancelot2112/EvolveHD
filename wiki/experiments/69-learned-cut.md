# 69 · A cut learned from the text's own statistics (2026-11-08)

*[All experiments](README.md)* · [← 68](68-practice-like-the-target.md)

**The question:** in experiment 68, a hand-written cut into escape units (`TOKSPLIT`) let the token view read libmagic's packed hex escapes. Can the expert find such units itself, from counts it keeps of the characters in its descriptions? And how does that compare with the hand-written cut, on practice C and on the real entries?

**Terms used below:**
- **The expert, the token view, told, the expert's own cost, the mixed code, R/W, Rm/Rn/W/N:** as in [experiment 68](68-practice-like-the-target.md). In short, the token view learns a byte table per word of a description. *Told* means the expert is given which segments are descriptions.
- **Unit:** what the token view treats as a word. Without a cut, a whole space-separated word is one unit, and a unit never seen before has a flat table.
- **Practice C:** experiment 60's made-up stream respelled as libmagic writes it (`0 string \xf1\xc2\x6b`). Hex escapes are used for even-numbered formats and octal for odd ones. **Practice O** is the original spelling (`magic f1 c2 6b`).
- **Branching entropy:** how uncertain the next character is after a string. Inside a unit it falls as the string grows. Where a unit ends, it rises again, because many different units can follow.

## The cut

`Cutter` in `src/acts.rs` (env `CUT`, `CUT_K`, `CUT_MINC`) cuts by branching entropy (after Harris, and Tanaka-Ishii's version).
- **Counts:** for every substring of 1 to K characters inside a word (the word padded with a marker at both ends), it counts which character comes next and which came before.
- **The score at a gap:** for k = 2..K, add the rise in next-character entropy when the string before the gap grows from k−1 to k characters, and likewise the rise in previous-character entropy for the string after the gap. Only rises count; falls count as zero. A string seen fewer than `minc` times gives no evidence.
- **Cut** where the score is above the threshold.
- **Learning:**
  - The counts are learned from every description the expert begins on, after it has read that description, so each cut uses only earlier text.
  - Trailing `.`, `,` and `:` are stripped first.
  - Nothing is reset or frozen.
- **Setting:** `CUT=2`, K = 3, minc = 4. It was chosen before any full run, on practice C alone (stand-alone test, last quarter):

  | cut | R − W, all | even (hex) | odd (octal) |
  |---|---|---|---|
  | none | −0.63 | −0.59 | −0.67 |
  | threshold 1 | −1.31 | −1.90 | −0.78 |
  | **threshold 2** | −1.32 | −1.97 | −0.73 |
  | threshold 3 | −1.38 | −2.08 | −0.75 |
  | threshold 2, K = 4 | −1.42 | −2.01 | −0.89 |
  | threshold 1, K = 4 | −1.17 | −1.84 | −0.58 |
  | threshold 1, minc 16 | −1.31 | −1.98 | −0.71 |
  | `TOKSPLIT` (oracle) | −2.69 | −2.65 | −2.73 |

  - The settings are close to each other. Threshold 2 sat in the middle and was kept.
  - On experiment 60's original notes it changed nothing: R − W −4.53 without the cut, −4.52 with it.
  - A variant that also cut where the next character was surprising shredded unseen English words in the real part, so it was dropped.
- **The Rust cutter gives the same cuts as the Python prototype** on all 2,374 escaped words of practice C.

## What we found

PLACEHOLDER

## Setup

- **Binary:** `runs/exp36/sit-69a`. It adds `Cutter` and the `units()` choice to `src/acts.rs`: the learned cut if `CUT` is set, else `TOKSPLIT` if set, else whole words. With neither set it is the same path as before.
- **Runs:** `runs/exp69/run.sh "style rotation [arm],..."`, with experiment 66's flags and `--desc-expert --desc-oracle` (token view alone, told).
  - `cut`: env `CUT=2`, after practice C and after practice O, all four rotations (8 runs).
  - `none`: no cut, after practice C, all four rotations (4 runs). Experiment 68 did not run this arm.
  - Comparisons: experiment 68's `tsplit` runs (`TOKSPLIT=1`, after C and after O) and experiment 66's `oracle` runs (no cut, after O).
- **Stand-alone test:** `examples/spelltest.rs` now also prints the last quarter's R − W for even and odd formats. Env `CUT_SHOW` prints the units of every escaped word.
- **Scorers:** `scripts/exp68/practice.py`, `scripts/exp67/notation.py` and `scripts/exp66/score.py`, as in experiment 68.
