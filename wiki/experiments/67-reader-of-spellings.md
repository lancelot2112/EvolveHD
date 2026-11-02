# 67 · A reader of spellings (2026-11-02)

*[All experiments](README.md)* · [← 66](66-real-files-real-manuals.md)

**The question:** in experiment 66 the description expert read a note written in hex (`magic 89 50 4e 47`) on real formats, but not the formats' real manual entries. There the magic is spelled inside a token, as in `\x89PNG\x0d\x0a` or `\037\213`. If the expert can also read the characters inside a token, will it read those entries? And how much does practice on experiment 60's made-up formats matter?

**The change: a spelling view beside the token view** (`--desc-spell`, in `src/acts.rs`, `Spell`).
- **Positions:** the characters of the description's tokens, up to 512 positions; tokens of up to 64 bytes.
- **Pieces:** each position is described by the 1-, 2- and 3-character pieces that end at it, within its token. Byte 1 stands for "before the token's start".
  - Each piece has its own learned byte distribution.
  - How much each piece length counts is a learned share, kept apart for pieces that have little credit yet (novices).
  - Unknown pieces predict uniformly. A piece is created when it first earns credit (cap 400,000 pieces).
- **Moving and starting:**
  - The view learns its own jumps between characters.
  - Where it starts reading is learned per (previous token, character offset in the token). The token view keys its start by the token's position instead.
- **How the two views combine:** a learned share per offset bucket (the byte's offset in the file, 0 to 8, with 8 and beyond pooled) mixes them. Each view learns as if it were alone, from its own responsibility against the background.
  - A first version split the credit between the views instead. That raised the expert's cost on whole files by about 1 bit per byte in a pilot, so it was dropped.
- **Flag off:** with `--desc-spell` off, the binary (`sit-67b`) reproduces experiment 66's runs byte for byte.

**The streams:** experiment 66's, unchanged.
- 28 real formats, 6 files each.
- Before each file comes one of: its manual entry (Rm), a hex note (Rn), another format's entry (W) or nothing (N).
- Four rotations, so every file is seen once under each condition.
- `real`: the real files alone. `prereal`: experiment 60's made-up stream first.

**Three new arms**, compared with experiment 66's `base`, `desc`, `role` and `oracle` on the same streams:
- `spell`: the expert with the spelling view, guessing that the previous segment is always a description.
- `srole`: the same, with experiment 65's role gate (`--desc-role-contrast --desc-role-null 2`).
- `soracle`: the same, told which segments are descriptions.

**The key test:** the magic bytes of each format's first file, scored on the expert's own cost, right entry minus wrong entry (Rm − W). Each byte is classed by how its entry writes it:
- **hex escape** (`\x89`: png, llvmbc, pyc; 9 bytes);
- **hex number** (`0xffd8ff`: jpeg and jmod, big-endian; lz4, little-endian);
- **octal escape** (`\037`: 8 formats, 25 bytes);
- **plain character** (18 formats, 71 bytes);
- **other escape** (`\n`, `\<`; 3 bytes).

## What we found

- **The spelling view doesn't read the manual entries.** No arm reads them, guessing or told, with or without the made-up stream first.
  - After the made-up stream, told which segments are descriptions, the expert's own cost on hex-escape bytes is 0.24 bits higher after the right entry than after a wrong one. In experiment 66 it was 0.14 higher.
  - Octal bytes: −0.09, against +0.17. Plain characters: +0.04, against +0.30. All are within about one standard error of zero, or worse.
- **It reads the notes much better, if it learned the notation first.**
  - After the made-up stream and told which segments are descriptions, the expert's cost on a note's magic falls from 5.82 to 5.19 bits per byte (Rn − W −2.2 to −3.3 by notation class, against −1.9 to −2.7).
  - The stack's mixed code on that magic falls from 6.76 to 6.24 bits per byte. Against nothing before the file, per format, that is −2.16 bits per byte against −1.56, lower in 26 of 28 formats in both.
  - Guessing instead of told, the view adds nothing to the note: 6.93 against 6.94 on the expert alone.
- **On the real files alone, it changes nothing specific.** Every Rm − W and Rn − W is within noise. Whole files are the same (5.34 against 5.34 guessing; 5.36 against 5.35 told).
- **Whole files after the made-up stream:** unchanged when guessing (5.42 against 5.41 on first files). Told, they cost more: 5.48 against 5.32 on first files.
  - The extra cost sits deep in the file: the expert alone is at 6.98 against 6.39 at offsets 32–255, and 7.26 against 6.51 beyond 256.
  - Offsets 8 and beyond share one bucket, both for the mix between the views and for the background share.
- **The role gate didn't start after the made-up stream.**
  - It said Go once, on the first segment, and never again, in all four rotations, so the arm is the base stack (identical bits).
  - On the real files alone it does start (Go on 56–96 segments). As in experiment 66, its teacher calls the right entry and a wrong one descriptions about equally often.

## The magic bytes by notation

**The expert's own cost on the magic bytes of each format's first file, after the made-up stream** (bits per byte, pooled over the four rotations). Rm − W pairs each byte under the right and wrong entry; "cheaper" counts the bytes cheaper after the right entry.

| arm | notation | bytes | Rm | Rn | W | Rm − W (se) | cheaper | Rn − W (se) |
|---|---|---|---|---|---|---|---|---|
| oracle | hex escape | 9 | 8.43 | 5.60 | 8.30 | +0.14 (0.41) | 1 | −2.69 (0.58) |
| soracle | hex escape | 9 | 8.40 | 4.83 | 8.16 | +0.24 (0.26) | 3 | −3.33 (0.52) |
| oracle | hex number | 7 | 7.13 | 5.50 | 7.42 | −0.29 (0.30) | 3 | −1.92 (0.67) |
| soracle | hex number | 7 | 7.03 | 5.17 | 7.39 | −0.37 (0.32) | 5 | −2.22 (0.52) |
| oracle | octal | 25 | 7.52 | 5.47 | 7.36 | +0.17 (0.09) | 11 | −1.88 (0.33) |
| soracle | octal | 25 | 7.48 | 5.19 | 7.57 | −0.09 (0.15) | 14 | −2.38 (0.39) |
| oracle | plain character | 71 | 8.21 | 6.00 | 7.91 | +0.30 (0.11) | 21 | −1.91 (0.17) |
| soracle | plain character | 71 | 7.86 | 5.22 | 7.82 | +0.04 (0.06) | 39 | −2.60 (0.16) |
| role | hex escape | 9 | 8.25 | 6.00 | 8.33 | −0.07 (0.24) | 3 | −2.33 (0.50) |
| spell | hex escape | 9 | 8.35 | 7.30 | 8.61 | −0.26 (0.21) | 4 | −1.31 (0.37) |
| spell | octal | 25 | 7.57 | 6.95 | 7.77 | −0.21 (0.19) | 17 | −0.83 (0.26) |
| spell | plain character | 71 | 7.48 | 6.83 | 7.51 | −0.02 (0.09) | 44 | −0.68 (0.13) |

- **The little-endian hex number** (lz4, 4 bytes) and the other escapes (3 bytes) are too few to read anything from. Without the oracle they go both ways.
- **The stack's copy act still does the plain characters.** In the mixed code, the base stack's Rm − W on plain-character bytes is −1.41 (57 of 71 bytes cheaper). Every expert arm is at −0.89 to −1.12, so none adds to it.
- **No note byte is ever read sharply.** Told which segments are descriptions, no magic byte after a note costs the expert under 2 bits, and 13% cost under 4 (9% in experiment 66). That fits a reader that knows about one hex digit of each byte (4 bits) plus some doubt about where it is, not two digits.

## Why the entries aren't read (not yet tested)

Two things in how the view learned the notes don't carry over to the entries:
- **Where in the word the byte is read.** In a note each byte is its own two-character word (`89`). Its first character has the pieces "start + `8`" and "start, start + `8`", which only occur at the start of a word.
  - If the view learned the byte at that character, what it knows is anchored at word starts.
  - In `\x89PNG` the `8` is mid-word, after `x`, so those pieces never occur there. The 2-bit floor above suggests the view reads at the first digit.
- **Where reading starts.** The view learned to start after the word `magic` at offset 0 of the next word. In the entries the magic follows `string`, `belong` or `beshort`, at character offset 2 for an escape. That context was never seen.

## Whole files and the role gate

**Mixed code on all bytes of first files** (bits per byte):

| stream | base | desc | spell | oracle | soracle |
|---|---|---|---|---|---|
| after the made-up stream, Rm | 5.59 | 5.41 | 5.42 | 5.32 | 5.48 |
| after the made-up stream, N | 5.62 | 5.42 | 5.43 | 5.62 | 5.62 |
| real files alone, Rm | 5.60 | 5.34 | 5.34 | 5.35 | 5.36 |

**The expert's own cost by offset in first files, after the made-up stream** (bits per byte):

| arm | 0 | 1–3 | 4–8 | 9–31 | 32–255 | 256+ |
|---|---|---|---|---|---|---|
| oracle | 7.70 | 7.35 | 6.82 | 6.17 | 6.39 | 6.51 |
| soracle | 7.65 | 7.14 | 6.52 | 6.22 | 6.98 | 7.26 |
| desc | 8.04 | 7.58 | 6.59 | 5.94 | 6.28 | 6.44 |
| spell | 7.91 | 7.36 | 6.39 | 5.76 | 6.40 | 6.65 |

The view helps at the start of files and costs deep in them.

**The role gate after the made-up stream:**
- **How it starts:** its weights start at zero, so it says Go on the first segment, where the expert learns.
  - In experiment 66, the first time the teacher said "description" was at file 65. There the token view beat both the decoy and no description by 2.97 bits over 16 bytes (108.69 against 111.66), just over the 2-bit threshold.
  - With the spelling view, the shared background had learned more on that one Go segment. At file 65 no description cost 109.42 and the previous segment 110.56, so the teacher said no. It never said yes in the whole stream, and the gate stayed off.
- **Two teacher changes, piloted on the first rotation, didn't start it either:**
  - reading only spelling positions with a learned piece;
  - taking the best of the token view, the spelling view and their mix, for both the segment and the decoy.

  Both were reverted, so the code is what ran.
- **What this shows:** experiment 65's gate starts from one segment's chance gain, which is fragile.

**The role gate on the real files alone** (`srole`; gate's Go rate and teacher's label rate at the start of each file):

| before | files | gate Go | teacher Go |
|---|---|---|---|
| Rm | first | 0.18 | 0.18 |
| W | first | 0.32 | 0.18 |
| Rn | first | 0.50 | 0.39 |
| N | first | 0.07 | 0.04 |

The teacher still can't tell a right entry from a wrong one. `srole`'s octal Rm − W on the real stream alone (−0.63, se 0.28) is one of about 40 comparisons, and nothing else in that arm moves, so it is read as noise.

## How it was run

- **Binary:** `runs/exp36/sit-67b`.
- **Runs:** `runs/exp67/run.sh`, 24 runs: two streams × four rotations × three arms. Experiment 66's flags, plus `--desc-spell`.
  - Runs took about 20 minutes after the made-up stream and about 4.5 minutes on the real files alone, two at a time.
  - The first background job hit its 2-hour limit. A restart reran the two unfinished runs from scratch.
- **Scorers:**
  - `scripts/exp67/notation.py ITEMS_PATTERN NAME=BITS_PATTERN ...` classes each magic byte by its entry's notation (from the entry's test line) and gives per-class means, with paired Rm − W and Rn − W.
  - `scripts/exp66/score.py` and `scripts/exp66/role_cond.py`, as in experiment 66.
- **Pilot diagnostics** (not committed, described above): a teacher-cost printout (`COSTS k= prev= dec= null=`) on the first rotation, with and without the spelling view.

## What this means

- **Reading inside a token is necessary but not enough.** The view uses what it learned from notes to read notes better, but none of it carries over to the libmagic spellings.
  - The pieces it learned are tied to where a byte sat in the practice notation (its own word, read at the word's start, after `magic`).
  - A byte spelled mid-word or after another word looks new to it.
- **The practice has to look like the target, or the reader has to learn without the target's layout.** Either:
  - practise on descriptions that spell bytes as libmagic does (escapes mid-word, after `string`); or
  - make the pieces independent of word boundaries and the start independent of the previous word, so the same digits are recognised wherever they sit.
- **The role gate needs a sturdier start.** One chance segment decides whether it ever turns on. That was hidden in experiment 66 and exposed by a small change to what the background learned.
- **Caveats:**
  - Each run was done once.
  - There are only 9 hex-escape bytes in 3 formats, 25 octal bytes in 8.
  - The reason given for the miss is a hypothesis that fits the 2-bit floor, not a tested result.

## Next

1. **Test the hypothesis directly.** Add a practice stream in libmagic's own spelling: experiment 60's made-up formats, with entries written `0 string \x3c\x3f...`. Then rerun the real entries.
   - If the real entries are then read, the layout was the barrier.
   - If they aren't, the pieces' anchoring is.
2. **A start for the role gate that doesn't hang on one segment.** For example, let the expert learn a little on NoGo segments (a small learning rate rather than none), so the teacher's signal can grow before the gate trusts it.
3. **Finer offset buckets beyond 8** for the view mix and the background share, to remove the cost deep in files under the oracle.
