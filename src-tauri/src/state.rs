use std::path::PathBuf;

use trace_deck_core::{AppPaths, ProfileStore};

pub(crate) use trace_deck_core::{SystemStreams, TailStreams, TerminalSessions};

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
