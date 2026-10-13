# 05 · Claustrum consensus loop (2026-10-11)

*[All experiments](README.md)* · [← 04](04-audit-sims.md) · [06 →](06-growth-interfaces.md)

**Concepts:** [Broadcast](../concepts/broadcast.md)

**Question.** Can a global broadcast make conflicting area-level interpretations settle into one unified reading? And does it work by cancellation or by competition?

## Setup

- **Shared space:** D = 2048. Two interpretations, A and B, with 64-bit codes sharing 16 bits.
- **Areas:** eight areas each sample the same ambiguous stimulus independently. Each A bit is present with probability qA and each B bit with qB, plus 32 noise bits. Each area has two matchers (A and B), fires at popcount ≥ 24, does winner-take-all, and outputs the winning code.
- **Claustrum, 4 sub-ticks per frame:**
  - v = Σ (2y − 1) over fired areas (a bipolar sum), and u = (v ≥ 1).
  - An area agreeing less than 0.6 with u is muted for the frame; otherwise it gets a top-down boost of 0.25·|u & M|.
  - u persists across frames.
- **Adaptation:** the winner's fatigue rises by 0.6 per frame and all fatigue decays ×0.97 per frame.
- 6 seeds, 3,000 frames.

## Results

**Ambiguous stimulus (qA = qB = 0.5)**

| Condition | Clean single reading | Blend | Areas agreeing | Mean dominance (frames) | CV |
|---|---|---|---|---|---|
| No claustrum, OR readout | 0.01 | 0.99 | 0.01 | – | – |
| No claustrum, bipolar-sum readout | 0.73 | 0.00 (27% partial) | 0.50 | 1.6 | 0.60 |
| No claustrum, local adaptation only | 0.81 | 0.00 | 0.52 | 1.6 | 0.59 |
| Consensus loop | 1.00 | 0.00 | 0.99 | locked for the whole run | – |
| Consensus loop + adaptation | 1.00 | 0.00 | 0.91 | 38.9 | 0.20 |

**Weak evidence: fraction of clean frames on the favoured reading A**

| Condition | qA 0.52 | qA 0.55 |
|---|---|---|
| No claustrum, bipolar sum | 0.86 | 0.99 |
| Consensus loop | 0.83 (one seed locked onto B) | 1.00 |
| Consensus + adaptation | 0.56 | 0.67 |

**Evidence switches after 500 frames: frames until B is held for 5 frames**

| Condition | 0.6 / 0.4 | 0.55 / 0.45 |
|---|---|---|
| No claustrum, bipolar sum | 0 | 0.8 |
| Consensus loop | 3.3 | 233 (2 of 6 never switched) |
| Consensus + adaptation | 0 | 0.2 |

## Findings

1. **An OR readout blends 99% of the time.** Bipolar-sum cancellation gives one reading per frame, but it flickers and only half the areas agree with it. Cancellation removes conflict without producing agreement.
2. **The consensus loop produces real unity (99% of areas agree) but locks permanently.** It can lock onto the weaker reading, and it ignores modest evidence changes for more than 200 frames: hysteresis.
3. **Consensus + adaptation behaves like perceptual rivalry.** There is always one clean reading, with about 90% of areas agreeing. It alternates every ~39 frames, dominance tracks the evidence (as in Levelt's laws), and it switches at once when evidence genuinely changes.
4. **Dominance CV is 0.20, against about 0.4–0.6 in human rivalry.** Noisier adaptation would widen it.

## What it changed

The semantic, phasic form of Broadcast (a consensus "Bus") needs an adaptation term or a noradrenaline-style reset. Without one it trades flicker for lock-in. See the broadcast taxonomy in [brain mapping](../brain-mapping.md).
