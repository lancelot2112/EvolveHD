# Shift-bind

Binding turns two codes into one pair code that **is not similar to either part.** That is what lets a downstream cell respond to "A with B" without also responding to "A with anything". EvolveHD binds by shifting one code by an amount the other determines, then cleaning up. The [Expand](../primitives.md) primitive (bind nodes) is built on it.

## How it works

- **Toy form ([04](../experiments/04-audit-sims.md)):** rotate a by a min-hash of b, and b by a min-hash of a, then take the union.
- **Growth sims:** a bind node's input is X's winner code OR Y's winner code rotated by a fixed offset (977 bits), giving 64 bits. A [matcher pool](matcher-pools.md) over that input recruits one cell per seen pair. Because the pool sees both parts, a cell for (a, b) shares only about half its mask with (a, c), and the recruit bar (40 of 48) keeps them apart.
- **Bind after cleanup.** Bind winner codes, never raw noisy input. Noise changes the shift amount, so the pair code lands somewhere else.

## Why

Pair-code overlap with a pair sharing one part (unrelated pairs ≈ 1 bit), and accuracy on a conjunctive task where neither part alone carries information ([04](../experiments/04-audit-sims.md)):

| Binding | Shared bits | Accuracy, clean inputs | Accuracy, 20% noise before binding |
|---|---|---|---|
| None | – | 0.25 (chance) | 0.26 |
| XOR ρ(a ^ ρ(b)) | 15.3 | 0.30 | 0.26 |
| CDT | 10.9 | 0.45 | 0.29 |
| Shift-bind | 1.1 | 0.91 | 0.69 |
| Oracle (fresh code per pair) | – | 0.92 | 0.95 |

Binds of binds are how depth is reached ([08 · bootstrapping](../experiments/08-bootstrapping.md)). The cost grows multiplicatively, which is why [Compress](growth-operators.md#distill) exists.

## Fusion

Shift-bind generalises from two parts to n roles: rotate each part's code by its role's shift and OR the results ([22](../experiments/22-fusion.md)). On 2,048 bits about 8 role-bound parts superpose cleanly; thinning the result back to 32 bits loses most of the resolution, and XOR is not a binding on sparse codes (it is nearly OR). A matcher pool whose cells sample 24 bits of the fused input thins it the same way, so matching doesn't realise that resolution. If fused codes were fused again, rotations would alias (they commute), so nested roles need fixed random permutations instead.

## Related

[Sparse codes](sparse-codes.md) · [Matcher pools](matcher-pools.md) · [Growth operators](growth-operators.md) · [Stepping stones](stepping-stones.md)

## Code

`BindReal` and `BindSym` in `text_sim.py`; the bind cache keyed by (X cell, Y cell, generations) skips matching for known pairs.

## Evidence in

[04 · audit sims](../experiments/04-audit-sims.md) · [07 · graph growth](../experiments/07-graph-growth.md) · [22 · fusion](../experiments/22-fusion.md)

*[All concepts](README.md) · [Wiki home](../README.md)*
