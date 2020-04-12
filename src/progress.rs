use crate::walker::ClientState;
use console::style;
use indicatif::{HumanBytes, ProgressBar, ProgressStyle};
use jwalk::DirEntry;

use prettytable::{cell, row, table};

use serde::export::Formatter;

use std::path::PathBuf;
use std::time::{Duration, Instant};

pub struct WalkProgress {
    errors: u64,
    total: u64,
    total_size: u64,

    root: PathBuf,
    update_frequency: Duration,
    started: Instant,
    last_update: Instant,
}

impl WalkProgress {
    pub fn new(root: PathBuf) -> WalkProgress {
        let update_frequency = Duration::from_millis(500);
        let started = Instant::now();

        WalkProgress {
            errors: 0,
            total: 0,
            total_size: 0,

            root,
            update_frequency,
            started,
            last_update: started,
        }
    }

    pub fn create_progress_bar(&self) -> ProgressBar {
        let progress_bar = ProgressBar::new_spinner();
        progress_bar.set_style(
            ProgressStyle::default_spinner()
                .template("[{elapsed_precise}] Per sec: {per_sec:.cyan/blue} | {msg}"),
        );
        progress_bar
    }

    pub fn should_update(&self) -> bool {
        self.last_update.elapsed() > self.update_frequency
    }

    pub fn update(&mut self, progress_bar: &ProgressBar) {
        self.last_update = Instant::now();
        progress_bar.set_position(self.total);
        progress_bar.set_message(
            format!(
                "Files: {} | Size: {} | Errors: {}",
                style(self.total).green(),
                style(HumanBytes(self.total_size)).green(),
                style(self.errors).red(),
            )
            .as_ref(),
        );
    }

    pub fn record_progress(&mut self, item: &Result<DirEntry<((), ClientState)>, jwalk::Error>) {
        self.total += 1;
        match item {
            Err(_) => self.errors += 1,
            Ok(dir_entry) => match &dir_entry.client_state {
                Some(metadata) => {
                    self.total_size += metadata.size;
                }
                None => self.errors += 1,
            },
        }
    }
}

impl std::fmt::Display for WalkProgress {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let elapsed = chrono::Duration::from_std(self.started.elapsed()).unwrap();
        let runtime = chrono_humanize::HumanTime::from(elapsed);
        let runtime_text = runtime.to_text_en(
            chrono_humanize::Accuracy::Precise,
            chrono_humanize::Tense::Present,
        );
        let errors = if self.errors > 0 {
            style(self.errors).red()
        } else {
            style(self.errors).green()
        };
        let mut table = table!(
            ["Root", style(self.root.display()).blue()],
            [
                "Total Size",
                style(indicatif::HumanBytes(self.total_size)).green()
            ],
            ["Files", style(self.total).green()],
            ["Duration", style(runtime_text).green()],
            ["Errors", errors]
        );
        table.set_format(*prettytable::format::consts::FORMAT_CLEAN);
        write!(f, "{}", table.to_string())
    }
}
