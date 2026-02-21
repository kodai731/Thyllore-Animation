---
name: implement-fix
description: Takes an IssueHistory MD file (with a Detailed Fix Plan section) as input and enters plan mode to create a concrete implementation plan. Ensures CLAUDE.md compliance, SSOT, ECS architecture, and robust design. Use after /plan-fix has appended investigation results to the file.
user-invocable: true
allowed-tools: Read, Grep, Glob, Task, Edit, EnterPlanMode, ExitPlanMode
argument-hint: "<path-to-issue-md>"
---

# Implement Fix — Plan Mode Implementation from Issue Reports

Takes an IssueHistory MD file that already contains a "Detailed Fix Plan" section
(typically created by `/plan-fix`) and enters plan mode to design a concrete,
architecture-compliant implementation.

## Usage

```
/implement-fix .claude/local/IssueHistory/SomeIssue.md
```

## Preconditions

- The MD file specified in `$ARGUMENTS` must exist and contain a "Detailed Fix Plan" section.
- If `$ARGUMENTS` is empty, display an error message and stop.

## Execution Flow

### Step 1: Gather Context

Read the following files in parallel:

1. The issue MD file from `$ARGUMENTS`
2. `CLAUDE.md` (project root) — for coding conventions and architecture rules
3. `.claude/rules/coding.md` — for robust coding guidelines (industry-standard)
4. `~/.claude/CLAUDE.md` (user global) — for personal coding preferences

Extract from the issue MD:
- Root Cause
- Recommended Approach
- Implementation Steps
- Affected file paths and line numbers

### Step 2: Investigate Affected Code

Launch the following agents **in parallel** using the Task tool:

#### expert-explore Agent

Investigate the code that will be modified. Instructions:

- Read every file listed in the Implementation Steps section
- For each proposed change, verify the current code at the specified lines
- Identify all dependencies and consumers of the code being changed
- Check for existing patterns in the codebase that the fix should follow
- Evaluate SSOT compliance: identify where the authoritative source of each
  piece of data lives and whether the fix introduces duplication
- Do NOT modify any code. Research only.

#### ecs-architecture-checker Agent

Verify that the proposed changes comply with ECS rules. Instructions:

- Check whether any proposed change violates ECS data-behavior separation
- Verify components remain data-only, systems remain pure functions
- Check that resources hold only dynamic per-frame state
- Confirm mod.rs files contain only module declarations
- Verify query patterns are used instead of stored entity IDs
- Report any violations or concerns

### Step 3: Enter Plan Mode

After gathering all context, call `EnterPlanMode` to design the implementation.

### Step 4: Design the Plan

In plan mode, write a detailed implementation plan that satisfies ALL of the following
design principles:

#### Principle 1: CLAUDE.md Compliance

- Self-explanatory code without unnecessary comments
- Verb-prefixed function names
- Functions 80-120 lines maximum
- Meaningful paragraph separation
- No definitions in mod.rs
- No feature-specific logic in app/update.rs
- Follow rustfmt.toml formatting

#### Principle 2: Single Source of Truth (SSOT)

For every piece of data touched by the fix:
- Identify the single authoritative source
- Ensure the fix does not create a second copy or shadow
- If data flows through multiple systems, ensure it originates from one place
  and is derived (not duplicated) elsewhere
- Document which struct/field is the source of truth in the plan

#### Principle 3: ECS Architecture

- Components: data-only structs in `ecs/component/`
- Resources: global dynamic state in `ecs/resource/`
- Systems: pure functions in `ecs/systems/`
- Bundles: predefined component combinations in `ecs/bundle/`
- Query pattern for entity access, not stored IDs
- RefCell-based interior mutability (`resource::<T>()` / `resource_mut::<T>()`)

#### Principle 4: Robust Design (Bug Prevention)

**MUST read `.claude/rules/coding.md`** and verify EVERY rule is satisfied.
The rules are derived from Google, Microsoft, Apple, Epic Games, and Rust
official guidelines. Key rules to check:

1. **Make invalid states unrepresentable** (Rule 1): Use enums over
   `Option<T>` + bool. Wrap primitives in semantic types.
2. **Validate at boundaries, trust the interior** (Rule 2): Boundary
   functions validate; internal functions assume valid data.
3. **Consistent error handling** (Rule 3): All fallible ops return `Result`.
   No mixing panics and `Option` for the same failure mode.
4. **RAII resource lifetime** (Rule 4): Every resource released via `Drop`.
   No stored borrowed references in struct fields.
5. **Small focused functions** (Rule 5): 80-120 lines max. Return values
   over output parameters. Limit to 3-4 parameters.
6. **Exhaustive matching** (Rule 6): No catch-all `_ =>` when variants
   could be added later.
7. **No shadowed variables** (Rule 7): Do not redeclare in nested scopes.
8. **No boolean parameters** (Rule 8): Use enum types instead.
9. **Fail fast** (Rule 9): `debug_assert!` / `assert!` for invariants.
10. **Explicit closure captures** (Rule 10): No implicit large-scope captures.
11. **Contiguous memory** (Rule 11): `Vec<T>` for bulk data.
12. **Test contracts, not internals** (Rule 12): Verify public API behavior.

### Step 5: Write the Plan File

Write the plan to the plan file. The plan must include:

1. **Overview**: One-paragraph summary of what the fix does and why
2. **Design Decisions**: How each of the 4 principles is satisfied
3. **File Changes**: For each file:
   - File path
   - What changes and why
   - Before/after code sketches (not full code, just the key parts)
4. **New Files** (if any): Path, purpose, and which module exports them
5. **Verification**: How to confirm the fix works (build, test, visual)
6. **SSOT Map**: Table showing each piece of data, its source of truth, and consumers

Example SSOT Map:
```
| Data | Source of Truth | Consumers |
|------|----------------|-----------|
| Bone transforms | Skeleton.bones[].local_transform | renderer, exporter, animation |
| Unit scale | FBX GlobalSettings.UnitScaleFactor | loader, exporter |
```

### Step 6: Exit Plan Mode

Call `ExitPlanMode` for user approval. After approval, append the approved plan
to the original issue MD file under a new `## Approved Implementation Plan` section.

## Important Rules

- Always read CLAUDE.md before designing the plan.
- Never skip the SSOT analysis. Every data field in the fix must have a clear owner.
- Never skip the ECS compliance check.
- The plan must be concrete enough that another developer can implement it
  without ambiguity.
- Respond to the user in Japanese. Write the plan content in English.
