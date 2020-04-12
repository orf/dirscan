use filesize::PathExt;

use jwalk::{DirEntryIter, Parallelism};

use std::path::PathBuf;

pub struct Walker {
    threads: usize,
    actual_size: bool,
    ignore_hidden: bool,
    with_size: bool,
    sorted: bool,
}

pub type WalkDir = jwalk::WalkDirGeneric<((), ClientState)>;
pub type WalkDirIter = DirEntryIter<((), ClientState)>;
pub type ClientState = Option<MetadataWithSize>;

impl Walker {
    pub fn new(
        threads: usize,
        actual_size: bool,
        ignore_hidden: bool,
        with_size: bool,
        sorted: bool,
    ) -> Walker {
        Walker {
            threads,
            actual_size,
            ignore_hidden,
            with_size,
            sorted,
        }
    }

    pub fn walk_dir(self, path: &PathBuf) -> WalkDirIter {
        let actual_size = self.actual_size;
        let with_size = self.with_size;
        let sorted = self.sorted;
        WalkDir::new(path)
            .follow_links(false)
            .skip_hidden(self.ignore_hidden)
            .sort(true)
            .process_read_dir(move |_, result| {
                result.retain(|r| r.is_ok());
                // Sort items by their file type - files come first, then directories after.
                if sorted {
                    result.sort_by_key(|f| f.as_ref().unwrap().file_type.is_dir());
                }
                if with_size {
                    result.iter_mut().for_each(|dir_entry_result| {
                        if let Ok(dir_entry) = dir_entry_result {
                            if let Ok(metadata) = dir_entry.metadata() {
                                let is_dir = dir_entry.file_type.is_dir();
                                let file_size = if is_dir {
                                    0
                                } else if actual_size {
                                    match dir_entry.path().size_on_disk_fast(&metadata) {
                                        Ok(size) => size,
                                        Err(_) => metadata.len(),
                                    }
                                } else {
                                    metadata.len()
                                };
                                dir_entry.client_state =
                                    Some(MetadataWithSize::new(metadata, file_size, is_dir))
                            }
                        }
                    });
                }
            })
            .parallelism(Parallelism::RayonNewPool(self.threads))
            .into_iter()
    }
}

#[derive(Debug)]
pub struct MetadataWithSize {
    pub metadata: std::fs::Metadata,
    pub size: u64,
    pub is_dir: bool,
}

impl MetadataWithSize {
    pub fn new(metadata: std::fs::Metadata, size: u64, is_dir: bool) -> MetadataWithSize {
        MetadataWithSize {
            metadata,
            size,
            is_dir,
        }
    }
}
