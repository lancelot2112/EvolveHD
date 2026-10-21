# 31 · Reading comprehension with a hippocampal store (2026-10-20)

*[All experiments](README.md)* · [← 28](28-width-fast-working-memory.md) · [32 →](32-learned-bitwise-store.md)

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

## Learned roles

The store above binds everything in a clause to everything else, with no relations. A version that learns them (`--epi-roles`):
- **Roles from context:** each content word's role is its two preceding words, with content words replaced by a wildcard. So "went / moved / journeyed to the X" all give X the role *[to the]*, while "got the X", "picked up the X" and "dropped the X" give X different roles. Nobody names the roles; they are the slots the text has.
- **One channel per relation:** bindings are kept per pair of roles (the cue's role, the partner's role), and each pair is its own recall channel; the second hop chains two pairs.
- **The question chooses the relation:** the copy gains can be keyed on the question's form, the last four words with content words wildcarded (`--epi-ctx 4`), instead of the previous word alone. Each channel's gain per form is learned by the same local rule.

Accuracy (half-life 20, gain learning rate 0.03 except the first row):

| Store | Gate on | Person | Object | Who has | All | Bits |
|---|---|---|---|---|---|---|
| Plain (above) | previous word | 99.5% | 92.4% | 98.8% | 98.4% | 0.14 |
| Plain | question form | 99.9% | 94.8% | 98.5% | 99.0% | 0.09 |
| Learned roles only | previous word | 71.5% | 62.5% | 79.1% | 71.3% | 0.98 |
| Learned roles only | question form | 76.9% | 63.7% | 78.6% | 75.3% | 0.86 |
| Plain + learned roles | previous word | 99.5% | 93.1% | 98.9% | 98.5% | 0.10 |
| **Plain + learned roles** | **question form** | **99.9%** | **94.9%** | **100.0%** | **99.2%** | **0.06** |

What the gates learned (roles only, gated on question form; *[\* .]* is a sentence's first word):

| Question form | Channel opened most | Gain |
|---|---|---|
| where is X ? | hop 1 *[\* .]* → *[to the]*: the person's place | +7.9 |
| who has the X ? | hop 1 *[got the]* → *[\* .]*: who got it | +7.6 |
| who has the X ? | hop 1 *[up the]* → *[\* .]*: who picked it up | +7.4 |
| who has the X ? | hop 1 *[gave the]* → *[\* to]*: who it was given to | +3.1 |
| where is the X ? | hop 2 *[got the]* → *[\* .]*, then *[\* .]* → *[to the]*: the getter's place | +4.8 |
| where is the X ? | hop 2, the same via *[up the]* | +4.5 |

- **The network finds the relations and which question needs which,** from next-word prediction alone: every strong gate is the right relation for its question form.
- **But learned roles alone answer less well (75%)** than the plain store (99%). The evidence is spread over dozens of channels, each learning from a fraction of the data, and roles fragment: the first sentence of a story gives its subject the role *[story .]*, separate from *[\* .]*.
- **Plain and role channels together are best** (99.2%, 0.06 bits per answer): the plain store carries the bulk and the roles sharpen it, mostly on "who has it?" (100%). The differences between the top rows are within what one run can resolve.

## Findings

1. **A fast one-shot store of clause-level bindings, recalled by cue, lets the network answer questions** about templated stories it has read: 98.4% on three bAbI-style question types, against 38.7% without it and 10.6% for order-2 counts.
2. **Recall needs a gain that depends on context:** the network learned to open the store's output at the question mark, a thalamus-like gate, with the same local learning rule as everything else.
3. **Chaining recall (two hops) solves two-fact questions,** and **forgetting fast** keeps old episodes from interfering.
4. **Relations can be learned:** with roles taken from each word's wildcarded left context and the gate keyed on the question's form, the network learned which relation answers which question (where is X → the person's *to the* partner; who has the X → the getter or recipient; where is the X → the getter's place). Alone the learned roles answer 75%; with the plain store, 99.2%.

Caveats: one run per configuration, with no seed variation, on a synthetic, templated task with a 42-word vocabulary; the store is given the content-word split and the clause boundaries.

## What next

Followed up in [experiment 32](32-learned-bitwise-store.md): a store with nothing hand-coded, on held-out phrasings.

- **Roles shared through meaning:** define roles by meaning-code classes rather than exact words, so synonyms ("walked into the") share a role, and test on phrasings held out of training.
- **Real text:** the same store on Austen, measured on the bits spent on names and places, where knowing who is with whom should pay.

## Code

`src/episodic.rs` (the store; `--epi-roles`, `--epi-both`, `--epi-ctx N`); `brain.rs`: copy channels (`Brain::set_copy`, `copy_gains`); `main.rs`: `wordslots --epi --epi-skip N --epi-bounds ids --epi-half H --epi-k K --epi-hops 1|2`, `--copy-lr`, `--copy-nocontext`, `--dump-bits PREFIX`; `scripts/babi_gen.py`, `scripts/babi_score.py`. Runs: `runs/exp30b/`.
