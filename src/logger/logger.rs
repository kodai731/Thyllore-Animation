use chrono::Local;
use once_cell::sync::Lazy;
use std::fs::{File, OpenOptions};
use std::io::{Result, Write};
use std::sync::Mutex;

pub static LOGGER: Lazy<Mutex<Logger>> = Lazy::new(|| {
    let logger = Logger::new("log/log").expect("Failed to initialize logger");
    Mutex::new(logger)
});

pub struct Logger {
    file: File,
    base_path: String,
    current_index: usize,
    max_file_size: u64, // bytes
}

impl Logger {
    pub fn new(base_path: &str) -> Result<Self> {
        // Max file size: 10MB
        let max_file_size = 10 * 1024 * 1024;

        // Start with index 0
        let current_index = 0;
        let file_path = format!("{}_{}.txt", base_path, current_index);

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true) // ← 初期化（ファイル内容を消去）
            .open(&file_path)?;

        Ok(Logger {
            file,
            base_path: base_path.to_string(),
            current_index,
            max_file_size,
        })
    }

    fn rotate_if_needed(&mut self) -> Result<()> {
        // Check current file size
        let metadata = self.file.metadata()?;
        let current_size = metadata.len();

        if current_size >= self.max_file_size {
            // Close current file and open new one
            self.current_index += 1;
            let new_path = format!("{}_{}.txt", self.base_path, self.current_index);

            self.file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&new_path)?;

            // Log rotation info to new file
            let time = Local::now();
            writeln!(
                self.file,
                "[{}] === Log file rotated from {}_{}.txt ===",
                time.format("%Y-%m-%d %H:%M:%S"),
                self.base_path,
                self.current_index - 1
            )?;
        }

        Ok(())
    }

    pub fn log(&mut self, args: std::fmt::Arguments) -> Result<()> {
        // Check if rotation is needed before logging
        self.rotate_if_needed()?;

        let time = Local::now();
        writeln!(self.file, "[{}] {}", time.format("%Y-%m-%d %H:%M:%S"), args)?;

        // Flush to ensure log is written immediately
        self.file.flush()
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
