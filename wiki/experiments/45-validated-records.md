# 45 · Store questions with their answers, validate the records (2026-10-25)

*[All experiments](README.md)* · [← 44](44-builders-from-nothing.md)

**The owner's idea:** "Do we need to keep questions out? Storing a question with its answer would be good … Store the context with its prediction, and that prediction gets validated or not."

**Results (one seed each, experiment 42's best stack unless stated):**
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

## What this means

- **Storing a question with its answer is good only if it is keyed by the question's content.** Today a stored question becomes bindings of "?", "the" and "where" to the answer, which the route learner then uses as a cheap recency trick. A validated record would be keyed by (question form, entity) → answer.
- **Validation belongs on the routes that give answers, including working-memory slots of function words,** and Go/NoGo already does that per route. Per-record validation of heads adds nothing here.
- **The question detector should not depend on L.** Untying it is what lets the ledger answer at all on qa1.

## Next

1. Store the answer as a record keyed by the question form and the cue entity, not as raw bindings of the question's words, with its own hit and miss count. Keep the question drop for the raw bindings.
2. G2 with the L-free detector on long BABILong and full qa2, and the union of k90's and the ledger's proposals, so book words stop entering L.
