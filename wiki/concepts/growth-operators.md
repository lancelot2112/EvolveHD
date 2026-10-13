# Growth operators

Each operator adds one kind of node in response to one measurable pressure. A node is only ever a candidate: its edge still has to pass a [shadow trial](shadow-trials.md).

| Operator | Primitive | Pressure | Adds |
|---|---|---|---|
| [Edge](#edge) | Associate | A source screens above 0.02 bits | A Prime edge from an existing node |
| [Bind](#bind) | Expand | A pair carries information beyond its best part | A bind node over the pair |
| [Conjoin](#conjoin) | Expand | A source screened well but its edge failed | bind(source, target's own state) |
| [Distill](#distill) | Compress | A bind edge is kept with a large gain | A cluster variable over the bind's cells |
| [Delay / hold](#delay-and-hold) | Associate / Gate | A delayed or event-held value screens well | A view of the past |
| [Chunk](#chunk) | Expand + Gate | A surprise-reset run screens well (held-out gain) | A recurrent bind, reset at surprising bytes |
| [Granule layer](#granule-layer) | Correct | Screens dry for 3 checks on an unsolved target | A brute-force expansion and readout |
| [Cleanup](#cleanup) | – | A node unused, tried, older than 15k ticks | Removal |

## Edge

The basic move. Candidates are type-matched: any node into any target. They are ranked by [screens](screens.md).

## Bind

Synergy or gain-over-best-part over pairs anchored on the target and its kept sources. The pair must have enough data per context. Binds of binds give depth ([08](../experiments/08-bootstrapping.md)). External pairs such as A × B were found at 5k ticks, when single edges never could ([07](../experiments/07-graph-growth.md)).

## Conjoin

When the screen sees information but an additive edge can't use it, the information is conditional on the target's own state. Conjoin binds the source with the target. This is how a context × self conjunction (T4 = perm_L(T4)) was solved ([07](../experiments/07-graph-growth.md)). It falls out of a rule and resembles HTM's context cells.

The partner need not be the target. The previous word told little beyond the current byte, but 0.21 bits beyond the current [chunk](#chunk). So conjoin bound it with the chunk: one cell per (previous word, current word so far). With [trial extensions](shadow-trials.md#extensions-for-many-state-sources) it was kept in every exact run, cutting 0.05–0.09 bits per byte. That is the first word-level structure growth has kept ([13](../experiments/13-word-pairs.md)). The trigger is a [running screen](screens.md#running-count-screen) measured beyond the partner.

## Distill

Compress a kept bind into a small variable:
- **toy worlds:** the edge's argmax guess (8 states);
- **text:** 32 k-means clusters of the bind's cells by predicted successor profile.

Without distill, level 3 of the staged world stalled, because its bind would have had 4,096 cells; with distill it was solved in 6/6 runs ([08](../experiments/08-bootstrapping.md)). **On text, distill never paid** ([11](../experiments/11-real-text.md)). Grouping completed chunks into 64 classes by what follows them (the next chunk's first two bytes) failed the same way: class edges gained at most 0.009 bits, and the classes added nothing beyond the current chunk. What a later level needs from a word was the word itself, in conjunction with the next one ([13](../experiments/13-word-pairs.md)).

## Delay and hold

See [time: delays and holds](time-delays-and-holds.md).

## Chunk

A recurrent bind, chunk_t = bind(chunk_{t−1}, x_t), reset whenever the target's surprise rises above its running mean plus z standard deviations. Its state is "the run since the last surprise". Several z are screened from stored history with a held-out gain, and the best is built.

On text it was the most valuable single edge: 0.20 bits at trial, and −0.08 bits per byte overall with real pools. Its cuts are morph-like rather than whole words ([12](../experiments/12-words.md)). It is Expand (the recurrent bind) gated by [Broadcast](broadcast.md) (the surprise event); see [time](time-delays-and-holds.md).

## Granule layer

The cerebellar fallback for structure with no stepping stones:
- a shared expansion with one cell per joint value of every k-subset of sources;
- an error-driven readout per unexplained target.

A 3-input puzzle went from chance to 0.72 with 10k cells. A 4-input puzzle needed 490k cells and reached only 0.27 in 160k ticks ([10](../experiments/10-full-primitives.md)). Use it last; see [stepping stones](stepping-stones.md).

## Cleanup

Every 10k ticks, remove derived nodes that nothing uses, that have been tried at least once, and that are older than 15k ticks. Never remove a part of a live node. This is the structural form of [sleep](sleep-and-consolidation.md) and keeps the structure budget recycling ([capacity and budget](capacity-and-budget.md)).

## Related

[Growth grammar](../growth-grammar.md) · [Screens](screens.md) · [Shadow trials](shadow-trials.md) · [Shift-bind](shift-bind.md) · [Primitives](../primitives.md)

## Evidence in

[06 · growth interfaces](../experiments/06-growth-interfaces.md) · [07 · graph growth](../experiments/07-graph-growth.md) · [08 · bootstrapping](../experiments/08-bootstrapping.md) · [10 · full primitives](../experiments/10-full-primitives.md) · [11 · real text](../experiments/11-real-text.md) · [12 · words](../experiments/12-words.md) · [13 · word pairs](../experiments/13-word-pairs.md)

*[All concepts](README.md) · [Wiki home](../README.md)*
