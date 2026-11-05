//! UI-independent persistence primitives shared by TraceDeck front ends.

mod paths;
mod profiles;

pub use paths::AppPaths;
pub use profiles::{ConnectionProfile, ProfileStore};
