//! Dev Track 0.1.15 — Team Workspace Foundation.
//!
//! Team identity + multi-member registry linked to mesh peers / proxy profiles.
//! Not enterprise admin; not multi-region cloud.

pub mod contract;
pub mod storage;

pub use contract::{
    validate_team_member, validate_team_workspace, TeamMember, TeamMemberRole, TeamWorkspace,
    TEAM_DIR, TEAM_PROTOCOL_VERSION,
};
pub use storage::{
    add_team_member, init_team_workspace, list_team_members, read_team_workspace,
    remove_team_member, team_dir, write_team_workspace, TeamStorageError,
};
