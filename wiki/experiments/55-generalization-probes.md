# 55 · Measuring generalization: bits per word, and probes the reader never saw (2026-10-27)

*[All experiments](README.md)* · [← 54](54-learning-by-surprise.md)

**The owner's question:** how do we measure generalization? Perplexity?

**Two measures, both added here:**
- **Prequential bits per word, on every run.** Each word is coded before anything learns from it, so the code length is a fair test score for every word, not only at answers. This is perplexity done honestly for a learner that never stops learning.
- **Systematic generalization probes.** The test part of a stream is changed in one controlled way the reader never saw in training: new names, a new verb, new question wording, or longer stories. Training stays the same, and every answer stays right.

**Results (default stack, never frozen, three seeds):**
- **The reader is not a language model.**
  - On ordinary words it codes worse than a word-pair count model: small qa2 test 2.2 against 1.5 bits per word.
  - At answers it is far better: 0.25 against 2.61 bits (three-seed mean; mixture 0.29).
  - Mixed with the counts, the mixture learns to trust the reader almost fully while its act gate is open, and little or not at all otherwise.
  - The net gain over counts alone, on the test words: 0.02 bits per word on small qa2 and 0.16 on the story set. Over the whole stream: 0.017 and 0.12.
- **One change at a time is mostly absorbed:**
  - On small qa2:
    - new question wording costs nothing (99.7%, against 97.0% unchanged);
    - neither does a new verb (98.3%);
    - longer stories cost 1 point (96.0%);
    - new names cost 3.3 points (93.7%; 83% on the first 25 questions).
  - On the story set:
    - longer stories cost 0.6 points, new wording 1.3 and the new verb 1.7;
    - new names cost 7.4 points (89.8%). Answers that are names fall from 93.7% to 64.6% and object places from 90.2% to 69.1%; where-person questions barely move (99.1% → 98.3%).
- **All four changes at once break small qa2:** 75.7%, against 93.7% or better for each change alone (64% on the first 25 questions). On the story set the combination costs about what new names alone cost (91.0%).
- **The break on small qa2 is new names together with the new verb.** Pairs of changes, three seeds:
  - names and verb: 70.3%;
  - names and question wording: 89.7%; names and longer stories: 90.7%;
  - verb and question wording: 97.0%.
  - In bAbI the movement verb is most of a person's context, so with both new, nothing familiar surrounds the new names.
  - Then the new names don't join the old names' kind (the groups of words that fill the same places). With names and verb new they end up as "where nora oscar" and "alice peter"; with names alone they join mary, daniel, sandra and john.
- **A familiar kind is not enough, though.** On the story set the new names alone do join the old names' kind in every seed, and who-has answers still fall 29 points. In the combined probe there they form a kind of their own, and the score is no worse (91.0% against 89.8%). So kinds are part of it, and something else costs the story set: likely routes and memories keyed by the names' exact ids, learned in training for the old names. Not tested yet.

## Prequential bits per word (binary `sit-55a`)

Every `assoc --sit` run now writes `OUT.bits` and prints a summary line (not with `--sit-shards`, which skips the per-word loop; words read under `--sit-answer-only` without a prediction are coded by the counts alone and left out of the summary). Per word, before anything learns from it:
- **A count model:** a word-pair model with the single-word model as fallback (Witten-Bell), built in the harness from the words seen so far.
- **The reader:** its candidate ranking (top 16), turned into probabilities by counting how often the word turns out to be at each rank. The counts are kept per context and are also prequential. The share for "not among the candidates" is spread over the other words in proportion to the count model. The reader's own scores are not calibrated: on small qa2 (seed 0) its top score averages 0.15 where it is right 58% of the time; on the story set 0.19 and 42%.
- **The mixture:** a Bayesian mixture over eight fixed weights (0 to 0.97), one mixture per context.
  - The context is the reader's own state: is its act gate open, and how sure is it (its top score, in eighths).
  - The mixture is itself a prequential code. Each context's mixture costs at most 3 bits over the best weight for that context in hindsight; at most 48 bits in all over the best weight per context.
  - **The learned weights:** acting, 0.97 in every context with more than two words. Not acting, mostly 0 to 1/16; on the story set up to 1/4 or 1/2 in some middle-confidence contexts.
- Self slots (the reader's own words) are not scored. `scripts/exp55/score_bits.py` reports bits by part of the stream, at answers and elsewhere.

Small qa2, seed 0, bits per word:

| | Count model | Reader | Mixture |
|---|---|---|---|
| Training, answers (n 1,000) | 2.657 | 0.896 | 0.895 |
| Training, other words | 1.517 | 2.338 | 1.517 |
| Test, answers (n 100) | 2.608 | 0.187 | 0.222 |
| Test, other words | 1.515 | 2.235 | 1.515 |

A fixed half-and-half mixture, tried first, made the code longer (2.27 bits per word against 1.53, on a check run whose output was overwritten). So did one learned weight for all words: the best single weight was 0, because answers are only 1% of the words.

## The probes (`scripts/exp55/probe_stream.py`)

Each probe changes every story from the first whole story at or after the test split. Small qa2: from word 102,303, 100 questions. Story set: from word 12,443, 590 questions.

| Probe | What changes |
|---|---|
| names | every person gets a new name never seen in training (one fixed mapping: alice, peter, nora, oscar, ...) |
| verb | every movement verb (went, moved, walked, travelled, journeyed) becomes "wandered" |
| question | "where is X ?" becomes "in which room is X ?"; "who has the X ?" becomes "who is holding the X ?" |
| longer | each story gets as many sentences again: moves by people whose moves can't change any answer (story set: people absent from the story; bAbI: people who never touch the object asked about) |
| all | the four together |

- `scripts/exp55/check_answers.py` simulates every story (places, holders, dropped objects). It agrees with all 1,100 and 1,197 answers in the original streams and in every probe stream.
- The unchanged streams reproduce experiment 53 exactly when scored the same way (story set seeds 0–2: 96.6 / 97.3 / 97.5% on its 596 test questions). The tables below count only the 590 probed questions, so story set seed 1 shows 97.5% there.
- Script `runs/exp55/p.sh SET PROBE SEED`; all single probes `runs/exp55/all.sh`; the pairs were built with the same script (`probe_stream.py ... names,verb`). Stack: the length-weighted judge, the act path with the hysteresis gate, relation merging, nothing frozen.

## Results

Top-1 over the probed questions; "first 25" is the first 25 probed questions (mean of three seeds). Bits are per answer: count model / mixture.

**Small qa2** (100 questions):

| Probe | Seeds 0 / 1 / 2 | Mean | First 25 | Bits at answers |
|---|---|---|---|---|
| unchanged | 98 / 95 / 98% | 97.0% | 99% | 2.61 / 0.29 |
| names | 94 / 93 / 94% | 93.7% | 83% | 2.61 / 0.50 |
| verb | 97 / 100 / 98% | 98.3% | 97% | 2.61 / 0.20 |
| question | 100 / 100 / 99% | 99.7% | 100% | 2.61 / 0.19 |
| longer | 96 / 95 / 97% | 96.0% | 96% | 2.61 / 0.34 |
| **all** | **76 / 74 / 77%** | **75.7%** | **64%** | 2.61 / 1.50 |
| names + verb | 73 / 74 / 64% | 70.3% | 77% | |
| names + question | 92 / 89 / 88% | 89.7% | 76% | |
| names + longer | 92 / 93 / 87% | 90.7% | 73% | |
| verb + question | 95 / 97 / 99% | 97.0% | 97% | |

**Story set** (590 questions):

| Probe | Seeds 0 / 1 / 2 | Mean | First 25 | Where-person | Where-object | Who-has | Bits at answers |
|---|---|---|---|---|---|---|---|
| unchanged | 96.6 / 97.5 / 97.5% | 97.2% | 83% | 99.1% | 90.2% | 93.7% | 3.60 / 0.33 |
| names | 89.7 / 90.5 / 89.2% | 89.8% | 75% | 98.3% | 69.1% | 64.6% | 3.93 / 0.87 |
| verb | 95.4 / 94.7 / 96.3% | 95.5% | 69% | 98.6% | 88.5% | 85.6% | 3.60 / 0.46 |
| question | 94.9 / 96.4 / 96.3% | 95.9% | 72% | 99.2% | 85.2% | 88.6% | 3.60 / 0.42 |
| longer | 96.4 / 97.1 / 96.4% | 96.6% | 80% | 98.8% | 87.7% | 94.1% | 3.60 / 0.38 |
| all | 91.2 / 91.9 / 89.8% | 91.0% | 72% | 99.0% | 68.7% | 70.0% | 3.91 / 0.81 |

**The kinds learned by the end** (the same in all three seeds):
- **Small qa2, names:** the new names join the old ones (mary daniel sandra john alice peter nora oscar).
- **Small qa2, verb:** "wandered" joins the verbs.
- **Small qa2, names + verb:** the new names form "where nora oscar" and "alice peter".
- **Small qa2, all:** the new names form two kinds of their own, "peter oscar in" and "alice nora"; "wandered" stays alone.
- **Story set, names:** the new names join the old ones, except "sam".
- **Story set, all:** the new names form one kind of their own, apart from the old names.

**Where the new-name errors go (story set, seed 0):**
- Wrong who-has answers: 25 of 31 are another new name (3 old names, 2 places, 1 "grabbed").
- Wrong where-object answers: 20 of 22 are another place.
- The errors are spread over the whole test and don't fall steadily: late blocks of 25 swing between 72% and 100% (block 23 is 72% in seeds 0 and 2).

## What this means

- **Bits per word is the standing measure for the reader as a whole**, and it says plainly what the reader is: a question answerer, not a predictor of text. Outside its acts it adds nothing over word-pair counts. That is fair: none of its machinery is meant to predict glue words.
  - It also makes a raw-bytes reader measurable from the start: bits per byte against a count model, before there are any questions.
- **Generalization here starts with placing a new word by the company it keeps.**
  - Each single change is absorbed best where the rest of the sentence is familiar.
  - On small qa2, new names with a new verb lose 27 points against the unchanged stream, and the new names fail to join the person kind.
  - New names are the hardest single change. On the story set they cost even when they join the person kind: who has something depends on the name itself being the answer, and the reader's routes were tuned on the old names.
- **This points at two next pieces:**
  - **One-shot kinds:** a new word should take the kind of the words that held the same places before ("X ___ to the kitchen") right away, not after enough co-occurrence to build a kind by counting. A role, learned in one shot from position in a familiar frame.
  - **Routes by kind, not by word:** the route learner's features should name the kind of an answer's source, so that what it learned about "mary" carries over to "alice".

## Next

1. One-shot kinds: a new word takes the kind of the words that held the same frame slot before. Test on the names, names-and-verb and all probes.
2. Find what costs the story set's new names when they do join the kind (route features by exact id, long-term heads keyed by the old names).
3. Probes for deeper chains (objects given from person to person) on the story set, where "gave" and "handed" exist.
4. Bits per byte on raw files, as the first measure of a bytes reader.
