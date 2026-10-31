# 65 · A learned role gate for descriptions: what teaches it (2026-10-31)

*[All experiments](README.md)* · [← 64](64-description-roles.md)

**The question:** experiment 64 showed that most of the description expert's gap in the stack was the role: told which segments are descriptions (the oracle), it reads the magic of a new format nearly as well as the prototype. Can a small learner find that role itself, from the segment's words, taught only by what happens next?

**The learner:** not the full word reader but a gate like its act gate. It is a Go/NoGo perceptron over the previous segment's words (hashed, present or not), a bias and the segment's length. At each new segment it decides whether the segment before is this one's description. With NoGo the expert neither learns from this segment nor is used on it. The teacher labels each segment once its first 16 bytes are known. The four teachers tried differ only in that label.

**Results** (experiment 60's progressive formats stream; magic, version and constant bytes: second half, first files of never-seen formats; all-file-bytes figures: the whole stream. One run each. The brain is the same in every run (identical symbols); only the expert stage differs):
- **Which teacher matters more than anything else.** The best two (the previous segment must explain the next 16 bytes better than *no description*) close most of the oracle's gap on whole files and about half on the magic:
  - All file bytes with the right description: no gate 3.431, best gate **3.337**, oracle 3.324. First files: 3.770 → **3.630** (oracle 3.588).
  - Magic bytes, mixed: 8.83 → **7.79** and **7.66** (oracle 6.49). Expert alone: 6.78 → 5.94 and **5.60** (oracle 4.94).
  - Version bytes, expert alone: 6.75 → 5.46 (oracle 4.80).
- **The gate learns its teacher; the teacher's accuracy sets the result.** In every run the gate's agreement with the truth equals its teacher's to within 0.001, and with the background teachers the gate agrees with its teacher on 94–95% of segments. So the gate can read the role from the words. What limits it is how often the teacher's label matches the truth.
- **Contrast against a decoy fails because a wrong description is worse than none.** The decoy (the segment three back) is often another format's description, which predicts wrong bytes. A previous binary file that says nothing beats it by 2 bits just by being less wrong. The gate then said Go after about half of files. Costing a decoy with no known words by the background, instead of a flat 8 bits per byte, didn't change that (Go after files 48.5% → 46%).
- **No description is the right baseline.** Against the background alone, Go after files falls to 29% (36% in the second half). Requiring the previous segment to beat both the decoy and the background gives 17% (22%), with Go after 88% (95%) of descriptions. That is 84.5% agreement with the truth.

## The teachers

Each teacher labels a segment as having a description before it when, over its first 16 bytes:

| Run | Teacher (binary) | Agrees with truth | Go after a description | Go after a file |
|---|---|---|---|---|
| `role` | the expert's shadow gain over the code before it is more than 1 bit (`sit-65a`) | 75.3% | 69.9% | 21.2% |
| `rolec` | the previous segment's cost is more than 2 bits below a decoy's (`sit-65b`) | 67.4% | 91.9% | 48.5% |
| `rolec2` | the same; a decoy with no known words costed by the background (`sit-65c`) | 67.8% | 89.2% | 46.1% |
| `rolen` | the previous segment's cost is more than 2 bits below the background alone's (`sit-65d`) | 81.3% | 96.8% | 28.7% |
| `rolenb` | more than 2 bits below both the decoy and the background (`sit-65d`) | **84.5%** | 87.7% | **17.5%** |

- **Agreement** is the gate's decisions against the truth (is the previous segment a description?), over the 5,875 taught segments. A description precedes 39.4% of them. Script `scripts/exp65/role_agree.py`.
- **The second half:** `rolen` 77.8% (Go after descriptions 99.1%, after files 36.3%); `rolenb` 84.4% (94.8%, 22.4%).
- **Costs:** all teachers but the first use the expert's tables read-only (`shadow_cost`): the alignment of the 16 bytes against the given segment's tokens, with the background for bytes no token explains. The background alone is that cost with no tokens.
- **The first teacher's errors:** it says Go for 57% (62% in the second half) of the files that follow another file. On this stream the previous file does help the stack's code of the next one, so "useful before" and "a description" differ there. Its gate, though, says Go after only about a fifth of files, whatever follows them.

## The magic bytes of never-seen formats

Bits per byte, second half, first file of each new format.

| Run | Right description | Wrong description | None |
|---|---|---|---|
| Expert, guessing (exp 63), alone / mixed | 6.78 / 8.83 | 9.18 / 9.84 | 8.36 / 9.05 |
| Shadow gain teacher (`role`) | 6.23 / 8.83 | 8.91 / 9.67 | 8.21 / 9.49 |
| Decoy teacher (`rolec`) | 5.92 / 8.37 | 8.95 / 9.60 | 8.16 / 9.31 |
| Decoy, background for empty decoys (`rolec2`) | 6.20 / 8.46 | 9.06 / 9.62 | 8.25 / 9.34 |
| Background teacher (`rolen`) | 5.94 / 7.79 | 9.09 / 9.49 | 8.23 / 9.37 |
| **Decoy and background (`rolenb`)** | **5.60 / 7.66** | 9.02 / 9.47 | 8.17 / 9.48 |
| Oracle (exp 64) | 4.94 / 6.49 | 9.04 / 9.22 | 8.00 / 9.31 |

**Version and constant bytes** (right description, expert alone / mixed):

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
| Expert, guessing | 3.431 | 3.521 | **3.624** | 3.770 | 3.287 |
| `role` | 3.422 | 3.521 | 3.647 | 3.752 | 3.281 |
| `rolec` | 3.441 | 3.533 | 3.640 | 3.778 | 3.297 |
| `rolec2` | 3.416 | 3.518 | 3.635 | 3.745 | 3.276 |
| `rolen` | **3.337** | **3.475** | 3.638 | **3.630** | **3.211** |
| `rolenb` | 3.348 | 3.487 | 3.644 | 3.642 | 3.222 |
| Oracle | 3.324 | 3.465 | 3.671 | 3.588 | 3.211 |

- **Files with a wrong description** gain too (3.521 → 3.475, oracle 3.465), so the gate's gain isn't only on files whose own description came before.
- **Files with no description lose a little with every gate** (3.624 guessing against 3.635–3.647). The guessing expert reads something from the previous file there, and the gates mostly stop it. The oracle, which stops it always, loses more (3.671).
- **Description bytes:** 0.578 with no expert or the oracle, 0.587–0.594 with a gate, 0.603 guessing.

## The rules (binaries `sit-65a` to `sit-65d`)

- **`--desc-role`:** the role gate (`acts::RoleGate`).
  - Features: a bias, the previous segment's length (its power of two), and up to 64 of its words (split on whitespace, at most 32 bytes, hashed into 4,096 integer weights).
  - Go when the summed weights are at least zero, so at first everything is Go, as without the gate.
  - On a mistake each active weight moves by one toward the label (perceptron).
  - NoGo: the expert's learning rate is 0 on this segment and the stage passes the code before it through unchanged.
  - Without the options below, the teacher is the shadow gain (`role`).
- **`--desc-role-contrast`:** the label comes once 16 bytes of the segment are known: the previous segment's read-only cost must be more than 2 bits below the reference's.
  - The reference is the decoy, the segment three back (`rolec`, `rolec2`).
- **`--desc-role-null 1`:** the reference is the background alone (`rolen`). **`--desc-role-null 2`:** the lower of the decoy and the background (`rolenb`).
- **Runs:** `runs/exp63/desc2.sh` (commands at its end). Position sense on, stride act off. `ROLE_DUMP=1` prints each label with the gate's decision.

## What this means

- **A learner can read a description's role from its words, once the teacher asks the right question.** "Does this segment tell me something right about what follows, more than nothing would?" is that question. "More than another segment would" isn't, when other segments can be confidently wrong.
- **That teacher is local credit from the level above,** as in experiment 64's learning by trust. Here, though, the credit decides *what* the expert reads, not how fast it learns. This keeps the notation's specific gain, which learning by trust lost.
- **What's left of the gap is the teacher's errors after files.** The best gate still says Go after about a fifth of files. Some of those may be real (a previous file can help); the oracle shows the expert gains more when those are NoGo (magic, mixed 7.66 against 6.49). On files with no description the gates lose a little against guessing (3.635–3.647 against 3.624), so some of those Go decisions did pay.
- **One run each, on one synthetic stream.** The gate's features are words, and on this stream descriptions are English and files mostly binary, so words separate them easily. The gate isn't yet tested where descriptions and files look alike.

## Next

1. **Make it the expert's default** (`--desc-role --desc-role-contrast --desc-role-null 2`), and test it on the byte stream of experiment 57 with real files and real manuals.
2. **A teacher with no fixed threshold:** the 2-bit margin is hand set. It could be judged by the gate's own record (how often Go paid), in the spirit of "learn fast, then slower".
3. **The full word reader as the gate:** its act path, instead of a bag of words, for descriptions that aren't separable by words alone.
