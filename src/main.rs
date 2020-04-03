use ignore::{DirEntry, Error, ParallelVisitor, ParallelVisitorBuilder, WalkBuilder, WalkState};
use indexmap::IndexSet;
use std::collections::HashMap;
use std::ops::Index;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::SystemTime;
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

    let (tx, rx) = channel();

    let handler = std::thread::spawn(|| {
        let mut io_errors: u64 = 0;
        let mut other_errors: u64 = 0;

        let mut path_components_set: IndexSet<String> = IndexSet::with_capacity(100);
        let mut path_stat: HashMap<Vec<usize>, DirectoryStat> = HashMap::with_capacity(100);

        for result in rx {
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
                    path_components_set.insert("lol ".to_string());
                    println!("{:?}", path_components_set.get_full(&"lol ".to_string()));
                }
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
                Err(e) => {
                    tx_thread.send(WalkResult::OtherError).unwrap();
                }
            }
            WalkState::Continue
        })
    });

    handler.join();
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

#[derive(Default, Debug)]
struct DirectoryStat {
    file_count: u64,
    total_size: u64,

    latest_created: Option<SystemTime>,
    latest_accessed: Option<SystemTime>,
    latest_modified: Option<SystemTime>,
}

impl DirectoryStat {
    // Please oh god tell me this can be generalized.
    pub fn update_last_created(&mut self, created: SystemTime) {
        match self.latest_created {
            Some(latest_created) if latest_created < created => {
                self.latest_created.replace(created);
            }
            None => {
                self.latest_created = Some(created);
            }
            _ => {}
        }
    }

    pub fn update_last_access(&mut self, accessed: SystemTime) {
        match self.latest_accessed {
            Some(latest_accessed) if latest_accessed < accessed => {
                self.latest_accessed.replace(accessed);
            }
            None => {
                self.latest_accessed = Some(accessed);
            }
            _ => {}
        }
    }

    pub fn update_last_modified(&mut self, modified: SystemTime) {
        match self.latest_modified {
            Some(latest_modified) if latest_modified < modified => {
                self.latest_modified.replace(modified);
            }
            None => {
                self.latest_modified = Some(modified);
            }
            _ => {}
        }
    }
}
