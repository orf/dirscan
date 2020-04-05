use crate::walker::MetadataWithSize;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone)]
pub struct DirectoryStat {
    pub total_size: u64,
    pub file_count: u64,
    pub path: PathBuf,

    pub latest_created: Option<DateTime<Utc>>,
    pub latest_accessed: Option<DateTime<Utc>>,
    pub latest_modified: Option<DateTime<Utc>>,
}

impl DirectoryStat {
    pub fn from_metadata(path: PathBuf, metadata: &MetadataWithSize) -> DirectoryStat {
        let file_count = if metadata.is_dir { 0 } else { 1 };
        let total_size = metadata.size;

        DirectoryStat {
            total_size,
            file_count,
            path,

            latest_created: metadata.metadata.created().map(|f| f.into()).ok(),
            latest_accessed: metadata.metadata.accessed().map(|f| f.into()).ok(),
            latest_modified: metadata.metadata.modified().map(|f| f.into()).ok(),
        }
    }

    pub fn merge(&mut self, other: &DirectoryStat) {
        self.total_size += other.total_size;
        self.file_count += other.file_count;
        if let Some(created) = other.latest_created {
            self.update_latest_created(created);
        }
        if let Some(accessed) = other.latest_accessed {
            self.update_latest_accessed(accessed);
        }
        if let Some(modified) = other.latest_modified {
            self.update_latest_modified(modified);
        }
    }

    // Please oh god tell me this can be generalized somehow.
    pub fn update_latest_created(&mut self, created: DateTime<Utc>) {
        match self.latest_created {
            None => {
                self.latest_created.replace(created);
            }
            Some(dt) if dt < created => {
                self.latest_created.replace(created);
            }
            _ => {}
        }
    }

    pub fn update_latest_accessed(&mut self, accessed: DateTime<Utc>) {
        match self.latest_accessed {
            None => {
                self.latest_accessed.replace(accessed);
            }
            Some(dt) if dt < accessed => {
                self.latest_accessed.replace(accessed);
            }
            _ => {}
        }
    }

    pub fn update_latest_modified(&mut self, modified: DateTime<Utc>) {
        match self.latest_modified {
            None => {
                self.latest_modified.replace(modified);
            }
            Some(dt) if dt < modified => {
                self.latest_modified.replace(modified);
            }
            _ => {}
        }
    }
}
