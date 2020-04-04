use chrono::prelude::*;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Serialize, Deserialize, Clone)]
pub struct DirectoryStat {
    pub file_count: u64,
    pub total_size: u64,
    pub path: Option<PathBuf>,

    pub latest_created: DateTime<Utc>,
    pub latest_accessed: DateTime<Utc>,
    pub latest_modified: DateTime<Utc>,
}

// Sometimes there are weird dates, like 2098,1,1. Just filter them out here.
// There is most likely a better way, but for now it works.
lazy_static! {
    static ref BLACKLISTED_DATES: [Date<Utc>; 1] = { [Utc.ymd(2098, 1, 1)] };
}

impl DirectoryStat {
    pub fn new(
        total_size: u64,
        created: Option<SystemTime>,
        accessed: Option<SystemTime>,
        modified: Option<SystemTime>,
    ) -> DirectoryStat {
        DirectoryStat {
            file_count: 1,
            latest_created: created.unwrap_or(SystemTime::UNIX_EPOCH).into(),
            latest_accessed: accessed.unwrap_or(SystemTime::UNIX_EPOCH).into(),
            latest_modified: modified.unwrap_or(SystemTime::UNIX_EPOCH).into(),
            total_size,
            path: None,
        }
    }

    /// Merge another DirectoryStat into this one
    pub fn merge(&mut self, other: &DirectoryStat) {
        self.total_size += other.total_size;
        self.file_count += other.file_count;
        // This is nasty, but whatever
        if self.latest_created < other.latest_created
            && !BLACKLISTED_DATES
                .iter()
                .any(|d| other.latest_created.date() == *d)
        {
            self.latest_created = other.latest_created;
        }
        if self.latest_accessed < other.latest_accessed
            && !BLACKLISTED_DATES
                .iter()
                .any(|d| other.latest_accessed.date() == *d)
        {
            self.latest_accessed = other.latest_accessed;
        }
        if self.latest_modified < other.latest_modified
            && !BLACKLISTED_DATES
                .iter()
                .any(|d| other.latest_modified.date() == *d)
        {
            self.latest_modified = other.latest_modified;
        }
    }

    // Please oh god tell me this can be generalized somehow.
    pub fn update_last_created(&mut self, created_option: Option<SystemTime>) {
        if let Some(created) = created_option {
            let created_dt = created.into();
            if self.latest_created < created_dt {
                self.latest_created = created_dt;
            }
        }
    }

    pub fn update_last_access(&mut self, accessed_option: Option<SystemTime>) {
        if let Some(accessed) = accessed_option {
            let accessed_dt = accessed.into();
            if self.latest_accessed < accessed_dt {
                self.latest_accessed = accessed_dt;
            }
        }
    }

    pub fn update_last_modified(&mut self, modified_option: Option<SystemTime>) {
        if let Some(modified) = modified_option {
            let modified_dt = modified.into();
            if self.latest_modified < modified_dt {
                self.latest_modified = modified_dt;
            }
        }
    }
}
