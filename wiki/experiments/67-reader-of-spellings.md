# 67 · A reader of spellings (2026-11-02)

*[All experiments](README.md)* · [← 66](66-real-files-real-manuals.md) · [68 →](68-practice-like-the-target.md)

**The question:** in experiment 66 the description expert read a note written in hex (`magic 89 50 4e 47`) on real formats, but not the formats' real manual entries. There the magic is spelled inside a token, as in `\x89PNG\x0d\x0a` or `\037\213`. If the expert can also read the characters inside a token, will it read those entries? And how much does practice on experiment 60's made-up formats matter?

**Terms used below:**
- **The expert:** the stack's description expert. It predicts each byte of a file from the segment just before it (the candidate description), by aligning the file's bytes to that segment's words.
- **The background:** a learned byte distribution the expert falls back on. **The background share** is how much the expert trusts it rather than the description, learned per offset bucket.
- **The expert's own cost:** what the expert alone would pay, in bits per byte. **The mixed code** is what the whole stack pays once the expert is mixed in.
- **The role gate:** it decides per segment whether the previous segment is a description.
  - **Go:** the expert learns from the file and is mixed in.
  - **NoGo:** the expert neither learns nor is mixed in.
- **The gate's teacher:** after 16 bytes of a file, it labels the previous segment a description if that segment explains those bytes at least 2 bits better than both a **decoy** (the segment three before) and no description at all. It reads the expert's current tables without changing them.

**The change: a spelling view beside the token view** (`--desc-spell`, in `src/acts.rs`, `Spell`).
- **Positions:** the characters of the description's tokens, up to 512 positions; tokens of up to 64 bytes.
- **Pieces:** each position is described by the 1-, 2- and 3-character pieces that end at it, within its token. Byte 1 stands for "before the token's start".
  - Each piece has its own learned byte distribution.
  - How much each piece length counts is a learned share, kept apart for pieces with little credit yet (novices, credit < 1).
  - Unknown pieces predict uniformly. A piece is created when it first earns credit (cap 400,000 pieces).
- **Moving and starting:**
  - The view has its own jumps between characters.
  - Where it starts reading is learned, from each file's first byte only, per (previous token, character offset in the token, capped at 15). The first token has a "no previous token" marker.
  - The token view keys its start by the token's position instead.
- **How the two views combine:** a learned share mixes them, per offset bucket (offsets 0 to 7 each, 8 and beyond pooled: 9 buckets).
  - Each view learns its emissions, start and piece shares as if it were alone, from its own responsibility against the background.
  - Both views' jumps, and the share between them, are driven by the combined responsibility.
  - A first version split the credit between the views instead. That raised the expert's cost on whole files by about 1 bit per byte in a pilot, so it was dropped.
- **Flag off:** with `--desc-spell` off, the binary (`sit-67b`) reproduces experiment 66's runs byte for byte. This was checked on real0 `desc`, `role` and `oracle`.

**The streams:** experiment 66's, unchanged.
- 28 real formats, 6 files each.
- Before each file comes one of: its manual entry (Rm), a hex note (Rn), another format's entry (W) or nothing (N).
- Four rotations, so every file is seen once under each condition.
- `real`: the real files alone. `prereal`: experiment 60's made-up stream first.
- **The made-up part is identical in all four prereal rotations** (the first 593,485 bytes), so everything learned there is one practice history, not four.

**Three new arms**, compared with experiment 66's `base`, `desc`, `role` and `oracle` on the same streams:
- `spell`: the expert with the spelling view, guessing that the previous segment is always a description.
- `srole`: the same, with experiment 65's role gate (`--desc-role-contrast --desc-role-null 2`).
- `soracle`: the same, told which segments are descriptions.

**The key test:** the magic bytes of each format's first file, on the expert's own cost, right entry minus wrong entry (Rm − W). Each byte is classed by how its entry writes it:
- **hex escape** (`\x89`: png, llvmbc, pyc; 9 bytes);
- **hex number** (jpeg's `0xffd8ff00` and jmod's, big-endian; lz4's, little-endian);
- **octal escape** (`\037`: 8 formats, 25 bytes);
- **plain character** (18 formats, 71 bytes);
- **other escape** (`\n`, `\<`; 3 bytes).

## What we found

- **The spelling view doesn't read the manual entries.** No arm reads them, guessing or told, with or without the made-up stream first.
  - After the made-up stream, told which segments are descriptions, hex-escape bytes cost the expert 0.24 bits more after the right entry than after a wrong one (experiment 66: +0.14). Both are within one standard error of zero.
  - **The view does remove a penalty.** In experiment 66, octal and plain-character bytes cost more after the right entry: +0.17 (se 0.09) and +0.30 (0.11). With the spelling view they are at −0.09 (0.15) and +0.04 (0.06). That is about zero, not a reading.
- **It reads the notes much better, if it learned the notation first.**
  - After the made-up stream, told which segments are descriptions:
    - the expert's cost on a note's magic falls from 5.82 to 5.19 bits per byte (Rn − W −2.2 to −3.3 by notation class, against −1.9 to −2.7);
    - the stack's mixed code on that magic falls from 6.76 to 6.24 bits per byte;
    - against nothing before the file, per format, that is −2.16 bits per byte against −1.56, lower in 26 of 28 formats in both.
  - **Guessing**, the view leaves the note's cost unchanged (6.93 against 6.94 on the expert alone). It makes the wrong entry cheaper, though, so the note's lead over a wrong entry shrinks:
    - octal −0.83 against −1.36;
    - big-endian hex number −0.26 against −1.41.
- **On the real files alone, it changes nothing specific.** Rm − W and Rn − W are mostly within about 2 standard errors of experiment 66's. Whole files are the same: 5.34 against 5.34 guessing, 5.36 against 5.35 told.
- **Whole files after the made-up stream:** unchanged when guessing (5.42 against 5.41 on first files). Told, they cost more: 5.48 against 5.32.
  - The extra cost sits deep in the file. Under the three conditions with a segment before the file, the expert alone is at 6.64 against 5.85 at offsets 32–255, and at 7.02 against 6.02 beyond 256.
- **The role gate never started after the made-up stream.**
  - It said Go on the first segment and never again, so the arm is the base stack (identical bits).
  - That is one event, since the made-up part is identical in all four rotations.
  - On the real files alone it does start, with Go on 56–96 segments per rotation. There it is more cautious than experiment 66's gate. As in experiment 66, its teacher calls the right entry and a wrong one descriptions about equally often.

## The magic bytes by notation

**The expert's own cost on the magic bytes of each format's first file, after the made-up stream**, in bits per byte, pooled over the four rotations.
- **Rm − W** pairs each byte under the right and the wrong entry.
- **"Cheaper"** counts the bytes cheaper after the right entry.
- `desc` and `spell` guess, and the oracle arms are told.

| arm | notation | bytes | Rm | Rn | W | Rm − W (se) | cheaper | Rn − W (se) |
|---|---|---|---|---|---|---|---|---|
| desc | hex escape | 9 | 8.63 | 7.17 | 8.74 | −0.11 (0.14) | 3 | −1.57 (0.47) |
| spell | hex escape | 9 | 8.35 | 7.30 | 8.61 | −0.26 (0.21) | 4 | −1.31 (0.37) |
| oracle | hex escape | 9 | 8.43 | 5.60 | 8.30 | +0.14 (0.41) | 1 | −2.69 (0.58) |
| soracle | hex escape | 9 | 8.40 | 4.83 | 8.16 | +0.24 (0.26) | 3 | −3.33 (0.52) |
| desc | hex number | 7 | 7.76 | 6.63 | 8.04 | −0.28 (0.34) | 4 | −1.41 (0.49) |
| spell | hex number | 7 | 7.16 | 6.92 | 7.18 | −0.01 (0.29) | 3 | −0.26 (0.31) |
| oracle | hex number | 7 | 7.13 | 5.50 | 7.42 | −0.29 (0.30) | 3 | −1.92 (0.67) |
| soracle | hex number | 7 | 7.03 | 5.17 | 7.39 | −0.37 (0.32) | 5 | −2.22 (0.52) |
| desc | octal | 25 | 8.10 | 6.92 | 8.28 | −0.18 (0.12) | 15 | −1.36 (0.27) |
| spell | octal | 25 | 7.57 | 6.95 | 7.77 | −0.21 (0.19) | 17 | −0.83 (0.26) |
| oracle | octal | 25 | 7.52 | 5.47 | 7.36 | +0.17 (0.09) | 11 | −1.88 (0.33) |
| soracle | octal | 25 | 7.48 | 5.19 | 7.57 | −0.09 (0.15) | 14 | −2.38 (0.39) |
| desc | plain character | 71 | 7.62 | 6.89 | 7.63 | −0.01 (0.10) | 38 | −0.74 (0.14) |
| spell | plain character | 71 | 7.48 | 6.83 | 7.51 | −0.02 (0.09) | 44 | −0.68 (0.13) |
| oracle | plain character | 71 | 8.21 | 6.00 | 7.91 | +0.30 (0.11) | 21 | −1.91 (0.17) |
| soracle | plain character | 71 | 7.86 | 5.22 | 7.82 | +0.04 (0.06) | 39 | −2.60 (0.16) |

- **Too few to read:** the little-endian hex number (lz4, 4 bytes) and the other escapes (3 bytes).
  - The other escapes cost more after the right entry in every arm, in experiment 66 too (for example `desc` +0.56 on the real files alone).
- **The experts take away from the stack's copy act on plain characters.** In the mixed code, the base stack's Rm − W on plain-character bytes is −1.41 (57 of 71 bytes cheaper). Every expert arm brings that to between −0.89 and −1.12.

## Why the entries aren't read (a hypothesis, not tested)

Two things in how the view learned the notes may not carry over to the entries.

- **Where in the word the byte is read.**
  - In a note each byte is its own two-character word (`89`).
  - Its first character's 2- and 3-character pieces include "before the token's start". So whatever the view learned at the first digit is tied to word starts, and in `\x89PNG` those pieces never occur.
  - Some pieces are shared, though:
    - the 1-character pieces (`8`, `9`), which say little on their own;
    - the second digit's 2-character piece (`89`).
  - So the entry's spelling isn't wholly new to the view. It would read it only if the byte had been learned at the second digit, with the `89` piece trusted.
  - Nothing here shows which digit the byte was learned at. No note byte falls under 2 bits, but the token view and the made-up stream's own magic show the same floor, so the floor says nothing about digits.
- **Where reading starts.**
  - The start is learned from each file's first byte, and in the notes the magic follows the word `magic`.
  - In the entries the magic follows `string`, `belong` or `beshort`, at character offset 2 for an escape. That context was never seen at a file's start.

## Whole files and the role gate

**Mixed code on all bytes of first files** (bits per byte):

| stream | base | desc | spell | oracle | soracle |
|---|---|---|---|---|---|
| after the made-up stream, Rm | 5.59 | 5.41 | 5.42 | 5.32 | 5.48 |
| after the made-up stream, N | 5.62 | 5.42 | 5.43 | 5.62 | 5.62 |
| real files alone, Rm | 5.60 | 5.34 | 5.34 | 5.35 | 5.36 |

**The expert's own cost by offset in first files, after the made-up stream** (bits per byte, conditions Rm, Rn and W; under N the told arms are silent):

| arm | 0 | 1–3 | 4–7 | 8–31 | 32–255 | 256+ |
|---|---|---|---|---|---|---|
| oracle | 7.61 | 7.13 | 6.40 | 5.60 | 5.85 | 6.02 |
| soracle | 7.53 | 6.85 | 6.03 | 5.64 | 6.64 | 7.02 |
| desc | 7.99 | 7.52 | 6.59 | 5.89 | 6.29 | 6.46 |
| spell | 7.79 | 7.28 | 6.39 | 5.80 | 6.44 | 6.68 |

- **The view helps in a file's first 8 bytes and costs deep in it,** by 0.8–1.0 bits per byte when told.
- **One candidate cause, untested:** offsets 8 and beyond share one bucket, both for the mix between the views and for the background share.

**The role gate after the made-up stream:**
- **How it starts:** its weights start at zero, so it says Go on the first segment, and the expert learns there.
  - After that, the gate stays on only if the teacher says "description". Under NoGo the expert doesn't learn, so its cost can't improve, and a teacher that says no keeps saying no.
- **What happened in experiment 66:** the teacher first said "description" at file 65.
  - The candidate description cost 108.69 bits over the 16 bytes, against 111.66 for no description and 118.43 for the decoy: a margin of 2.97 bits over the better of the two, just above the threshold of 2.
- **With the spelling view:** the shared background had learned more during that one Go segment, so no description was cheaper.
  - At file 65, no description cost 109.42 bits. The background is learned the same way in every build here, so that figure holds for the run too.
  - The candidate cost more than that under both piloted readings, so the teacher said no:
    - 111.97 reading only learned spelling pieces (decoy 115.30);
    - 110.56 taking the best of the views (decoy 114.72).
  - The run's own reading wasn't printed, but its dump shows the same "no".
  - It never said yes in the whole stream, and the gate stayed off.
  - These costs come from a diagnostic printout in pilot builds. It isn't committed.
- **Two teacher changes didn't start it either** (piloted on the first rotation, then reverted, so the code is what ran):
  - reading only the spelling positions that have a learned piece;
  - taking the best of the token view, the spelling view and their mix, for both the candidate and the decoy.
- **What this suggests:** experiment 65's gate starts from one segment's chance gain, which is fragile. This is seen once, in a single practice history.

**The role gate on the real files alone:** the gate's Go rate and the teacher's label rate at the start of each first file. Experiment 66's `role` gate Go rate is given for comparison.

| before | srole gate Go | srole teacher Go | exp 66 role gate Go |
|---|---|---|---|
| Rm | 0.18 | 0.18 | 0.36 |
| W | 0.32 | 0.18 | 0.64 |
| Rn | 0.50 | 0.39 | 0.39 |
| N | 0.07 | 0.04 | 0.22 |

- **The teacher still can't tell a right entry from a wrong one.**
- **Octal looks like a result but probably isn't.** `srole`'s octal Rm − W on the real files alone is −0.63 (se 0.28). It is one of about 40 comparisons, and nothing else in that arm moves, so it is read as noise.

## How it was run

- **Binary:** `runs/exp36/sit-67b`. The current source builds to the same binary.
- **Runs:** `runs/exp67/run.sh`, 24 runs: two streams × four rotations × three arms. They use experiment 66's flags plus `--desc-spell`.
  - Runs took about 20 minutes after the made-up stream and about 4.5 minutes on the real files alone, two at a time.
  - The first background job hit its 2-hour limit. A restart reran the two unfinished runs from scratch.
- **Scorers:**
  - `scripts/exp67/notation.py ITEMS_PATTERN NAME=BITS_PATTERN ...` classes each magic byte by its entry's notation (from the entry's test line). It gives per-class means and the paired Rm − W and Rn − W.
  - `scripts/exp66/score.py` and `scripts/exp66/role_cond.py`, as in experiment 66.

## What this means

- **Reading inside a token was not enough here.** The view uses what it learned from notes to read notes better, but none of it carries over to the libmagic spellings.
  - What it learned may be tied to how a byte sat in the practice notation: its own word, at the start of a file's reading, after `magic`.
- **Either the practice has to look like the target, or the reader has to stop depending on the layout.**
  - Practise on descriptions that spell bytes as libmagic does.
  - Or make the start and the trusted pieces depend less on word boundaries and the previous word.
- **The role gate needs a sturdier start.** Here one chance segment decided whether it ever turned on.
- **Caveats:**
  - Each run was done once, and the made-up practice is a single history.
  - There are only 9 hex-escape bytes in 3 formats, and 25 octal bytes in 8.
  - The reasons given for the miss are hypotheses.

## Next

1. **Separate the two explanations.** Make two practice streams from experiment 60's made-up formats.
   - **A:** the hex note, with the bytes spelled as one escaped token (`magic \x3c\x3f\x32\x08`). That changes the place in the word, not the word before.
   - **B:** the note's spaced hex after `string` (`0 string 3c 3f 32 08`). That changes the word before, not the place in the word.
   - Then rerun the real entries after each.
2. **A start for the role gate that doesn't hang on one segment.** For example, let the expert learn a little on NoGo segments, at a small learning rate rather than none, so the teacher's signal can grow before the gate trusts it.
3. **Finer offset buckets beyond 8** for the mix between the views and the background share, to test whether they cause the extra cost deep in files when told.
