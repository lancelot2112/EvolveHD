# Screens

A screen is a **cheap estimate, from stored history, of how much a candidate would tell a target.** Screens rank candidates so that only promising ones get an expensive [shadow trial](shadow-trials.md). They are why inline growth scales and blind search doesn't.

## Kinds

| Screen | Measures | Used for |
|---|---|---|
| Conditional MI with a shift null | I(y[t+1]; s[t] \| y[t]) − the same with s circularly shifted | Ranking edge candidates |
| Synergy (toy) | I(y; X, Y) − I(y; X) − I(y; Y) | External pairs where neither part alone helps |
| Gain over best part (text) | I(y; X, Y) − max(I(y; X), I(y; Y)) | Binds where parts are partly redundant |
| Virtual temporal screen | Conditional MI of a delayed or held series computed from history | [Delays and holds](time-delays-and-holds.md) |
| Held-out predictive gain | Counts on the first half of the window, Witten-Bell interpolation, log-loss gain on the second half | High-cardinality contexts such as [chunks](../experiments/12-words.md) |
| [Running count screen](#running-count-screen) | Witten-Bell counts kept from the candidate's proposal on, each tick scored before it is learned, gain averaged over the last 200k ticks | Contexts with thousands of states that need more data than a window holds ([13](../experiments/13-word-pairs.md)) |
| Lagged co-activity | G-statistic of a node's states at lags 3–20 | Deciding to grow a compress node (toy) |
| Reward-action synergy | I(R; S, A) | Select's context candidates |

## Rules

- **Debias by shifting, not by trusting.** The circular shift keeps every marginal and breaks the relation, so its plug-in MI estimates the bias.
- **Respect the data.** A pair with more than W/4 distinct values in the window (or more than 2 × a real pool's capacity) is not screened: there is too little data per context.
- **The shift null fails for contexts with thousands of states,** especially ones that contain the target's own state. It came out at −0.5 bits for chunks that in fact add 0.37. Use a held-out gain there ([12](../experiments/12-words.md)).
- **After a world change, screen only post-change data** ([broadcast](broadcast.md)).

## Running count screen

A stored window gives a held-out screen only 15k ticks of counts. That is too few for a context with thousands of states: a previous-word context screened at −0.06 bits there, but +0.13 with a running screen over 400k ticks ([13](../experiments/13-word-pairs.md)).

A running screen keeps its own counts for p(y′ | s, b) and p(y′ | b), where b is the baseline: the target's state, or a kept node such as the current chunk. It scores each tick before learning from it, so it is always held out. It costs a few hash lookups per tick and no stored history. Counts are halved every million ticks, so the tables hold only recent contexts.

Choose the baseline carefully. Measured beyond the current byte, the previous word looked weak (+0.13 at 400k ticks); beyond the current chunk it was +0.21 and growing, to about 0.30 on 2M bytes. That difference is what said the information was a conjunction, and so pointed to [conjoin](growth-operators.md#conjoin).

## Why

Random candidate order found 2 of 6 structures in 50k ticks; screening found all of them by 10k ([07](../experiments/07-graph-growth.md)). Co-activity grew a compress node in 6/6 worlds with hidden structure and 0/6 without ([06](../experiments/06-growth-interfaces.md)).

## Cost

Every check compares every source with every target, and synergy compares pairs, so cost grows with the square of the node count. Running counts, local candidates and sampled pairs are the scaling plan ([open questions](../open-questions.md)). In the Rust port the screens take more than half the run time; running count screens already replace the windowed held-out screen for the word moves ([13](../experiments/13-word-pairs.md)).

## Related

[Shadow trials](shadow-trials.md) · [Growth operators](growth-operators.md) · [Attention](attention.md) (screens feed the opportunity signal) · [Capacity and budget](capacity-and-budget.md)

## Code

`ent`, `mi`, `cmi`, `screen`, `synergy` and `temporal` in `text_sim.py`; `heldout_gain` in `word_sim.py`; `info.rs` and `RunScreen` in `brain.rs` in the Rust crate.

## Evidence in

[06 · growth interfaces](../experiments/06-growth-interfaces.md) · [07 · graph growth](../experiments/07-graph-growth.md) · [09 · attention temporal](../experiments/09-attention-temporal.md) · [11 · real text](../experiments/11-real-text.md) · [12 · words](../experiments/12-words.md) · [13 · word pairs](../experiments/13-word-pairs.md)

*[All concepts](README.md) · [Wiki home](../README.md)*
