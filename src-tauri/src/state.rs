use std::{
    collections::HashMap,
    io::Write,
    path::PathBuf,
    process::Child,
    sync::{Mutex, atomic::AtomicU64},
};

use trace_deck_core::{AppPaths, ProfileStore};

pub(crate) use trace_deck_core::TailStreams;

pub(crate) struct AppState {
    pub(crate) paths: AppPaths,
    pub(crate) known_hosts_file: PathBuf,
}

impl AppState {
    pub(crate) fn new(paths: AppPaths) -> Self {
        let known_hosts_file = paths.known_hosts_file();
        Self {
            paths,
            known_hosts_file,
        }
    }

    pub(crate) fn profile_store(&self) -> ProfileStore {
        ProfileStore::new(self.paths.profile_file())
    }
}

pub(crate) struct SystemStreams {
    pub(crate) children: Mutex<HashMap<u64, Child>>,
    pub(crate) next_id: AtomicU64,
}

pub(crate) struct TerminalProcess {
    pub(crate) child: Box<dyn portable_pty::Child + Send + Sync>,
    pub(crate) _master: Box<dyn portable_pty::MasterPty + Send>,
    pub(crate) writer: Box<dyn Write + Send>,
}

pub(crate) struct TerminalSessions {
    pub(crate) processes: Mutex<HashMap<u64, TerminalProcess>>,
    pub(crate) next_id: AtomicU64,
}

impl SystemStreams {
    pub(crate) fn new() -> Self {
        Self {
            children: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }
}

impl TerminalSessions {
    pub(crate) fn new() -> Self {
        Self {
            processes: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        }
    }
}
