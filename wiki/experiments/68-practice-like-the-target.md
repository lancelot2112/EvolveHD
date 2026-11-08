# 68 · Practice that looks like the target (2026-11-08)

*[All experiments](README.md)* · [← 67](67-reader-of-spellings.md)

**The question:** in experiment 67 the expert read the practice notes (`magic f1 c2 6b`) but not the real libmagic entries (`0 string \xf1\xc2\x6b`). Two things differ between them: where the bytes sit in the word (each its own word, or packed inside one word), and the word before the magic (`magic` or `string`). Which one stops the reading? And once that is known, can the expert read the real entries?

**Terms used below:** as in [experiment 67](67-reader-of-spellings.md).
- **The expert:** the stack's description expert. **The token view** aligns a file's bytes to the description's words and learns a byte table per word. **The spelling view** (experiment 67) reads 1- to 3-character pieces inside words.
- **The expert's own cost:** what the expert alone would pay, in bits per byte. **The mixed code:** what the whole stack pays.
- **Told:** the expert is given which segments are descriptions (an oracle), so the role gate plays no part.
- **R and W:** on the practice stream, a file after its own description (R) or another format's (W). On the real part, as in experiment 66: Rm (its manual entry), Rn (a hex note), W (a wrong entry), N (nothing).

**Three practice streams** (`scripts/exp68/build.py`). Experiment 60's made-up stream, with only the descriptions rewritten; every file byte is the same. Then experiment 66's real part, unchanged, in its four rotations.
- **A:** `magic \xf1\xc2\x6b` — the bytes packed into one word, still after `magic`. That changes the place in the word, not the word before.
- **B:** `0 string f1 c2 6b` — spaced hex after `0 string`. That changes the word before, not the place in the word.
- **C:** `0 string \xf1\xc2\x6b` — both, as libmagic writes it.
- Escapes are hex (`\xf1`) for even-numbered formats and octal (`\361`) for odd ones, since libmagic uses both.
- Practice lengths: A 602,940, B 600,607, C 610,062 bytes (the original practice, O, is 593,485).

## What we found

- **The place in the word is the barrier, not the word before.** Experiment 67's arm (spelling and token views, told) on the practice stream, last quarter, expert's own cost on the magic, R against W:

  | practice | even formats (hex) | odd formats (octal) |
  |---|---|---|
  | original notes (exp 67) | 4.11 / 8.57 | — |
  | A: escapes after `magic` | 7.92 / 8.67 (−0.74) | 7.87 / 8.71 (−0.83) |
  | B: spaced hex after `string` | 3.71 / 8.58 (−4.88) | 3.73 / 8.67 (−4.94) |
  | C: escapes after `string` | 7.94 / 8.65 (−0.72) | 7.88 / 8.68 (−0.80) |

  - B reads as well as the original notes. A and C barely read, even on the practice itself.
  - So experiment 67's spelling view does not read bytes packed into a word at all, even where it has practised them. The rest of this grid (rotations 1–3, real-part scoring for A and B) was stopped there.
- **The spelling view reads nothing on its own.** Predicting from the spelling view alone (env `SPELL_ONLY`), on small made-up worlds (8 and 30 formats) in the spellings tried (original, A, B), R − W stays between −0.25 and +0.12 bits per byte, the worst quarter included. Two causes showed in its tables:
  - **The 1-character pieces take all the credit.** Credit is shared by how well each piece already predicts, so the most frequent pieces win early and keep winning: total shares were 1686 for 1-character pieces against 183 for 2-character ones. The 2- and 3-character pieces, the only ones that could tell `\xf1` from `\xf2`, get credit of about 0.1 each.
  - **When each piece learns alone, it learns the background.** With every piece credited in full (env `SPELL_ALONE`), all pieces learn the most common file byte (0x00, above 0.85), and the jumps learn to stand still.
  - Smaller priors, a different novice threshold and forward-only jumps (envs `SPELL_PRIOR`, `SPELL_NOV`, `SPELL_FWD`) changed nothing.
  - So experiment 67's better reading of the notes must have come through the combination (shared jumps and the mix between the views), not from the spelling view reading on its own. That was not separated.
- **With a perfect cut into escape units, the token view reads the packed practice, at about half strength.** `TOKSPLIT` is a hand-written oracle cut: each `\xHH`, `\ooo` or `\c` becomes its own word, as do the runs of other characters between them. It uses the token view alone, told. On practice C, last quarter, R − W is −2.64 (even formats) and −2.72 (odd), against −4.49 and −4.55 for the same arm on the original notes (and −0.72 for experiment 67's arm on C).
  - Why perfect units still read worse than spaced hex (−4.88 in B) was not tested.
- **On the real entries, after practice C, that cut gives the first reading of hex escapes.** Expert's own cost on the magic bytes of each format's first file, pooled over the four rotations, Rm − W, with standard errors and the number of bytes cheaper after the right entry:

  | notation | bytes | after practice C | after original practice | exp 66 oracle |
  |---|---|---|---|---|
  | hex escape | 9 | **−0.69** (0.26), 7 | +0.19 (0.38), 1 | +0.14 (0.41), 1 |
  | hex number | 7 | +0.02 (0.25), 2 | +0.13 (0.27), 3 | −0.29 (0.30), 3 |
  | hex number, little-endian | 4 | −0.10 (0.46), 2 | +0.05 (0.12), 1 | −0.00 (0.08), 2 |
  | octal escape | 25 | −0.11 (0.25), 17 | +0.07 (0.09), 14 | +0.17 (0.09), 11 |
  | plain character | 71 | +0.20 (0.09), 27 | +0.29 (0.11), 25 | +0.30 (0.11), 21 |
  | other escape | 3 | +0.71 (0.96), 1 | +0.57 (0.99), 1 | −0.03 (0.37), 1 |

  - Hex escapes: −0.69, 2.7 standard errors, in 7 of 9 bytes (png, llvmbc and pyc only). The same cut after the original practice reads nothing (+0.19). **Both the units and practice in the target's spelling are needed.**
  - Octal escapes stay at about zero. Every 3-digit octal unit of the real entries had appeared 6 to 27 times in practice C, the same range as the hex units (15 to 27), so it is not a lack of practice with those units. The reason is untested. One octal format, ole2, holds 8 of the 25 bytes.
  - Plain characters keep experiment 66's penalty (+0.20). This is built in: the token view learns a table per word, and for a format's first file a word such as `GIF8` has never been seen, so its table is the flat prior. Practice never spelled a byte as its own character, either.
- **Practice C costs the hex notes.** After practice C, the note's lead over a wrong entry (Rn − W) is −0.85 to −1.58 by class. After the original practice it is −1.15 to −2.65. Per format, on the expert, a note against nothing goes from −2.22 to −1.43 bits per byte.
- **The mixed code and whole files barely change.** Magic of first files, mixed code, against nothing, per format: Rm − N −0.80 after practice C against −0.77 after the original practice. Whole first files: 5.34 against 5.33 after the right entry.
- **The tokenizer change leaves everything else alone.** After the original practice, the `TOKSPLIT` binary (`sit-68a`) gives the same bits as experiment 66's oracle, byte for byte, for the whole practice stream in all four rotations. They first differ 74 to 1,155 bytes into the real part, after its first escapes (23 to 235 bytes in).

## Setup

- **Binaries:**
  - `runs/exp36/sit-67b` (experiment 67's, unchanged) for the A/B/C practice grid.
  - `runs/exp36/sit-68a` for the `tsplit` arm. It adds env-gated probes to `src/acts.rs` (`SPELL_ONLY`, `SPELL_ALONE`, `SPELL_R`, `SPELL_FWD`, `SPELL_PRIOR`, `SPELL_NOV`, `TOKSPLIT`); with none set it behaves as before.
- **Runs:** `runs/exp68/run.sh "style rotation arm,..."`, with experiment 66's flags.
  - `soracle`: `--desc-expert --desc-spell --desc-oracle`, after A, B and C practice, rotation 0 (C's practice reading only, from that run).
  - `tsplit`: `--desc-expert --desc-oracle` with env `TOKSPLIT=1`, after C and after the original practice (O), all four rotations: 8 runs, about 1.5 hours two at a time.
  - The practice part is identical in all four rotations, so its reading is one history: the four C runs give the same practice numbers to the last digit.
- **Stand-alone test:** `examples/spelltest.rs` runs the expert alone on a stream, told, and prints R and W by quarter (`--spell` adds the spelling view; env `KEYS` dumps the spelling view's tables). Small made-up worlds of 8 and 30 formats were used for the spelling-view diagnosis.
- **Scorers:**
  - `scripts/exp68/practice.py NAME=ITEMS_JSON:BITS ...`: the expert's cost on the practice magic after R and W, by quarter, for even and odd formats.
  - `scripts/exp67/notation.py` and `scripts/exp66/score.py`, as before.

## What this means

- **Bytes packed into a word are the real barrier, and the spelling view as built can't get past it.** Its credit sharing lets the shortest pieces starve the longer ones, which are the only ones that can tell one escape from another.
- **Units are the key.** Once each escape is its own word, the token view reads packed escapes in practice and, for hex, in real manual entries it has never seen. But the cut used here is written by hand for libmagic. The owner's principle is that the system should find such structure from the text's own statistics.
- **Practice has to cover the target's notations.** The same units with the original practice read nothing. Practice in the libmagic spelling, in turn, made the old hex notes harder to read.
- **Plain characters need a different mechanism.** A table per word can't read a word on first sight. Reading `GIF8` as the bytes G, I, F, 8 needs a mapping shared across words: "this character spells this byte".
- **Caveats:**
  - Each run was done once, and the practice part is a single history.
  - The hex-escape reading rests on 9 bytes in 3 formats.
  - `TOKSPLIT` is an oracle, not a learned cut. It is an upper bound for this token view, not a result of the system.

## Next

1. **Learn the cut from local statistics.** Cut a word where the next character is hard to predict from the ones before (low mutual information across the boundary), with counts kept by the expert itself. Compare against `TOKSPLIT` on practice C and the real entries.
2. **A shared "spells itself" path.** One table for every word that predicts a byte from the characters of its unit (a copy, then learned corrections), so that a new unit such as `GIF8` or `\x89` is readable on first sight.
3. **Fix the spelling view's credit, or replace it.** For example, back off as PPM does: the longest piece seen before predicts first, and shorter pieces only take what it leaves. Then every length gets credit of its own.
4. **Practise on mixed notation:** plain characters, hex and octal escapes in the same practice, so practice covers what real entries use without losing the notes.
