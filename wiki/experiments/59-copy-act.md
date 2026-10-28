# 59 · A copy act beside the byte brain, and format docs first (2026-10-28)

*[All experiments](README.md)* · [← 58](58-floor-guard-two-maps.md)

**The questions:**
1. xz still beat the byte brain most on the ELF executables (experiment 58: 2.98 / 3.52 against 2.43 / 2.49 bits per byte), because it finds long repeats and copies them. Does a seek-and-copy act close the gap?
2. The owner's question: would reading a format's documentation before reading the files help?

**Results:**
- **The copy act helps wherever there are repeats.** Whole stream: 3.22 → 3.08 bits per byte (order-4 counts 3.66, xz 2.89). About 0.01 of that is not copying: the gzip files gain 0.09 each (8.09 → 8.00) from the mixture's extra uniform expert, with almost no offers there.
  - JSON: round A 1.03 → 0.53 (xz 0.41); round B 1.92 → 1.48 (xz 1.27).
  - ELF: round A 2.98 → 2.82; round B 3.52 → 3.05. Still behind xz (2.43 / 2.49).
  - xml, Rust and the bitmaps gain 0.04–0.10, prose in the stream 0.03–0.04; csv barely moves (0.003 / 0.023).
  - Round A without gzip 2.19 → 2.05; round B without gzip 2.86 → 2.69.
  - Plain text (three Austen novels): no gain and almost no cost, second half 1.8632 against 1.8618. Half of its offers there are right (49.9%), and prose repeats are short.
- **Two details matter:**
  - **The act must be mixed with the brain after its floor, not with the raw brain and a uniform guess side by side.** Mixed raw, the first ELF file got worse (2.98 → 3.11) and the second gained less (3.52 → 3.36, against 3.12 mixed after the floor). A floored brain is a better single expert than a switch between "trust the brain" and "know nothing".
  - **Reading on past a wrong byte helps binaries.** In machine code a repeat often differs in one byte (an address, a constant). Keeping the pointer through up to 8 wrong bytes takes ELF from 2.93 to 2.85 (round A) and 3.12 to 3.08 (round B). Trusting the act by its recent record as well as by match length then takes them to 2.82 and 3.05.
- **Format docs first: no help, as expected for this reader.** 24 KB of the ELF manual page placed just before the first ELF file:
  - ELF file 2.821 bits per byte after the manual;
  - 2.846 after an unrelated manual page of the same length (socket(7));
  - 2.821 with nothing inserted.
  - The manual is better than another manual by 0.025, but against nothing inserted it is worth nothing. With one run of each, 0.025 is within how much an insertion moves other files (round B's Rust, csv and text move by 0.04–0.16, both ways). By 4 KB, the manual made the first four chunks of the ELF file slightly worse, the header region it describes included; only the fifth improved.

## The act (`--copy-act`, binary `sit-59f`)

**Seeking:**
- At each byte, a hash of the last 5 bytes points to the last place that context occurred.
- When the act has no pointer (or its match is shorter than 16 bytes), it looks the context up and checks the match backwards, up to 32 bytes. A match of 5 or more becomes its pointer, if longer than the current one.

**Offering and reading on:**
- It offers the byte that followed there.
- If right, the pointer moves on and the match grows.
- If wrong, the pointer still moves on with its length reset; it survives 8 wrong bytes in a row and is dropped at the 9th.

**Trust, counted:**
- How often offers were right, per context: match length (in pairs of bytes, up to 32) × the act's recent record (right in 0–1, 2–4, 5–6 or 7–8 of its last 8 offers).
- Counts are prequential and halved past 4,096.
- As an expert, the act gives its offered byte the probability c and every other byte (1 − c) / 255.

**The code:**
- A switching mixture (fixed share 1/1000) over three experts: the brain after its floor (experiment 58), the act, and a uniform guess.
- There is one mixture per context, the same contexts as the trust counts.
- The brain's learning is unchanged: it never sees the act.

**Over the stream:**
- 310,015 offers (81% of bytes), 62.5% right; 39,493 seeks that found a match.
- The first version, which dropped its pointer at the first wrong byte, made 213,933 offers, 81.2% right.

Stack (all runs with the act; "Before" is the same without `--copy-act`): `stack --levels 1 --words --run-window 200000 --conjoin-words --long-trials 5 --grow-pools --pool-budget 524288 --readout3 --screen-mix --fresh-extensions --floor --prune-guard --map-two 127 --copy-act`.

## Bits per byte, per file

Columns:
- **Before:** experiment 58's floor, guard and two maps.
- **First act:** the first act version, mixed with the raw brain.
- **Mixed after floor:** the same act, mixed with the floored brain.
- **Reads on:** the act also keeps its pointer past wrong bytes.
- **+ record (final):** reads on, and trust by recent record.

| | Type | WB4 | xz | Before | First act | Mixed after floor | Reads on | + record (final) |
|---|---|---|---|---|---|---|---|---|
| A | text | 2.813 | 3.157 | 3.669 | 3.642 | 3.633 | 3.633 | 3.631 |
| A | json | 0.709 | 0.408 | 1.033 | 0.536 | 0.528 | 0.525 | 0.528 |
| A | csv | 3.172 | 2.900 | 2.815 | 2.810 | 2.812 | 2.812 | 2.812 |
| A | rust | 2.695 | 2.337 | 2.464 | 2.383 | 2.382 | 2.381 | 2.383 |
| A | xml | 1.250 | 0.913 | 1.046 | 0.950 | 0.951 | 0.950 | 0.950 |
| A | elf | 4.080 | 2.431 | 2.976 | 3.113 | 2.930 | 2.847 | 2.821 |
| A | bmp | 1.517 | 1.049 | 1.344 | 1.301 | 1.278 | 1.253 | 1.245 |
| A | gzip | 11.304 | 8.123 | 8.090 | 8.001 | 8.001 | 8.001 | 8.001 |
| B | text | 3.031 | 3.168 | 3.333 | 3.311 | 3.302 | 3.300 | 3.303 |
| B | json | 1.754 | 1.273 | 1.923 | 1.489 | 1.478 | 1.479 | 1.482 |
| B | csv | 3.559 | 3.064 | 3.305 | 3.283 | 3.283 | 3.283 | 3.282 |
| B | rust | 2.719 | 2.712 | 2.921 | 2.870 | 2.866 | 2.866 | 2.870 |
| B | xml | 2.355 | 1.655 | 1.793 | 1.716 | 1.718 | 1.717 | 1.717 |
| B | elf | 3.539 | 2.487 | 3.524 | 3.357 | 3.117 | 3.076 | 3.053 |
| B | bmp | 3.447 | 2.567 | 3.190 | 3.200 | 3.166 | 3.148 | 3.147 |
| B | gzip | 10.578 | 8.065 | 8.090 | 7.998 | 7.998 | 7.998 | 7.998 |
| All | | 3.658 | 2.894 | 3.220 | 3.123 | 3.090 | 3.079 | 3.076 |
| A without gzip | | 2.319 | 1.885 | 2.193 | 2.105 | 2.073 | 2.057 | 2.053 |
| B without gzip | | 2.915 | 2.418 | 2.856 | 2.747 | 2.704 | 2.696 | 2.693 |

- **Trust counts halved faster** (past 255 or 63 instead of 4,096; on the first act, mixed raw) changed the whole stream by under 0.003, though the first ELF file by 0.02–0.03 (3.11 → 3.09 / 3.08).
- **Recent record alone** (on the first act, mixed raw) changed nothing (3.124 against 3.123; a control run of the same binary without it reproduced the first act exactly). It pays once the act reads on past wrong bytes; that it is because its record then varies is a guess, not tested.
- **An offline copy of the act** (`scripts/exp57/copy_sim.py`, mixing the first act with experiment 58's floored code) found the cause of the raw-mixing loss: the same act mixed after the floor gave ELF 2.93 / 3.10 instead of 3.11 / 3.36. It is an approximate copy: an exact 5-byte dictionary (no hash collisions) and counts never halved; it offers on 58.2% of bytes, 79.0% right (the brain's first act: 55.7%, 81.2%).

## Plain text (three Austen novels, 2,026,385 bytes)

| Run | Whole | Second half |
|---|---|---|
| Experiment 58 before its changes | 1.956 | 1.877 |
| Floor, guard, two maps (experiment 58) | 1.945 | 1.862 |
| + copy act | 1.946 | 1.863 |

The act made 2,023,721 offers there (99.9% of bytes), 49.9% right, with 705,815 seeks that found a match.

## Format docs first

Script `scripts/exp57/insert_doc.py`. It places 24,000 bytes of a manual page (from its first `.SH DESCRIPTION` on) just before round A's ELF file. The manual pages come from the Linux man-pages repository (`man5/elf.5` and the control `man7/socket.7`), stored in `runs/exp57/docs/`.

| Inserted before ELF A | ELF A | ELF A by 4 KB | ELF B | The inserted page |
|---|---|---|---|---|
| nothing | 2.821 | 1.58 2.41 1.27 1.95 5.44 4.27 | 3.053 | — |
| elf(5), the ELF manual | 2.821 | 1.62 2.42 1.33 1.99 5.32 4.25 | 3.058 | 2.160 |
| socket(7), unrelated | 2.846 | 1.64 2.41 1.33 2.05 5.37 4.28 | 3.056 | 2.630 |

- **What the files share is too little.** The first 24 KB of `/usr/bin/xz` contains none of the section names (".text", ".data", ".dynsym", ".shstrtab" sit in the section-name table near the end of the file, around byte 86,700 of 89,008). It contains "ELF" once and "GLIBC" 11 times. The inserted part of the manual (its bytes 1,986–25,986 of 51,032) also stops just before the manual's list of section names, so of those names the reader only saw ".bss".
- **What the reader takes from text is too little.** The byte brain reads the manual as more text to predict. Nothing in it can turn "e_phoff holds the program header table's file offset" into an expectation about bytes 32–39.

## What this means

- **Copying is the reader's long memory for bytes,** and it is an act in the project's sense: seek (a pointer into the past), read on, credited by whether its byte is right, trusted by its record. It belongs beside the brain as one expert among several, chosen by a mixture.
- **The ELF gap to xz is now 0.4–0.6 bits per byte.** xz still has more candidates (several match distances, repeat offsets) and codes the next literal in the context of the byte the match would have given. Those are the obvious next pieces for the act.
- **Documentation helps a reader that can ground words in data, and this one can't yet.** The word reader binds words to roles and answers questions about them. A doc would pay once it can connect "the first four bytes are 0x7f E L F" (a description) to the bytes it reads (an observation): the same binding of names to places that it already does for stories.
  - A first test of that would be a stream where a short description precedes each file of a made-up format ("field width is 4; field 2 is the length"), and see whether the reader codes the file better after the description than without it.
  - Real format manuals are a later stage.

## Next

1. **A better act:** two pointers (the last and the second-to-last occurrence), and the next byte coded in the context of the act's offer even when it is wrong.
2. **Descriptions before data:** a made-up format with short descriptions, as above.
3. **Recall by context** (from experiment 58): recognise the kind of material and bring back what was learned on it.
