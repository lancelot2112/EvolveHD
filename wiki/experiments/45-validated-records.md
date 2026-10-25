# 45 · Store questions with their answers, validate the records (2026-10-25)

*[All experiments](README.md)* · [← 44](44-builders-from-nothing.md)

**The owner's idea:** "Do we need to keep questions out? Storing a question with its answer would be good … Store the context with its prediction, and that prediction gets validated or not."

**Results (one seed each, experiment 42's best stack unless stated):**
- **The union of k90 and the ledger as the proposer is the best all-round set so far:** small qa2 100%, story set 97.8%, pilot 100 / 100%, long 100% / 7 of 8; full qa2 87 / 25%. The full qa2 4k test swings from 25% to 71% across proposers and seeds (G2 itself: 71% on seed 0, 47% on seed 1).
- **Question records keyed by (question form, entity), each with its own validation, change nothing:** a repeated question's stored answer is right only 18–26% of the time.
- **Questions stored with no drop cost nothing on small qa2 or the pilot, but cost the story set's where-object questions** (64% against 92.6%).
- **The damage is not stale facts. It is shortcuts through function words.** A stored "where is the apple ? kitchen" gives "?", "the" and "where" place slots holding the latest answer. At the next where-object question the top route reads "the"'s or "?"'s place slot (the previous answer, or the last place mentioned) instead of the chain apple → mary → kitchen.
- **Validating each head by its own hits and misses does not help,** in any of four variants (story set 71.5–92.6%, against 94.5% without validation). The wrong answers mostly come from working-memory slots of function words, not from long-term heads.
- **G2 (experiment 44's collision ledger) with a question detector that does not depend on L answers on the pilot: 58 / 38%,** against 0 / 0% when the detector was tied to L.

## The rules

- `--sit-no-qdrop`: no question drop; question clauses are committed like any other.
- `--sit-validate`: each long-term head keeps (hits, misses) at answers, and a head with more misses than hits is not read.
  - **Version 1:** every head offered with a value of the answer's kind gains a hit if it is the answer, else a miss; counts clear when the head takes a new value.
  - **Version 2** (`sit-45c`): a hit for every head that held the answer, a miss only for the head behind the answer given (as NoGo blames only the route taken).
  - `--sit-validate-slot`: counts kept across rewrites, so the slot (entity, key, kind) builds a track record rather than its current value.

Binaries `sit-45a` (version 1, `--sit-no-qdrop`), `sit-45b` (`--sit-validate-slot`), `sit-45c` (version 2). Scripts `runs/exp45/v.sh` and `run1.sh`.

| Run | Story set (where-object) | Small qa2 | Pilot 0k / 4k |
|---|---|---|---|
| Question drop (exp 42's best) | 98.5% (92.6%) | 100% | 100 / 100% |
| No drop | 94.5% (64.2%) | 100% | 100 / 100% |
| No drop + validation v1 | 90.1% (65.4%) | 88% | 98 / 94% |
| Drop + validation v1 | 95.6% (88.9%) | 86% | 86 / 94% |
| No drop + validation v1, slot counts | 71.5% (60.5%) | — | — |
| No drop + validation v2 | 91.8% (59.3%) | — | — |
| No drop + validation v2, slot counts | 92.6% (66.7%) | — | — |
| G2 (ledger proposes) + L-free contrast detector | 96.5% (88.9%) | 100% | **58 / 38%** |

- **Version 1 punishes correct heads.** Chains offer the place heads of every hop-1 entity, so the correct places of people who don't hold the object gain misses. With slot counts, people's place heads accumulate those misses and stop being read (71.5%).
- **Version 2 barely acts** (11–13 misses in the whole story run). The answers given are rarely wrong through a head, so heads aren't where the errors come from.
- **The route trace at "where is the apple ?" (truth kitchen):**
  - with the drop, the top route is apple → mary → kitchen;
  - without it, the top routes are "the"'s place slot (hallway, the last place mentioned) and "?"'s place slot (kitchen, the previous answer).
  - Traces are in the session scratchpad, `q_nd.txt` and `q_dr.txt`.
- **The L-free detector on the pilot finds "sandra went daniel mary ? john sugar":** it fires on "?" among other words, so the reader answers (2,076 predictions), but L is still full of book words, which caps the score.

## Question records keyed by what the question asks (`--sit-qrec`)

The fix the first round pointed to, built as the owner asked ("store the context with its prediction, and that prediction gets validated or not"):
- **The record.** At an answer, each content cue word e gets a record (question form, e) → answer. The form is the cue with every content word replaced by its kind, so "where is the milk ?" and "where is the apple ?" share a form.
- **Validation.** When the same question about e is answered again, the record's value is checked against the answer (a hit or a miss), then set to the new answer.
- **Reading.** At a question, each content cue word's record is offered as a candidate. Its validation (hits and misses, capped at 3) and whether e has a newer long-term head are features, so the route learner learns when to trust it. A record with more misses than hits is not offered.
- **The question drop stays** for the raw bindings. Binary `sit-45d`.

| Run | Story set (where-object) | Small qa2 | Pilot 0k / 4k | Records: written / hits / misses |
|---|---|---|---|---|
| Question drop (exp 42's best) | 98.5% (92.6%) | 100% | 100 / 100% | — |
| Drop + question records | 98.5% (92.6%) | 100% | 100 / 100% | story 32 / 262 / 771; qa2 177 / 1,627 / 4,598; pilot 762 / 387 / 1,803 |
| No drop + question records | 94.1% (64.2%) | — | — | 32 / 262 / 771 |

- **The records change no answer.** When the same question comes back, the stored answer is right only 18–26% of the time (bAbI moves things between questions). Most records fail validation and are never offered, and the rest never outrank the facts.
- **They do not cure the shortcut either:** with no drop, the story set is unchanged (94.1%). The damage comes from the raw question bindings, which the records do not replace.
- **So in these tasks the facts carry the answers, not remembered answers.** Validation works as intended: it keeps stale answers out. But a remembered answer is rarely worth keeping here.

## G2 follow-ups: an L-free detector, and the union of k90 and the ledger

- **G2 + L-free contrast detector** (`--sit-q-contrast`): the ledger proposes, the judge disposes, and the question detector no longer needs "?" in L.
- **Union** (`--sit-bnd-union`, binary `sit-45e`): L = k90's proposals ∪ the ledger's, the judge disposes, the usual detector on L. k90's hysteresis sees only k90's own set.

| Run | Small qa2 | Story set | Pilot 0k / 4k | Full qa2 0k / 4k | Long 16k / 64k |
|---|---|---|---|---|---|
| Exp 42's best (k90 + judge) | 100% | 98.5% | 100 / 100% | 87 / 34% | — |
| Old L + cap (exp 41) | — | — | — | — | 93.3% / 8 of 8 |
| G2 (exp 44, L-tied detector) | 100% | 95.8% | 0 / 0% | 92 / 71% | 0 / 0% |
| G2 + L-free detector | 100% | 96.5% | 58 / 38% | 87 / 36% | **100% / 8 of 8** |
| **Union + judge** | **100%** | 97.8% | **100 / 100%** | 87 / 25% | **100% / 7 of 8** |
| G2, seed 1 | — | — | — | 88 / 47% | — |

- **The union keeps "?" on qa1 book text, so the pilot is back to 100 / 100%**, and long BABILong reaches 100% / 7 of 8. On small qa2 "the" is proposed again by k90, and the judge removes it (100%).
- **On full qa2 the 4k test swings from 25% to 71% across these variants,** with the same plain-text score (87–92%). G2's 71% (exp 44) is not robust: the same proposer with a different detector gives 36%. The 4k test on one seed cannot rank these rules: G2 on seed 1 gives 88 / 47%.
- **The L-free detector picks content words as well as "?"** ("milk went apple ? football owner stories" on full qa2), so it predicts at many extra places. That's harmless for scoring, but it adds noise to head credit.

## What this means

- **Storing a question with its answer is good only if it is keyed by the question's content.** Today a stored question becomes bindings of "?", "the" and "where" to the answer, which the route learner then uses as a cheap recency trick. A validated record would be keyed by (question form, entity) → answer.
- **Validation belongs on the routes that give answers, including working-memory slots of function words,** and Go/NoGo already does that per route. Per-record validation of heads adds nothing here.
- **The question detector should not depend on L.** Untying it is what lets the ledger answer at all on qa1.

## Next

1. (Done above: question records change nothing, because stored answers are stale 3 times in 4.)
2. (Done above: the union keeps the pilot at 100 / 100% and long at 100% / 7 of 8; the full qa2 4k test is too noisy on one seed to rank proposers.)
3. Three seeds of the union and of exp 42's best on full qa2 and long, before choosing a proposer.
