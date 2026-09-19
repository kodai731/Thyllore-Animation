---
paths:
  - "src/**/*.rs"
  - "crates/**/*.rs"
---

# Logging

- The logger and its macros are defined in `crates/thyllore-log-core/src/logger.rs`; `src/logger/` re-exports
  them
- Logs are output to `log/log_N.txt`
- Do NOT output to standard console; use the log files instead

## Log Levels

| Macro | Level | Release build | Use for |
|-------|-------|---------------|---------|
| `log!()` | Info | Compiled out | General info, diagnostics, ML inference values |
| `log_warn!()` | Warning | Active | Recoverable failures, fallbacks, unexpected state |
| `log_error!()` | Error | Active | Unrecoverable failures, critical errors |

In `src/` the macros are in scope as `crate::log!` etc.; a crate imports them with
`#[macro_use] extern crate thyllore_log_core;` in its `lib.rs` (see `thyllore-vulkan-core`).

## Prohibited

Do NOT use the Rust `log` crate macros (`log::debug!()`, `log::info!()`, `log::warn!()`, `log::error!()`,
`log::trace!()`) anywhere except the Vulkan validation callback in `src/app/init/instance.rs`. That callback
is the single place that echoes to the console (`env_logger`, enabled with `RUST_LOG=debug`) in addition to
the log file; every other message goes to the log file only.

## Usage

```rust
log!("Loading model: {}", path);
log_warn!("Failed to load texture {}: {}", path, e);
log_error!("Frame error: {:?}", e);
```
