# 68 · Practice that looks like the target (2026-11-08)

*[All experiments](README.md)* · [← 67](67-reader-of-spellings.md)

**The question:** in experiment 67 the expert read the practice notes (`magic f1 c2 6b`) but not the real libmagic entries (`0 string \xf1\xc2\x6b`). Two things differ between them: where the bytes sit in the word (each its own word, or packed inside one word), and the word before the magic (`magic` or `string`). Which one stops the reading? And once that is known, can the expert read the real entries?

**Terms used below:**
- **The expert:** the stack's description expert. It predicts each byte of a file from the description just before it.
  - **The token view** aligns a file's bytes to the description's words and learns a byte table per word.
  - **The spelling view** (experiment 67) reads 1- to 3-character pieces inside words, each piece with its own byte table.
  - **Jumps:** how the reading position moves from one byte of the file to the next (stay, step forward, skip). They are learned.
  - **Prior:** the count each byte value starts with in a new table. A larger prior means a table needs more evidence before it predicts sharply.
  - **Credit:** how much of each byte's lesson a table gets. It is shared out by how well each table already predicted that byte.
- **The expert's own cost:** what the expert alone would pay, in bits per byte. **The mixed code:** what the whole stack pays once the expert is mixed in.
- **Told:** the expert is given which segments are descriptions (an oracle), so the role gate (experiment 65's learned switch that decides this) plays no part.
- **R and W:** on the practice stream, a file after its own description (R) or another format's (W). On the real part, as in experiment 66: Rm (its manual entry), Rn (a hex note), W (a wrong entry), N (nothing).

**Three practice streams** (`scripts/exp68/build.py`). Experiment 60's made-up stream, with only the descriptions rewritten; every file byte is the same. Then experiment 66's real part, unchanged, in its four rotations.
- **A:** `magic \xf1\xc2\x6b`: the bytes packed into one word, still after `magic`. That changes the place in the word, not the word before.
- **B:** `0 string f1 c2 6b`: spaced hex after `0 string`. That changes the word before, not the place in the word.
- **C:** `0 string \xf1\xc2\x6b`: both, as libmagic writes it.
- Escapes are hex (`\xf1`) for even-numbered formats and octal (`\361`) for odd ones, one notation per format, since libmagic uses both.
- Practice lengths: A 602,940, B 600,607, C 610,062 bytes (the original practice, O, is 593,485).

## What we found

- **The place in the word is the barrier, not the word before.** Experiment 67's arm (spelling and token views, told) on the practice stream, last quarter, expert's own cost on the magic, R / W (R − W):

  | practice | even formats | odd formats |
  |---|---|---|
  | original notes (exp 67) | 4.11 / 8.57 (−4.45) | 4.15 / 8.65 (−4.50) |
  | A: escapes after `magic` | 7.92 / 8.67 (−0.74) | 7.87 / 8.71 (−0.83) |
  | B: spaced hex after `string` | 3.71 / 8.58 (−4.88) | 3.73 / 8.67 (−4.94) |
  | C: escapes after `string` | 7.94 / 8.65 (−0.72) | 7.88 / 8.68 (−0.80) |

  - B reads as well as the original notes. A and C barely read, even on the practice itself.
  - So experiment 67's arm barely reads bytes packed into a word, even where it has practised them. The little it does may be the token view memorising each format's whole packed word; that was not tested.
  - The A, B and C runs went on through the real part for rotation 0 only. They are not scored there, since one rotation doesn't balance the conditions. Rotations 1–3 were stopped.
- **The spelling view alone reads nothing at its default settings.** Predicting from the spelling view alone (env `SPELL_ONLY`) with the stand-alone test, on two small made-up worlds:
  - At the default prior (0.05), R − W stays near zero in every quarter, in both worlds and every spelling tried (for example −0.15 for B in the 30-format world).
  - **In the 30-format world** (3-byte magic, spellings original and A), smaller priors (0.002, 0.0005) and a lower novice threshold left R − W between −0.25 and +0.12.
  - **In the 8-format world** (1-byte magic), smaller priors let it read strongly: last quarter −1.36 and −2.53 on the original spelling (prior 0.002 and 0.0005), −0.37 and −2.85 on packed escapes. The token view alone gets −0.60 there. It probably memorises whole pieces such as `x3c`, which works when there are only 8 magics.
  - Forward-only jumps (env `SPELL_FWD`) changed nothing (8-format world: +0.01 and 0.00).
- **Why it reads nothing at the defaults:** its tables showed three things.
  - **The 1-character pieces take nearly all the credit.** The most frequent pieces predict best early, so they keep winning the credit. In the 8-format world (original spelling), the learned share of 1-character pieces totalled 1686 against 183 for 2-character pieces; the 30-format world is similar (1917–1997 against 159–175). Most 2- and 3-character pieces, the only ones that can tell `\xf1` from `\xf2`, got credit of 0.12 to 0.19.
  - **The prior swamps that credit.** A new table starts with 0.05 × 256 = 12.8 counts, against the 0.1 to 0.2 each longer piece earns, so its tables stay close to flat. This fits the 8-format world reading once the prior is cut.
  - **The jumps learn to stay put.** In the 30-format world "stay" holds 0.26–0.34 of the jump weight, against 0.04–0.06 for one step forward. With every piece credited in full (env `SPELL_ALONE`), all pieces learn the most common file byte (0x00, at 0.86 to 0.99) and "stay" takes 74% of the jump weight.
  - So experiment 67's better reading of the real notes (Rn − W 0.53 better than experiment 66, se 0.12) probably came through the combination (the shared jumps and the mix between the views), not from the spelling view reading on its own. On practice, the two arms read alike (−4.45 against −4.49 for the token view alone); the spelling view lowered R and W by about 0.5 together. This was not separated.
- **With a perfect cut into escape units, the token view reads the packed practice at a bit over half strength.** `TOKSPLIT` is a hand-written oracle cut: each `\xHH`, `\ooo` or `\c` becomes its own word, as do the runs of other characters between them. It uses the token view alone, told. On practice C, last quarter, R − W is −2.64 (even formats) and −2.72 (odd), against −4.49 and −4.55 for the same arm on the original notes (about 60%). Experiment 67's arm on C gets −0.72.
  - Why perfect units still read worse than spaced words was not tested. (B was not run with the token view alone.)
- **On the real entries, after practice C, that cut gives the first reading of hex escapes.** Expert's own cost on the magic bytes of each format's first file, pooled over the four rotations, Rm − W, with standard errors and the number of bytes cheaper after the right entry:

  | notation | bytes | after practice C | after original practice | exp 66 oracle |
  |---|---|---|---|---|
  | hex escape | 9 | **−0.69** (0.26), 7 | +0.19 (0.38), 1 | +0.14 (0.41), 1 |
  | hex number | 7 | +0.02 (0.25), 2 | +0.13 (0.27), 3 | −0.29 (0.30), 3 |
  | hex number, little-endian | 4 | −0.10 (0.46), 2 | +0.05 (0.12), 1 | −0.00 (0.08), 2 |
  | octal escape | 25 | −0.11 (0.25), 17 | +0.07 (0.09), 14 | +0.17 (0.09), 11 |
  | plain character | 71 | +0.20 (0.09), 27 | +0.29 (0.11), 25 | +0.30 (0.11), 21 |
  | other escape | 3 | +0.71 (0.96), 1 | +0.57 (0.99), 1 | −0.03 (0.37), 1 |

  - **Hex escapes:** −0.69, 2.7 standard errors, in 7 of 9 bytes.
    - By format: png −0.60, llvmbc −1.07, pyc −0.52.
    - Most of it is on the first escaped byte of each entry (−1.64, −1.69, −1.51). The later escapes average about −0.23.
  - **The units alone are not enough.** The same cut after the original practice reads nothing (+0.19), so practice in the target's spelling is also needed. Practice C without the cut was not scored on the real entries.
  - **Octal escapes stay at about zero.** 23 of the 25 octal bytes have units seen 6 to 27 times in practice C (mean about 12, against about 20 for the hex units). mo's `\22` and `\4` were never practised, since practice always writes 3 digits. Why octal doesn't read was not tested. One format, ole2, holds 8 of the 25 bytes.
  - **Plain characters keep experiment 66's penalty** (+0.20). Probably because the token view learns a table per word: for a format's first file, a word such as `GIF8` has never been seen, so its table is the flat prior, and practice never spelled a byte as its own character. That would explain no reading, but not why the right entry costs more than a wrong one (Rm 8.19 against W 7.99 and N 8.00).
- **Practice C costs the hex notes.** After practice C, the note's lead over a wrong entry (Rn − W) is −0.85 to −1.58 by class. After the original practice it is −1.15 to −2.65. Per format, on the expert, a note against nothing goes from −2.22 to −1.43 bits per byte.
- **On the mixed code, the right entry and whole files barely change; the notes lose.** Magic of first files, mixed code, against nothing, per format:
  - Rm − N −0.80 after practice C against −0.77 after the original practice;
  - Rn − N −1.05 against −1.62;
  - W − N −0.18 against −0.27.
  - Whole first files: 5.34 against 5.33 after the right entry.
- **The cut leaves text without escapes alone.** After the original practice, the `tsplit` runs (binary `sit-68a`, `TOKSPLIT=1`) give the same bits as experiment 66's oracle, byte for byte, for the whole practice stream in all four rotations. They first differ at the first byte of the file after the first description with an escape: 74 to 3,345 bytes into the real part.

## Setup

- **Binaries:**
  - `runs/exp36/sit-67b` (experiment 67's, unchanged) for the A/B/C practice grid.
  - `runs/exp36/sit-68a` for the `tsplit` arm. It adds env-gated probes to `src/acts.rs`: `SPELL_ONLY`, `SPELL_ALONE`, `SPELL_R` (spelling-view credit from the combined responsibility; tried, no result kept), `SPELL_FWD`, `SPELL_PRIOR`, `SPELL_NOV` and `TOKSPLIT`.
  - Sameness with the old binary is shown only for the token-view path, on text without escapes (above). The spelling-view path of `sit-68a` with no env set was not compared with `sit-67b`.
- **Runs:** `runs/exp68/run.sh "style rotation arm,..."`, with experiment 66's flags.
  - `soracle`: `--desc-expert --desc-spell --desc-oracle`, after A, B and C practice, rotation 0.
  - `tsplit`: `--desc-expert --desc-oracle` with env `TOKSPLIT=1`, after C and after the original practice (O), all four rotations: 8 runs, about 1.5 hours two at a time.
  - The practice part is identical in all four rotations, so its reading is one history: the four C runs give the same practice numbers to the last digit.
- **Stand-alone test:** `examples/spelltest.rs` runs the expert alone on a stream, told, and prints R and W by quarter. `--spell` adds the spelling view; env `KEYS` dumps the spelling view's tables.
  - The two small worlds: 8 formats with a 1-byte magic, and 30 formats × 20 files with a 3-byte magic, each written in the original spelling and in A (the 30-format world also in B).
- **Scorers:**
  - `scripts/exp68/practice.py NAME=ITEMS_JSON:BITS ...`: the expert's cost on the practice magic after R and W, by quarter, for even and odd formats.
  - `scripts/exp67/notation.py` and `scripts/exp66/score.py`, as before.

## What this means

- **Bytes packed into a word are the real barrier, and the spelling view as built can't get past it.** Its short pieces take the credit and its prior keeps the long pieces flat, yet only the long pieces can tell one escape from another.
- **Units are needed but not enough.** Once each escape is its own word, the token view reads packed escapes in practice and, for hex, in real manual entries it has never seen. Octal doesn't read with the same units and practice. And the cut here is written by hand for libmagic, while the system should find such structure from the text's own statistics.
- **Practice has to cover the target's notations.** The same units with the original practice read nothing. Practice in the libmagic spelling, in turn, made the old hex notes harder to read.
- **Plain characters need a different mechanism.** A table per word can't read a word on first sight. Reading `GIF8` as the bytes G, I, F, 8 needs a mapping shared across words: "this character spells this byte".
- **Caveats:**
  - Each run was done once, and the practice part is a single history.
  - The hex-escape reading rests on 9 bytes in 3 formats. Its standard error treats them as independent, but 5 come from png. Most of the gain is on the first escaped byte of each entry.
  - `TOKSPLIT` is an oracle, not a learned cut. It is an upper bound for this token view, not a result of the system.

## Next

1. **Learn the cut from local statistics.** Cut a word where the next character is hard to predict from the ones before (low mutual information across the boundary), with counts kept by the expert itself. Compare against `TOKSPLIT` on practice C and the real entries.
2. **A shared "spells itself" path.** One table, shared by every word, that predicts a byte from the characters of its unit (a copy, then learned corrections), so that a new unit such as `GIF8` or `\x89` is readable on first sight.
3. **Fix the spelling view's credit and prior, or replace it.** For example, back off from long pieces to short ones, as text compressors do: the longest piece seen before predicts first, and shorter pieces only take what it leaves. Then every length gets credit of its own. Pair it with a small prior.
4. **Practise on mixed notation:** plain characters, hex and octal escapes in the same practice, so practice covers what real entries use without losing the notes.
