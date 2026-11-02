# 66 · Real files after real manuals (2026-11-02)

*[All experiments](README.md)* · [← 65](65-role-gate.md)

**The question:** experiments 60 to 65 used made-up formats with descriptions in one controlled wording. Can the description expert, with experiment 65's role gate, use a real format's manual to read real files of that format the first time it meets them? And does it matter whether it learned the notation first on the made-up formats?

**The stream:** 28 real formats, 6 files of each from this machine (the first 1,024 bytes of each), chosen by `file`(1)'s verdict.
- **The formats:** png, tzif, tfm, vf, mo, lz4, gzip, bzip2, xcursor, gvariant, llvmbc, ar, woff2, otf, ttf, pyc, elf, jpeg, gif, wav, zip, pdf, tar, jmod, ole2, svg, xml and ps.
- **The manual:** the format's entry in `file`(1)'s magic database (file 5.45 sources), verbatim.
  - That is up to 4 comment lines, the top-level test line, and up to 10 continuation lines, at most 600 bytes.
  - It states the magic in libmagic's own notation, for example `0 string \x89PNG\x0d\x0a\x1a\x0a`, `0 string \037\213` (gzip) or `0 string PK\003\004` (zip).
- **Before each file comes one of:**
  - **Rm:** its format's manual entry.
  - **Rn:** a note that puts the same test in experiment 60's notation, for example `format png. magic 89 50 4e 47 0d 0a 1a 0a.` Where the test isn't at offset 0, the note says `magic at offset 257:` instead.
  - **W:** another format's manual entry.
  - **N:** nothing.
- **Order:** formats appear progressively (a new one with probability 0.35 at each step), so most first files come in the first 40% of the stream.
- **Rotations:** four, with the same files in the same order. The k-th file of format f gets condition (f + k + rotation) mod 4. Across the four rotations, every file is seen once under each condition, so the comparisons are paired by file.
- **Two versions:**
  - `real`: the 187 KB stream alone.
  - `prereal`: experiment 60's progressive made-up-formats stream (594 KB) first. There the expert learns that a token like `89` means byte 0x89, and the role gate learns what a description looks like.
- **None of the 28 formats is in the made-up stream.** So every first file is of a format never seen before.

**Four arms** (the exp 63–65 stack; binary `sit-65d`):
- `base`: no expert.
- `desc`: the expert, guessing that the previous segment is always a description.
- `role`: experiment 65's best gate. It is taught by whether the previous segment beats both a decoy and no description.
- `oracle`: the expert, told which segments are descriptions.

**Scored:**
- The magic bytes: the test's bytes at its offset, on files where they match. That is 167 of the 168 files; one svg file starts differently.
- The first 32 bytes, and all file bytes.
- First files of a format are scored apart from later ones.

## What we found

- **On real files alone, the manuals don't help, right or wrong.** On the magic of new formats, the expert gains about as much after a wrong entry as after the right one. Its gain over `base` is +0.83 bits per byte after the right entry, +0.99 after a wrong one and +0.56 after nothing.
  - Right minus wrong, paired by format: Rm −0.16 ± 0.19, Rn −0.49 ± 0.14 (positive would favour the right one).
  - The expert's own prediction on that magic is about 7.7 bits per byte under every condition. Its gain in the stack is generic: it comes from more words and statistics, not from reading the description.
- **After the made-up formats, the note in hex is read, and it works on real files.** The expert learned the notation on invented formats and reads it in notes about real ones.
  - Expert alone, on the magic of new formats, nothing before (N) → the note (Rn): 7.85 → 6.94 without a gate, 7.73 → 6.19 with the gate, and 8.00 → 5.82 with the oracle.
  - With the gate it is better after the note on all 28 formats. The gains run from 4.3 bits per byte (gzip) to 0.4 (gvariant).
  - In the mixed code: `base` 8.10 → `role` **7.16**, oracle 6.76.
  - Right minus wrong, paired by format, gain over `base`: `role` +0.47 ± 0.23, oracle +0.59 ± 0.25. Without a gate it is +0.05 ± 0.25.
- **The verbatim manual entries are not read, even after the made-up formats.**
  - Expert alone, Rm minus N on new-format magic: +0.04, −0.02 and −0.07 in the three expert arms. In the mixed code, Rm minus W is −0.29, −0.18 and −0.17.
  - The likely reason is notation. libmagic packs the whole magic into one token (`\x89PNG\x0d\x0a\x1a\x0a`, `\177ELF`, `PK\003\004`). The expert's unit is a whole whitespace token, so it can't learn that `\x89` or `\177` names one byte, or that `PNG` spells three. In the note, every byte is its own token.
- **The role gate learns that a segment is a manual, not that it is this file's manual.** Its teacher, after the made-up formats, says Go after a wrong entry about as often as after the right one (58–68% against 57–72%).
  - After a file, the gate says Go 18–31% of the time (first and later files), against 17% on the made-up stream (exp 65).
  - Overall agreement with the truth on the real part: 64–71% per rotation, against 84.5% on the made-up stream.
  - Without the made-up stream, the gate is more cautious. It says Go after about half of descriptions and about 10% of files, and about 37% after notes.
- **On whole files, every expert arm gains about the same under all three description conditions, about 0.2 bits per byte on first files.** The oracle gains nothing where no description came before, because there it is kept out.
  - Even the oracle's whole-file gain is the same after a wrong entry as after the right one: right minus wrong is −0.01 ± 0.02 after the made-up stream and +0.00 alone.
  - So most of the expert's whole-file gain on real files is generic too.
- **The guessing expert hurts later files' magic, where the stack already knows it.**
  - After the made-up formats, the note before a later file: `base` 4.09, `desc` 4.77, `role` 4.27, oracle 4.08.
  - The entry before a later file: 4.94, 5.27, 4.97, 4.87.
  - The gate removes most of this harm.

## The magic bytes of new formats

Bits per byte, first file of each format, pooled over the four rotations (119 bytes in each cell).

**Mixed code** (what the stack pays):

| Stream | Arm | Rm (its entry) | Rn (note) | W (wrong entry) | N (nothing) |
|---|---|---|---|---|---|
| real alone | `base` | 8.19 | 8.19 | 8.82 | 8.44 |
| | `desc` | 7.48 | 7.75 | 7.88 | 7.92 |
| | `role` | 7.91 | 8.09 | 8.35 | 8.37 |
| | oracle | 7.55 | 7.76 | 8.10 | 8.28 |
| after made-up | `base` | 7.96 | 8.10 | 8.87 | 8.43 |
| | `desc` | 7.49 | 7.27 | 8.04 | 8.05 |
| | `role` | 7.66 | **7.16** | 8.32 | 8.38 |
| | oracle | 7.42 | **6.76** | 8.08 | 8.32 |

**Expert alone:**
- This is the expert's own prediction on every byte.
- When it has no words to align, for example under NoGo or where the oracle says there is no description, it predicts 8 bits per byte. So across arms, the N column and the whole-file figures mostly measure how often the expert is kept out.

| Stream | Arm | Rm | Rn | W | N |
|---|---|---|---|---|---|
| real alone | `desc` | 7.76 | 7.71 | 7.70 | 7.74 |
| | `role` | 7.78 | 7.75 | 7.79 | 7.83 |
| | oracle | 7.90 | 7.63 | 7.79 | 8.00 |
| after made-up | `desc` | 7.88 | 6.94 | 7.92 | 7.85 |
| | `role` | 7.80 | **6.19** | 7.73 | 7.73 |
| | oracle | 8.01 | **5.82** | 7.80 | 8.00 |

**Paired by format** (28 formats, each format's first file under each condition across the four rotations):
- Each cell is the gain over `base` in the mixed code, in bits per byte, with the right description minus the wrong entry ± its standard error.
- "Better in" is the count of the 28 formats where the right description won.

| Stream | Arm | Rm − W | Rn − W |
|---|---|---|---|
| real alone | `desc` | −0.16 ± 0.19 (better in 10) | −0.49 ± 0.14 (10) |
| | `role` | −0.24 ± 0.16 (7) | −0.38 ± 0.15 (7) |
| | oracle | −0.01 ± 0.16 (13) | −0.29 ± 0.16 (11) |
| after made-up | `desc` | −0.29 ± 0.22 (9) | +0.05 ± 0.25 (14) |
| | `role` | −0.18 ± 0.21 (8) | **+0.47 ± 0.23** (14) |
| | oracle | −0.17 ± 0.21 (8) | **+0.59 ± 0.25** (18) |

- **The wrong entry hurts `base` on its own** (after the made-up stream: W 8.87 against N 8.43).
  - Part of every arm's gain under W is the expert making up some of that loss. So the "gain over `base`" columns flatter W.
  - Compared with nothing before (N), the note after the made-up stream is better in the mixed code for 23 of 28 formats in both `desc` and `role`, and for 26 in the oracle.
  - With the gate, the note is better than the wrong entry in the mixed code by 1.16 bits per byte (7.16 against 8.32), but the paired standard error is large. Most of the spread comes from the formats where `base` itself does well.

## First 32 bytes and whole files

Mixed code, bits per byte, first files (each cell 896 bytes for the first 32, 28,672 for whole files):

| Stream | Arm | 32 bytes: Rm / Rn / W / N | Whole: Rm / Rn / W / N |
|---|---|---|---|
| real alone | `base` | 6.24 / 6.28 / 6.30 / 6.27 | 5.60 / 5.62 / 5.62 / 5.62 |
| | `desc` | 5.86 / 5.96 / 5.92 / 6.10 | 5.34 / 5.31 / 5.37 / 5.35 |
| | `role` | 6.08 / 6.18 / 6.15 / 6.26 | 5.51 / 5.52 / 5.45 / 5.57 |
| | oracle | 5.84 / 6.02 / 5.93 / 6.31 | 5.35 / 5.32 / 5.37 / 5.62 |
| after made-up | `base` | 6.17 / 6.26 / 6.27 / 6.29 | 5.59 / 5.61 / 5.61 / 5.62 |
| | `desc` | 5.85 / 5.81 / 5.94 / 6.04 | 5.41 / 5.40 / 5.43 / 5.42 |
| | `role` | 5.94 / 5.87 / 5.98 / 6.23 | 5.45 / 5.44 / 5.44 / 5.56 |
| | oracle | 5.73 / 5.75 / 5.83 / 6.32 | 5.32 / 5.35 / 5.33 / 5.62 |

- **Later files, whole:** 4.86–4.89 for `base` under every condition. The expert arms reach 4.68–4.79.
- **Description bytes:**
  - Real alone: `base` and oracle 2.702, `role` 2.682, `desc` 2.515.
  - After the made-up stream: 2.671, 2.610 and 2.407.
  - The guessing expert also predicts the next description from the one before, since it treats every segment as one.
- **The made-up stream first doesn't help real files as a whole.** For example, whole first files with `role` cost 5.44–5.56 after it against 5.45–5.57 alone. Its gain is on the note's magic.

## The role gate on real files

Go rate at the start of each real file, by what came before it, over the four rotations (28 first files and 140 later files per condition):

| Before | Real alone: gate / teacher, first · later | After made-up: gate / teacher, first · later |
|---|---|---|
| Rm, its entry | 0.36 / 0.50 · 0.57 / 0.59 | 0.50 / 0.57 · 0.66 / 0.72 |
| Rn, note | 0.39 / 0.39 · 0.37 / 0.36 | 0.71 / 0.68 · 0.73 / 0.76 |
| W, wrong entry | 0.64 / 0.54 · 0.52 / 0.51 | 0.61 / 0.68 · 0.65 / 0.58 |
| N, a file | 0.22 / 0.11 · 0.09 / 0.14 | 0.18 / 0.21 · 0.31 / 0.34 |
| a file, before a description | 0.09 / 0.07 · 0.13 / 0.10 | 0.18 / 0.17 · 0.35 / 0.34 |

- **The teacher's question is right for the role and wrong for this job.**
  - It asks whether the previous segment explains the next 16 bytes better than nothing does (and better than a decoy).
  - A wrong manual entry often does. Its tokens have been aligned with many files' bytes before, so they know common bytes such as `00`.
  - So the gate learns "manual" (W is a manual), and the expert reads wrong entries as well. The mixed code still pays for them: W is the worst condition for `role`.
- **After the made-up stream, the gate agrees with its teacher on 78–80% of segments in the real part** (exp 65: 93.6% on the made-up stream). The real manual entries are long, varied and full of tokens the gate has never seen.
- **Script:** `scripts/exp66/role_cond.py`. Agreement overall: `scripts/exp65/role_agree.py` with the stream's offset.

## How it was run

- **Stream builder:** `scripts/exp66/build_real.py` (seed 66). It takes:
  - the list of files with their `file`(1) verdicts (`runs/exp66/ftypes_all.tsv`);
  - the magic database sources (file 5.45, `Magdir`);
  - the format table: name, verdict pattern, magic file and the entry's line.
- **Format fixes:**
  - tfm's anchor is the `0012` test, which is what this machine's TFM files have.
  - "ispell" was dropped, because `file` called TFM files ispell.
- **Excerpts:**
  - The excerpts are the real entries, but some carry neighbouring comments. gzip's, for example, has four comment lines about other formats above its test line.
  - The note's wording comes from the test line and its message: `format NAME. MESSAGE. magic HEX.`
- **Runs:** `runs/exp66/run.sh`, with 32 runs: two streams × four rotations × four arms.
  - These are experiment 63–65's shared flags, with `--file-starts` and `ROLE_DUMP=1`.
  - Runs took 2–3.5 minutes on the real stream alone, and about 20 minutes after the made-up stream.
- **Scorer:** `scripts/exp66/score.py ITEMS_PATTERN NAME=BITS_PATTERN ...`, pooling the four rotations. The first named run is the reference for the paired gains.

## What this means

- **The expert can carry what it learned about a notation from invented formats to real ones.** After the made-up stream, a short note in hex lowers the expert's cost on a real format's magic by 1.6 bits per byte (with the gate) on every one of the 28 formats. It does so the first time the expert meets the format.
  - This is the first time a description has helped on real files in the right-minus-wrong comparison.
  - But it helps only in the notation it practised. Without the made-up stream first, the note does nothing specific.
- **Real manuals aren't read.** The way they are written is the barrier, not the role: the oracle knows the role and still gains nothing specific from them.
  - A whitespace token can't be decoded into bytes. `\x89PNG` is one token, used by one format, so no other format can teach what it means.
  - Reading them needs the expert to read inside a token: escapes (`\x89`, `\037`) as bytes, and plain letters as the bytes they spell.
- **The role gate needs a teacher that tells the right manual from a wrong one.** "Better than nothing" was the right question on the made-up stream, where a wrong description rarely beat nothing. On real files it often does, because manual tokens learn bytes that are common everywhere.
- **Caveats:**
  - One seed for each run. Each comparison is paired by file over four rotations, but 28 formats is few, and the paired standard errors are large (about 0.2 bits per byte).
  - Only the first 1,024 bytes of each file are used.
  - The files come from one Linux machine, so some formats are narrow there. For example, all the ttf files may be from one font family.

## Next

1. **A reader of spellings.** Let the expert align byte-level pieces of a token, as well as whole tokens:
   - libmagic's escapes as the bytes they name;
   - letters as their own bytes, so that `PNG` predicts `50 4e 47`.

   Then test the verbatim entries again, alone and after the made-up stream.
2. **A role teacher that credits the right description only.** For example, against a wrong segment of the same kind (another description) rather than nothing, now that the gate already knows which segments are descriptions.
3. **Keep the expert out of later files that the stack already predicts**, where it now costs 0.2–0.7 bits per byte on the magic.
