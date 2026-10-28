# 58 · The byte brain: a floor, a guard, and two maps (2026-10-27)

*[All experiments](README.md)* · [← 57](57-byte-stream-trial.md)

**The question:** experiment 57's byte brain learned text formats from raw bytes but had no floor (gzip at 18–19 bits per byte) and lost what it had learned (round B's text opened worse than a fresh start). Can it survive what it can't read?

**Results:**
- **A learned floor ends the overconfidence.** Every gzip file now costs 8.09 bits per byte (was 18.0 and 19.3). The first bitmap falls from 2.23 to 1.55 and the first ELF file from 3.64 to 3.08. Whole stream: 5.26 → 3.62.
- **A guard against pruning on noise keeps more of the structure:** 5 kept edges through the gzip file instead of 1. Round B (without gzip): 3.73 → 3.42.
- **Most of the forgetting was in the map from cells to bytes.**
  - The brain matches each byte to a cell, and a count table per cell maps it back to the byte.
  - The uncapped counts carry the earlier files' bytes into every later file. The map cost was 0.08 bits per byte on round A's text and 1.34 on round B's (falling to about 0.74–0.92 by its last blocks, still about ten times round A's). The likely source is the gzip file, where every byte value was counted into the tables, but the bitmap and the ELF file before it already had high map costs (0.80 and 1.05) and no run isolates the gzip file.
  - A map whose counts halve past a cap follows the change. With a cap of 127, round B costs 2.86 (was 3.42), and round B's text 3.33, better than round A's in the same run (3.72). (They are different books, so this is not a measure of what was kept.)
  - A single capped map costs where nothing changes: plain text 1.945 bits per byte against 1.877.
- **Two maps side by side, mixed, keep most of both.** A slow map (all counts) and a fast map (halved past 127) per cell, with one switching mixture (one pair of weights for all cells) over the two:
  - **Byte stream:** 3.22 bits per byte. Round A without gzip 2.19 (better than either map alone: 2.23 slow, 2.24 fast); round B without gzip 2.86 (as the fast map alone). Order-4 counts: 3.66; xz in the stream: 2.89. File by file it is not always the best of the two: slightly worse than the slow map alone on round A's JSON, csv and Rust (by 0.01–0.03).
  - **Plain text (three Austen novels, 2 MB):** second half 1.862, against 1.877 before. One run, so a modest gain, but no cost where nothing is noise.
- **The floor and the guard cost almost nothing on plain text** (1.879 against 1.877; the guard skipped no judgments there, so this is the floor's mixture overhead).

## The rules (binaries `sit-58a`–`sit-58c`)

**`--floor`:**
- The code length reported for each byte comes from a switching mixture (fixed share, 1/1000 per byte) over nine fixed weights l of a uniform guess: (1 − l) p + l / 256, with l from 1/4096 to 15/16.
- It is prequential: the weights used for a byte come from the bytes before it.
- The brain's own learning (surprise, screens, chunk starts, the guard) still reads its raw bits, so the floor changes the code, not what is learned: runs with and without it choose the same cells byte for byte.

**`--prune-guard`:**
- A kept edge is not judged over a window where the level codes worse than a uniform guess (raw bits per byte above 8). Nothing predicts there, so the window says nothing about the edge.
- Its window starts again instead. 29 judgments were skipped in the byte stream (23 with two maps).

**`--map-cap N`:** a cell's byte counts are halved together once they pass N.

**`--map-two N`:**
- A second, fast map per cell (halved past N) beside the slow one.
- The byte's probability given the cell is a switching mixture of the two maps (fixed share, 1/1000).
- This changes the brain's raw bits too, so its surprise (chunk starts, screens) and the guard read the better map. The edges' training does not: it uses the cell's probability, which leaves the map out.

All other flags as in experiment 57: `stack --levels 1 --words --run-window 200000 --conjoin-words --long-trials 5 --grow-pools --pool-budget 524288 --readout3 --screen-mix --fresh-extensions`.

## The byte stream (bits per byte, per file)

The stream and columns are experiment 57's. Script `B=runs/exp36/sit-58c T=tag BEXTRA="flags" runs/exp57/run.sh brain`; table `scripts/exp57/brains.py`.

| | Type | WB4 | xz | Before | Floor | + guard | + cap 127 | + two maps (127) |
|---|---|---|---|---|---|---|---|---|
| A | text | 2.813 | 3.157 | 3.696 | 3.669 | 3.669 | 3.721 | 3.669 |
| A | json | 0.709 | 0.408 | 1.037 | 1.023 | 1.023 | 1.086 | 1.033 |
| A | csv | 3.172 | 2.900 | 2.806 | 2.807 | 2.807 | 2.873 | 2.815 |
| A | rust | 2.695 | 2.337 | 2.438 | 2.436 | 2.436 | 2.537 | 2.464 |
| A | xml | 1.250 | 0.913 | 1.056 | 1.057 | 1.057 | 1.114 | 1.046 |
| A | elf | 4.080 | 2.431 | 3.643 | 3.076 | 3.076 | 3.010 | 2.976 |
| A | bmp | 1.517 | 1.049 | 2.232 | 1.549 | 1.549 | 1.347 | 1.344 |
| A | gzip | 11.304 | 8.123 | 18.010 | 8.092 | 8.092 | 8.090 | 8.090 |
| B | text | 3.031 | 3.168 | 5.214 | 5.047 | 4.481 | 3.325 | 3.333 |
| B | json | 1.754 | 1.273 | 3.750 | 3.661 | 2.674 | 1.924 | 1.923 |
| B | csv | 3.559 | 3.064 | 3.860 | 3.830 | 3.693 | 3.304 | 3.305 |
| B | rust | 2.719 | 2.712 | 3.459 | 3.442 | 3.134 | 2.929 | 2.921 |
| B | xml | 2.355 | 1.655 | 2.261 | 2.245 | 2.112 | 1.797 | 1.793 |
| B | elf | 3.539 | 2.487 | 5.747 | 4.632 | 4.622 | 3.526 | 3.524 |
| B | bmp | 3.447 | 2.567 | 5.698 | 3.249 | 3.243 | 3.194 | 3.190 |
| B | gzip | 10.578 | 8.065 | 19.316 | 8.093 | 8.093 | 8.090 | 8.090 |
| All | | 3.658 | 2.894 | 5.264 | 3.619 | 3.485 | 3.242 | 3.220 |
| A without gzip | | 2.319 | 1.885 | 2.415 | 2.231 | 2.231 | 2.241 | 2.193 |
| B without gzip | | 2.915 | 2.418 | 4.284 | 3.730 | 3.423 | 2.857 | 2.856 |

Other caps (with floor and guard; all / round A / round B, both without gzip):

| Cap | All | Round A | Round B |
|---|---|---|---|
| 127 | 3.242 | 2.241 | 2.857 |
| 255 | 3.301 | 2.214 | 3.020 |
| 511 | 3.288 | 2.210 | 2.994 |
| 1023 | 3.345 | 2.212 | 3.122 |
| two maps, fast 63 | 3.252 | 2.192 | 2.932 |

- **Map bits** (the part of the cost paid for cell → byte), round B's text:
  - before: 1.34;
  - one map capped at 127: 0.35;
  - two maps: 0.35.
- Round A's text: 0.08 before and with two maps; 0.13 with the single capped map.
- A rerun of the cap 255 run after a container restart gave identical numbers (the runs are deterministic).

## Plain text (three Austen novels, 2,026,385 bytes)

Script `runs/exp57/text.sh` (binary from `B`).

| Run | Whole | Second half |
|---|---|---|
| Before | 1.956 | 1.877 |
| Floor + guard | 1.958 | 1.879 |
| + cap 127 | 2.032 | 1.945 |
| + cap 255 | 1.983 | 1.900 |
| **+ two maps (127)** | **1.945** | **1.862** |

## What this means

- **"I don't know" has to be a code, not an error.** The floor is a learned mixture with a uniform guess, chosen byte by byte from what came before. It costs almost nothing where the brain is right (plain text +0.002) and caps it where it isn't.
- **A stretch that nothing predicts says nothing about what predicted before.** Judging edges there throws away good structure. The guard simply waits.
- **Two timescales side by side beat one compromise.** This matches the project's principle that compression sits next to the uncompressed version: a fast map that follows change sits next to a slow map that keeps everything, and a mixture chooses per byte. It wins on the mixed stream and on plain text (one run each). The same pattern may pay elsewhere: the edges' own weights, and the word reader's route weights (experiment 53 found full-rate learning best, but never tried two rates side by side).
- **Round B still doesn't recall round A.** Round B's text still opens worse than round A's fresh start, though by much less than before (two maps: 7.16 against 5.97 bits per byte over the first 500 bytes, 6.58 against 5.30 over the first 1,000; before: 11.3 against 7.0 over the first 500). It catches up after about 8 KB. Nothing in the brain recognizes "this is text again" and brings back what it knew.

## Next

1. **A copy act** for the ELF files (3.52 against xz's 2.49): continue from the last occurrence of the current context.
2. **Two rates side by side for the edges' weights.**
3. **Recall by context:** a code for the kind of material (from the recent bytes) that selects or weights what was learned on similar material.
