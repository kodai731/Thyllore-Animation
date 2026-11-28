//! File-based debug logging for the Blender addons, compiled only under the `debug-log` feature.

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Mutex, OnceLock};

fn log_file() -> &'static Mutex<Option<std::fs::File>> {
    static LOG_FILE: OnceLock<Mutex<Option<std::fs::File>>> = OnceLock::new();
    LOG_FILE.get_or_init(|| Mutex::new(None))
}

/// Open (create/append) the log file. Subsequent calls replace the target.
pub fn init(path: &str) -> std::io::Result<()> {
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    *log_file().lock().expect("debug log mutex") = Some(file);
    Ok(())
}

/// Append one timestamped line. No-op until `init` has opened a file.
pub fn log_line(message: &str) {
    let mut guard = log_file().lock().expect("debug log mutex");
    if let Some(file) = guard.as_mut() {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(file, "{timestamp} INFO {message}");
        let _ = file.flush();
    }
}

#[cfg(feature = "python")]
mod python {
    use pyo3::exceptions::PyOSError;
    use pyo3::prelude::*;

    /// Open (create/append) the addon debug log at `path`.
    #[pyfunction]
    fn debug_log_init(path: &str) -> PyResult<()> {
        super::init(path).map_err(|e| PyOSError::new_err(e.to_string()))
    }

    /// Append one timestamped line to the addon debug log.
    #[pyfunction]
    fn debug_log(message: &str) {
        super::log_line(message);
    }

    /// Add `debug_log_init` / `debug_log` to a wheel module.
    pub fn register(m: &Bound<PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(debug_log_init, m)?)?;
        m.add_function(wrap_pyfunction!(debug_log, m)?)?;
        Ok(())
    }
}

#[cfg(feature = "python")]
pub use python::register;
