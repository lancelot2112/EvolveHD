# 61 · A position sense, and pretraining on primitive encodings (2026-10-28)

*[All experiments](README.md)* · [← 60](60-descriptions-before-files.md)

**The owner's plan:**
1. Give the byte brain a position sense.
2. Pretrain it on the primitive encodings that real files are built from (integers, floats, varints, UTF-8, strings, raw images, and the same numbers shown side by side in hex and in bytes).
3. See whether that transfers to real files.

**Results:**
- **The position sense is used where position matters, at a cost elsewhere.**
  - On experiment 60's made-up formats (about 6,000 file starts), the brain keeps position nodes in its predictions, e.g. (byte × position), (position mod 8 × the last bytes).
  - Without a description, fields at fixed offsets get cheaper: count 5.64 → 5.23 bits per byte, version 6.15 → 5.95, magic 9.53 → 9.39.
  - Names, values and constants get slightly worse. Whole files: 3.62 → 3.65.
  - On experiment 57's 16 files it keeps no position node and nothing changes (3.077 against 3.076).
- **Pretraining on 1 MB of primitives does not help the real files; 1 MB of plain English does.** The test is experiment 57's 16 files, read after the pretraining. Bits per byte over all 16 files:
  - nothing before: 3.08;
  - primitives before: 3.09;
  - English before: 2.92 (books: round A 3.67 → 2.09; the other types about level).
- **The first run was worse, and showed why the brain can't hold many kinds of material at once.** Primitives first: 3.34.
  - While reading the primitives its kept structure churned and shrank from 10 kept edges to 2.
  - **Pruning:** each kept edge is judged on its gain over 10,000 bytes. With a new kind of primitive every 8 KB, an edge that helps integers is judged on floats or UTF-8 and pruned.
  - **Garbage collection:** it removed the position nodes while they were unused. That was a bug: senses should not be collectable. Fixed for the runs above.
- **Pruning only for harm doesn't fix it.** `--prune-harmful`: prune an edge only when it does clear harm (mean gain below zero, z below −1), not merely too little.
  - The brain keeps more (about 14 edges instead of 6).
  - Primitives still don't help: 3.14, against 3.06 with nothing before and 2.89 after English.

## The position sense (`--file-starts FILE`, binary `sit-61b`)

- **What it is given:** where each file starts (a reader of files knows when one is opened).
- **Five nodes**, added as candidates like any derived node:
  - the offset from the file's start (0–31 exactly, then one state per power of two);
  - the offset modulo 2, 4, 8 and 16.
- **What the brain does with them:** each state gets a random code on first use. The brain can keep them as edges or bind them with other nodes, as with any source.
- **Never collected:** they are senses, so garbage collection never removes them (from `sit-61b`; `sit-61a` let it).

Formats stream (experiment 60's `fmt2`; brain alone, no description expert), file bytes read without a description:

| Field | Without | With the position sense |
|---|---|---|
| magic | 9.528 | 9.386 |
| version | 6.147 | 5.951 |
| count | 5.644 | 5.225 |
| kind | 6.092 | 6.187 |
| value | 5.581 | 5.615 |
| name | 1.612 | 1.646 |
| constant | 7.237 | 7.375 |
| all file bytes (N) | 3.623 | 3.650 |

Kept at the end with it: (byte × pos), (pos × (byte@3 × byte)), and (byte@6 × (byte@2 × (byte@3 × (pos%8 × byte)))), among others.

## The primitives (`scripts/exp61/primitives.py`)

128 files of up to 8,000 bytes (1,021,800 bytes in all; some image files are shorter), 16 of each kind, shuffled:

| Kind | What a file holds |
|---|---|
| int | an array of 1-, 2-, 4- or 8-byte integers, little or big endian, signed or not: a counter, a random walk, small values, or sorted values |
| float | an array of 4- or 8-byte IEEE floats: sums of sines, a ramp, or noise |
| varint | LEB128 integers, unsigned or zigzag signed |
| utf8 | made-up languages in Greek, Cyrillic, Armenian, Hebrew, Devanagari, CJK or Hangul letters |
| ascii | a slice of a Gutenberg book |
| strtab | function names, zero-terminated or with 1- or 2-byte lengths |
| image | raw RGB or grey pixel rows (widths 16–256) from ImageMagick's built-in images and gradients |
| rosetta | numbers side by side: "0x04d2 " then the same number as 2- and 4-byte little endian, then a newline |

- **The English control:** 8,000-byte Gutenberg slices, cut to the primitives' length (1,021,800 bytes).
- **The test:** experiment 57's 384,000 bytes.
- **Settings:** floor, guard, two maps, copy act, position sense. Scripts `runs/exp61/run.sh`, `runs/exp61/all.sh`; scorer `scripts/exp61/score.py`.

## Results (test stream, bits per byte)

| | Nothing before | English before | Primitives before |
|---|---|---|---|
| First run (`sit-61a`, position nodes collectable) | 3.077 | 2.930 | 3.336 |
| Position nodes kept (`sit-61b`) | 3.079 | 2.921 | 3.088 |
| + prune only for harm | 3.061 | 2.893 | 3.139 |

Per file, `sit-61b`, default pruning (nothing / English / primitives):

| | Type | Nothing | English | Primitives |
|---|---|---|---|---|
| A | text | 3.666 | 2.091 | 3.091 |
| A | json | 0.570 | 0.471 | 0.543 |
| A | csv | 2.774 | 2.725 | 2.812 |
| A | rust | 2.411 | 2.253 | 2.534 |
| A | xml | 0.952 | 0.954 | 1.054 |
| A | elf | 2.833 | 2.882 | 2.933 |
| A | bmp | 1.247 | 1.249 | 1.262 |
| B | text | 3.413 | 2.452 | 3.530 |
| B | json | 1.496 | 1.591 | 1.629 |
| B | csv | 3.233 | 3.273 | 3.245 |
| B | rust | 2.751 | 2.850 | 2.872 |
| B | xml | 1.725 | 1.706 | 1.732 |
| B | elf | 3.054 | 3.082 | 3.022 |
| B | bmp | 3.147 | 3.153 | 3.144 |

(gzip files: 8.00 in every run.)

- **Primitives help where the same bytes appear:** the books in round A (3.67 → 3.09, from the ascii primitive files) and the second ELF file a little (3.05 → 3.02).
- **They cost Rust and xml** (up to 0.10 and 0.12), and in the first run nearly everything.

**Kept edges during the runs** (progress reports every 200,000 bytes):
- primitives, first run: 10, 7, 5, 3, 3, 3, 2;
- primitives, `sit-61b`: 8, 7, 6, 6, 6, 5, 8;
- English, `sit-61b`: 7, 8, 6, 7, 7, 6, 5;
- primitives, prune only for harm: 13, 12, 11, 13, 11, 14, 14.

**During pretraining** (`sit-61b`), each kind's mean over its 16 files:
- float 7.64 bits per byte (next to nothing learned);
- varint 4.90;
- int 2.60;
- strtab 2.38;
- rosetta 2.34 (its first file 4.34, its last 1.19).

## What this means

- **A curriculum can only teach what the reader can represent.** The byte brain predicts the next byte from the identities of earlier bytes. What the primitives have to teach is about values:
  - a counter's next value is this one plus a step;
  - a float's exponent byte changes slowly while its low bytes are noise;
  - "04d2" in hex and the bytes d2 04 are the same number.
  These need operations the brain lacks: differences, carries, the same value in two places. Shown many primitives, it learns their byte statistics, which don't carry over to real files. Shown English, it learns English, which does carry over to the books.
- **The brain has one global economy of kept edges,** judged over fixed windows of the whole stream. Mixed material breaks it: what helps one kind is judged on another. The guard (experiment 58) fixed this for noise; for many kinds of material, edges need to be judged, and used, per kind of material ("recall by context").
- **The position sense is a sense worth keeping.** It costs a little where it's kept but not needed (whole files 3.62 → 3.65), and it is what lets fixed-offset fields be predicted at all without a description.

## Next

1. **Recall by kind of material:**
   - a code for the kind of the recent bytes (learned clusters of byte statistics over a window: the "aperture");
   - edges judged per kind, pruned only if useless in every kind, and weighted per kind in the prediction.
   Then rerun this pretraining test.
2. **Value operators for the byte brain:** a node for the difference between the byte now and the byte k back (for counters and strides), and one for the same byte value k back (for repeats inside records). These would give primitive pretraining something to teach.
