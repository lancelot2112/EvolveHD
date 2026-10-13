# Time: delays and holds

Prediction often needs the past. EvolveHD has two ways to look back, both built from existing [primitives](../primitives.md):

- **A delay tap reads a node d ticks ago:** a fixed lag.
- **An event-gated hold latches a value when a rare event fires and keeps it until the next one:** a lag set by events. This is a working-memory slot (Gate plus a self-sustaining loop).
- **A chunk** carries everything since the last surprising event: a lag set by the data's own predictability ([12](../experiments/12-words.md)).

## How it works

- **Delay tap (X, d):** state = X's state d − 1 ticks earlier. It is a view, with no cells of its own. Candidate lags are 2, 3, 4, 5, 6, 8, 12, 16.
- **Hold (X | Y = v):** whenever Y shows value v, latch X's state; otherwise keep it. Candidate events are values that Y shows on 0.5% to 20% of ticks.
- **Chunk (z):** a recurrent bind of its own last state with the byte, reset when the byte's bits exceed μ + zσ ([chunk operator](growth-operators.md#chunk)).
- **Virtual screening:** candidates are scored from stored history before anything is built. Delays and holds are cheap to propose, so the [screen](screens.md) is the filter.

In the engine layer, every read term already has its own integer delay, chosen at spawn and kept by selection ([engine model](../engine-model.md)).

## Why

- **Temporal world** (Td = value 8 ticks ago; Q = value at the last rare flag) ([09](../experiments/09-attention-temporal.md)):
  - without temporal moves, both targets stay at chance;
  - with them, both reach ceiling in 6/6 runs, and the right lag and latch were picked out of every candidate by about 10k ticks.
- **Exact against log-spaced delays:** log spacing works only for slow features; fast features need exact lags ([01](../experiments/01-early-engine.md)).
- **Text:** delay taps were the first thing grown (byte@2 at 10k ticks), and holds keyed on spaces were kept with tiny gains (about 0.01 bits) ([11](../experiments/11-real-text.md)).
- **Words:** a chunk at z = 0.5 was kept with 0.20 bits at trial and cut 0.08 bits per byte. A previous-chunk hold screened negative at every check ([12](../experiments/12-words.md)).
- **A pitfall:** a delay tap that carries information only together with the target's own state fails as a plain edge. Before the fix, such taps hogged a target's trial slot ([10](../experiments/10-full-primitives.md)). The temporal screen now scores what an additive edge can use, and leaves conjunctions to [conjoin](growth-operators.md#conjoin).

## Related

[Screens](screens.md) · [Growth operators](growth-operators.md) · [Drive and prime](drive-and-prime.md) · [Engine model](../engine-model.md)

## Code

`Delay` and `Hold` in `text_sim.py`; `ChunkReal`, `ChunkSym` and `PrevWord` in `word_sim.py`; `temporal` in `curio_sim.py` and `text_sim.py`.

## Evidence in

[01 · early engine](../experiments/01-early-engine.md) · [09 · attention temporal](../experiments/09-attention-temporal.md) · [11 · real text](../experiments/11-real-text.md) · [12 · words](../experiments/12-words.md)

*[All concepts](README.md) · [Wiki home](../README.md)*
