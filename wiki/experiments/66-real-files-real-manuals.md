# 66 · Real files after real manuals (2026-11-02)

*[All experiments](README.md)* · [← 65](65-role-gate.md) · [67 →](67-reader-of-spellings.md)

**The question:** experiments 60 to 65 used made-up formats with descriptions in one controlled wording. Can the description expert, with experiment 65's role gate, use a real format's manual to read real files of that format the first time it meets them? And does it matter whether it learned the notation first on the made-up formats?

**The stream:** 28 real formats, 6 files of each from this machine (the first 1,024 bytes of each), chosen by `file`(1)'s verdict.
- **The formats:** png, tzif, tfm, vf, mo, lz4, gzip, bzip2, xcursor, gvariant, llvmbc, ar, woff2, otf, ttf, pyc, elf, jpeg, gif, wav, zip, pdf, tar, jmod, ole2, svg, xml and ps.
- **The manual:** the format's entry in `file`(1)'s magic database (file 5.45 sources), verbatim.
  - That is up to 4 comment lines above the entry, its first line, and up to 10 continuation lines, at most 600 bytes.
  - It states the magic in libmagic's own notation, for example `0 string \x89PNG\x0d\x0a\x1a\x0a`, `0 string \037\213` (gzip) or `0 string PK\003\004` (zip).
- **Before each file comes one of:**
  - **Rm:** its format's manual entry.
  - **Rn:** a note that puts the same test in experiment 60's notation, for example `format png. magic 89 50 4e 47 0d 0a 1a 0a.` Where the test isn't at offset 0, the note says `magic at offset 257:` instead.
  - **W:** another format's manual entry.
  - **N:** nothing.
- **Order:** formats appear progressively (a new one with probability 0.35 at each step). 25 of the 28 first files come in the first 40% of the stream, and all of them in the first 42%.
- **Rotations:** four, with the same files in the same order. The k-th file of format f gets condition (f + k + rotation) mod 4. Across the four rotations, every file is seen once under each condition, so the comparisons are paired by file.
- **Two versions:**
  - `real`: the 187 KB stream alone.
  - `prereal`: experiment 60's progressive made-up-formats stream (594 KB) first. There the expert learns that a token like `89` means byte 0x89, and the role gate learns what a description looks like.
- **None of the 28 formats is in the made-up stream.** Its formats are numbered and have random 2–4-byte magics, so every first file is of a format never seen before. One made-up magic (`3c 3f 32 08`) shares its first two bytes with xml's `<?`.

**Four arms** (the exp 63–65 stack; binary `sit-65d`):
- `base`: no expert.
- `desc`: the expert, guessing that the previous segment is always a description.
- `role`: experiment 65's best gate. It is taught by whether the previous segment beats both a decoy and no description by more than 2 bits over the next 16 bytes.
- `oracle`: the expert, told which segments are descriptions.

**Scored:**
- The magic bytes: the test's bytes at its offset, on files where they match. That is 167 of the 168 files; one svg file starts differently.
- The first 32 bytes, and all file bytes.
- First files of a format are scored apart from later ones.

## What we found

- **On real files alone, no description helps the expert specifically, right or wrong.**
  - In the stack, the guessing expert (`desc`) gains about as much on new-format magic after a wrong entry as after the right one. Its gains over `base` are +0.83 bits per byte after the right entry, +0.99 after a wrong one and +0.56 after nothing.
  - The other arms show the same pattern. For `role` the three gains are +0.27, +0.50 and +0.06; for the oracle +0.74, +0.75 and +0.16.
  - The expert's own prediction on that magic is about 7.7 bits per byte under every condition. Paired by format, the note minus a wrong entry is +0.02 ± 0.03 (`desc`), −0.05 ± 0.12 (`role`) and −0.14 ± 0.09 (oracle).
  - **So the gain in the stack is generic.** On a new format's first bytes, `base` is worse than a uniform guess (8.2–8.9 bits per byte). The mixture hedges it with a near-uniform expert and its own uniform part.
    - The oracle gains +0.16 over `base` even with no description before, where its expert is exactly uniform.
    - W shows the biggest "gain" because `base` is worst there: a wrong entry before a file hurts `base` itself (8.82 against 8.44 after nothing).
- **After the made-up formats, the note in hex is read on real files.** The expert learned the notation on invented formats, and it reads it in notes about real ones.
  - **Expert alone, note minus wrong entry, paired by format:**
    - `desc` −1.01 ± 0.15 bits per byte, better in 27 of 28 formats;
    - `role` −1.58 ± 0.20, 27 of 28;
    - oracle −2.01 ± 0.19, 28 of 28.
    - On the real stream alone the same figures are +0.02, −0.05 and −0.14.
  - **Expert alone, nothing before (N) → the note (Rn):** 7.85 → 6.94 (`desc`), 7.73 → 6.19 (`role`), and 8.00 → 5.82 (oracle). In `role` it is lower after the note on all 28 formats, by 0.4 (gvariant) to 4.3 bits per byte (gzip), and 1.6 on average.
  - **In the mixed code**, the note's magic: `base` 8.10, `desc` 7.27, `role` **7.16**, oracle 6.76.
    - Per format, the note against a wrong entry, raw bits: `role` −1.20 ± 0.23 (better in 23 of 28), oracle −1.32 ± 0.22 (25 of 28).
    - Against nothing: better in 23, 23 and 26 of 28 formats for `desc`, `role` and the oracle.
    - Measured as gain over `base` (which already loses under W), the note's edge over a wrong entry is smaller: `role` +0.47 ± 0.23, oracle +0.59 ± 0.25, `desc` +0.05 ± 0.25.
  - **Where the magic sits in the note matters.**
    - In 6 notes the hex starts at the third token (`format X. magic …`), the place experiment 60's descriptions put it. In the other 19 a message sentence comes first.
    - The first group gains about twice as much (expert alone, Rn − N): `role` −2.43 against −1.33; oracle −3.41 against −1.83.
- **The expert doesn't read the verbatim manual entries, even after the made-up formats.**
  - Expert alone, the entry minus a wrong entry, paired by format: `desc` −0.06 ± 0.09, `role` −0.00 ± 0.10, oracle +0.13 ± 0.11 (worse).
  - The likely reason is notation. libmagic packs the whole magic into one token (`\x89PNG\x0d\x0a\x1a\x0a`, `\177ELF`, `PK\003\004`). The expert's unit is a whole whitespace token, so it can't learn that `\x89` or `\177` names one byte, or that `PNG` spells three. In the note, every byte is its own token.
  - **But the stack without the expert does use them,** by copying plain-ASCII magic from the entry.
    - On first files, `base` pays 0.5–5 bits per byte on gvariant's bytes 2–7 after its entry, and the same on `<arch>`'s bytes 3–6 for ar.
    - Per format, the entry against a wrong one in raw bits: `base` −0.71 ± 0.25 (better in 21 of 28) after the made-up stream, −0.43 ± 0.25 (20 of 28) alone.
    - So the right entry does help the stack, through copying and not through the expert.
- **The role gate learns that a segment is a manual, not that it is this file's manual.**
  - **Teacher:** after the made-up formats, it says Go after a wrong entry about as often as after the right one (58–68% against 57–72%).
  - **Go after a file:** after the made-up stream the gate says Go 18–35% of the time, against 17% on the made-up stream itself (exp 65).
  - **Agreement with the truth:**
    - On the real part after the made-up stream: 64–71% per rotation, against 84.5% on the made-up stream (exp 65).
    - On the real stream alone it is 70–72% (teacher 71–73%), and the gate is more cautious. It says Go after 36–64% of manual entries, 37–39% of notes and about 10% of files.
- **On whole files, every expert arm gains about the same under all three description conditions: 0.1–0.3 bits per byte on first files.**
  - The oracle gains nothing where no description came before, because there it is kept out.
  - Even the oracle's whole-file gain is the same after a wrong entry as after the right one. Right minus wrong is −0.01 ± 0.02 after the made-up stream, and +0.00 alone.
  - So the expert's whole-file gain on real files is generic too.
- **The guessing expert hurts later files' magic, where the stack already knows it.**
  - After the made-up formats:
    - With the note before a later file: `base` 4.09, `desc` 4.77, `role` 4.27, oracle 4.08.
    - With the entry: 4.94, 5.27, 4.97, 4.87.
  - On the real stream alone it is worse. With the note: `base` 4.01, `desc` 4.99, `role` 4.25, oracle 4.45.
  - The gate removes most of this harm.
  - After a wrong entry or nothing, the expert helps later files instead (after the made-up stream, W: `base` 7.68, `desc` 6.60, `role` 7.07, oracle 6.84).

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
- This is the expert's own prediction on every byte, whether or not the stack uses it.
- Under NoGo the expert still predicts from the previous segment, but doesn't learn there and isn't mixed in. Only where the oracle says there is no description does it predict exactly 8 bits per byte (no words, so uniform).
- So the oracle's N column measures being kept out. The `role` figures show an expert that learned only on Go segments.

| Stream | Arm | Rm | Rn | W | N |
|---|---|---|---|---|---|
| real alone | `desc` | 7.76 | 7.71 | 7.70 | 7.74 |
| | `role` | 7.78 | 7.75 | 7.79 | 7.83 |
| | oracle | 7.90 | 7.63 | 7.79 | 8.00 |
| after made-up | `desc` | 7.88 | 6.94 | 7.92 | 7.85 |
| | `role` | 7.80 | **6.19** | 7.73 | 7.73 |
| | oracle | 8.01 | **5.82** | 7.80 | 8.00 |

**Right minus wrong, paired by format:**
- Each cell is the mean over 28 formats, ± its standard error, with the number of formats where the right description was cheaper. Negative means the right description was cheaper.
- Raw bits per byte on each format's first file, under each condition across the four rotations.

| Stream | Arm | Expert alone: Rm − W | Expert alone: Rn − W | Mixed: Rm − W | Mixed: Rn − W |
|---|---|---|---|---|---|
| real alone | `base` | | | −0.43 ± 0.25 (20) | −0.59 ± 0.16 (19) |
| | `desc` | +0.07 ± 0.04 (10) | +0.02 ± 0.03 (11) | | |
| | `role` | −0.01 ± 0.05 (15) | −0.05 ± 0.12 (15) | −0.19 ± 0.23 (17) | −0.22 ± 0.15 (13) |
| | oracle | +0.11 ± 0.06 (11) | −0.14 ± 0.09 (18) | −0.41 ± 0.18 (19) | −0.31 ± 0.12 (18) |
| after made-up | `base` | | | −0.71 ± 0.25 (21) | −0.73 ± 0.17 (24) |
| | `desc` | −0.06 ± 0.09 (18) | **−1.01 ± 0.15 (27)** | | |
| | `role` | −0.00 ± 0.10 (14) | **−1.58 ± 0.20 (27)** | −0.52 ± 0.26 (16) | **−1.20 ± 0.23 (23)** |
| | oracle | +0.13 ± 0.11 (11) | **−2.01 ± 0.19 (28)** | −0.54 ± 0.21 (19) | **−1.32 ± 0.22 (25)** |

- **`base` is cheaper after the right entry or the note than after a wrong entry.** A long wrong entry hurts it, and it copies plain-ASCII magic from the right one. So the mixed columns include that effect, and the expert-alone columns are the clean test of reading.
- **Gain over `base`:** `scripts/exp66/score.py` also prints the right-minus-wrong difference in each arm's gain over `base`.
  - After the made-up stream: `role` Rm −0.18 ± 0.21, Rn +0.47 ± 0.23; oracle Rm −0.17, Rn +0.59 ± 0.25. Positive favours the right one.
  - In 5 formats (tzif, vf, lz4, elf, jmod) `role` said NoGo under both conditions, so it ties with `base` there.
  - The spread of the Rn figure comes from formats where `base` does badly, such as gzip, pyc, zip and xcursor (`base` 8.4–9.9).

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

- **Later files, whole:** 4.85–4.89 for `base` under every condition, and 4.68–4.89 for the expert arms.
- **Description bytes:**
  - Real alone: `base` and oracle 2.702, `role` 2.682, `desc` 2.515.
  - After the made-up stream: 2.671, 2.610 and 2.407.
  - The guessing expert also predicts the next description from the one before, since it treats every segment as one.
- **The made-up stream first doesn't help real files as a whole.** For example, whole first files with `role` cost 5.44–5.56 after it, against 5.45–5.57 alone. Its gain is on the note's magic.

## The role gate on real files

The table gives the Go rate at the start of each segment, by what came before it, over the four rotations.
- **Counts:** 28 first files and 140 later files per condition. The exception is the real stream's N row for first files: 27, because the stream's first file has nothing before it.
- **The last row** is the start of descriptions: 81 or 84 for first files and 420 for later ones.

| Before | Real alone: gate / teacher, first · later | After made-up: gate / teacher, first · later |
|---|---|---|
| Rm, its entry | 0.36 / 0.50 · 0.57 / 0.59 | 0.50 / 0.57 · 0.66 / 0.72 |
| Rn, note | 0.39 / 0.39 · 0.37 / 0.36 | 0.71 / 0.68 · 0.73 / 0.76 |
| W, wrong entry | 0.64 / 0.54 · 0.52 / 0.51 | 0.61 / 0.68 · 0.65 / 0.58 |
| N, a file | 0.22 / 0.11 · 0.09 / 0.14 | 0.18 / 0.21 · 0.31 / 0.34 |
| a file, before a description | 0.09 / 0.07 · 0.13 / 0.10 | 0.18 / 0.17 · 0.35 / 0.34 |

- **The teacher can't tell the right manual from a wrong one, because the expert can't read manuals.**
  - Its signal is the expert's own read-only cost on the next 16 bytes. The previous segment must beat both a decoy (the segment three back) and no description, by more than 2 bits.
  - A wrong manual entry often does: its tokens have been aligned with many files' bytes, so they know common bytes such as `00`.
  - The decoy is already another description for most W files (30–37 of the 42 in each rotation). Even then the teacher says Go after W nearly as often as after the right entry. Counting only those W files, it is 57% after the made-up stream and 45% alone, against 66% and 48% after Rm.
  - So the gate learns "manual" (W is a manual). The mixed code still pays for wrong entries: W is the worst of the three description conditions for `role`.
- **Gate and teacher agree on 78–80% of segments in the real part after the made-up stream**, and on 78–84% alone (exp 65: 93.6% on the made-up stream). The real manual entries are long, varied and full of tokens the gate has never seen.
- **Scripts:** `scripts/exp66/role_cond.py`. Agreement overall: `scripts/exp65/role_agree.py`, with the stream's offset for `prereal`.

## How it was run

- **Stream builder:** `scripts/exp66/build_real.py` (seed 66). It takes:
  - the list of files with their `file`(1) verdicts (`runs/exp66/ftypes_all.tsv`);
  - the magic database sources (file 5.45, `Magdir`);
  - a table of formats, giving for each its name, verdict pattern, magic file and the entry's line.
- **Format fixes:**
  - tfm's anchor is the `0012` test, which is what this machine's TFM files have.
  - "ispell" was dropped, because `file` called TFM files ispell.
- **The excerpts are the real entries, but not always clean ones.**
  - Some carry neighbouring comments. gzip's has four comment lines about other formats above its test line, and never names gzip.
  - tar's has no top-level test line, only a comment and `>257 string =ustar`.
  - The note's wording comes from the test line and its message: `format NAME. MESSAGE. magic HEX.`
- **Runs:** `runs/exp66/run.sh`, with 32 runs: two streams × four rotations × four arms.
  - These are experiment 63–65's shared flags, with `--file-starts` and `ROLE_DUMP=1`.
  - Runs took 2–3.5 minutes on the real stream alone, and 20–22 minutes after the made-up stream.
- **Scorer:** `scripts/exp66/score.py ITEMS_PATTERN NAME=BITS_PATTERN ...`, which pools the four rotations. The first named run is the reference for the paired gains.

## What this means

- **The expert can carry what it learned about a notation from invented formats to real ones.**
  - After the made-up stream, a short note in hex lowers the expert's own cost on a real format's magic by 1.6 bits per byte on average (0.4–4.3), against nothing before. That is in the `role` arm, and it is lower on all 28 formats.
  - Against a wrong entry the reduction is the same, 1.6. The magic the stack pays falls by 1.2 bits per byte against a wrong entry, cheaper in 23 of 28 formats.
  - This happens the first time the expert meets the format.
  - **It works only in the notation it practised.** Without the made-up stream first, the note does nothing specific.
  - It works best where the note looks most like that practice, with the hex at the third token.
- **The expert doesn't read real manuals.** The way they are written is the barrier, not the role: the oracle knows the role and still gains nothing specific from them.
  - A whitespace token can't be decoded into bytes. `\x89PNG` is one token, used by one format, so no other format can teach what it means.
  - **The stack's copy act already gets some of the plain-ASCII magic from the right entry** (gvariant, ar, gif). Escapes like `\x89`, `\037` and `\177` are what nothing reads.
- **The role gate's teacher can't credit the right description over a wrong one until the expert can read it.** The teacher's signal is the expert's own cost. On real manuals that cost doesn't separate right from wrong, and a wrong entry often beats nothing because manual tokens learn bytes that are common everywhere.
- **Caveats:**
  - **Sample size:** each run was done once. Each comparison is paired by file over four rotations, but 28 formats is few.
  - **The generic part of the gain:** it hedges a stack that is worse than uniform on a new format's first bytes. Part of every arm's gain is that.
  - **Coverage:** only the first 1,024 bytes of each file are used.
  - **Source:** the files come from one Linux machine, so some formats may be narrow there. For example, all the ttf files may be from one font family.

## Next

1. **A reader of spellings.** Let the expert align byte-level pieces of a token as well as whole tokens: libmagic's escapes (`\x89`, `\037`, `\177`) as the bytes they name. Letters as their own bytes would add to what the copy act already gets.
   - Then test the verbatim entries again, alone and after the made-up stream.
   - Their magic token sits at varying positions behind comments, so the expert's alignment prior for where the description starts matters too.
2. **Then a role teacher that credits the right description only.** For example, contrast against another description rather than nothing. That only helps once item 1 lets the expert's cost tell a right manual from a wrong one.
3. **A targeted check on the expert in later files.**
   - Where the stack already predicts the magic, the expert costs up to 1.0 bit per byte without the gate (up to 0.25 with it).
   - After a wrong entry or nothing, the expert helps later files, by up to 1.1. So simply keeping it out of later files won't do.
