# 39 · One rule at every level (2026-10-24)

*[All experiments](README.md)* · [← 38](38-read-then-ask.md)

**In short:**
- One chain rule for every level of nesting.
- Word kinds from an area of matchers with contested anti-Hebbian and sticky membership.
- Together they answer 96–100% of found questions about a book read by the index pass, three levels deep and up to a million words back (single seed).
- On BABILong the area matches leader clustering at 2.6× the speed.
- A clause type cannot come from its first word's kind: "part" and "volume" share one.
- Level 0's boundary rule is still its own statistic.

The owner asked for the forum's generalizations, made practical: a self-similar rule that works at each level of the recursion, so no level is designed by hand. In the owner's terms, an area of matchers runs three local rules, and the same area serves at every scale:

- **threshold:** a matcher fires when its overlap with the input passes it;
- **inhibition:** the firing matchers compete and one wins;
- **Hebbian swap:** the winner moves toward the input; with anti-Hebbian swaps, a loser that fired moves away, which pulls apart patterns that overlap.

Forum round five (project doc `claude/forum-round-five.md`) reduced the reader's hand-written map to four primitives. This page works on two of them:
- the recurrence operator over the structure levels (units, nesting, epochs);
- the classifier at each level (word kinds now; clause types read from them).

It also tests one level-0 boundary statistic offline.

All runs use experiment 37's configuration (`runs/exp37/F.flags`), seed 0 and one seed throughout. Runs shared 2 cores.

## New pieces

| Flag or tool | What it does |
|---|---|
| `--sit-levels` | One rule for nesting at any depth. The question's labels are ordered by the level of their types: the log2 of the mean recurrence interval, counting the open interval up to now. From each label as the root (its first and latest occurrence, in its type's current epoch), a chain descends. Each lower label's first occurrence inside the current unit narrows the unit to its own ("chapter twelve" in "part three" in "volume two"). Every depth is an anchor of its own (up to 8), and route credit picks how deep to trust. It replaces the pairs of `--sit-nest`. |
| `src/area.rs`, `--sit-area-kinds` | Word kinds from an area of matchers instead of leader clustering. Masks are over the 512-bit kind code. A matcher fires at Jaccard ≥ 2/5 (by cross-multiplication); the best overlap wins. The winner takes a Hebbian swap (one mask bit the input lacks out, one input bit the mask lacks in). With no firing matcher, one is recruited with the input as its mask, named by the word that recruited it. Candidates come from an inverted index read only at the input's rarest bits (prefix filtering): no scan. Words are presented only when their counts changed since the last presentation. |
| `--sit-area-anti 1/2/3` | Anti-Hebbian swaps for matchers that fired but lost. 1: a shared bit out, a random bit outside the input in. 2: one contested bit out (held by the input, the winner and the loser), down to half the mask's first size. 3: as 2, but only in a near-tie (the loser's Jaccard at least 3/4 of the winner's). |
| `--sit-area-sticky` | An item's incumbent matcher keeps it while it still fires, and a matcher swaps one time in 2^(bitlen(wins)/2), so established kinds hold still. |
| `--sit-kind-bits N` | Folds the 512 frame features onto N bits (feature b onto bit b mod N), so codes are narrower and overlap more. |
| crowding report | The area counts its presentations, those where two or more matchers fired (co-firing) and those with a near-tie: its own measure of how much the code's patterns overlap. |
| `--sit-open-kind` | A clause's type (for units, epochs and anchors) is the kind of its first word, the word level's output, instead of the word itself. |
| `longdoc_gen.py vols=V` | Three levels: V volumes ("volume N ."), each of P parts numbered from one again, each of C chapters numbered from one again. Found is asked as "what did X find in chapter M of part N of volume V ?". |
| `scripts/exp39/score_levels.py` | Scores found questions by whether the fact is in the volume being read or in an earlier one. |

## Level 0: a boundary statistic tested offline

The forum's first primitive was boundaries by surprise. A frequency-free version is the information that crosses a word: does knowing the word before w help predict the word after it? Three estimates were tried (`scratchpad` scripts, not committed). Each was run on the synthetic book (`ld200k`) and on the BABILong pilot's first 300k words.

- **Plug-in conditional entropy** (contexts with 8 or more uses).
    - **Synthetic book:** exactly "." and "?" stand out. Across "." the side before keeps 97–98% of the uncertainty about the side after; across "?", 72–74%. No other word is that open on both sides.
    - **BABILong:** it does not separate them. The share kept is 0.60/0.63 for ".", 0.49/0.65 for ",", 0.41/0.69 for "of", 0.59/0.62 for "that" and 0.71/0.58 for "was".
- **A prequential estimate** (counts before each event, interpolated): the smoothing made gains negative for most words; unusable as it stands.
- **The sides seen as classes** (the 255 most frequent words, and "rare"): "." is not among the 60 most open words, because most words before "." are rare.

Level 0 keeps the both-sides-open rule with its sticky hysteresis (`--sit-bnd-hyst`): a boundary stays one while its own openness holds at least a quarter of its peak.

## Structure levels: one chain rule

**From scratch on a three-level book** (`ld1m_v`: 1,004,771 words, 3 volumes × 4 parts × 8 chapters, questions in the text):

| Found, by where the fact is | `--sit-nest` pairs (`R_v_base`) | Chains (`R_v_lev2`) |
|---|---|---|
| this volume, 8k–64k | 97.2% (1,794) | 98.3% |
| this volume, 64k–512k | 77.5% (841) | 79.4% |
| earlier volume, 64k–512k | 61.7% (823) | 75.2% |
| earlier volume, 512k+ | 19.6% (275) | 23.3% |

- **The first version lost** (`R_v_lev`: found 30.7% at 64k–512k against 69.7%), for two reasons:
    - **Levels.** Headings that occurred before boundaries were learned were never stored, so "part" had one recorded start and level 0. Chains rooted at the chapter and never at the part.
    - **Depth.** Chains descended past the right depth into answer clauses.
- **The fixes:** the level counts the open interval up to now, and each depth is an anchor.
- **The 512k+ facts are all in volume one.** Its heading at word 0 came before any boundary was learned, so no chain can start there.
- **No loss on two levels:** on the 200k book every score is the same (`R_c2_base`, `R_c2_lev`: found 93.7% at 64k+, home 99.3%, owner 100%). The run took 302 s against 190 s.

**Read then ask** (`ld2m_vv`).
- **Setup:**
    - Book A (`ld1m_v`) is read in full, learning on.
    - Book B (`ld1m_v8`, seed 8, 1,036,582 words) has no questions in its text. It is read by the index pass with skill frozen, and 2,130 questions come at its end (2,094 found).

| Book B, found | Pairs (`R_vv_base`) | Chains (`R_vv_lev`) |
|---|---|---|
| this volume, 64k–512k | 94.0% (565) | 94.5% |
| **earlier volume, 64k–512k** | **15.8% (355)** | **94.6%** |
| **earlier volume, 512k+** | **13.9% (1,045)** | **81.1%** |
| right answer among 8 candidates, 512k+ | 71.3% | 98.3% |
| run time (both books) | 1,149 s | 1,155 s |

- **Pairs can only take "part N" in the current volume** (the part type's epoch). A question about an earlier volume finds the right chapter of the wrong part.
- **The chain takes volume, then part, then chapter.**
- **What is left at 512k+ is ranking:** the answer is among the candidates 98% of the time.

## Word kinds from an area of matchers

**Story set** (seed 0; leader clustering 98.5%), all questions:

| Code width | Leader | Area, no anti | Contested anti (2) | Near-tie anti (3) | Co-firing (no anti) |
|---|---|---|---|---|---|
| 512 bits | 98.5% | 97.7% | 97.0% | 96.5% | 158‰ |
| 128 bits | 96.0% | 96.5% | 96.3% | 96.0% | 167‰ |
| 64 bits | 95.1% | 97.3% | 96.0% | 97.5% | 165‰ |
| 32 bits | 95.8% | 97.3% | 94.8% | 96.5% | 223‰ |

- **Other variants at 512 bits:**
    - anti-Hebbian rule 1: 96.6%;
    - no swaps at all: 96.0% (masks frozen at their first, immature codes split the people in two);
    - anti-Hebbian without Hebbian: 95.5%;
    - contested anti with `--sit-area-sticky`: 95.3%.
- **The story set has about 50 words in about 30 kinds,** too few to crowd even 32 bits.

**BABILong pilot** (`bl_pilot`, 308,702 words, vocabulary 19,018; training then a frozen, answer-only test; the stack of `R_blpF`):

| Width, kinds | Test 0k / 4k | Training 1k | Co-firing / near-ties | Place kind | Time |
|---|---|---|---|---|---|
| 512, leader | 100 / 100% | 96.0% | – | 149 words | 112 s |
| 512, area, no anti | 0 / 0%\* | 84.0% | 228‰ / 154‰ | 7 (no hallway) | 71 s |
| 512, area, contested | 100 / 100% | 95.5% | 156‰ / 112‰ | 14 | 66 s |
| 512, area, near-tie | 100 / 98% | 95.0% | 172‰ / 107‰ | 7 | 65 s |
| 512, area, contested, sticky | 100 / 100% | 99.0% | 148‰ / 112‰ | – | 68 s |
| 128, leader | 100 / 94% | 95.0% | – | 191 | 123 s |
| 128, area, no anti | 11 / 30% | 92.5% | 484‰ / 333‰ | 21 | 97 s |
| 128, area, contested | 100 / 76% | 84.5% | 363‰ / 260‰ | 13 | 83 s |
| 128, area, near-tie | 0 / 0%\* | 74.5% | 395‰ / 261‰ | 11 (4 places lost) | 89 s |
| 64, leader | 64 / 60% | 95.5% | – | 295 | 163 s |
| 64, area, no anti | 100 / 52% | 89.5% | 858‰ / 612‰ | 23 | 137 s |
| 64, area, contested | 100 / 100% | 85.0% | 791‰ / 664‰ | 27 | 148 s |
| 64, area, near-tie | 0 / 0%\* | 84.5% | 802‰ / 661‰ | 38 | 139 s |

\* No predictions at question boundaries: the question detector's mood counters fell below threshold during training (after wrong answers), so the frozen test never answered. This failure is all or nothing, so single runs are noisy.

- **The crowding measure tracks the code's overlap:**
    - co-firing is about 150–230‰ at 512 bits, 360–480‰ at 128 and 790–860‰ at 64;
    - anti-Hebbian lowers it at every width.
- **Contested anti-Hebbian is the one rule that keeps the test at 100% at both 512 and 64 bits;** leader clustering falls to 64/60% at 64 bits. It does not hold everywhere: at 128 bits its 4k score is 76%, and training at 1k falls to about 85% at the narrow widths.
- **It is self-gated by overlap:** it acts only when two matchers fire on one input, so its rate is the co-firing rate. A separate gate on near-ties (rule 3) was worse.
- **Area kinds are purer:** the place kind holds 7–27 words against leader clustering's 149–295.
- **At 512 bits the area is the fastest** (66 s against 112 s). At 64 bits, more matchers fire per input and it slows (148 s).

**BABILong long test** (`bl_long`: 30 qa1 documents at 16k, about 13k words each, and 8 at 64k, about 51k words; frozen, answer-only):

| Kinds | 16k (n 30) | 64k (n 8) | Time (950k words) |
|---|---|---|---|
| leader (`R_bl_long`) | 93.3% | 87.5% | 739 s |
| area, contested (`R_bl_long_ak2`) | 60.0% | 37.5% | 278 s |
| area, contested, sticky (`R_bl_long_ak2s`) | 93.3% | 75.0% | 279 s |

- **Without sticky, kinds drifted during the frozen test.**
    - The area keeps learning, so words moved between matchers.
    - Route skill trained on kind ids stopped matching: the wrong answers are words like "the", "and" and "be", not other places.
- **With sticky membership and win-scaled plasticity, 16k matches leader clustering** at 2.6× the speed. 64k is one document short (6 of 8 against 7 of 8).

## Clause type from the word level: failed

`--sit-open-kind` reads a clause's type from the kind of its first word, so the level above uses the level below's classes. On the three-level read then ask (`ld2m_vv`, with chains):

| Book B, found | Chains, type = first word (`R_vv_lev`) | Type = leader kind (`R_vv_ok`) | Type = area kind (`R_vv_self`) | Type = sticky area kind (`R_vv_selfs`) |
|---|---|---|---|---|
| this volume, 64k–512k | 94.5% | 30.4% | 22.7% | 99.8% |
| earlier volume, 64k–512k | 94.6% | 21.7% | 7.9% | 10.1% |
| earlier volume, 512k+ | 81.1% | 13.8% | 10.0% | 10.1% |

- **Why it fails:** in every kind rule, "part" and "volume" share a kind, because both are followed by a number word and ".". Their clauses then have one type.
    - A part's unit runs to the next part or volume heading.
    - A volume's own units and level are lost.
    - Leader clustering also merged "scene" with "where".
- **What a clause type needs:** something the word's distribution does not hold, which here is the word itself. The first word stays the clause type.
- **The sticky area's 99.8% on the current volume** suggests its kinds are otherwise steadier. One seed.

## Together: chains plus sticky area kinds

The chain rule with kinds from the sticky, contested area (`--sit-levels --sit-area-kinds --sit-area-anti 2 --sit-area-sticky`, clause type = first word; `R_vv_aks`), on the three-level read then ask:

| Book B | Pairs, leader kinds (`R_vv_base`) | Chains, leader kinds (`R_vv_lev`) | **Chains, sticky area kinds (`R_vv_aks`)** |
|---|---|---|---|
| found, this volume, 64k–512k | 94.0% (565) | 94.5% | **99.8%** |
| found, earlier volume, 64k–512k | 15.8% (355) | 94.6% | **99.2%** |
| found, earlier volume, 512k+ | 13.9% (1,045) | 81.1% | **96.3%** |
| home, 8k–64k / 64k–512k | – | 10.0% (20) / 0.0% (3) | **100% / 100%** |
| owner, all | – | 100% (12) | 100% |
| run time (both books) | 1,149 s | 1,155 s | 999 s |

- **This is the best read-then-ask result so far,** on a book one level deeper than experiment 38's.
- **Home without echoes is 100%** here, against 10% with leader kinds, but only on 24 questions. Owner has 12 questions.
- **One seed.**

## Open

- **Level 0's boundary rule** is still its own statistic.
- **The relation-class, skippable-word and stand-in rules** are not yet on the area.
- **Credit and forgetting** still use five separate settings.
- **The store grows on long book text** (experiment 38).
- **The question detector** collapses all or nothing.

## Caveats

- **One seed throughout.** Small test samples on BABILong: 100, 50, 30 and 8 documents.
- **Story-set differences of 1–2 points are within what one seed can show.**
- **Runs shared 2 cores,** so compare times only within a table.
