# 65 · A learned role gate for descriptions: what teaches it (2026-10-31)

*[All experiments](README.md)* · [← 64](64-description-roles.md)

**The question:** experiment 64 showed that most of the description expert's gap in the stack was the role. Told which segments are descriptions (the oracle), it reads the magic of a new format nearly as well as the prototype. Can a small learner find that role itself, from the segment's words, taught only by what happens next?

**The learner:** not the full word reader, but a gate like its act gate.
- It is a Go/NoGo perceptron over the previous segment's words (hashed, present or not), a bias and the segment's length.
- At each new segment it decides whether the segment before is this one's description.
- With NoGo, the expert's counts don't change on this segment and the stack doesn't use it there.
- The teacher labels each segment once its first 16 bytes are known. The runs differ only in that label: four teachers, plus a costing variant of one.

**Results** (experiment 60's progressive formats stream; one run each):
- Magic, version and constant bytes: second half, first files of never-seen formats. All-file-bytes figures: the whole stream.
- The brain is the same in every run (identical symbols); only the expert stage differs.

**What we found:**
- **Which teacher it has matters more than anything else.** The two best ask whether the previous segment explains the next 16 bytes better than *no description* does. They close most of the oracle's gap on whole files and about half of it on the magic (figures as `rolen` / `rolenb`):
  - All file bytes with the right description: no gate 3.431, gate **3.337 / 3.348**, oracle 3.324.
  - First files: 3.770 → **3.630 / 3.642** (oracle 3.588).
  - Magic bytes, mixed: 8.83 → **7.79 / 7.66** (oracle 6.49).
  - Magic bytes, expert alone: 6.78 → 5.94 / **5.60** (oracle 4.94).
  - Version bytes, expert alone: 6.75 → 5.46 / 5.45 (oracle 4.80). Mixed they barely move: 7.34 → 7.26 / 7.16 (oracle 6.00).
- **With these teachers the gate reads the role from the words.** It agrees with its teacher on 94.7% and 93.6% of segments, so what limits it is how often the teacher's label is right. With the other teachers it agrees with them on only 67–81% of segments. Its overall agreement with the truth still equals the teacher's to within 0.001 in every run, but that follows almost automatically: a perceptron that can tell descriptions from files ends up with its teacher's Go rate in each group, even when it can't follow the individual labels.
- **Contrast against a decoy works on the magic, not on whole files.**
  - The decoy is the segment three back. Magic, mixed: 8.83 → 8.37; expert alone 6.78 → 5.92, as good as `rolen`.
  - But on whole files it is no better than guessing (3.441 against 3.431), and the gate says Go after about half of files (67.4% agreement).
  - **A likely reason:** a wrong description is worse than none (magic, expert alone: 9.18 with a wrong one against 8.36 with none). The decoy is another format's description in half the cases where a file comes before, so a previous file needs only to be less wrong to beat it by more than 2 bits. This mechanism wasn't tested directly.
  - Costing any segment with no known words by the background, rather than a flat 8 bits per byte (`rolec2`), hardly changed that: Go after files 48.5% → 46%.
- **No description is the better baseline.**
  - Against the background alone, Go after files falls to 29% (36% in the second half).
  - Requiring the previous segment to beat both the decoy and the background gives 17% (22%), with Go after 88% (95%) of descriptions. That is 84.5% agreement with the truth.

## The teachers

Each teacher labels a segment as having a description before it when, over its first 16 bytes:

| Run | Teacher (binary) | Agrees with truth | Agrees with teacher | Go after a description | Go after a file |
|---|---|---|---|---|---|
| `role` | the expert's shadow gain over the code before it is more than 1 bit (`sit-65a`) | 75.3% | 66.7% | 69.9% | 21.2% |
| `rolec` | the previous segment's cost is more than 2 bits below a decoy's (`sit-65b`) | 67.4% | 80.5% | 91.9% | 48.5% |
| `rolec2` | the same; any segment with no known words costed by the background (`sit-65c`) | 67.8% | 79.1% | 89.2% | 46.1% |
| `rolen` | the previous segment's cost is more than 2 bits below the background alone's (`sit-65d`) | 81.3% | 94.7% | 96.8% | 28.7% |
| `rolenb` | more than 2 bits below both the decoy and the background (`sit-65d`) | **84.5%** | 93.6% | 87.7% | **17.5%** |

- **Agreement:** the gate's decisions against the truth (is the previous segment a description?) and against its teacher's labels, over the 5,875 taught segments. A description precedes 39.4% of them. Script: `scripts/exp65/role_agree.py`.
- **The second half:** `rolen` 77.8% (Go after descriptions 99.1%, after files 36.3%); `rolenb` 84.4% (94.8%, 22.4%).
- **Who teaches:**
  - Only the first teacher is credit from the level above: it compares the stack's code with and without the expert.
  - The others use the expert's tables read-only (`shadow_cost`): the cost of aligning the 16 bytes against the given segment's words, with the background for bytes no word explains. The background alone is that cost with no words. The expert judges, after the fact, which segment would have helped it.
  - In `rolec` a segment with no known words cost a flat 8 bits per byte. From `rolec2` on it is costed by the background. With no decoy (the first segments), the decoy cost is still a flat 128 bits.
- **The first teacher's errors:** it says Go for 57% (62% in the second half) of the files that follow another file, but for 3% of the descriptions that follow a file.
  - On this stream the previous file does help the stack's code of the next file, so "useful before" and "a description" differ there.
  - The gate can't see what follows, and says Go after about a fifth of files either way.

## The magic bytes of never-seen formats

Bits per byte, second half, first file of each new format; each cell is expert alone / mixed.
- "Expert alone" is the expert's prediction on every byte, including segments where NoGo kept it out of the stack. So its gains show better-learned tables, not only the gate's choices.

| Run | Right description | Wrong description | None |
|---|---|---|---|
| Expert, guessing (exp 63) | 6.78 / 8.83 | 9.18 / 9.84 | 8.36 / 9.05 |
| Shadow gain teacher (`role`) | 6.23 / 8.83 | 8.91 / 9.67 | 8.21 / 9.49 |
| Decoy teacher (`rolec`) | 5.92 / 8.37 | 8.95 / 9.60 | 8.16 / 9.31 |
| Decoy, background for segments with no known words (`rolec2`) | 6.20 / 8.46 | 9.06 / 9.62 | 8.25 / 9.34 |
| Background teacher (`rolen`) | 5.94 / 7.79 | 9.09 / 9.49 | 8.23 / 9.37 |
| **Decoy and background (`rolenb`)** | **5.60 / 7.66** | 9.02 / 9.47 | 8.17 / 9.48 |
| Oracle (exp 64) | 4.94 / 6.49 | 9.04 / 9.22 | 8.00 / 9.31 |

**Version and constant bytes** (right description; expert alone / mixed):

| Run | Version | Constant |
|---|---|---|
| Guessing | 6.75 / 7.34 | 8.00 / 8.26 |
| `rolen` | 5.46 / 7.26 | 7.33 / 7.73 |
| `rolenb` | 5.45 / 7.16 | 7.21 / 7.67 |
| Oracle | 4.80 / 6.00 | 6.95 / 7.68 |

## All file bytes

| Run | Right | Wrong | None | First files, right | Later files, right |
|---|---|---|---|---|---|
| No expert | 3.494 | 3.586 | 3.658 | 3.874 | 3.332 |
| Expert, guessing | 3.431 | 3.521 | 3.624 | 3.770 | 3.287 |
| `role` | 3.422 | 3.521 | 3.647 | 3.752 | 3.281 |
| `rolec` | 3.441 | 3.533 | 3.640 | 3.778 | 3.297 |
| `rolec2` | 3.416 | 3.518 | 3.635 | 3.745 | 3.276 |
| `rolen` | 3.337 | 3.475 | 3.638 | 3.630 | 3.211 |
| `rolenb` | 3.348 | 3.487 | 3.644 | 3.642 | 3.222 |
| Oracle | 3.324 | 3.465 | 3.671 | 3.588 | 3.211 |

- **Files with a wrong description gain too** (3.521 → 3.475, oracle 3.465), so the gate's gain isn't only on files whose own description came before.
- **Files with no description lose a little with every gate** (3.624 guessing against 3.635–3.647). The guessing expert reads something from the previous file there, and the gates mostly stop it. The oracle, which always stops it, loses more (3.671).
- **Description bytes:** 0.578 with no expert or the oracle, 0.587–0.594 with a gate, 0.603 guessing.

## The rules (binaries `sit-65a` to `sit-65d`)

- **`--desc-role`:** the role gate (`acts::RoleGate`).
  - **Features:** a bias, the previous segment's length (its bit length), and up to 64 of its words. Words are split on whitespace, at most 32 bytes, and hashed into 4,096 integer weights.
  - **Decision:** Go when the summed weights are at least zero. So at first everything is Go, as without the gate.
  - **Learning:** on a mistake, each active weight moves by one toward the label (perceptron).
  - **NoGo:** the expert's learning rate is 0 on this segment, and the stage passes the code before it through unchanged; the mixture's weights don't change. The expert still adds the previous segment's words to its vocabulary.
  - **Default teacher:** without the options below, the shadow gain (`role`): more than 1 bit over the 16 bytes.
- **`--desc-role-contrast`:** the label comes once 16 bytes of the segment are known. The previous segment's read-only cost must be more than 2 bits below the reference's.
  - **The reference:** the decoy, the segment three back (`rolec`, `rolec2`).
- **`--desc-role-null 1`:** the reference is the background alone (`rolen`).
- **`--desc-role-null 2`:** the reference is the lower of the decoy and the background (`rolenb`).
- **Runs:**
  - The commands are at the end of `runs/exp63/desc2.sh`. Position sense is on, stride act off.
  - `ROLE_DUMP=1` prints each label with the gate's decision, to stderr. The dumps are the `ROLE` lines of each run's stderr, kept in `runs/exp63/*_dump.txt`.
  - `role_dump.txt` came from a rerun of `role` with the dump added (its counts match `fmt_role` exactly).

## What this means

- **A learner can read a description's role from its words, once the teacher asks the right question.**
  - The right question: "does this segment say something right about what follows, more than nothing would?"
  - "More than another segment would" isn't it, when other segments can be confidently wrong.
- **The winning teacher is the expert judging itself, not credit from the level above.** It compares, with its own tables, how well the segment would have explained the next bytes against having nothing.
  - The teacher that used the level above (the stack's code with and without the expert) was worse. On this stream the previous file often helps the stack, so its labels mix "useful" with "description".
  - Unlike learning by trust (exp 64), the gate decides *what* the expert reads, not how fast it learns, and keeps the notation's specific gain.
- **The rest of the gap to the oracle isn't separated.** The best gate still says Go after about a fifth of files, and NoGo after 12% of descriptions (5% in the second half). Earlier wrong Go decisions also train the expert's tables on segments that aren't descriptions. Which of these costs the remaining 1.2 bits on the magic (7.66 against 6.49) wasn't measured. Some Go decisions after files do pay: on files with no description, every gate is slightly worse than guessing.
- **Caveats:**
  - One run each, on one synthetic stream.
  - The gate's features are words. On this stream descriptions are English and files mostly binary, so words separate them easily. The gate isn't yet tested where descriptions and files look alike.
  - `rolen` and `rolenb` differ by about 0.01 bits per byte on whole files (`rolen` better) and 0.13 on the magic (`rolenb` better).

## Next

1. **Make a background teacher the expert's default** (`--desc-role --desc-role-contrast --desc-role-null 1` or `2`; the two are within one run's noise on whole files), and test it on experiment 57's byte stream with real files and real manuals.
2. **A teacher with no fixed margin:** the 2-bit margin is hand set. The gate could judge it by its own record (how often Go paid), in the spirit of "learn fast, then slower".
3. **The full word reader as the gate:** its act path, instead of a bag of words, for descriptions that words alone don't separate.
