# 60 · Descriptions before files: can a reader use a format's description to decode it? (2026-10-28)

*[All experiments](README.md)* · [← 59](59-copy-act.md)

**The owner's question:** what would let a reader connect a manual to its files and use what the manual says when decoding? Experiment 59 found that a real manual (ELF) read before an ELF file is worth nothing to the byte brain.

**The first stage:** made-up binary formats, each with a short description in plain controlled wording, and a three-way test.
- Each file is read after its own format's description (R), after a different format's description (W), or after nothing (N).
- A reader that really uses descriptions should code files better under R than N, and worse under W than R. The wrong description is the control: it is just as much text, of the same kind.
- The test that matters is files of formats never seen before. Later files of a format can also be helped by recall, without any reading.

**Results:**
- **The byte brain uses descriptions only as names.**
  - On later files of a format, the right description helps a lot. In the second half of the stream, magic bytes cost 5.3 bits per byte under R, against 8.5 under W and 9.5 under N.
  - On the first file of a format, it doesn't help at all: 11.1 under R against 9.7 under N.
  - The description works as a key: the copy act recalls what followed the same description last time. That is association, not reading.
- **A description expert learns the notation from nothing.**
  - It learns which token of the description each file byte comes from (an alignment, as in word alignment for translation) and what byte each token stands for.
  - By the end, for 238 of the 256 two-digit hex tokens, the byte it expects most is the token's own value ("4b" → 0x4b). Nothing told it that.
- **With it, descriptions help decode formats never seen before.** Over the second half of the stream, on the first file of each new format:
  - **Magic bytes:** the expert codes them at 5.0 bits per byte under R, against 9.0 under W and 8.0 under N. The first magic byte costs 3.3 bits under R and 8.9 under W.
  - **Mixed with the brain:** 6.3 under R, against 9.1 under W and 8.4 under N.
  - **Constant bytes stated in the description** ("byte 7f"): 6.9 under R, 7.8 under W, 8.0 under N (expert alone). Mixed with the brain the difference is gone (7.72 R, 7.67 W).
  - On the magic, the right description helps and the wrong one hurts, so the reader is using what the description says. On the other fields it is not yet clear: a wrong description doesn't hurt the constants, and the version digit is not read at all.
- **The gain is still small over whole files.** On the first files of new formats (whole stream), mixed:
  - 3.57 bits per byte under R, against 3.59 under W and 3.71 under N.
  - Most of R's gain over N also comes with a wrong description: the expert learns generic shapes, such as count and version bytes being small numbers.
  - The part that depends on the description being right is the stated literal bytes, a small part of each file.
  - The expert reads only literal bytes. It doesn't yet use widths, byte order or counts ("value 4 bytes", "little endian").

## The stream (`scripts/exp60/formats.py`)

**A format:**
- a magic of 2–4 random bytes;
- a version byte (0–9);
- a byte order (little or big);
- a count width (1 or 2 bytes);
- a record of 2–4 fields from: a kind byte (0–7), a value (2 or 4 bytes, under 1,000), a name (4–12 bytes, a word padded with spaces or zero bytes), a constant byte.

**A file:** magic, version, count n (3–12), then n records.

**A description,** for example:

> format 0. magic b7 6f 47. version 4. little endian. count 1 byte. record: byte 14, name 12 bytes padded with 00, byte 9b. end.

**The stream:**
- 1,200 formats, 3 files each, in 3,600 items of a description (or none) and a file.
- 593,485 bytes, of which 284,735 are file bytes.
- Each item's condition is drawn at random.
- Formats arrive through the stream: a format's first file comes at its turn, its other files within 60 formats after it. So new formats keep coming to the end.
- A first stream (300 formats, 8 files each, all introduced at the start) had no new formats late in the stream. It is kept for the first baseline and expert runs (`runs/exp60/fmt`). The example description above is that stream's format 0.

## The byte brain (baseline)

Binary `sit-60a` (experiment 59's code, rebuilt after the container moved to another CPU; on experiment 59's byte stream its output is identical to experiment 59's). Stack: floor, guard, two maps, copy act (`runs/exp60/run.sh`).

| Second half, magic bytes | R | W | N |
|---|---|---|---|
| First file of a new format | 11.05 | 11.09 | 9.73 |
| Later files | 5.26 | 8.52 | 9.47 |

The brain is worse than uniform (8 bits) on new magics under every condition, with no description too (9.73). Why was not tested; at a file's start the copy act may be offering bytes that followed the same context elsewhere.

## The description expert (`scripts/exp60/desc_expert.py`)

It is a prototype outside the brain, mixed with the brain's per-byte code.

**What it is given:**
- where each file starts (the position sense: a reader of files knows when a file is opened);
- the tokens of the bytes just before the file: runs of non-space bytes, with a trailing "." "," or ":" dropped.

**What it learns, online and prequentially (each byte coded before it is learned from):**
- **Start:** which token position the first byte comes from.
- **Moves:** how the alignment moves from one byte to the next, in jumps of −16 to +8 tokens.
- **Emissions:** for each token, a distribution over bytes.
- **Background:** a distribution for bytes that don't come from the description, with a share per file offset.

**How it uses them:**
- Before each byte it predicts from its belief over tokens and the background.
- After it, Bayes' rule updates the belief. The start and emission counts take the posterior as soft counts, weighted by the description's share in explaining the byte (online EM; emission updates under 0.001 are skipped). The jump counts are hard: one count for the jump between the likeliest positions, when the description explains the byte more than the background does.
- With no description it says nothing (uniform).

**The mixture:** a switching mixture (fixed share 1/1000) over the brain's code, the expert and a uniform guess, one per context: file offset (0–7, 8+) by the expert's confidence (quarters).

**Second half of the progressive stream, first files of new formats** (bits per byte; brain / expert alone / mixed):

| Field | R | W | N |
|---|---|---|---|
| magic | 11.05 / 5.04 / 6.30 | 11.09 / 9.02 / 9.11 | 9.73 / 8.00 / 8.43 |
| version | 7.06 / 4.79 / 6.42 | 6.89 / 4.59 / 6.26 | 7.08 / 8.00 / 7.34 |
| constant byte | 8.46 / 6.86 / 7.72 | 8.11 / 7.75 / 7.67 | 8.11 / 8.00 / 8.07 |

- **Magic, by byte** (expert, R / W; 168 files for bytes 0–1 under R, 67 for byte 3): byte 0 3.31 / 8.91; byte 1 5.42 / 8.94; byte 2 5.98 / 9.08; byte 3 6.79 / 9.48. The alignment finds the start but loses its place after it.
- **Its moves are poor:** it learns "stay" (jump 0) half the time. Bytes the background explains still move the belief, and the jump counts take the likeliest positions, not the full posterior.
- **The version digit is not read:** R and W are alike (4.79 / 4.59). The expert learns that version bytes are small, not which one the description states.

**Whole stream, file bytes** (brain / mixed):

| | R | W | N |
|---|---|---|---|
| All files | 3.438 / 3.300 | 3.531 / 3.426 | 3.623 / 3.608 |
| First file of a format | 3.815 / 3.574 | 3.753 / 3.591 | 3.730 / 3.711 |

- **Without the background distribution** (a first version, on the first stream), the tokens soaked up the bytes they don't explain ("00" came to stand for "e", "10" for "i"; printed by that version, which was then changed).
- **The background did not change the R / W pattern on that stream:** over all files, R −0.141 and W −0.115 against the brain without it, R −0.141 and W −0.116 with it.
- **It doesn't stop the soaking either.** On the progressive stream "20" still stands for "r", "10" for "l", "11" for 0x00. The background's share past offset 8 (most of a file's body: names, values) is only 0.05, so the tokens still explain most body bytes.
- **How the first token is found:** the start distribution puts 0.75 on token 3, the first magic token (a probe of the run, not printed by the script).

## What this means

- **A description can be read from nothing, as far as its literal parts go.** Notation is learnable by co-occurrence across many description-and-file pairs: "4b" stands for 0x4b, and the token after "magic" is where the file starts. This is cross-situational learning, the same way the word reader learns which word names which person.
- **The wrong-description control is what makes this a test.** Recall (the brain's gain on later files) and generic shapes (the expert's gain under W) both look like "the description helps" if only R and N are compared.
- **Most of what a manual says is not literal.** Widths, byte order, counts and record structure are statements about shape. Using them needs the pieces planned in this direction:
  - a position sense inside the reader;
  - facts read from the description (field, width, kind), using the word reader's binding;
  - a self-question at each byte ("what does the description say about here?") whose answer is an expert.

## Next

1. **Better alignment:** learn moves from the posterior over pairs, and let background bytes keep the alignment's place (no move without evidence).
2. **Shape statements:** "value 4 bytes, little endian" as an expert for the high bytes of small numbers, and "padded with 00" for a name's tail.
3. **Move the expert into the readers:** the position sense into the byte brain, the description into the word reader's fact store, the expert into the act path.
