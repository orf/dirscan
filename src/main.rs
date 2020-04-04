use crate::directory_stat::DirectoryStat;
use console::Style;
use crossbeam_channel::unbounded;
use ignore::{Error, WalkBuilder, WalkState};
use indexmap::IndexSet;
use indicatif::{HumanBytes, ProgressBar, ProgressStyle};
use prettytable::{cell, row, Table};
use serde_json::Deserializer;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};
use structopt::StructOpt;
mod directory_stat;

#[derive(StructOpt)]
#[structopt(name = "dirscan", about = "Scan directories.")]
struct Args {
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(StructOpt)]
enum Command {
    Scan {
        #[structopt(short = "t", long = "threads")]
        threads: Option<usize>,

        #[structopt(short = "i", long = "ignore-hidden", help = "Ignore hidden files")]
        ignore_hidden: bool,

        #[structopt(short = "o", long = "output", parse(from_os_str))]
        output: Option<PathBuf>,
        #[structopt(parse(from_os_str))]
        input: PathBuf,

        #[structopt(
    short = "f",
    long = "format",
    default_value = "json",
    parse(try_from_str = parse_output)
    )]
        format: Format,
    },
    Parse {
        #[structopt(short = "d", long = "depth", default_value = "3")]
        depth: usize,

        #[structopt()]
        prefix: String,
        #[structopt(parse(from_os_str))]
        input: PathBuf,

        #[structopt(
    short = "f",
    long = "format",
    default_value = "json",
    parse(try_from_str = parse_output)
    )]
        format: Format,
    },
}

#[derive(Debug)]
enum Format {
    JSON,
    CSV,
}

fn parse_output(src: &str) -> Result<Format, String> {
    match src.to_lowercase().as_str() {
        "json" => Ok(Format::JSON),
        "csv" => Ok(Format::CSV),
        _ => Err(format!("Invalid format: {}", src)),
    }
}

fn main() -> Result<(), Error> {
    let args: Args = Args::from_args();
    match args.cmd {
        Command::Scan {
            threads,
            ignore_hidden,
            output,
            input,
            format,
        } => scan(input, output, format, ignore_hidden, threads),
        Command::Parse {
            depth,
            prefix,
            input,
            format,
        } => read(input, format, depth, prefix),
    }
}

fn read(input: PathBuf, format: Format, depth: usize, prefix: String) -> Result<(), Error> {
    let file = File::open(input)?;
    let reader = io::BufReader::new(file);

    let stats: Box<dyn Iterator<Item = DirectoryStat>> = match format {
        Format::JSON => Box::new(
            Deserializer::from_reader(reader)
                .into_iter::<DirectoryStat>()
                .map(|f| f.unwrap()),
        ),
        Format::CSV => Box::new(
            csv::Reader::from_reader(reader)
                .into_deserialize::<DirectoryStat>()
                .map(|f| f.unwrap()),
        ),
    };

    let pbar = ProgressBar::new_spinner();
    pbar.enable_steady_tick(Duration::from_secs(1).as_millis() as u64);
    pbar.set_draw_delta(10_000);
    pbar.set_style(
        ProgressStyle::default_spinner().template(
            "[{elapsed_precise}] Total: {pos:.cyan/blue} | Per sec: {per_sec:.cyan/blue} ",
        ),
    );

    let filtered_stats = pbar.wrap_iter(stats).filter(|r| {
        let path = r.path.as_ref().unwrap();
        path.starts_with(&prefix)
    });

    let mut stats: HashMap<PathBuf, DirectoryStat> = HashMap::new();

    for mut stat in filtered_stats {
        let unwrapped_path = stat.path.unwrap();
        // This is a bit of a mess. We set it to None to avoid some borrow checker issues.
        // This needs some refactoring!
        stat.path = None;

        let relative_path = unwrapped_path.strip_prefix(&prefix).unwrap();
        // Only take the 'depth' number of components, thus truncating the path to a the depth
        let relative_path_with_depth: PathBuf = relative_path.components().take(depth).collect();
        stats
            .entry(relative_path_with_depth)
            .and_modify(|p| p.merge(&stat))
            .or_insert(stat);
        // println!("Rel: {:?}", relative_path);
        // println!("depth: {:?}", relative_path_with_depth);
    }

    let mut table = Table::new();
    table.add_row(row![
        "Prefix", "Files", "Size", "created", "accessed", "modified"
    ]);

    let now = chrono::Utc::now();
    let formatter = timeago::Formatter::new();

    let mut stats_vec: Vec<_> = stats.into_iter().collect();
    stats_vec.sort_by_key(|f| std::cmp::Reverse(f.1.total_size));

    for (key, value) in stats_vec {
        table.add_row(row![
            format!("{}{}", prefix, key.as_path().display()),
            value.file_count,
            HumanBytes(value.total_size),
            formatter.convert_chrono(value.latest_created, now),
            formatter.convert_chrono(value.latest_accessed, now),
            formatter.convert_chrono(value.latest_modified, now)
        ]);
    }

    table.printstd();
    Ok(())
}

fn scan(
    input: PathBuf,
    output: Option<PathBuf>,
    output_format: Format,
    ignore_hidden: bool,
    threads: Option<usize>,
) -> Result<(), Error> {
    let path = input.as_path();
    let threads = threads.unwrap_or_else(num_cpus::get);

    if !path.is_dir() {
        eprintln!(
            "Error: {} is not a directory or does not exist.",
            path.display()
        );
        std::process::exit(exitcode::USAGE)
    }

    let mut walker = WalkBuilder::new(path);
    walker
        .hidden(ignore_hidden)
        .parents(false)
        .ignore(false)
        .git_ignore(false)
        .git_global(false)
        .git_exclude(false)
        .follow_links(false);

    let (tx, rx) = unbounded();

    let handler = std::thread::spawn(move || {
        let mut io_errors: u64 = 0;
        let mut other_errors: u64 = 0;
        let mut total_size: u64 = 0;

        let mut path_components_set: IndexSet<OsString> = IndexSet::with_capacity(100);
        let mut path_stat: HashMap<Vec<usize>, DirectoryStat> = HashMap::with_capacity(100);

        let update_every = Duration::from_millis(250);
        let mut last_update = Instant::now();

        let red_style = Style::new().red();
        let blue_style = Style::new().blue();
        let green_style = Style::new().green();

        let pbar = ProgressBar::new_spinner();
        pbar.enable_steady_tick((update_every.as_millis() + 50) as u64);
        pbar.set_draw_delta(100_000);
        pbar.set_style(
            ProgressStyle::default_spinner()
                .template("[{elapsed_precise}] Files/s: {per_sec:.cyan/blue} | Total: {pos:.green/green} | {msg}"),
        );

        for result in pbar.wrap_iter(rx.iter()) {
            if last_update.elapsed() > update_every {
                last_update = Instant::now();
                let total_components = path_components_set.len();
                let total_directories = path_stat.len();
                let percentage_components =
                    ((total_components as f64 / total_directories as f64) * 100 as f64) as i32;
                let percentage_display = if percentage_components < 50 {
                    green_style.apply_to(percentage_components)
                } else {
                    red_style.apply_to(percentage_components)
                };
                let msg = format!(
                    "Directories: {} | Size: {} | Components: {} ({}%) | Errors: IO={} Other={}",
                    green_style.apply_to(total_directories),
                    green_style.apply_to(HumanBytes(total_size)),
                    blue_style.apply_to(total_components),
                    percentage_display,
                    red_style.apply_to(io_errors),
                    red_style.apply_to(other_errors),
                );
                pbar.set_message(msg.as_str());
            }

            match result {
                WalkResult::IOError => io_errors += 1,
                WalkResult::OtherError => other_errors += 1,
                WalkResult::File {
                    created,
                    accessed,
                    modified,
                    size,
                    parent,
                } => {
                    total_size += size;
                    let path_integers: Vec<usize> = parent
                        .components()
                        .map(|c| {
                            let component = c.as_os_str().to_os_string();
                            let (idx, _) = path_components_set.insert_full(component);
                            idx
                        })
                        .collect();
                    path_stat
                        .entry(path_integers)
                        .and_modify(|s| {
                            s.total_size += size;
                            s.file_count += 1;
                            s.update_last_access(accessed);
                            s.update_last_created(created);
                            s.update_last_modified(modified);
                        })
                        .or_insert_with(|| DirectoryStat::new(size, created, accessed, modified));
                }
            }
        }

        let stat_enumerator = path_stat.into_iter().map(|(key, mut dir_stat)| {
            dir_stat.path = Some(
                key.into_iter()
                    .map(|i| path_components_set.get_index(i).unwrap())
                    .collect(),
            );
            dir_stat
        });

        let mut output_file = get_writer(output);
        match output_format {
            Format::JSON => {
                stat_enumerator.for_each(|s| {
                    let res = &serde_json::to_vec(&s).expect("Error serializing to JSON");
                    output_file
                        .write_all(res)
                        .expect("Error writing to output file");
                    writeln!(output_file).expect("error writing newline");
                });
            }
            Format::CSV => {
                let mut wtr = csv::WriterBuilder::new()
                    .has_headers(true)
                    .from_writer(output_file);
                stat_enumerator.for_each(|s| {
                    wtr.serialize(s).expect("Error serializing to CSV");
                })
            }
        }
    });

    walker.threads(threads).build_parallel().run(move || {
        let tx_thread = tx.clone();
        Box::new(move |entry| {
            let dir_entry = match entry {
                Err(Error::Io(_e)) => {
                    tx_thread.send(WalkResult::IOError).unwrap();
                    return WalkState::Continue;
                }
                Err(_e) => {
                    tx_thread.send(WalkResult::OtherError).unwrap();
                    return WalkState::Continue;
                }
                Ok(dir_entry) if dir_entry.depth() == 0 => {
                    return WalkState::Continue;
                }
                Ok(dir_entry) => dir_entry,
            };
            if let Some(file_type) = dir_entry.file_type() {
                if !file_type.is_file() {
                    return WalkState::Continue;
                }
            }

            let entry_path = dir_entry.path();
            let parent_path = match entry_path.parent() {
                Some(parent) => parent.to_path_buf(),
                None => {
                    tx_thread.send(WalkResult::OtherError).unwrap();
                    return WalkState::Continue;
                }
            };
            match dir_entry.metadata() {
                Ok(metadata) => {
                    tx_thread
                        .send(WalkResult::File {
                            created: metadata.created().ok(),
                            accessed: metadata.accessed().ok(),
                            modified: metadata.modified().ok(),
                            size: metadata.len(),
                            parent: parent_path,
                        })
                        .unwrap();
                }
                Err(_e) => {
                    tx_thread.send(WalkResult::OtherError).unwrap();
                }
            }
            WalkState::Continue
        })
    });

    handler.join().expect("Error in thread");
    Ok(())
}

#[derive(Debug)]
enum WalkResult {
    IOError,
    OtherError,
    File {
        created: Option<SystemTime>,
        accessed: Option<SystemTime>,
        modified: Option<SystemTime>,
        size: u64,
        parent: PathBuf,
    },
}

fn get_writer(path: Option<PathBuf>) -> Box<dyn io::Write> {
    match path {
        None => Box::new(io::stdout()),
        Some(buf) => Box::new(
            OpenOptions::new()
                .write(true)
                .read(false)
                .create(true)
                .create_new(true)
                .open(buf)
                .expect("Error opening output file"),
        ),
    }
}
