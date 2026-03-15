---
name: check-all
description: Unified code quality check combining ECS architecture, robust coding guidelines, and safe code patterns. Runs all checks in parallel for token efficiency.
user-invocable: true
allowed-tools: Read, Grep, Glob, Bash, Agent
argument-hint: "[file-or-directory]"
---

# Unified Code Quality Check

Run all three code quality checks in parallel:
1. **ECS Architecture** — component/system separation, layer boundaries
2. **Robust Coding** — design patterns, function size, error handling
3. **Safe Code** — panic!, unwrap(), static mut, bool params, catch-all matches

## Usage

```
/check-all
/check-all src/ecs/systems/animation_systems.rs
/check-all src/renderer/
```

## Target

If `$ARGUMENTS` is provided, check those files or directories.
If no arguments, check recently modified `.rs` files plus full-project pattern scans.

## Execution Strategy

Launch **3 parallel agents** (expert-explore, haiku model) to minimize token usage.
Each agent receives only the rules it needs — no redundant rule loading.

### Agent 1: ECS Architecture Check

Prompt the agent with these specific checks:

**Scope**: If arguments given, check those files. Otherwise, check `src/ecs/` for rules 1-6
and `src/platform/` + `src/app/` for rule 7.

1. **Components data-only**: In `src/ecs/component/**/*.rs`, search for `impl` blocks
   with `fn` methods that do computation (not just `new()`, `Default`, getters).
2. **Systems as free functions**: In `src/ecs/systems/**/*.rs`, verify functions take
   World/Components/Resources as parameters.
3. **mod.rs clean**: In `src/ecs/**/mod.rs`, search for `struct` or `enum` definitions.
4. **Layer boundaries**: In `src/platform/**/*.rs` and `src/app/**/*.rs`, search for:
   - `resource_mut::<` patterns with business logic
   - `match.*UIEvent` with inline mutation logic
   - Multiple ECS system function calls (not single dispatch entry points)

Report: file path, line number, violation description.

### Agent 2: Safe Code Pattern Check

Prompt the agent with these specific Grep patterns:

**Scope**: All `src/**/*.rs` excluding test code.

1. `panic!` — Search pattern: `panic!` in `src/` (exclude `#[test]`, `#[cfg(test)]`, `tests/`)
2. `.unwrap()` — Search pattern: `\.unwrap()` in `src/` (exclude test code)
3. `static mut` — Search pattern: `static mut` in `src/`
4. `bool` params — Search pattern: `pub fn.*\bbool\b` in `src/`
5. `_ =>` catch-all — Search pattern: `_ =>` in `src/` (exclude matches on primitives/strings)
6. Long functions — Count lines between `fn` opening `{` and closing `}`, flag > 120 lines

For each finding, note whether it falls under an allowed exception:
- `.unwrap()` in test code → exception
- `panic!` in `main()` or `Drop` → exception
- `_ =>` on numeric/string types → exception
- `bool` in private functions or `set_visible(bool)` style → exception

Report: file path, line number, category, violation or exception.

### Agent 3: Robust Design Check

Prompt the agent with these specific checks on TARGET files only (from arguments or recent changes):

**Scope**: Target files only (for token efficiency).

1. **Invalid states**: Search for structs with `Option<T>` + `bool` field combinations
2. **Boundary validation**: Check if file I/O / FFI functions validate inputs
3. **RAII**: Check if GPU handles have `Drop` implementations
4. **Shadowed variables**: Search for `let` bindings reusing outer scope names
5. **Closure captures**: Check stored closures for implicit large-scope captures
6. **Contiguous memory**: Verify bulk data (vertices, keyframes) uses `Vec<T>`

Report: file path, line number, rule number, violation description.

## Merging Results

After all 3 agents complete:

1. Deduplicate findings (e.g., `unwrap` appears in both safe-code and coding checks)
2. Classify severity:
   - **Critical**: panic in production, unwrap in production, static mut, invalid states, RAII violations
   - **Warning**: long functions, bool params, catch-all matches, layer boundary violations
   - **Info**: shadowed variables, closure captures, contiguous memory suggestions
3. Present unified report

## Output Format

```
## Unified Code Quality Check Results

### Target
- Files checked: [list or count]
- Checks run: ECS Architecture, Robust Coding, Safe Code

### Summary
| Category | Critical | Warning | Info |
|----------|----------|---------|------|
| ECS Architecture | N | N | N |
| Safe Code | N | N | N |
| Robust Design | N | N | N |
| **Total** | **N** | **N** | **N** |

### Critical Issues
- [file:line] (category) description
  Suggestion: ...

### Warnings
- [file:line] (category) description
  Suggestion: ...

### Info
- [file:line] (category) description

### Compliant Areas
- [list of passing checks]
```

## Important Rules

- Do NOT modify any code. This skill is **read-only analysis**.
- All 3 agents MUST run in parallel (single message with 3 Agent tool calls).
- Use haiku model for agents to minimize token cost.
- Each agent should use Grep with specific patterns, not read entire files.
- Respond to the user in Japanese. Write the check results in English.
- Keep agent prompts focused — no redundant rule text.
