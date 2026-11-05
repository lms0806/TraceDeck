use std::{
    env, fs,
    path::{Path, PathBuf},
};

/// Files TraceDeck owns in a platform-appropriate per-user data directory.
#[derive(Debug, Clone)]
pub struct AppPaths {
    data_dir: PathBuf,
}

impl AppPaths {
    pub fn from_data_dir(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    /// Resolves TraceDeck's per-user application data directory without a UI framework.
    pub fn for_current_user() -> Result<Self, String> {
        let data_dir = if cfg!(target_os = "windows") {
            env::var_os("APPDATA")
                .map(PathBuf::from)
                .ok_or("APPDATA 환경 변수를 찾을 수 없습니다.")?
                .join("TraceDeck")
        } else if cfg!(target_os = "macos") {
            env::var_os("HOME")
                .map(PathBuf::from)
                .ok_or("HOME 환경 변수를 찾을 수 없습니다.")?
                .join("Library")
                .join("Application Support")
                .join("TraceDeck")
        } else {
            env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| {
                    env::var_os("HOME")
                        .map(PathBuf::from)
                        .map(|home| home.join(".local").join("share"))
                })
                .ok_or("XDG_DATA_HOME 또는 HOME 환경 변수를 찾을 수 없습니다.")?
                .join("TraceDeck")
        };
        Ok(Self::from_data_dir(data_dir))
    }

    pub fn ensure_exists(&self) -> Result<(), String> {
        fs::create_dir_all(&self.data_dir).map_err(|error| error.to_string())
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn profile_file(&self) -> PathBuf {
        self.data_dir.join("profiles.json")
    }

    pub fn known_hosts_file(&self) -> PathBuf {
        self.data_dir.join("known_hosts")
    }
}
