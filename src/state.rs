use crate::walker::MetadataWithSize;

use crate::formats::FormatWriter;

use crate::directory_stat::DirectoryStat;
use std::path::PathBuf;

pub struct WalkState {
    current: Option<DirectoryStat>,
    writer: Box<dyn FormatWriter>,
    depth: Option<usize>,
}

impl WalkState {
    pub fn new(writer: Box<dyn FormatWriter>, depth: Option<usize>) -> WalkState {
        WalkState {
            current: None,
            writer,
            depth,
        }
    }

    fn is_equivalent_path(root: &PathBuf, target: &PathBuf, depth: Option<usize>) -> bool {
        // Are these two directory paths the same, or given a depth are the first N
        // components the same?
        match depth {
            None => root == target,
            Some(depth) => root
                .components()
                .take(depth)
                .eq(target.components().take(depth)),
        }
    }

    pub fn add_path(&mut self, path: PathBuf, metadata: &MetadataWithSize) {
        // println!("{} - {}", path.display(), path.is_dir());
        match &mut self.current {
            None => {
                self.current = Some(DirectoryStat::from_metadata(path, metadata));
            }
            Some(stat) if WalkState::is_equivalent_path(&stat.path, &path, self.depth) => {
                // Same directory, update in place
                if !metadata.is_dir {
                    stat.total_size += metadata.size;
                    stat.file_count += 1;
                    if metadata.size > stat.largest_file_size {
                        stat.largest_file_size = metadata.size
                    }
                    if let Ok(created) = metadata.metadata.created() {
                        stat.update_latest_created(created.into());
                    }
                    if let Ok(accessed) = metadata.metadata.accessed() {
                        stat.update_latest_accessed(accessed.into());
                    }
                    if let Ok(modified) = metadata.metadata.modified() {
                        stat.update_latest_modified(modified.into());
                    }
                }
            }
            Some(stat) => {
                // New directory! Write the current directory to the output file
                self.writer
                    .write_stat(stat)
                    .expect("Error writing directory statistic");
                self.current = Some(DirectoryStat::from_metadata(path, metadata));
            }
        }
    }
}

impl Drop for WalkState {
    fn drop(&mut self) {
        if let Some(stat) = &self.current {
            self.writer
                .write_stat(stat)
                .expect("Error writing directory statistic");
        }
    }
}
