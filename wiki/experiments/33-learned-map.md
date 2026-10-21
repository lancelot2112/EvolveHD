# 33 · A learned map: place codes and direction codes (2026-10-21)

*[All experiments](README.md)* · [← 32](32-learned-bitwise-store.md)

**Concepts:** [Shift-bind](../concepts/shift-bind.md)

Question: can the network build a map from relations it reads, keeping best guesses that improve with each fact? This is the job of place cells (where an item is) and direction cells (which way is which) in the hippocampus and entorhinal cortex. Phase codes fit it directly: a move is adding a phase step, going back is subtracting it.

## The benchmark

`scripts/spatial_gen.py`: each story places 9 rooms (of 12 names) on a hidden 3 × 3 grid at random. Adjacencies are stated one at a time, either way round ("the kitchen is north of the garden ." or "the garden is south of the kitchen ."), in random order, 7 to 12 per story. Questions come along the way: "what is north of the garden ?".

2,000 stories, 293,502 words, 15,102 questions. Each question is tagged:
- **Stated** (5,966): the fact was given, in either direction.
- **Implied only** (1,008): never stated, but the facts so far chain through other rooms to it.
- **Not determined yet** (8,128): no chain of facts connects the two rooms yet.

## The map

`scripts/spatial_map.py`, integer phases throughout (a prototype, not yet in the engine):
- **Place codes:** each room's position in the current story is a phase vector, 256 elements of 16 phases each.
- **Direction codes:** each direction word has a step code, a phase vector. "X is north of Y" means p_X = p_Y + s_north. **Nothing about the words is given:** the step codes start random.
- **Best guesses within a story:**
  - The reference room of the first fact (Y in "X is r of Y") is pinned at a fresh random code.
  - A fact whose two rooms both have no place yet starts a weak anchor of its own. The anchor is dropped as soon as a fact links its rooms to the pinned room's.
  - Every fact adds evidence for both its rooms (p_Y + s_r for X, p_X − s_r for Y).
  - After each fact, 12 passes recompute every room's place as the phase of its summed evidence: phasor sums of integer cos and sin tables, read back as the table phase they match best. So one new fact can re-place a whole group of rooms and line it up with the rest.
- **Answering:** for "what is r of Y ?", pick the room of Y's group whose code best matches p_Y + s_r. If none matches clearly (overlap below a fifth of a perfect match), guess one of the rooms *not* linked to Y yet, preferring rooms already mentioned in the story (the pick among them is a fixed rotation, not chance). The fewer such rooms remain, the better the guess.
- **Learning the directions, across stories:** at each story's end, each fact "X is r of Y" whose rooms the other facts still connect is measured on the map rebuilt without it (leave one out). The difference p_X − p_Y there is added to s_r's accumulator, and s_r is re-read from it.

## Results

| Stories | Stated | Implied only | Not determined yet |
|---|---|---|---|
| 1–50 | 98% | 100% (25 questions) | 11% |
| 51–200 | 100% | 100% | 13% |
| 201–1,000 | 100% | 100% | 14% |
| 1,001–2,000 | 100% | 100% | 13% |

- **The map answers everything the facts determine,** including relations never stated, from step codes it learned itself.
- **Guesses improve with information:** on questions not yet determined, over the last 1,000 stories, accuracy rises with the number of facts so far: 11% (1–3 facts), 13%, 16%, 20% (10–12 facts). Choosing among the 8 other rooms at random would give 12.5%.
- **The directions' meanings are learned quickly.** How close north + south and east + west come to cancelling exactly (1 = exact opposites):

| Stories seen | 1 | 2 | 6 | 11 | 21 | 51 | 101 |
|---|---|---|---|---|---|---|---|
| north + south | −0.03 | 0.56 | 0.83 | 0.88 | 0.93 | 0.97 | 1.00 |
| east + west | 0.07 | 0.53 | 0.86 | 0.98 | 0.99 | 1.00 | 1.00 |

  North and east stay unrelated (overlap +0.01).
- **Two details mattered:**
  1. **Leave-one-out learning.** Measuring each relation on the full map only reads back the current guess: north and south stalled at 0.60 of exact opposites, and implied answers at 62–80%.
  2. **Dropping a group's weak anchor once it joins the main map.** Without that, the anchor and the linking fact pulled to a tie (implied answers 80% → 100%).

## Findings

1. **Place codes plus direction step codes, all integer phases, build a consistent map** from relations stated one at a time, and answer relations that were never stated.
2. **The relation words' meanings come from consistency alone:** that north and south cancel and east and west cancel is learned from random starts: 0.97 of the way by 51 stories, exact by 101.
3. **Best guesses improve as facts arrive,** both inside a story (the map) and across stories (the step codes).

Caveats: one seed; a clean synthetic task (a full 3 × 3 grid, no contradictory facts); the per-story room table is kept explicitly, not yet in a memory of the store's kind.

## What next

- Keep the place codes in a sparse, contextified trace ([experiment 32](32-learned-bitwise-store.md)), so maps from many stories share one memory.
- Relations other than space (before and after, family, ranks), each with its own step codes.
- Contradictory or noisy facts, where averaging should keep the best guess.

## Code

`scripts/spatial_gen.py STORIES OUT [seed]`, `scripts/spatial_map.py IDS INFO [D] [L]`. Runs: `runs/exp32/`.
