use chrono::Local;
use once_cell::sync::Lazy;
use std::fs::{File, OpenOptions};
use std::io::{Result, Write};
use std::sync::Mutex;

pub static LOGGER: Lazy<Mutex<Logger>> = Lazy::new(|| {
    let logger = Logger::new("log/log.txt").expect("Failed to initialize logger");
    Mutex::new(logger)
});

pub struct Logger {
    file: File,
}

impl Logger {
    pub fn new(path: &str) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true) // ← 初期化（ファイル内容を消去）
            .open(path)?;
        Ok(Logger { file })
    }

    pub fn log(&mut self, args: std::fmt::Arguments) -> Result<()> {
        let time = Local::now();
        writeln!(self.file, "[{}] {}", time.format("%Y-%m-%d %H:%M:%S"), args)
    }
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        $crate::logger::logger::LOGGER
            .lock()
            .expect("Failed to lock logger")
            .log(format_args!($($arg)*))
            .expect("Failed to write log");
    }};
}
