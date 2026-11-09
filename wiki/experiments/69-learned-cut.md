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
  - The counts are learned from every description the expert begins on, right after cutting that description (before its file is predicted), so each cut uses only earlier text.
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
  | `TOKSPLIT` (the oracle cut) | −2.69 | −2.65 | −2.73 |

  - The settings are close to each other. Threshold 2 sat in the middle of thresholds 1–3 and was kept, although threshold 3 (−1.38) and threshold 2 with K = 4 (−1.42) scored a little better.
  - On experiment 60's original notes it changed nothing: R − W −4.53 without the cut, −4.52 with it.
  - A variant that also cut where the next character was surprising shredded unseen English words in the real part, so it was dropped.
- **The Rust cutter gives the same cuts as the Python prototype** on all 2,374 escaped words of practice C.

## What we found

- **On practice C, the learned cut reads hex at about three quarters of the hand-written cut's strength, and octal not at all.** The expert's own cost on the magic, last quarter, R / W (R − W), full stack, told:

  | arm | even formats (hex) | odd formats (octal) |
  |---|---|---|
  | no cut | 8.16 / 8.77 (−0.60) | 8.18 / 8.85 (−0.67) |
  | learned cut | 6.97 / 8.93 (−1.96) | 8.24 / 8.98 (−0.73) |
  | `TOKSPLIT` (exp 68) | (−2.64) | (−2.72) |

  - On hex it keeps improving across the quarters (−0.65, −1.14, −1.69, −1.96). It is not a one-off.
  - The four rotations share one practice history, so this is one history, not four.
- **Octal fails: the cut doesn't find where an octal escape ends.** Its cuts against `TOKSPLIT`'s on practice C's escaped words, after each quarter's learning so far:
  - **Hex:** it finds every boundary (recall 0.99 to 1.00 in every quarter), with precision 0.40 to 0.43. The extra cuts all fall inside escapes, as `\x|37|\x|81|\x|6b` against `\x37\x81\x6b`: 5 cuts where the oracle makes 2. So every unit after `\x` is the two hex digits, which serve as well as whole escapes.
  - **Octal:** recall 0.01 to 0.04, precision 0.04 to 0.28. Most octal words are left whole (`\174\026\165`) or cut in the wrong place (`\2|53\017`).
  - **The end of an octal escape is marked by a fall, and the score counts only rises.**
    - At the true boundary after `\174` (in `\174\026\165`), the next character's entropy goes 2.66 after `4`, 1.75 after `74`, 0.72 after `174` and 0.00 after `\174`: after a backslash and three digits, a backslash always follows.
    - So the counts do mark the end, but as the next character becoming certain. The score ignores falls.
    - Nor is there a rise looking backward from the next escape (`\0` 2.89 against `0` 2.77), because digits sit on both sides.
    - In hex, the fixed `x` after each backslash gives the rise that octal lacks.
    - The digit strings are also sparse (`174` seen 25 times in practice C).
- **The cut leaves the practice's plain text alone, but cuts some real words at odd places.**
  - In the whole of practice O (60,583 words) it cut one word (`11|92`, a number). Bits are the same as experiment 66's told runs without a cut (`oracle`) up to byte 592,493 of 593,485, in all four rotations. On practice O the reading is the same to two decimals (−4.49 and −4.55, as with `TOKSPLIT`).
  - On practice C it cut 1,953 of 58,250 words: 1,753 of the 2,374 escaped words (all but 2 hex words; 578 of 1,197 octal ones), and 200 numbers (`27|6`, `10|52`).
  - In the real entries it cut 177 to 245 of about 2,100 to 2,190 words per rotation.
    - About a third of the cut words (78 of 237 in rotation 0 after C) are clean cuts such as `!:|mime` and `!:|ext`. Some URLs and types are cut sensibly (`https://|www.|w3.org/|TR/WOFF2/`, `application/x-|tex-tfm`).
    - About a quarter (65 of 237; 47 of 190 after O) are plain words cut at odd places: `arch|ive` (9 times), `mod|ule` (7), `J|ava` (6), `t|ext`, `depend|ent`, `directo|ry`. Many URL cuts are odd too (`https://|d|evel|oper.gnome.org|/...`).
    - The magic's hex numbers (`0xffd8ff00`) are never cut.
- **On the real entries, after practice C, the learned cut reads hex escapes more weakly than the oracle, and the evidence is thin.** The expert's own cost on the magic bytes of each format's first file, pooled over the four rotations, Rm − W, with standard errors and the number of bytes cheaper after the right entry:

  | notation | bytes | no cut | learned cut | `TOKSPLIT` | learned cut after O |
  |---|---|---|---|---|---|
  | hex escape | 9 | +0.09 (0.23), 1 | **−0.50** (0.42), 4 | −0.69 (0.26), 7 | +0.25 (0.33), 1 |
  | hex number | 7 | −0.59 (0.32), 5 | −1.02 (0.88), 3 | +0.02 (0.25), 2 | −0.32 (0.29), 3 |
  | hex number, little-endian | 4 | +0.09 (0.11), 2 | −0.24 (0.68), 1 | −0.10 (0.46), 2 | +0.01 (0.10), 2 |
  | octal escape | 25 | +0.29 (0.23), 11 | +0.14 (0.13), 13 | −0.11 (0.25), 17 | +0.20 (0.09), 10 |
  | plain character | 71 | +0.12 (0.08), 28 | +0.18 (0.08), 25 | +0.20 (0.09), 27 | +0.34 (0.11), 25 |
  | other escape | 3 | +0.04 (0.47), 1 | +0.19 (0.17), 0 | +0.71 (0.96), 1 | −0.12 (0.15), 1 |

  - **Hex escapes, paired by byte against no cut:** the learned cut −0.59 (se 0.36, better on 6 of 9), `TOKSPLIT` −0.77 (se 0.33, 7 of 9). The learned cut against `TOKSPLIT`: +0.19 (se 0.54).
  - **The no-cut arm reads nothing** on hex escapes (+0.09). This settles what experiment 68 left open: practice C without a cut doesn't read the real hex escapes, so the units are needed.
  - **Where the learned cut's units differ from the oracle's, it reads different bytes.** By byte, Rm − W, learned cut against `TOKSPLIT`:
    - png's first byte: +0.63 against −1.64. png's entry is `\x89PNG\x0d\x0a\x1a\x0a`, which the cut makes `\x|89PNG|\x0d|\x|0a|...`. The unit `89PNG` was never seen in practice, so its table is flat. `TOKSPLIT` also keeps `\x0d` whole, but in practice the learned cut always split `\x` from the digits, so for it a whole `\x0d` is a new unit with a flat table.
    - png's last byte: −2.34 against −0.49.
    - llvmbc (`BC\xc0\xde`, cut `BC|\xc0|\x|de`): byte 3, `de`, −2.24 against −0.46; byte 2, the whole `\xc0` (a new unit, as above), +1.18 against −1.69.
    - pyc's first byte reads in all three arms (−1.52, −1.51, −1.54 with no cut), so it does not come from the units.
  - **Hex numbers:** −1.02 for the learned cut rests mostly on one byte (jpeg's first, −5.76). Hex numbers are not cut, the no-cut arm also reads them (−0.59), and the three arms disagree byte by byte, so this is not counted as a reading.
  - **Octal escapes stay at about zero** in every arm. **Plain characters keep a small penalty** in every arm: +0.12 to +0.20 after practice C, +0.34 after O.
  - **After practice O,** the learned cut reads nothing, as the oracle didn't (+0.25 against +0.19).
- **The cut wins back part of what practice C cost the notes.** Magic of first files, against nothing, per format:
  - mixed code, a note against nothing (Rn − N): no cut −0.74, learned cut −0.98, `TOKSPLIT` −1.05, against −1.56 after practice O;
  - the expert's own cost: no cut −1.09, learned cut −1.43, against −2.21 after practice O.
  - Why the units help the notes was not tested.
- **On the mixed code, the right entry barely changes, and whole files don't change at all.**
  - Magic of first files, against nothing: Rm − N −0.68 (no cut), −0.73 (learned cut), −0.80 (`TOKSPLIT`); W − N −0.11, −0.05, −0.18.
  - Paired against no cut, the learned cut's Rm − W improves by 0.11 (se 0.06, 18 of 28 formats).
  - Whole first files after the right entry: 5.33, 5.33 and 5.34.

## What this means

- **A cut from local statistics can find units, where units are marked in the text's statistics.**
  - Hex escapes are: `\x` is a fixed prefix, and after two digits anything can follow. The cut finds those boundaries without being told, and the token view then reads hex at about three quarters of the oracle's strength in practice.
  - It gets this from the expert's own counts, learned as the text arrives, with no rules for libmagic. It leaves the practice's plain text alone. In real text some cuts are sensible (`!:|mime`) and some are not (`arch|ive`).
- **A unit whose end is marked by certainty rather than surprise is out of its reach.** After `\ddd` the next character is certain, but the score counts only rises in uncertainty.
  - K = 4 helped octal only slightly in the stand-alone test (−0.89 against −0.73, where the oracle gets −2.73).
  - Possible fixes: a score that also uses falls, or the cut learning from how well its units predict the file (credit from the level above).
- **On the real entries the learned cut gets part of the oracle's hex reading, but the evidence is weak.**
  - Against no cut it is −0.59 (1.6 standard errors) on 9 bytes from 3 formats.
  - The bytes it reads are not the oracle's: it misses png's first byte because its unit `89PNG` runs the escape into the plain characters after it.
- **A unit still has to be seen in practice before it reads.** A table per unit can't read `89PNG`, `\xc0` or `GIF8` on first sight. This is the same limit as experiment 68's plain characters, and the same fix applies: a shared path from a unit's characters to the byte it spells.
- **Caveats:**
  - Each arm was run once. The practice part is one history, shared by the four rotations.
  - The threshold was chosen on practice C before the full runs, but from a handful of settings on the same practice stream that is scored here, and it was not the best of them (the differences are small).
  - The hex-escape results rest on 9 bytes, 5 of them from png. Their standard errors treat bytes as independent.

## Setup

- **Binary:** `runs/exp36/sit-69a`. It adds `Cutter` and the `units()` choice to `src/acts.rs`: the learned cut if `CUT` is set, else `TOKSPLIT` if set, else whole words. With neither set it should be the same path as before. This was checked only indirectly: with `CUT` set, practice O (where it cut one word) gives the same bits as experiment 66 up to that word.
- **Runs:** `runs/exp69/run.sh "style rotation [arm],..."`, with experiment 66's flags and `--desc-expert --desc-oracle` (token view alone, told).
  - `cut`: env `CUT=2`, after practice C and after practice O, all four rotations (8 runs).
  - `none`: no cut, after practice C, all four rotations (4 runs). Experiment 68 did not run this arm.
  - Comparisons: experiment 68's `tsplit` runs (`TOKSPLIT=1`, after C and after O) and experiment 66's `oracle` runs (told, no cut, after O). On this page, "the oracle" means `TOKSPLIT`.
- **Stand-alone test:** `examples/spelltest.rs` now also prints the last quarter's R − W for even and odd formats. Env `CUT_SHOW` prints the units of every escaped word.
- **Scorers:** `scripts/exp68/practice.py`, `scripts/exp67/notation.py` and `scripts/exp66/score.py`, as in experiment 68.

## Next

1. **A shared "spells itself" path** (experiment 68's Next item 2). Once units are found, `89PNG`, `\xc0` and `GIF8` need to be read on first sight. One table shared by every unit, from its characters to the byte it spells.
2. **Let falls count, or credit the cut from the level above.** A score that also uses falls in next-character entropy could find octal ends. Or keep a cut where its units predict the file better than the uncut word did, and drop it where they predict worse. That could also stop the odd cuts in plain words.
3. **Practise on mixed notation** (experiment 68's Next item 4), now with the learned cut.
