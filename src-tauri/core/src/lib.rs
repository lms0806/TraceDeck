//! UI-independent persistence primitives shared by TraceDeck front ends.

mod paths;
mod profiles;
pub mod sftp;
pub mod ssh;

pub use paths::AppPaths;
pub use profiles::{ConnectionProfile, ProfileStore};
pub use sftp::RemoteEntry;
