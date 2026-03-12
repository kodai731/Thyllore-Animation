---
paths:
  - "src/**/*.rs"
---

# Logging

- Logs are defined in `src/logger/logger.rs`
- Logs are output to `log/log_N.txt`
- Do NOT output to standard console; use the log files instead

## Log Levels

| Macro | Level | Release build | Use for |
|-------|-------|---------------|---------|
| `crate::log!()` | Info | Compiled out | General info, diagnostics, ML inference values |
| `crate::log_warn!()` | Warning | Active | Recoverable failures, fallbacks, unexpected state |
| `crate::log_error!()` | Error | Active | Unrecoverable failures, critical errors |

## Prohibited

Do NOT use the Rust `log` crate macros:
- `log::debug!()`, `log::info!()`, `log::warn!()`, `log::error!()`, `log::trace!()`

These produce zero output in this project because no `log` crate subscriber is configured.

## Usage

```rust
crate::log!("Loading model: {}", path);
crate::log_warn!("Failed to load texture {}: {}", path, e);
crate::log_error!("Frame error: {:?}", e);
```
