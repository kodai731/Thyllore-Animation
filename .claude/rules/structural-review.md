---
paths:
  - "**"
---

# Structural and Domain Review

Review the choice of model and the structure before checking local numerical and control-flow
correctness. Passing tests, boundary checks, and avoiding division by zero do not establish that the
implementation represents the problem well.

Read the applicable design and domain rules. Detailed requirements remain in `hierarchy.md`,
`ecs-architecture.md`, `module-structure.md`, `naming.md`, `coding.md`, and `comments.md`;
this review process does not replace them.

## Choose Models That Support Human Understanding

Ask whether the problem can be expressed directly through an established mathematical or domain model.
Prefer a model that lets a reader reason about the intended behavior using familiar concepts instead
of reconstructing that behavior from implementation steps and unrelated coefficients.

An established model is useful because it provides shared vocabulary, groups many operations into a
small number of concepts, and supplies properties that can be used to reason about the implementation.
Its value is explanatory as well as computational: a reader should be able to predict what changing a
parameter means and why the result has the intended shape or behavior.

Evaluate the model itself before checking whether its equations were implemented correctly:

- Does it represent the essential quantities, degrees of freedom, constraints, and operations?
- Do parameters express meaningful properties of the problem rather than incidental calculation steps?
- Can its geometry, invariants, or other known properties explain and help verify the behavior?
- Does it reduce the concepts a reader must reconstruct, including conversions required by consumers?
- Are its assumptions, singularities, and numerical or computational costs suitable for supported use?

Familiarity alone does not make a model appropriate. Compare it with the actual problem and operations;
do not replace simple code with formalism or require one representation for every stage of a pipeline.
A model-level change is more than renaming variables: parameters, equations, and consumers must express
the same concepts.

### Example: Directions on a Sphere

When an operation places samples around an axis at a chosen angular spread, spherical coordinates
in a frame aligned with that axis expose the geometry directly: radius, polar angle, and azimuth.
For a Y-up convention with azimuth measured from +X toward +Z:

```text
x = r * sin(polar_angle) * cos(azimuth)
y = r * cos(polar_angle)
z = r * sin(polar_angle) * sin(azimuth)
```

A reader can infer that fixed radius keeps samples on a sphere, fixed polar angle makes a ring, and
varying azimuth moves around that ring. Independent component offsets followed by normalization may
produce valid unit vectors, yet hide these intended constraints and make angular spread hard to predict.
Review whether the spherical model makes the intent explicit before checking individual arithmetic.

This does not prescribe symbols or axes universally. Identify the convention used. Cartesian vectors
can remain the appropriate storage and computation format after constructing the direction; spherical
coordinates have poles and are not automatically the best representation for interpolation or rotation
composition. Also, equal angular steps do not by themselves guarantee uniform sampling by surface area.

### Example: An Animation Transition

For a transition that must begin and end at specified values with specified velocities, a cubic Hermite
curve expresses the contract through endpoints and endpoint tangents. A reader can reason about those
boundary conditions directly. Four unexplained polynomial coefficients may evaluate the same curve
but require the reader to reconstruct how each coefficient relates to the animation intent.

Prefer the endpoint-and-tangent model at the authoring boundary when those are the intended controls.
Polynomial coefficients may still be a useful derived runtime representation. A linear transition is
appropriate when constant velocity is the intended behavior; do not introduce cubic curves merely
because they are a familiar model. Distinguish a derivative per second from a tangent with respect to
normalized segment time when converting between these representations.

## Reconstruct Responsibilities and Contracts

Use design documents, callers, and consumers to establish intent; the implementation under review is
not its own specification. Trace enough code outside the diff to answer:

- Which domain owns each decision, authoritative value, and state transition?
- Do dependencies follow the established layers, and do generic modules remain independent of features?
- Does each API express a coherent domain operation, or require callers to coordinate internal steps?
- Who updates, invalidates, rebuilds, and releases derived state after edits or reloads?
- Are facts, formulas, defaults, or policies independently defined in multiple consumers?
- Across input, computation, rendering, and persistence, do identifier scopes, index domains, units,
  coordinate frames, ordering, and optional values retain the same meaning?

Consider a concrete extension already implied by the supported domain or stated design. Identify which
modules would change and why. Unexpected changes to generic coordinators or repeated policy edits can
expose coupling. Do not invent speculative features to justify an abstraction.

## Write Findings With Evidence

For each structural finding, identify the affected code, the model or applicable contract, and the
consequence for interpretation, behavior, or a concrete supported change. Explain what a proposed model
makes easier to understand or infer; "use a standard model" or "unclear name" alone is insufficient.
Prefer meaningful names, types, and boundaries over adding explanatory prose throughout source code.

Separate demonstrated defects and established rule violations from unresolved design questions.
Do not manufacture findings or impose a quota. Briefly state which structural perspectives were checked
and any material gaps in the review summary, even when there are no findings.
