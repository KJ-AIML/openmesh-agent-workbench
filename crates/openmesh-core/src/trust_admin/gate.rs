//! Query permission evaluation (fail-closed).

use crate::trust_admin::contract::{QueryAllowlistMode, TeamTrustPolicy};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryPermission {
    Allowed,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryPermissionDecision {
    pub permission: QueryPermission,
    pub reason: String,
}

/// Evaluate whether a remote team/mesh query to a target is allowed.
///
/// `target_member_id` and `target_mesh_peer_id` are matched case-insensitively
/// against the allowlist. Fail closed on DenyAll / disabled remote query.
pub fn evaluate_remote_query(
    policy: &TeamTrustPolicy,
    target_member_id: Option<&str>,
    target_mesh_peer_id: Option<&str>,
) -> QueryPermissionDecision {
    if !policy.remote_query_enabled {
        return QueryPermissionDecision {
            permission: QueryPermission::Denied,
            reason: "remote_query_enabled=false".into(),
        };
    }
    match policy.query_allowlist_mode {
        QueryAllowlistMode::DenyAll => QueryPermissionDecision {
            permission: QueryPermission::Denied,
            reason: "query_allowlist_mode=deny-all".into(),
        },
        QueryAllowlistMode::AllowAll => QueryPermissionDecision {
            permission: QueryPermission::Allowed,
            reason: "query_allowlist_mode=allow-all".into(),
        },
        QueryAllowlistMode::AllowlistOnly => {
            let member_key = target_member_id.map(|s| s.trim().to_ascii_lowercase());
            let peer_key = target_mesh_peer_id.map(|s| s.trim().to_ascii_lowercase());
            let hit = policy.query_allowlist.iter().any(|e| {
                let m_ok = match (&e.member_id, &member_key) {
                    (Some(m), Some(k)) => m.to_ascii_lowercase() == *k,
                    _ => false,
                };
                let p_ok = match (&e.mesh_peer_id, &peer_key) {
                    (Some(p), Some(k)) => p.to_ascii_lowercase() == *k,
                    _ => false,
                };
                m_ok || p_ok
            });
            if hit {
                QueryPermissionDecision {
                    permission: QueryPermission::Allowed,
                    reason: "allowlist match".into(),
                }
            } else {
                QueryPermissionDecision {
                    permission: QueryPermission::Denied,
                    reason: "target not on query allowlist".into(),
                }
            }
        }
    }
}
