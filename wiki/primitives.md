# Primitives and channels

Everything the network grows is one of **seven primitives**, joined by **six typed channels** that obey a few invariants. Delay taps and event-gated holds are compositions, not extra primitives, and the give-up rule is a policy of Select.

## Primitives as grown nodes

| Primitive | Grown as | State | Learns from | Grown when | Analogue |
|---|---|---|---|---|---|
| Expand | Bind node: one cell per seen combination of its parts | The pair, as one cell | Nothing (recruits on novelty) | A pair predicts the target beyond its best part (synergy), or conjoin fires | Dentate gyrus, layer 4, granule cells |
| Associate | Matcher pool plus Prime edges | Winner cell | Hebbian writes toward what followed | Input pools exist from the start; edges grow from screens | Layer 2/3, CA3, basolateral amygdala |
| Compress | Distill node: a kept bind's cells grouped by what they predict | Cluster id | k-means on predicted-successor profiles (toy worlds: the edge's argmax guess) | A bind edge is kept with a large gain | IT, hubs, CA1 |
| Gate | Event-gated hold; delay tap | Latched or delayed state | Nothing | A held or delayed value carries information the target lacks | Thalamus, PFC working-memory loop |
| Select | Go/NoGo tables over context cells, with a critic | Chosen action | Reward prediction error | The area has an action output; context grown by a reward-action screen | Striatum, central amygdala |
| Correct | Granule layer over every k-subset of sources, error-driven readout per target | Joint value per subset | The readout's own errors | Screens find nothing for 3 checks on an unsolved target | Cerebellum |
| Broadcast | Phasic alarm per target; attention score per input | One number | Nothing | Fast accuracy falls well below slow accuracy | LC, VTA |

Concept pages for each primitive:
- Expand: [shift-bind](concepts/shift-bind.md), [bind and conjoin](concepts/growth-operators.md#bind).
- Associate: [matcher pools](concepts/matcher-pools.md), [writer-mask readout](concepts/writer-mask-readout.md).
- Compress: [distill](concepts/growth-operators.md#distill), [stepping stones](concepts/stepping-stones.md).
- Gate: [time: delays and holds](concepts/time-delays-and-holds.md), [drive and prime](concepts/drive-and-prime.md).
- Select: [Go/NoGo selection](concepts/go-nogo-selection.md), [attention](concepts/attention.md).
- Correct: [granule layer](concepts/growth-operators.md#granule-layer).
- Broadcast: [broadcast](concepts/broadcast.md).

## Moves built from primitives

| Move | Made of | Why it is separate in the grammar |
|---|---|---|
| Delay tap (X, d) | Associate reading an older history slot | Screened virtually from stored history, so it is cheap to propose |
| Hold (X \| Y = v) | Gate plus a self-sustaining loop | Same; the event candidates are values seen on 0.5–20% of ticks |
| Conjoin | Expand of a source with the target's own state | Triggered by a screen/trial mismatch, not by synergy |
| [Chunk](concepts/growth-operators.md#chunk) | A recurrent bind, reset by a surprise gate | Its state is "everything since the last boundary"; it was the most valuable edge on text ([12 · words](experiments/12-words.md)) |
| Give-up | Select's valuation of an input | A policy, not a node |

## Channels

The Drive/Prime split is explained in [drive and prime](concepts/drive-and-prime.md); fixed sparsity in [sparse codes](concepts/sparse-codes.md).


| Channel | Carries | Contract |
|---|---|---|
| Drive | A code of fixed sparsity (32 of 2048 bits) | The only channel that can create a winner while sensing |
| Prime | Writer-mask votes into a target pool | Adds evidence to existing candidates; becomes drive only when a gate switches the area to free-run (imagine, replay) |
| Teach | A target code | Only into Correct readouts |
| Gate control | An event condition or an AND mask | Opens, latches or routes another channel |
| Scalar broadcast | One number | Sets parameters, never content |
| Bus | A code written by consensus | Read by every area as Prime |

## Invariants

1. **Impedance matching:** every node outputs a fixed-sparsity code (one winner, 32 of 2048 bits), so any output plugs into any input of the right type.
2. Every cycle contains a delayed edge, so there are no same-tick loops.
3. Binds read winner codes, never raw input: bind after cleanup ([audit sim 2](experiments/04-audit-sims.md)).
4. Content is learned only on driven ticks.
5. A global Associate is preceded by an Expand.
6. Edges are scored from source states at t − 1 and learn toward the target's state at t.

## Interfaces in code

Every node exposes the same interface:
- a current state (a cell id, or none);
- an output code per cell;
- a generation counter per cell, bumped when a cell is re-recruited, so edges reset their masks for it;
- a history of states.

Edges never look inside a node. That is what lets the controller wire any node into any target. See [growth grammar](growth-grammar.md).
