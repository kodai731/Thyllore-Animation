use std::backtrace::Backtrace;
use std::panic::{self, PanicHookInfo};

use crate::logger::LOGGER;

/// Mirror every panic into the engine log so a crash in a detached session leaves a trace.
pub fn install() {
    let previous = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        write_panic_to_log(info);
        previous(info);
    }));
}

fn write_panic_to_log(info: &PanicHookInfo<'_>) {
    let backtrace = Backtrace::force_capture();

    let Ok(mut logger) = LOGGER.try_lock() else {
        return;
    };
    let Some(logger) = logger.as_mut() else {
        return;
    };
    let _ = logger.log_with_prefix("[PANIC]", format_args!("{}\n{}", info, backtrace));
}
