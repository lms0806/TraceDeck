//! UI-independent persistence primitives shared by TraceDeck front ends.

pub mod logs;
mod paths;
mod profiles;
pub mod sftp;
pub mod ssh;
pub mod system;

pub use logs::{LogSource, RemoteFile, TailChunk, TailStreams};
pub use paths::AppPaths;
pub use profiles::{ConnectionProfile, ProfileStore};
pub use sftp::RemoteEntry;
pub use system::{RemoteSystemStatus, SystemStatusChunk, SystemStreams};
