# 61 · A position sense, and pretraining on primitive encodings (2026-10-28)

*[All experiments](README.md)* · [← 60](60-descriptions-before-files.md)

**The owner's plan:**
1. Give the byte brain a position sense.
2. Pretrain it on the primitive encodings that real files are built from (integers, floats, varints, UTF-8, strings, raw images, and numbers shown side by side in hex and in bytes).
3. See whether that transfers to real files.

**Results:**
- **The position sense is used where position matters, at a cost elsewhere.**
  - On experiment 60's made-up formats (5,974 file starts), the brain keeps position edges: (byte × pos) and (pos × (byte@3 × byte)).
  - Without a description, fields at fixed offsets get cheaper: count 5.64 → 5.20 bits per byte, version 6.15 → 5.94, magic 9.53 → 9.38.
  - Kind bytes, values, names and constants get worse (constants +0.14). Whole files: 3.62 → 3.65.
  - On experiment 57's 16 files it keeps no position node (default pruning) and nothing changes.
- **1 MB of primitives before experiment 57's files doesn't help them; 1 MB of plain English does.** All 16 test files, bits per byte:
  - nothing before: 3.08;
  - primitives before: 3.09;
  - English from other books before: 2.96.
- **What English does:**
  - The first test book gains most (round A text 3.67 → 2.09), and the second a little (3.41 → 3.31).
  - The English contains no passage of either test book.
  - A first English control did contain passages of both, and its number was better (2.92).
- **Primitives help and cost by file, with no clear pattern:**
  - **Help:** the first book (3.67 → 3.09 in this run, but 3.67 and 3.30 in the other two runs) and the second ELF file (3.05 → 3.02).
  - **Cost:** Rust (up to 0.12), JSON B (0.13), the second book (0.12), xml A (0.10), ELF A (0.10).
- **A first run with primitives was much worse (3.34)** and stalled:
  - its kept edges fell from 9 to 1 (not counting the byte's own edge);
  - it started no new trials after about 625,000 bytes;
  - it recorded no events at all between 785,000 and 1,234,000 bytes.
  - That build let garbage collection remove the position nodes. With them kept (the only change), the same run scored 3.09. Why the first run stalled is not known.
- **Pruning only for harm doesn't make primitives help.** `--prune-harmful`: prune an edge only when its mean gain is below zero and z below −1.
  - The brain keeps more edges (about 14 instead of 6–8).
  - Nothing before and English before both improve a little (3.06, and 2.89 with the first English control).
  - Primitives get worse (3.14).
- **A collection bug, found here and fixed:** a node young enough to survive collection could lose an older part. It then read that part's frozen state. Fixed from `sit-61c` (old rule: `--gc-old-parts`). The runs on this page used `sit-61a` and `sit-61b`.

## The position sense (`--file-starts FILE`)

- **What it is given:** where each file starts (a reader of files knows when one is opened).
- **Five nodes**, added as candidates like any derived node:
  - the offset from the file's start (0–31 exactly, then one state per power of two);
  - the offset modulo 2, 4, 8 and 16.
- **What the brain does with them:** each state gets a random code on first use. The brain can keep them as edges or bind them with other nodes, as with any source.
- **Never collected:** they are senses, so garbage collection never removes them (`sit-61b`; `sit-61a` let it).

Formats stream (experiment 60's `fmt2`; brain alone, no description expert; both runs `sit-61b`). File bytes read without a description:

| Field | Without | With the position sense |
|---|---|---|
| magic | 9.528 | 9.379 |
| version | 6.147 | 5.944 |
| count | 5.644 | 5.200 |
| kind | 6.092 | 6.189 |
| value | 5.581 | 5.618 |
| name | 1.612 | 1.646 |
| constant | 7.237 | 7.374 |
| all file bytes (N) | 3.623 | 3.650 |

- Kept at the end with it: (byte × pos), byte@2, chunk(z=0.5), byte@3, (pos × (byte@3 × byte)), (byte@2 × (byte@3 × byte)), byte@6, (byte@6 × (byte@2 × (byte@3 × byte))).
- An earlier run on `sit-61a` gave the same field numbers within 0.03. In it a kept bind read a "position mod 8" node that had already been collected (the collection bug above).
- **Experiment 57's 16 files:**
  - with `sit-61a`, 3.077 against 3.076 without, but the position nodes were collected early;
  - with `sit-61b`, default pruning keeps no position node (3.079);
  - pruning only for harm keeps (pos%2 × byte) (3.061).

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

- **English control (C2):** 8,000-byte slices of Gutenberg books other than the two test books, cut to the primitives' length (1,021,800 bytes). None of the test books' 748 64-byte chunks appears in it.
- **First English control (C):** slices of any book. It held about a third of the second test book verbatim, and three passages of the first test book's novel.
- **The test:** experiment 57's 384,000 bytes.
- **Settings:** floor, guard, two maps, copy act, position sense. Scripts `runs/exp61/run.sh`, `runs/exp61/all.sh`, `runs/exp61/clean.sh`; scorer `scripts/exp61/score.py`.

## Results (test stream, bits per byte)

| | Nothing before | English before | Primitives before |
|---|---|---|---|
| First run (`sit-61a`, position nodes collectable; first English control) | 3.077 | 2.930 | 3.336 |
| `sit-61b`, first English control | 3.079 | 2.921 | 3.088 |
| `sit-61b`, English without the test books | 3.079 | **2.960** | 3.088 |
| `sit-61b`, prune only for harm (first English control) | 3.061 | 2.893 | 3.139 |

Per file, `sit-61b`, default pruning:

| | Type | Nothing | English (no test books) | Primitives |
|---|---|---|---|---|
| A | text | 3.666 | 2.087 | 3.091 |
| A | json | 0.570 | 0.451 | 0.543 |
| A | csv | 2.774 | 2.806 | 2.812 |
| A | rust | 2.411 | 2.353 | 2.534 |
| A | xml | 0.952 | 0.944 | 1.054 |
| A | elf | 2.833 | 2.905 | 2.933 |
| A | bmp | 1.247 | 1.249 | 1.262 |
| B | text | 3.413 | 3.307 | 3.530 |
| B | json | 1.496 | 1.475 | 1.629 |
| B | csv | 3.233 | 3.129 | 3.245 |
| B | rust | 2.751 | 2.711 | 2.872 |
| B | xml | 1.725 | 1.710 | 1.732 |
| B | elf | 3.054 | 3.087 | 3.022 |
| B | bmp | 3.147 | 3.148 | 3.144 |

The gzip files are 8.00 in every run. Round A / round B without gzip: nothing 2.065 / 2.688; English 1.828 / 2.652; primitives 2.033 / 2.739.

**Kept edges** (progress reports every 200,000 bytes; each count includes the byte's own edge):
- primitives, first run: 10, 7, 5, 3, 3, 3, 2;
- primitives, `sit-61b`: 8, 7, 6, 6, 6, 5, 8;
- English (first control), `sit-61b`: 7, 8, 6, 7, 7, 6, 5;
- primitives, prune only for harm: 13, 12, 11, 13, 11, 14, 14.

During pretraining (`sit-61b`) the brain pruned 60 edges on primitives and 21 on the first English control.

**During pretraining** (`sit-61b`), each kind's mean over its 16 files:
- float 7.64 bits per byte (next to nothing learned);
- varint 4.90;
- int 2.60;
- strtab 2.38;
- rosetta 2.34 (its first file 4.34, its last 1.19).

## What this means

- **Primitives, as given here, don't transfer, and English does.** The English gain is on the books (other English helps read English), with small gains and costs on the other types. Where primitives help or cost, the pattern changes from run to run.
- **Two explanations fit, neither is tested:**
  - **The brain can't represent what primitives teach.** It predicts the next byte from the identities of earlier bytes. What primitives teach is about values: a counter's next value is this one plus a step; a float's exponent changes slowly while its low bytes are noise; "04d2" and the bytes d2 04 are the same number. That needs operations the brain lacks: differences, carries, one value in two places. It learns each primitive's byte statistics instead.
  - **One global economy of kept edges can't hold many kinds of material.** Edges are judged over fixed windows of the whole stream, so an edge that helps one kind can be judged on another.
    - For: the brain pruned three times as many edges on primitives as on English.
    - Against: pruning only for harm kept more edges but made primitives worse, not better.
- **The position sense is worth keeping.** It improves fixed-offset fields by 0.15–0.44 bits per byte when there are many files, costs 0.03 on whole files there, and costs about nothing where it isn't kept.

## Next

1. **Value operators** for the byte brain: a node for the difference between the byte now and the byte k back (counters, strides), and one for the same value k back (repeats inside records). Then rerun the primitive pretraining. If it now transfers, the first explanation was right.
2. **Recall by kind of material:** a code for the kind of the recent bytes (an "aperture": learned clusters of byte statistics over a window), with edges judged and weighted per kind. This tests the second explanation.
3. Rerun the main runs on `sit-61c` (the collection fix).
