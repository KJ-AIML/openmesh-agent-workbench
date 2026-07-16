//! Local evidence producers (Dev Track 0.1.3.6).

pub mod compose;
pub mod git;
pub mod heli;

pub use compose::{
    collect_git_signal, collect_heli_signal, compose_git_signal, compose_heli_signal,
    map_git_snapshot_to_kind, map_heli_snapshot_to_kind, CollectSignalError, CollectSignalOutcome,
};
pub use git::read_git_snapshot;
pub use heli::read_heli_snapshot;
