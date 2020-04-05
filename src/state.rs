use crate::walker::MetadataWithSize;

use crate::formats::FormatWriter;

use crate::directory_stat::DirectoryStat;
use std::path::PathBuf;

pub struct WalkState {
    current: Option<DirectoryStat>,
    writer: Box<dyn FormatWriter>,
}

impl WalkState {
    pub fn new(writer: Box<dyn FormatWriter>) -> WalkState {
        WalkState {
            current: None,
            writer,
        }
    }

    pub fn add_path(&mut self, path: PathBuf, metadata: &MetadataWithSize) {
        // println!("{} - {}", path.display(), path.is_dir());
        match &mut self.current {
            None => {
                self.current = Some(DirectoryStat::from_metadata(path, metadata));
            }
            Some(stat) if stat.path == path => {
                // Same directory, update in place
                if !metadata.is_dir {
                    stat.total_size += metadata.size;
                    stat.file_count += 1;
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
        match &self.current {
            Some(stat) => {
                self.writer
                    .write_stat(stat)
                    .expect("Error writing directory statistic");
            }
            _ => {}
        }
    }
}
