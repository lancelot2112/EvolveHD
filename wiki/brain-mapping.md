# Brain mapping

This page records the reasoning that turned neuroanatomy into seven primitives. The aim is not fidelity to biology. It is to borrow designs that evolution has already debugged, and keep only what helps a learner on a computer.

## What a cortical area computes

A cortical area is mainly a **driver-dependent contextual predictor**:

- It learns hetero-associations: context → what comes next, and area → area.
- It does some local autoassociative cleanup.
- It is not a global autoassociative memory. That role belongs to the hippocampus (CA3), which learns in one shot; cortex is the slow statistical learner.

Its inputs have two roles (Sherman & Guillery), plus local recurrence:

- **Driving input** reaches layer 4 from first-order thalamus or the area below. It decides *what* the area represents; remove it and sensory cortex falls to spontaneous activity.
- **Context and feedback** reach the apical dendrites in layer 1 ([drive and prime](concepts/drive-and-prime.md)). On their own they barely fire a cell. Together with basal drive they make it burst. So context **selects and amplifies** driven interpretations; it does not create them. [Audit sim 4](experiments/04-audit-sims.md) reproduced this: plan as a driver dropped perception to 0.24, and plan as priming kept it at 0.98.
- **Lateral recurrence** (layers 2/3) does local pattern completion. It is short-range cleanup, not an area-wide attractor.

Its outputs and gain control:

- **Layer 5** drives striatum, brainstem and higher-order thalamus, so every area has an action-like output.
- **Layer 6** sets the gain of its own thalamic relay.

### What separates areas

The circuit is roughly shared; the parameters and wiring differ:

1. **Connectivity:** what drives the area, and where its layer-5 output goes.
2. **Hierarchy position:** feedforward input lands in layer 4 and feedback avoids it. Sensory areas have a thick layer 4; motor and frontal areas are agranular.
3. **Timescale:** intrinsic timescales lengthen up the hierarchy, which matches log-spaced delay taps.
4. **Recurrence and persistence:** prefrontal cortex can hold an attractor without input (working memory); sensory areas can't.
5. **Plasticity schedule:** primary areas lock in early; association areas stay plastic.
6. **Thalamic partner:** first-order thalamus relays the world, higher-order thalamus relays cortex.
7. **Neuromodulator receptor density.**

## Expansion, convergence and association

- **Expansion interfaces** take a dense input, project it onto many more cells and keep a sparse set: dentate gyrus, layer 4 and cerebellar granule cells. They differ in how the expansion is learned:
  - granule cells: fixed random conjunctions;
  - dentate gyrus: grown, with young cells hyperexcitable;
  - layer 4: slow Hebbian learning, then frozen.
- **Convergence** sits where the system must abstract or decide: retina → optic nerve, the ventral stream into IT, association hubs, entorhinal cortex and CA1, Purkinje cells, the basal-ganglia funnel, and neuromodulator nuclei that compress everything into roughly one number.
- **Association** completes or predicts. Local and slow in cortical layers 2/3; global and one-shot in CA3.

Sparsity decides whether an expansion separates or preserves similarity:
- **Very sparse:** similar inputs get different codes (pattern separation, as in the dentate gyrus).
- **Moderately sparse, with overlapping masks:** similar inputs share codes. This is locality-sensitive hashing, as in the fly mushroom body.

*Semantic* codes need compression trained on context rather than form. Words that predict the same neighbours get similar codes.

## Basal ganglia and amygdala share a primitive

The amygdala is not one structure:
- The basolateral amygdala is cortex-like (Associate).
- The central and medial amygdala are striatum-like (Select).

Both run associate → select. The outputs differ:

| | Basal ganglia | Amygdala |
|---|---|---|
| Output | Releases one winner: an action | Graded valence broadcast to hypothalamus, brainstem, LC, VTA, basal forebrain |
| Exclusive? | Mutually exclusive choices | Co-exists with whatever else is happening |
| Learning | Incremental | Often one-shot |
| Unlearning | Erase (Go LTD) for fast reversal | Extinction is an overlay, gated by context, so fear can return |

## Broadcast has two axes

| | Scalar | Semantic (a code) |
|---|---|---|
| **Tonic** | Sets the regime: exploration temperature, thresholds, plasticity (LC tonic level; ACh for sensing vs recall) | Holds context: a task set or goal every area reads as priming |
| **Phasic** | Gates learning now: dopamine bursts and dips; LC bursts on surprise | Ignition: one content wins and is broadcast everywhere (global workspace; the [claustrum loop](experiments/05-claustrum-consensus.md)) |

## The resulting primitive set

Sorted by learning signal: unsupervised for cortex, reinforcement for the basal ganglia, supervised for the cerebellum (Doya, 1999). Two more primitives learn no content at all.

| Primitive | Job | Where |
|---|---|---|
| Expand | Separate or hash ([concept](concepts/shift-bind.md)) | Dentate gyrus, layer 4, granule cells |
| Associate | Complete or predict | Layer 2/3, CA3, basolateral amygdala |
| Compress | Abstract or converge | IT, hubs, CA1, entorhinal cortex |
| Select | Go/NoGo opponent masks, three-factor learning ([concept](concepts/go-nogo-selection.md)) | Striatum, central amygdala |
| Correct | Readout trained by a vector error | Purkinje cells and inferior olive |
| Gate | AND with a control vector: routing, attention, mode ([concept](concepts/time-delays-and-holds.md)) | Thalamus, reticular nucleus |
| Broadcast | Many inputs → one scalar or code → global setting ([concept](concepts/broadcast.md)) | VTA, LC, basal forebrain, raphe, claustrum |

Some circuits reduce to combinations of these. A central pattern generator is Associate with a delayed loop. An evidence accumulator is Associate with persistence. A working-memory slot is Gate plus a self-sustaining Associate.

## A strong actor as six loops

1. **Perception stack:** sensor → Expand → Associate → Compress, up the hierarchy, with each level priming the one below.
2. **Episodic loop:** hub → Expand (DG) → Associate (CA3, one-shot) → Compress (CA1) → reinstatement, plus offline replay.
3. **Valuation:** hub → Associate (BLA) → Select (CeA, overlay extinction) → broadcast.
4. **Action and thought selection:** hub + plan → Select → Gate. This gates both motor output and what enters working memory.
5. **Execution:** Expand (granule) → Correct (Purkinje), with a forward model that cancels self-generated input.
6. **Mode control:** setpoints and surprise → Broadcast → Gates (sense, imagine, sleep).

See [primitives and channels](primitives.md) for how these became grammar moves, and [concepts](concepts/README.md) for each mechanism in detail.
