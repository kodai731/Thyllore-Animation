---
name: check-safe-code
description: Check for unsafe code patterns that should be replaced with safe alternatives. Detects panic!, unwrap(), static mut, bool params, catch-all matches, and oversized functions in production code.
user-invocable: true
allowed-tools: Read, Grep, Glob, Bash, Agent
argument-hint: "[file-or-directory]"
---

# Safe Code Check

Scan Rust source code for unsafe or fragile patterns that should be replaced
with safe, robust alternatives. This skill targets the same categories of issues
fixed in PR #72 (`feature/make-safe-code`).

## Usage

```
/check-safe-code
/check-safe-code src/loader/
/check-safe-code src/vulkanr/core/device.rs
```

## Target

If `$ARGUMENTS` is provided, check those files or directories.

If no arguments are given, scan **all** `.rs` files under `src/`.

**Exclusions** — always skip these (violations are acceptable in test code):
- `tests/` directory
- `#[cfg(test)]` modules
- `#[test]` functions
- Lines containing `// safe-code-ignore` comment

## Categories to Check

### Category 1: `panic!()` in Production Code

Search for `panic!()` calls outside of test code. These should be replaced with
`Result<T, E>` returns or, if truly unreachable, `unreachable!()` with a reason.

**Pattern:**
```rust
panic!("something went wrong");
```

**Allowed exceptions:**
- `unreachable!()` for genuinely impossible states (with explanation)
- `panic!` inside `impl Drop` (Rust limitation)
- `panic!` in `main()` or top-level setup where recovery is impossible

### Category 2: `.unwrap()` in Production Code

Search for `.unwrap()` calls. These should be replaced with:
- `.expect("reason")` — when the invariant is guaranteed and documented
- `?` operator — when the error should propagate
- `.unwrap_or()` / `.unwrap_or_else()` — when a default is appropriate
- `if let` / `match` — when both paths need handling

**Pattern:**
```rust
value.unwrap()
map.get(key).unwrap()
result.unwrap()
```

**Allowed exceptions:**
- Test code (`#[test]`, `#[cfg(test)]`)
- After an explicit check (e.g., `if value.is_some() { value.unwrap() }`)
  — though `if let` is still preferred

### Category 3: `static mut` + `unsafe`

Search for `static mut` declarations. These should be replaced with:
- `thread_local!` + `RefCell<T>` for thread-local mutable state
- `std::sync::OnceLock` or `std::sync::LazyLock` for one-time initialization
- `std::sync::Mutex` or `RwLock` for shared mutable state

**Pattern:**
```rust
static mut SOME_STATE: Type = initial_value;
unsafe { SOME_STATE = new_value; }
```

### Category 4: `bool` Function Parameters

Search for public functions with `bool` parameters. These should be replaced
with a two-variant enum for readability at call sites.

**Pattern:**
```rust
pub fn create_device(validation_enabled: bool) { ... }
// Call site is unreadable: create_device(true)

// Should be:
pub enum ValidationMode { Enabled, Disabled }
pub fn create_device(validation: ValidationMode) { ... }
// Call site is clear: create_device(ValidationMode::Enabled)
```

**Allowed exceptions:**
- Private helper functions (`fn` without `pub`)
- Functions where the bool meaning is obvious from the name (e.g., `set_visible(bool)`)
- Trait implementations required by external crates

### Category 5: Catch-all `_ =>` in Match Expressions

Search for `_ =>` in match expressions on enums. These hide future variants
and should be replaced with exhaustive matching.

**Pattern:**
```rust
match tone_map_op {
    ToneMapOperator::Aces => { ... }
    _ => { ... }  // hides future variants
}
```

**Allowed exceptions:**
- Match on numeric types (`i32`, `u32`, `usize`, etc.) where exhaustive listing is impractical
- Match on `String` / `&str`
- Match with explicit comment `// exhaustive: remaining variants handled identically`

### Category 6: Functions Over 120 Lines

Count function body lines. Functions exceeding 120 lines should be split into
smaller helper functions.

**Detection:**
- Find `fn ` declarations
- Count lines from opening `{` to closing `}`
- Flag functions > 120 lines

**Allowed exceptions:**
- Functions that are primarily match expressions with many simple arms
- Generated code or FFI bindings

## Check Process

1. Determine target files from `$ARGUMENTS` or default to all `src/**/*.rs`
2. Launch **two parallel agents** using the Agent tool:

### Agent A: Pattern Search (expert-explore)

Scan for mechanical violations detectable by pattern matching:

- **Category 1**: Search for `panic!` in `src/` (exclude test modules)
- **Category 2**: Search for `.unwrap()` in `src/` (exclude test modules)
- **Category 3**: Search for `static mut` in `src/`
- **Category 4**: Search for `pub fn` with `bool` parameters in `src/`
- **Category 5**: Search for `_ =>` in match expressions in `src/`

For each finding, report:
- File path and line number
- The offending line
- Whether it falls under an allowed exception

Do NOT modify any code. Report findings only.

### Agent B: Function Length Check (expert-explore)

Scan for oversized functions:

- **Category 6**: For all `fn` definitions in target files, count body lines
  and flag any > 120 lines

For each finding, report:
- File path, function name, and line range
- Total line count

Do NOT modify any code. Report findings only.

3. Merge results from both agents
4. Filter out allowed exceptions
5. Report violations

## Output Format

```
## Safe Code Check Results

### Target
- Files checked: [count or list]

### Summary
| Category | Violations | Description |
|----------|-----------|-------------|
| 1 | N | `panic!()` in production code |
| 2 | N | `.unwrap()` in production code |
| 3 | N | `static mut` usage |
| 4 | N | `bool` function parameters |
| 5 | N | Catch-all `_ =>` patterns |
| 6 | N | Functions > 120 lines |
| **Total** | **N** | |

### Violations

#### Category 1: `panic!()` in Production Code
- src/foo/bar.rs:45 — `panic!("unexpected state")`
  Suggestion: Return `Err(AppError::UnexpectedState)` instead

#### Category 2: `.unwrap()` in Production Code
- src/foo/baz.rs:120 — `config.get("key").unwrap()`
  Suggestion: Use `.expect("config must have 'key' after validation")`

#### Category 3: `static mut` Usage
- src/foo/qux.rs:10 — `static mut COUNTER: u32 = 0;`
  Suggestion: Use `thread_local!` + `RefCell<u32>` or `AtomicU32`

#### Category 4: `bool` Function Parameters
- src/foo/device.rs:30 — `pub fn init(debug: bool)`
  Suggestion: Use `enum DebugMode { Enabled, Disabled }`

#### Category 5: Catch-all `_ =>` Patterns
- src/foo/mode.rs:50 — `_ => Mode::Default`
  Suggestion: List all variants explicitly

#### Category 6: Functions Over 120 Lines
- src/foo/loader.rs:100-250 — `load_scene()` is 150 lines
  Suggestion: Extract helper functions for each logical step

### Exceptions (Not Violations)
- src/foo/test_utils.rs:30 — `.unwrap()` in `#[cfg(test)]` (allowed)
- src/foo/main.rs:5 — `panic!` in `main()` setup (allowed)
```

## Important Rules

- Do NOT modify any code. This skill is **read-only analysis**.
- Always exclude test code from violation counts.
- Report allowed exceptions separately for transparency.
- Classify each finding clearly as violation or exception.
- Respond to the user in Japanese. Write the check results in English.
