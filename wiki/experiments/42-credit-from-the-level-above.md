# 42 · Credit from the level above: boundaries disposed by the clause level (2026-10-24)

*[All experiments](README.md)* · [← 41](41-forum-six-first-tests.md)

**The owner's principle:** "Predictions should be local, for the most part, with credit being a high-level prediction crediting the lower level, to align small goals with high-level goals."

Forum round seven (project doc `claude/forum-round-seven.md`) designed the first such rule, for the word-level boundaries. This page tests it.

**Results:**
- **The first learned rule that removes "the" as a binding stop without breaking the other sets.**
  - Small qa2 with no hand ban: 33% → 97–99% on three seeds (100% length-weighted).
  - Story set: 97.3–98.5%.
  - BABILong pilot: 96–100%.
  - These hold on three seeds for the plain judge, and on seed 0 for the length-weighted one.
- **On the full qa2 stream (book text from about word 102k), the plain judge put "the" back at word 170k.**
  - The length-weighted judge keeps "the" out: test 87 / 34%.
  - The hand ban with the same stack scores 97 / 65%, and the old rule 16 / 18%.
  - It also stops binding at "and" and "of", which may cost the gap.

## The rule (`--sit-bnd-unit`; with `--sit-bnd-unit-len`)

**The clause item.**
- A clause is a run between binding stops, with the stop left out.
- Its type is its first word, its path is a hash of its middle words, and its value is its last word.

**The prediction.**
- A unit head table keeps each (type, path)'s newest 2 values. This is the long-term heads rule, one level up.
- **The clause hits** when its value is in the table and the reader's own top proposal at that word was wrong. The clause level is credited only for what the word level missed.
- With `--sit-bnd-unit-len`, a hit counts the words its clause covers.

**The shadow.**
- At each occurrence of a proposed word u (u ∈ L), the shadow world flips u's decision: a stop's two clauses are merged, and a crossed u's clause is split there.
  - A stop is judged once, for the clause it opens.
  - A crossed word is judged only when it lies strictly inside a clause.
  - A clause longer than 256 words is trimmed without judgement.
- It is scored on the same words.
- Shadows are looked up and written in the unit table only, never in the bindings or long-term heads. There is no exploration coin.

**The credit.**
- Where the two worlds' hits differ, u's flip gets a win or a loss.
- At each re-pooling, the proposed word with the largest margin flips, if (wins − losses)² > 9 · (wins + losses). Its counts then restart.
- Both counts halve when wins + losses reaches 1,024 (checked at each re-pooling).
- L proposes and the clause level only disposes, so the rule never adds a stop.

Binaries `sit-41p` (`--sit-bnd-unit`, `--sit-q-contain`, `--sit-q-in-l`) and `sit-41q` (`--sit-bnd-unit-len`). Scripts `runs/exp41/f1.sh`, `f1s.sh` and `f1l.sh`. The stacks match experiment 41's: small qa2 and full qa2 with chains and `--sit-lt-budget 131072`; full qa2 also with `--sit-entity-fair`. There is no hand ban and no halving sketch.

## Small sets, no hand ban

| Run | Small qa2 | Story set | Pilot 0k / 4k | "the" leaves the stops at word |
|---|---|---|---|---|
| Old rule (exp 40–41) | 33% | 98.5% | 100 / 100% | never |
| Judge, seed 0 | **99%** | 98.5% | 100 / 100% | 20,000 |
| Judge, seed 1 | **97%** | 97.3% | 100 / 96% | 14,000 |
| Judge, seed 2 | **99%** | 97.5% | 100 / 100% | 14,000 |
| Length-weighted, seed 0 | **100%** | 98.5% | 100 / 100% | 8,000 |
| Judge + question guards, seed 0 | 100% | 97.7% | 63 / 50% | 24,000 |

Seed 0's counts at the end (flipping wins / losses):

| Word | Small qa2 | Story set | Pilot |
|---|---|---|---|
| "the" (now crossed) | 190 / 376 | — | — |
| "." | 130 / 599 | 89 / 634 | 179 / 820 |
| "?" | — | 11 / 449 | 0 / 379 |

"," "a" "and" "of" (pilot) also lose. On small qa2 the stops end as the hand-ban diagnostic's ("."). On the story set and the pilot nothing flips: their proposed stops (". ?" and ". ? , a and of") stay, which is what a ban on "the" would leave anyway.

**The question guards are not needed, and they hurt the pilot.** These are the contrast detector within L, plus a drop for any clause that contains a question word, with question isolation. This forum's shortcut fix is structural: a shadow world that is right where the default is also right never wins.

## The full qa2 stream (525,866 words; book text from about word 102k)

The plain training stories end at word 102,303. After that come the 1k training stories (with book text), the 0k test (plain, words 220.6k–231.1k) and the 4k test (book text, from about 231k). The judge keeps running after the freeze at word 220,618, so flips after that point happen during the test.

| Run | Test 0k / 4k | Training 0k / 1k | Binding stops at the end | Flips |
|---|---|---|---|---|
| Hand ban + cap (exp 41, `R_qa2_dnef0`) | 97 / 65% | 90.5 / 85.5% | . , and of | — |
| Judge + cap | 23 / 7% | 77.9 / 54.0% | the . , and | "the" off at 20k, **back on at 170k**; "of" flips 13 times (322k–522k, during the test), ending crossed |
| Length-weighted + cap | **87 / 34%** | 83.5 / 87.5% | . , | "the" off at 6k; "and" off at 152k; "of" off at 422k (during the test) |

- **The plain judge prefers cutting at "the" in book text.** Two short fragments ("the X") get two chances to repeat on the same words, while a whole clause rarely repeats. At the end, flipping "the" back to crossed loses 137 / 464.
- **Weighting a hit by its words fixes that,** and "the" stays out for the whole stream.
- **It also crosses "and" and "of" in book text.** The longer clauses there probably add one-off paths, which would explain the 4k gap (34% against 65% with the hand ban).
- One seed.

## Next

1. Three seeds of the length-weighted judge on the full stream, and long BABILong with the cap. Decide whether the judge, like other structure, should keep running in the test (it does now).
2. Find out whether crossing "and" and "of" is what costs 4k. Check the hand-ban stack with those two crossed.
3. **Make it self-similar.** Units today are cut by recurrence alone, so a learned opener set per level is needed for the same judge to credit unit cuts from the epoch level.
4. Route the same shadow comparison to heads and chain relations (the panel's second step).
