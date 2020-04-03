use ignore::{DirEntry, Error, ParallelVisitor, ParallelVisitorBuilder, WalkBuilder, WalkState};
use std::collections::HashMap;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "dirscan", about = "Scan directories.")]
struct Opt {
    // we don't want to name it "speed", need to look smart
    #[structopt(short = "t", long = "threads")]
    threads: Option<usize>,

    #[structopt(parse(from_os_str))]
    input: PathBuf,
}

fn main() {
    let opt: Opt = Opt::from_args();

    let path = opt.input.as_path();

    if !path.is_dir() {
        eprintln!(
            "Error: {} is not a directory or does not exist.",
            path.display()
        );
        std::process::exit(exitcode::USAGE)
    }

    let mut walker = WalkBuilder::new(path);
    walker
        .hidden(false)
        .parents(false)
        .ignore(false)
        .git_ignore(false)
        .git_global(false)
        .git_exclude(false)
        .follow_links(false);

    let threads = opt.threads.unwrap_or_else(num_cpus::get);

    walker.add(PathBuf::from("lol"));
    let mut collection_manager = CollectorManager::new();
    walker
        .threads(threads)
        .build_parallel()
        .visit(&mut collection_manager);
}

struct CollectorManager {}

impl CollectorManager {
    pub fn new() -> CollectorManager {
        CollectorManager {}
    }
}

impl<'a> ParallelVisitorBuilder<'a> for CollectorManager {
    fn build(&mut self) -> Box<dyn ParallelVisitor + 'a> {
        Box::new(Collector::new())
    }
}

struct Collector {
    io_errors: u64,
    other_errors: u64,
    directory_stats: HashMap<PathBuf, DirectoryStat>,
}

impl Collector {
    pub fn new() -> Collector {
        let directory_stats = HashMap::with_capacity(100);
        Collector {
            io_errors: 0,
            other_errors: 0,
            directory_stats,
        }
    }
}

impl ParallelVisitor for Collector {
    fn visit(&mut self, entry: Result<DirEntry, Error>) -> WalkState {
        // unimplemented!()
        println!("{:?} = {:?}", std::thread::current().id(), entry);
        let dir_entry = match entry {
            Ok(dir_entry) => dir_entry,
            Err(Error::Io(_e)) => {
                self.io_errors += 1;
                return WalkState::Continue;
            }
            Err(_e) => {
                self.other_errors += 1;
                return WalkState::Continue;
            }
        };

        // We only care about
        if let Some(file_type) = dir_entry.file_type() {
            if ! file_type.is_file() {
                return WalkState::Continue;
            }
        }

        let entry_path = dir_entry.path();

        let parent_path = match entry_path.parent() {
            Some(parent) => parent.to_path_buf(),
            None => {
                self.other_errors += 1;
                return WalkState::Continue;
            }
        };

        let directory_stat = self
            .directory_stats
            .entry(parent_path)
            .or_insert_with(Default::default);

        directory_stat.file_count += 1;

        if let Ok(metadata) = dir_entry.metadata() {
            directory_stat.total_size += metadata.len();
            // metadata.created()
            // metadata.accessed()
            // metadata.modified()
        }

        WalkState::Continue
    }
}

impl Drop for Collector {
    fn drop(&mut self) {
        println!(
"dropped"

        );
    }
}

#[derive(Default, Debug)]
struct DirectoryStat {
    file_count: u64,
    total_size: u64,
}
