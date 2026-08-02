//! Dev Track 0.1.16 — Team Cloud Beta.
//!
//! Team-scoped always-online / cloud-tier scaffold (local-sim first).
//! Selective sync only; not multi-tenant multi-region SaaS.

pub mod contract;
pub mod storage;
pub mod sync;

pub use contract::{
    validate_team_cloud_config, validate_team_cloud_sync_plan, TeamCloudConfig, TeamCloudMode,
    TeamCloudSyncPlan, TEAM_CLOUD_DIR, TEAM_CLOUD_PROTOCOL_VERSION,
};
pub use storage::{
    init_team_cloud, read_team_cloud, team_cloud_dir, write_team_cloud, TeamCloudStorageError,
};
pub use sync::{build_sync_scaffold, TeamCloudSyncError};
