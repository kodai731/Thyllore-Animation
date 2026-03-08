---
paths:
  - "src/**/*.rs"
---

# Logging

- Use the `crate::log!()` macro for logging (defined in `src/logger/logger.rs`)
- Logs are output to `log/log_N.txt`
- Do NOT output to standard console; use the log files instead

## Prohibited

Do NOT use the Rust `log` crate macros:
- `log::debug!()`, `log::info!()`, `log::warn!()`, `log::error!()`, `log::trace!()`

These produce zero output in this project because no `log` crate subscriber is configured.

## Usage

```rust
crate::log!("[tag] message: {}", value);
```
