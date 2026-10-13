//! Helm batch state for regression testing with JSONL inputs.

use std::path::{Path, PathBuf};

/// A single row from the batch input JSONL file.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct BatchRow {
    pub utterance: String,
    pub expected_tool: Option<String>,
}

/// Helm batch state stored as an ECS resource.
pub struct HelmBatchState {
    pub rows: Vec<BatchRow>,
    pub next: usize,
    pub out: PathBuf,
    pub results: Vec<serde_json::Value>,
    pub injected_last_frame: bool,
    pub ui_events_before: usize,
    pub rss_start_kb: usize,
    /// Set once all rows are recorded: frames still to run so queued UI events
    /// (camera_direction inference etc.) dispatch before the process exits.
    pub drain_frames_left: Option<u32>,
    pub exit_code: i32,
}

impl HelmBatchState {
    /// Read batch rows from a JSONL file. Each line is `{"utterance":"...","expected_tool":"..."}`.
    pub fn from_jsonl(path: &Path) -> std::io::Result<Vec<BatchRow>> {
        let contents = std::fs::read_to_string(path)?;
        let mut rows = Vec::new();
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let row: BatchRow = serde_json::from_str(line)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            rows.push(row);
        }
        Ok(rows)
    }

    /// Check if all rows have been processed.
    pub fn is_done(&self) -> bool {
        self.next >= self.rows.len()
    }

    /// Check if there is a next row to inject.
    pub fn has_next(&self) -> bool {
        self.next < self.rows.len()
    }

    /// Get the next row without advancing.
    pub fn peek_next(&self) -> Option<&BatchRow> {
        self.rows.get(self.next)
    }

    /// Advance to the next row.
    pub fn advance(&mut self) {
        self.next += 1;
    }
}
