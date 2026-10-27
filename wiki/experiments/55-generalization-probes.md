# 55 · Measuring generalization: bits per word, and probes the reader never saw (2026-10-27)

*[All experiments](README.md)* · [← 54](54-learning-by-surprise.md)

**The owner's question:** how do we measure generalization? Perplexity?

**Two measures, both added here:**
- **Prequential bits per word, on every run.** Each word is coded before anything learns from it, so the code length is a fair test score for every word, not only at answers. This is perplexity done honestly for a learner that never stops learning.
- **Systematic generalization probes.** The test part of a stream is changed in one controlled way the reader never saw in training: new names, a new verb, new question wording, or longer stories. Training stays the same, and every answer stays right.

**Results (default stack, never frozen, three seeds):**
- **The reader is not a language model.**
  - On ordinary words it codes worse than a word-pair count model: small qa2 test 2.2 against 1.5 bits per word.
  - At answers it is far better: 0.19 against 2.61 bits.
  - Mixed with the counts, the mixture learns to trust the reader only while its act gate is open. The net gain over counts alone is 0.02 bits per word on small qa2 and 0.16 on the story set.
- **One change at a time is mostly absorbed:**
  - On small qa2:
    - new question wording costs nothing (99.7%, against 97.0% unchanged);
    - neither does a new verb (98.3%);
    - longer stories cost 1 point (96.0%);
    - new names cost 3.3 points (93.7%; 83% on the first 25 questions).
  - On the story set:
    - longer stories cost 0.6 points, new wording 1.3 and the new verb 1.7;
    - new names cost 7.4 points (89.8%). Answers that are names fall from 93.7% to 64.6% and object places from 90.2% to 69.1%; where-person questions are unaffected (98.3%).
- **All four changes at once break small qa2:** 75.7%, against 93.7% or better for each change alone (64% on the first 25 questions). On the story set the combination costs about what new names alone cost (91.0%).
- **Likely cause: a new word is placed by the company it keeps.**
  - The reader learns kinds (groups of words that fill the same places) from context.
  - New names alone join the old names' kind: the context "X went to the" is familiar.
  - When the names and the only movement verb are both new, there is no familiar context to anchor them. Small qa2's new names then split into two kinds of their own ("peter oscar in" and "alice nora"), one of them merged with "in", the new first word of the questions.
  - Not tested yet: whether the split kinds are the cause of the drop, rather than a sign of it.

## Prequential bits per word (binary `sit-55a`)

Every `assoc --sit` run now writes `OUT.bits` and prints a summary line. Per word, before anything learns from it:
- **A count model:** a word-pair model with the single-word model as fallback (Witten-Bell), built in the harness from the words seen so far.
- **The reader:** its candidate ranking, turned into probabilities by counting how often the word turns out to be at each rank. The counts are kept per context and are also prequential. The share for "not among the candidates" is spread over the other words in proportion to the count model. The reader's own scores are not calibrated: its top score averages 0.15 where it is right 58% of the time.
- **The mixture:** a Bayesian mixture over eight fixed weights (0 to 0.97), one mixture per context.
  - The context is the reader's own state: is its act gate open, and how sure is it (its top score, in eighths).
  - The mixture is itself a prequential code; it costs at most 3 bits in all over the best single weight in hindsight.
  - The learned weights come out the same in every run: 0 or 1/16 while not acting, 0.97 while acting.
- Self slots (the reader's own words) are not scored. `scripts/exp55/score_bits.py` reports bits by part of the stream, at answers and elsewhere.

Small qa2, seed 0, bits per word:

| | Count model | Reader | Mixture |
|---|---|---|---|
| Training, answers (n 1,000) | 2.657 | 0.896 | 0.895 |
| Training, other words | 1.517 | 2.338 | 1.517 |
| Test, answers (n 100) | 2.608 | 0.187 | 0.222 |
| Test, other words | 1.515 | 2.235 | 1.515 |

A fixed half-and-half mixture, tried first, made the code longer (2.27 bits per word against 1.53). So did one learned weight for all words: the best single weight was 0, because answers are only 1% of the words.

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
- The unchanged streams reproduce experiment 53 exactly (seed 0: small qa2 98.0%, story set 96.6%).
- Script `runs/exp55/p.sh SET PROBE SEED`; all runs `runs/exp55/all.sh`. Stack: the length-weighted judge, the act path with the hysteresis gate, relation merging, nothing frozen.

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
| **all** | **76 / 74 / 77%** | **75.7%** | **64%** | 2.61 / 1.49 |

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
- **Small qa2, all:** the new names form two kinds of their own, "peter oscar in" and "alice nora"; "wandered" stays alone.
- **Story set, names:** the new names join the old ones, except "sam".
- **Story set, all:** the new names form one kind of their own, apart from the old names.

**Where the new-name errors go (story set, seed 0):**
- Wrong who-has answers are nearly all another new name.
- Wrong where-object answers are another place.
- The errors are spread over the whole test, falling slowly. The last blocks of 25 reach 92–100%.

## What this means

- **Bits per word is the standing measure for the reader as a whole**, and it says plainly what the reader is: a question answerer, not a predictor of text. Outside its acts it adds nothing over word-pair counts. That is fair: none of its machinery is meant to predict glue words.
  - It also makes a raw-bytes reader measurable from the start: bits per byte against a count model, before there are any questions.
- **Generalization here means placing a new word by the company it keeps.** Each single change is absorbed because the rest of the sentence is familiar.
  - New names are the hardest single change. Answers about who has something depend on the name itself being the answer, and nothing in training told the reader these words are people.
  - When the company is new too (all changes at once), small qa2 drops 21 points, and its new names end up in kinds of their own.
- **This points at the next piece:** a new word should take the kind of the words that held the same places before ("X ___ to the kitchen"), right away, not after enough co-occurrence to build a kind by counting. A role, in other words, learned in one shot from position in a familiar frame.

## Next

1. One-shot kinds: a new word takes the kind of the words that held the same frame slot before. Test on the names and all probes.
2. Probes for deeper chains (objects given from person to person) on the story set, where "gave" and "handed" exist.
3. Bits per byte on raw files, as the first measure of a bytes reader.
