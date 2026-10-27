# 57 · A first trial on raw bytes from real files (2026-10-27)

*[All experiments](README.md)* · [← 56](56-instant-kinds.md)

**The owner's question:** can the system read raw files, bytes and all, and learn the consistencies in them, rather than having everything interpreted for it? This trial is the first step: uncompressed files of several types, read from nothing, scored in bits per byte.

**The stream:** 16 real files, two of each type, each cut to its first 24,000 bytes and laid end to end with nothing between them (384,000 bytes). Round A has one file of each type, then round B the second of each type, in the same order:

| Type | Round A | Round B |
|---|---|---|
| text | *Persuasion* (from byte 2,000) | *The Man Who Was Thursday* (from byte 2,000) |
| json | Playwright device descriptors | Playwright API description |
| csv | sklearn breast cancer data | matplotlib stock prices |
| rust | `src/brain.rs` | `src/situation.rs` |
| xml | ImageMagick `mime.xml` | an OpenCV face cascade |
| elf | `/usr/bin/xz` | `/usr/bin/gzip` |
| bmp | ImageMagick logo, 200 × 120, 24-bit | ImageMagick wizard, 150 × 120, 24-bit |
| gzip (control) | a gcc changelog | an LLVM changelog |

The gzip files are a control: compressed data has nothing to learn, so 8 bits per byte is the best possible.

**Results:**
- **The byte brain (experiment 20's level 0) learns structured text formats quickly.** In round A it beats order-4 counts on csv, Rust source, xml and the ELF binary, and comes close to xz on Rust (2.44 against 2.34). On JSON it falls to about 0.3–0.4 bits per byte within 20 KB, but over the whole file it is behind order-4 counts (1.04 against 0.71) and xz (0.41).
- **It breaks on compressed data, and forgets.**
  - **No floor:** on the gzip files it codes at 18–19 bits per byte, far above the 8 bits that a uniform guess costs. Its predictions are confidently wrong and nothing caps them.
  - **It loses its structure:** during the gzip file its kept edges fall from 9 to 1 and its rows from about 58,000 to 193. Round B starts over: text 5.2 bits per byte, against 3.7 in round A, and the ELF binary and the bitmap 5.7.
- **The word reader is not a byte predictor:** 4.49 bits per byte overall, against 3.66 for order-4 counts.
  - It learns some byte groups by itself: the digits with "."; a large group of control bytes, high bytes and rare letters (the "binary" bytes); and space and newline as single bytes of their own.
  - Its learned boundaries are not meaningful: they include ")", ",", "=", ">", tab and 0x00, but not space or newline.
  - Its act gate almost never opens (0.1% of bytes). There are no questions in this stream.
- **Together the coders are complementary.** A switching mixture over the byte brain, the word reader, order-4 counts and uniform (fixed share, prequential) codes the stream at 3.08 bits per byte. That is better than any one of them (order-4 counts 3.66) but still behind xz in the stream (2.89).
  - The mixture beats xz on text (2.78 against 3.16 in round A), csv and the gzip control (8.00 against 8.12).
  - It loses on JSON, xml and above all the ELF binaries (3.31 against 2.43). xz finds long repeats far back in the stream and copies them; none of our coders can.

## Bits per byte, per file

Each column codes each byte after everything before it in the stream.
- **WB:** Witten-Bell counts of order 2 and 4.
- **xz:** the stream up to the end of the file minus the stream up to its start, compressed with `xz -9e`.
- **Word reader:** its mixture with its own count model.
- **Brain:** `stack --levels 1` with experiment 20's level-0 flags.
- **Switch:** the fixed-share mixture of brain, word reader, WB4 and uniform, alpha 1/1000.

| | Type | WB2 | WB4 | xz | Word reader | Brain | Switch |
|---|---|---|---|---|---|---|---|
| A | text | 2.993 | 2.813 | 3.157 | 3.568 | 3.696 | 2.782 |
| A | json | 1.577 | 0.709 | 0.408 | 3.408 | 1.037 | 0.634 |
| A | csv | 2.985 | 3.172 | 2.900 | 3.280 | 2.806 | 2.803 |
| A | rust | 3.194 | 2.695 | 2.337 | 3.970 | 2.438 | 2.372 |
| A | xml | 2.543 | 1.250 | 0.913 | 3.993 | 1.056 | 1.026 |
| A | elf | 3.996 | 4.080 | 2.431 | 3.852 | 3.643 | 3.313 |
| A | bmp | 1.593 | 1.517 | 1.049 | 1.813 | 2.232 | 1.202 |
| A | gzip | 11.301 | 11.304 | 8.123 | 10.892 | 18.010 | 8.001 |
| B | text | 3.454 | 3.031 | 3.168 | 4.217 | 5.214 | 3.004 |
| B | json | 3.084 | 1.754 | 1.273 | 4.255 | 3.750 | 1.721 |
| B | csv | 3.572 | 3.559 | 3.064 | 3.966 | 3.860 | 3.364 |
| B | rust | 3.308 | 2.719 | 2.712 | 4.039 | 3.459 | 2.696 |
| B | xml | 2.611 | 2.355 | 1.655 | 3.495 | 2.261 | 2.023 |
| B | elf | 3.615 | 3.539 | 2.487 | 3.802 | 5.747 | 3.322 |
| B | bmp | 3.425 | 3.447 | 2.567 | 3.373 | 5.698 | 3.019 |
| B | gzip | 10.572 | 10.578 | 8.065 | 9.954 | 19.316 | 7.999 |
| All | | 3.989 | 3.658 | 2.894 | 4.492 | 5.264 | 3.080 |

- Plain Witten-Bell counts also code the gzip files above 8 bits (10.6–11.3): their escape estimates are tuned on the earlier, predictable files. Only xz and the mixture's uniform coder stay near 8.
- **The byte brain over time** (bits per byte per 1,000 bytes):
  - JSON falls from 5.1 to about 0.3–0.4 within 20 blocks;
  - the gzip file runs at 17–18;
  - round B's text starts at 10.3 and only falls to 4.4–5.2.
- Its report at byte 200,000 (just after the first gzip file): 1 kept edge, 193 rows; at 150,000: 9 kept, 57,817 rows.
- The switching mixture puts most weight on the brain for csv, Rust, xml and the ELF binary in round A and for xml in round B; on WB4 for text, JSON and the bitmap in round A and the rest of round B; and on uniform for the gzip files.

## Runs

- Stream: `scripts/exp57/build_stream.py runs/exp57/bytes` (writes `.bin`, `.ids`, `.txt` with a name per byte, `.files.json`).
- Baselines: `scripts/exp57/baselines.py`.
- Both readers: `runs/exp57/run.sh reader brain`.
  - Word reader: binary `sit-56a`, the story-set flags with the default never-frozen stack and instant kinds, one seed.
  - Byte brain: binary `sit-57a` (adds `OUT/level0.bits`, the per-byte bits), `stack --levels 1 --words --run-window 200000 --conjoin-words --long-trials 5 --grow-pools --pool-budget 524288 --readout3 --screen-mix --fresh-extensions`.
- Scoring: `scripts/exp57/score.py`; mixture: `scripts/exp57/mix.py`.
- The word reader took 438 s; the byte brain 213 s.

## What this means

- **Reading raw files is not an impossible dream; the byte brain already learns text formats from nothing.** JSON, xml, csv and source code each become predictable within a few thousand bytes, at or near the level of general-purpose compressors.
- **Before it can read anything, it must survive what it can't read.** Two things are missing:
  - **A floor:** a coder that can say "I don't know" costs at most 8 bits per byte. The mixture's uniform coder shows what that is worth: the gzip files at 8.0 instead of 18–19.
  - **No forgetting on noise:** a stretch where nothing predicts should not evict the structure that predicted the earlier files. When the files come back in round B, the brain should recall them, not start again.
- **Binaries need a copy act.** xz's advantage on the ELF files (2.43 against 3.31) is long-range matching: find the last time this context occurred and read on from there. That is the "seek" act planned for the bytes reader: an act that points back into the stream, judged like any other route by whether its next byte is right.
- **The word reader is the wrong reader for raw bytes, but it found some kinds** (digits, binary bytes). Its question machinery has nothing to do here until a byte stream has something to answer, such as a key looked up in a JSON file.

## Next

1. **A floor and a guard in the byte brain:** mix a uniform coder into its readout, and stop eviction while every candidate is doing badly (the stretch is noise, not a better structure). Then rerun this stream: does round B recall round A?
2. **A copy act:** at each byte, a candidate that continues the last occurrence of the current context (any length that matches), credited by whether it is right. Test on the ELF files against xz.
3. **Questions on bytes:** a stream of JSON files followed by key look-ups, so the word reader's acts have something to do.
