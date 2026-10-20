# 31 · Reading comprehension with a hippocampal store (2026-10-20)

*[All experiments](README.md)* · [← 28](28-width-fast-working-memory.md)

**Concepts:** [Shift-bind](../concepts/shift-bind.md) · [Writer-mask readout](../concepts/writer-mask-readout.md) · [Growth operators](../concepts/growth-operators.md#story-state)

Question: can the network answer questions about what it has read? The long-range pieces so far (story state, kept words, meaning codes, working memory) hold what a passage is about, not who did what. Answering needs relations bound to their participants and recalled by a cue: the job of the hippocampus. Experiments 29 and 30 (the Rust port of the meaning codes and error-modulated writes) are written up once their whole-corpus runs finish.

## The benchmark

`scripts/babi_gen.py` writes synthetic stories in the style of the bAbI tasks as one word stream. Four people per story move between 8 places and pick up, drop and hand over 3 of 8 objects; questions come along the way, and the answer is the word right after the question mark, so a next-word predictor answers by predicting it:

> new story . emma moved to the garden . where is emma ? garden . sandra moved to the attic . sandra went to the office . where is sandra ? office . … sandra got the football . where is the football ? office .

- **Where is the person?** The person's latest place: one supporting fact, with updates.
- **Where is the object?** Where its holder is now, or where it was dropped: two supporting facts.
- **Who has the object?** Its holder, after hand-overs.

10,000 stories (about 80 words each), 803,000 words, 38,321 questions, 42 distinct words (18 function words, 24 names, places and objects). Everything is learned in one pass, prequentially: each answer is predicted before anything learns from it. Answers are scored over the second half (19,068 questions: 13,684 about a person's place, 2,712 about an object's place, 2,672 about its holder, so "all" is mostly the first kind) by accuracy (the top prediction is the answer) and bits paid for the answer (`scripts/babi_score.py`).

## The hippocampal store

`src/episodic.rs`, in the word level through `wordslots --epi`:
- **Bind, fast and one-shot:** at each clause boundary ("." or "?"), every pair of content words in the clause is bound, both ways, stamped with the time. A new binding of the same pair overwrites the old stamp.
- **Recall by cue:** before each word, the content words of the current and the previous clause cue the store. Each partner scores 2^(−age / half-life) for its binding, summed over cue words (cue words excluded). A second hop follows the recalled partners one step further (apple → john → office). Each hop's 8 strongest partners, normalised, form a copy channel.
- **A context-dependent gain (thalamus-like):** each copy channel adds gain × its scores to the word level's logits. The gain is a base value plus a learned term for the previous input cell, both learned by the same local rule as edge gains (the channel's score for the true word minus its expected score, RMS-normalised). It can turn recall up where answers are due.

The store is told what a content word is (the 18 function words have the lowest ids) and where clauses end; it is not told any roles, such as which word is the place or the holder.

## Results

| | Where is the person? | Where is the object? | Who has it? | All | Bits per answer |
|---|---|---|---|---|---|
| Chance among the candidates (8 places; the story's 4 people) | 12.5% | 12.5% | 25% | 14.3% | 2.86 |
| Order-2 counts | 12.2% | 10.3% | 2.2% | 10.6% | 3.29 |
| Engine, without the store | 48.8% | 13.4% | 12.6% | 38.7% | 2.73 |
| **Engine + store, half-life 20 words** | **99.5%** | **92.4%** | **98.8%** | **98.4%** | **0.14** |
| *Hand-written tracking rule (reference)* | *100%* | *100%* | *100%* | *100%* | |

All engine runs: `wordslots --data runs/exp30b/babi.ids --base 64 --symbolic --words --run-window 200000 --conjoin-words --long-trials 5 --readout3 --screen-mix --fresh-extensions --story 16 --story-write1 --story-skip 18 --err-write global --err-forget 2`, plus for the store `--epi --epi-skip 18 --epi-bounds 2,17 --epi-k 8 --epi-hops 2 --epi-half 20`. The hand-written rule tracks each person's place and each object's holder exactly; it shows the stream holds every answer. The headline half-life (20 words) is the best of five tried, chosen on the scored half, and the shortest tried.

What each part contributes (engine + store; the first four rows at a 200-word half-life, so compare them with 89.0%):

| Variant | Person | Object | Who has | All | Bits |
|---|---|---|---|---|---|
| Two hops, half-life 200 words | 90.4% | 73.6% | 98.0% | 89.0% | 0.57 |
| One hop only | 96.1% | 11.0% | 98.6% | 84.3% | 0.95 |
| No context-dependent gain | 81.2% | 14.0% | 95.8% | 73.7% | 2.00 |
| Without error-modulated writes | 93.4% | 70.9% | 97.8% | 90.8% | 0.65 |
| Half-life 1,000 / 100 / 50 / **20** words | | | | 79.3% / 96.4% / 97.7% / **98.4%** | 1.42 / 0.36 / 0.23 / **0.14** |

- **The second hop is what answers "where is the object?"** (73.6% against 11.0%): the object is bound to its holder, the holder to a place. For "where is the person?" the second hop only adds noise.
- **The context-dependent gain is essential.** Without it, accuracy falls from 89.0% to 73.7% and answers cost 2.00 bits instead of 0.57. With it, the gain learned the switch by itself: in every run its largest term is on the question mark (+10 to +43, against a base of +2 to +9), so recall is turned up there, though never off elsewhere.
- **Shorter-lived bindings worked better** across the range tried (1,000 down to 20 words), and accuracy was still rising at 20. Two readings, not separated: old stories' bindings fade before they interfere, and a short half-life makes the newest partner stand out more after normalisation.
- **Error-modulated writes make little difference here:** without them accuracy is a little higher (90.8% against 89.0%, at a 200-word half-life) and bits a little higher (0.65 against 0.57).

What still fails: about 8% of "where is the object?" answers. A likely cause, not checked case by case: when an object is dropped and its holder then moves, the second hop follows the holder to the new place. That needs the binding to know the object was let go: a role, which the store doesn't have.

## Findings

1. **A fast one-shot store of clause-level bindings, recalled by cue, lets the network answer questions** about templated stories it has read: 98.4% on three bAbI-style question types, against 38.7% without it and 10.6% for order-2 counts.
2. **Recall needs a gain that depends on context:** the network learned to open the store's output at the question mark, a thalamus-like gate, with the same local learning rule as everything else.
3. **Chaining recall (two hops) solves two-fact questions,** and **forgetting fast** keeps old episodes from interfering.
4. **The remaining errors need roles:** the store binds who and what was in a clause, not in what relation.

Caveats: one run per configuration, with no seed variation, on a synthetic, templated task with a 42-word vocabulary; the store is given the content-word split and the clause boundaries.

## What next

- **Roles:** bind role ⊛ filler (shift-bind) so "dropped" and "gave … to" are told apart from "picked up", first with roles given by word position, then learned.
- **Real text:** the same store on Austen, measured on the bits spent on names and places, where knowing who is with whom should pay.

## Code

`src/episodic.rs` (the store); `brain.rs`: copy channels (`Brain::set_copy`, `copy_gains`); `main.rs`: `wordslots --epi --epi-skip N --epi-bounds ids --epi-half H --epi-k K --epi-hops 1|2`, `--copy-lr`, `--copy-nocontext`, `--dump-bits PREFIX`; `scripts/babi_gen.py`, `scripts/babi_score.py`. Runs: `runs/exp30b/`.
