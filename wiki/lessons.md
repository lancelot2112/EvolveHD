# Lessons

These are the design rules the experiments forced, grouped by layer. Each links to the experiment that showed it.

## Learning rules

*Concepts: [credit without backprop](concepts/credit-without-backprop.md) · [sleep and consolidation](concepts/sleep-and-consolidation.md) · [Go/NoGo selection](concepts/go-nogo-selection.md)*

- **Judge a matcher by its track record, not its last miss.** Kill-on-miss made transfer unreliable and churned the pool; hit/miss counters fixed both ([01](experiments/01-early-engine.md)).
- **Unstick faster than you stick, and leak.** Without the leak, a lucky hit froze over-specific matchers ([01](experiments/01-early-engine.md)).
- **Anti-Hebbian only with consolidation.** On every inhibited firer it destroys siblings. With sticky bits it strips only flaky bits ([03](experiments/03-grow-sleep.md)).
- **Prune on "never consolidated", not "not used lately".** Pruning idle cells is catastrophic forgetting ([03](experiments/03-grow-sleep.md)).
- **A critic needs an away move.** Masks that only ever move toward the context leave V near zero and reward wrong choices ([04](experiments/04-audit-sims.md)).
- **Reversal needs Go depression on negative δ** ([02](experiments/02-bg-selection-bandit.md), [04](experiments/04-audit-sims.md)).

## Representation

*Concepts: [shift-bind](concepts/shift-bind.md) · [drive and prime](concepts/drive-and-prime.md) · [writer-mask readout](concepts/writer-mask-readout.md) · [matcher pools](concepts/matcher-pools.md)*

- **Bind with shift-bind, after cleanup.** XOR leaks half of each part into the pair code ([04](experiments/04-audit-sims.md)).
- **Context primes; only input drives.** A plan wired as an equal driver destroys perception ([04](experiments/04-audit-sims.md)).
- **Readouts must be graded for probabilistic data.** One binary writer mask encodes a set, not a distribution; text needed stochastic multi-level masks ([11](experiments/11-real-text.md)).
- **Give writer masks room.** A 640-bit level written 8 bits at a time loses rare successors to frequent ones; 1,024 bits and 16-bit writes recovered 0.08 bits per byte on hand-wired order 4 (0.14 for the full readout v2 on grown exact nodes), with no extra memory ([16](experiments/16-readout-and-generation.md)).
- **A readout's capacity sets how high the stack can go.** Its error grows with outcomes per row (0.06 bits at 16, 0.44 at 512): small for bytes, as large as all the structure for word-pieces ([17](experiments/17-level-stack.md)).
- **Count with every bit.** A stochastic counter that flips one coin per write puts each successor at a random height; one coin per code bit makes the count smooth. That, not capacity, was most of the readout's error ([18](experiments/18-sparser-codes.md)).
- **Make codes sparser only where rows fill up.** Fewer readout bits per symbol help rows that must hold many successors (a word-piece level) and make no difference, or cost a little, at bytes ([18](experiments/18-sparser-codes.md)).
- **Under noise, merge duplicate cells.** Rare low-overlap inputs and Hebbian drift split each symbol across cells ([11](experiments/11-real-text.md)).

## Growth

*Concepts: [shadow trials](concepts/shadow-trials.md) · [screens](concepts/screens.md) · [growth operators](concepts/growth-operators.md) · [capacity and budget](concepts/capacity-and-budget.md)*

- **Trials must not vote.** A voting trial adds noise that masks kept edges and causes false prunes. Shadow trials fixed thrash ([06](experiments/06-growth-interfaces.md)).
- **Keep and prune at different bars.** Hysteresis stops thrashing ([06](experiments/06-growth-interfaces.md)).
- **Single edges can't see pairs.** Synergy finds external pairs; conjoin finds context × self ([07](experiments/07-graph-growth.md)).
- **Screen before you trial.** Random candidate order found 2 of 6 structures in the same time screening found 6 ([07](experiments/07-graph-growth.md)).
- **Priority buys one trial, not a permanent slot.** Permanent priority let delay taps hog a target's slot until the right bind was garbage-collected ([10](experiments/10-full-primitives.md)).
- **Compress keeps depth affordable.** Without distill, representations grow multiplicatively and stall after two levels ([08](experiments/08-bootstrapping.md)).
- **Growth adapts to capacity.** With real 1024-cell pools it built factored skip-pair contexts and beat a hand-wired chain on the same pools ([11](experiments/11-real-text.md)).
- **Screen big contexts on held-out data.** A shift-null MI screen scored chunks at −0.5 bits when they add 0.37; held-out log-loss gain is honest ([12](experiments/12-words.md)).
- **Measure big contexts against the right baseline, with enough data.** The previous word looked worthless in a 15k-tick window and weak beyond the current byte, but beyond the current chunk it carried 0.21 bits and kept growing ([13](experiments/13-word-pairs.md)).
- **Give slow learners a long enough trial.** Writer masks need many writes per state; a 31,000-state bind showed at most 0.0045 bits in normal trials and was kept with an extension ([13](experiments/13-word-pairs.md)).
- **Let capacity follow pressure, and start at the size the evidence says.** Pools that double when they evict recently used cells reached exact-node quality; a bind that had to grow from 1,024 cells while on trial lost its trial ([15](experiments/15-growing-pools.md)).
- **Inline beats blind population search for wiring.** Use populations, if at all, for the grammar's own thresholds ([07](experiments/07-graph-growth.md)).

## Attention and time

*Concepts: [stepping stones](concepts/stepping-stones.md) · [attention](concepts/attention.md) · [broadcast](concepts/broadcast.md) · [time: delays and holds](concepts/time-delays-and-holds.md)*

- **The world must leave stepping stones.** Structure with no partial signal stays invisible to every screen. Only the brute-force fallback (Correct) can reach it, at a cost that explodes with order ([08](experiments/08-bootstrapping.md), [10](experiments/10-full-primitives.md)).
- **Never chase raw surprise.** It sends the budget to the noise source ([09](experiments/09-attention-temporal.md)).
- **Reward attention with the grammar's own signals:** significant progress, structure grown, and uncaptured screen information ([09](experiments/09-attention-temporal.md), [10](experiments/10-full-primitives.md)).
- **A surprise alarm must restart what the screens look at.** Faster learning and early re-checks alone did little ([10](experiments/10-full-primitives.md)).
- **Surprise is a good place to cut, not a word detector.** A context reset at surprising bytes paid more than any other edge, but its units are runs the model can follow, not words ([12](experiments/12-words.md)).
- **Compressing by what comes next keeps the wrong variable.** Successor clusters failed for byte contexts and again for words; the word itself, joined with the next one, was what paid ([11](experiments/11-real-text.md), [13](experiments/13-word-pairs.md)).
- **Global consensus needs fatigue,** or it locks ([05](experiments/05-claustrum-consensus.md)).
- **Hopeless and slow can't be told apart cheaply.** Ruling out brute force costs about 35k ticks per input ([10](experiments/10-full-primitives.md)).
