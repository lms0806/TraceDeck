use std::{
    collections::HashMap,
    path::PathBuf,
    process::Child,
    sync::{Mutex, atomic::AtomicU64},
};

pub(crate) struct AppState {
    pub(crate) profile_file: PathBuf,
    pub(crate) known_hosts_file: PathBuf,
}

pub(crate) struct TailStreams {
    pub(crate) children: Mutex<HashMap<u64, Child>>,
    pub(crate) next_id: AtomicU64,
}

impl TailStreams {
    pub(crate) fn new() -> Self {
        Self {
            children: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }
}
