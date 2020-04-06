use crate::args::{Args, Command, SortType};
use crate::formats::Format;
use crate::progress::WalkProgress;
use crate::state::WalkState;
use crate::walker::Walker;

use std::fs::File;
use std::io;

use crate::directory_stat::DirectoryStat;
use chrono_humanize::Humanize;
use indicatif::HumanBytes;
use prettytable::{cell, row, Table};
use std::collections::HashMap;
use std::io::{BufWriter, ErrorKind};
use std::path::PathBuf;
use structopt::StructOpt;

mod args;
mod directory_stat;
mod formats;
mod progress;
mod state;
mod walker;

fn main() {
    reset_signal_pipe_handler().expect("Error resetting signal pipe handler");
    let args: Args = Args::from_args();
    match args.cmd {
        Command::Scan {
            threads,
            ignore_hidden,
            actual_size,
            output,
            path,
            format,
        } => walk(
            path,
            ignore_hidden,
            threads.unwrap_or(num_cpus::get() * 2),
            actual_size,
            format,
            output,
        ),
        Command::Parse {
            depth,
            prefix,
            limit,
            input,
            format,
            sort,
        } => read(depth, prefix, input, format, sort, limit),
    }
}

pub fn walk(
    root: PathBuf,
    ignore_hidden: bool,
    threads: usize,
    actual_size: bool,
    format: Format,
    output: Option<PathBuf>,
) {
    let writer = format.get_writer(get_output_file(output));

    let walker = Walker::new(threads, actual_size, ignore_hidden);

    let mut walk_state = WalkState::new(writer);
    let mut walk_progress = WalkProgress::new();
    let progress_bar = walk_progress.create_progress_bar();

    for dir in &mut walker.walk_dir(&root) {
        walk_progress.record_progress(&dir);
        if walk_progress.should_update() {
            walk_progress.update(&progress_bar);
        }

        let dir_entry = dir.unwrap();

        if let Some(metadata) = &dir_entry.client_state {
            if dir_entry.file_type.is_dir() {
                walk_state.add_path(dir_entry.path(), metadata);
            } else {
                walk_state.add_path(dir_entry.parent_path.to_path_buf(), metadata);
            };
        }
    }

    progress_bar.finish_and_clear();
    eprintln!("{}", walk_progress);
}

fn read(
    depth: usize,
    prefix: String,
    input: PathBuf,
    format: Format,
    sort_type: SortType,
    limit: Option<usize>,
) {
    let file = File::open(input).expect("Error opening input file");
    let prefix = PathBuf::from(prefix);

    let items = format.parse_file(file);
    let filtered_items = items.filter(|p| p.path.starts_with(&prefix));
    let mut stats: HashMap<PathBuf, DirectoryStat> = HashMap::new();

    for stat in filtered_items {
        let unwrapped_path = &stat.path;
        // Only take the 'depth' number of components, thus truncating the path to a the depth
        let relative_path = unwrapped_path.strip_prefix(&prefix).unwrap();
        let base_path = PathBuf::new();
        let relative_paths_with_depth =
            relative_path
                .components()
                .take(depth)
                .scan(base_path, |state, component| {
                    state.push(component);
                    Some(state.to_path_buf())
                });
        for path in relative_paths_with_depth {
            stats
                .entry(path)
                .and_modify(|p| p.merge(&stat))
                .or_insert_with(|| stat.clone());
        }
    }

    let mut table = Table::new();
    table.set_format(*prettytable::format::consts::FORMAT_CLEAN);
    table.set_titles(row![
        "Prefix", "Files", "Size", "created", "accessed", "modified"
    ]);

    let now = chrono::Utc::now();
    let mut stats_vec: Vec<_> = stats.into_iter().collect();

    match sort_type {
        SortType::Name => stats_vec.sort_by_key(|(buf, _stat)| buf.to_path_buf()),
        SortType::Size => stats_vec.sort_by_key(|(_buf, stat)| std::cmp::Reverse(stat.total_size)),
        SortType::Files => stats_vec.sort_by_key(|(_buf, stat)| std::cmp::Reverse(stat.file_count)),
    };

    if let Some(limit) = limit {
        stats_vec.truncate(limit)
    }

    for (key, value) in stats_vec {
        let latest_created = value
            .latest_created
            .map_or_else(|| "Unknown".to_string(), |c| (c - now).humanize());
        let latest_accessed = value
            .latest_accessed
            .map_or_else(|| "Unknown".to_string(), |c| (c - now).humanize());
        let latest_modified = value
            .latest_modified
            .map_or_else(|| "Unknown".to_string(), |c| (c - now).humanize());
        table.add_row(row![
            format!("{}", prefix.as_path().join(key.as_path()).display()),
            value.file_count,
            HumanBytes(value.total_size),
            latest_created,
            latest_accessed,
            latest_modified,
        ]);
    }

    table.printstd();
}

fn get_output_file(path: Option<PathBuf>) -> Box<dyn io::Write> {
    match path {
        None => Box::new(io::stdout()),
        Some(buf) => Box::new(BufWriter::with_capacity(
            1024 * 1024,
            File::create(buf).expect("Error opening the output file"),
        )),
    }
}

pub fn reset_signal_pipe_handler() -> io::Result<()> {
    #[cfg(target_family = "unix")]
    {
        use nix::sys::signal;

        unsafe {
            signal::signal(signal::Signal::SIGPIPE, signal::SigHandler::SigDfl)
                .map_err(|e| io::Error::new(ErrorKind::Other, e))?;
        }
    }

    Ok(())
}
