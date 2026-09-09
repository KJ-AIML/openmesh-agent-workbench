use super::events::validate_utc_timestamp;
use serde::{Deserialize, Serialize};

// ============================================================================
// Dev Track 0.1.4 Checkpoint A — Work Proxy Profile domain contracts (pure, no I/O)
// ============================================================================

/// Wire-schema version for the My Work Proxy Profile (Dev Track 0.1.4).
pub const WORK_PROXY_PROFILE_VERSION: &str = "1.0";

pub const MAX_PROFILE_ID_BYTES: usize = 256;
pub const MAX_PROFILE_LABEL_BYTES: usize = 512;
pub const MAX_PROFILE_TEXT_BYTES: usize = 4096;
pub const MAX_PROFILE_RULES: usize = 64;
pub const MAX_PROFILE_LIST_ITEMS: usize = 64;
pub const MAX_PROFILE_LIMITATIONS: usize = 32;
pub const MAX_PROFILE_LIMITATION_BYTES: usize = 512;

/// Returns true when `version` is a supported on-disk Work Proxy Profile protocol.
pub fn is_supported_work_proxy_profile_version(version: &str) -> bool {
    version == WORK_PROXY_PROFILE_VERSION
}

/// Product Bible §13 authority ladder — policy states only; no answering behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProxyAuthorityLevel {
    CanAnswer,
    CanSuggest,
    CanDraft,
    MustAskHuman,
    CannotAnswer,
}

/// Privacy sensitivity for profile privacy rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrivacySensitivity {
    Public,
    Internal,
    Private,
    Sensitive,
    Secret,
}

/// How a topic may be used by future proxy behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrivacyAllowedUse {
    ReferenceOnly,
    SummarizeWithCaution,
    ExcludeFromAnswers,
}

/// Behavior when a claim lacks evidence support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnsupportedClaimBehavior {
    Refuse,
    AskHuman,
    SayUnknown,
}

/// Evidence source kinds the profile may cite in future proxy answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceSourceKind {
    FilePath,
    ProducerSignal,
    GitState,
    WorkEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommunicationPreferences {
    pub tone: String,
    pub detail_level: String,
    pub async_preference: String,
    pub correction_preference: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DecisionPreferences {
    pub decision_style: String,
    pub escalation_preference: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthorityRule {
    pub rule_id: String,
    pub scope: String,
    pub authority: ProxyAuthorityLevel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub conditions: Vec<String>,
    pub evidence_required: bool,
    pub human_confirmation_required: bool,
    #[serde(default)]
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrivacyRule {
    pub rule_id: String,
    pub topic: String,
    pub sensitivity: PrivacySensitivity,
    pub allowed_use: PrivacyAllowedUse,
    pub restriction: String,
    pub requires_human_confirmation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DefaultRefusalRule {
    pub rule_id: String,
    pub statement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidencePolicy {
    pub answer_without_evidence: bool,
    pub require_evidence_for_claims: bool,
    pub expose_limitations: bool,
    pub cite_source_kinds: Vec<EvidenceSourceKind>,
    pub unsupported_claim_behavior: UnsupportedClaimBehavior,
}

/// Local Work Proxy identity + authority/policy foundation (metadata only; no answering).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkProxyProfile {
    pub profile_id: String,
    pub workspace_id: String,
    pub owner_label: String,
    pub role_label: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub working_style: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub communication_style: String,
    pub communication_preferences: CommunicationPreferences,
    pub decision_preferences: DecisionPreferences,
    pub authority_rules: Vec<AuthorityRule>,
    #[serde(default)]
    pub privacy_rules: Vec<PrivacyRule>,
    #[serde(default)]
    pub sensitive_topics: Vec<String>,
    pub default_refusal_rules: Vec<DefaultRefusalRule>,
    pub evidence_policy: EvidencePolicy,
    pub limitations: Vec<String>,
    pub created_at: String,
    pub last_updated_at: String,
    pub profile_version: String,
}

/// Deterministic local profile id for a project workspace.
pub fn deterministic_work_proxy_profile_id(workspace_id: &str) -> String {
    format!("profile-{workspace_id}")
}

/// Conservative Work Proxy Profile defaults for `profile init` (no `can-answer` rules).
pub fn default_work_proxy_profile(
    workspace_id: impl Into<String>,
    profile_id: impl Into<String>,
    owner_label: impl Into<String>,
    role_label: impl Into<String>,
    timestamp: impl Into<String>,
) -> WorkProxyProfile {
    let timestamp = timestamp.into();
    WorkProxyProfile {
        profile_id: profile_id.into(),
        workspace_id: workspace_id.into(),
        owner_label: owner_label.into(),
        role_label: role_label.into(),
        working_style: String::new(),
        communication_style: String::new(),
        communication_preferences: CommunicationPreferences {
            tone: "direct".into(),
            detail_level: "medium".into(),
            async_preference: "prefer-async".into(),
            correction_preference: "surface-limitations".into(),
        },
        decision_preferences: DecisionPreferences {
            decision_style: "evidence-first".into(),
            escalation_preference: "ask-human-on-ambiguity".into(),
        },
        authority_rules: vec![AuthorityRule {
            rule_id: "rule-global-default".into(),
            scope: "*".into(),
            authority: ProxyAuthorityLevel::MustAskHuman,
            description: Some("Default safe baseline for unmatched scopes".into()),
            conditions: vec![],
            evidence_required: true,
            human_confirmation_required: true,
            limitations: vec!["proxy does not decide alone".into()],
        }],
        privacy_rules: vec![PrivacyRule {
            rule_id: "privacy-credentials-default".into(),
            topic: "credentials".into(),
            sensitivity: PrivacySensitivity::Secret,
            allowed_use: PrivacyAllowedUse::ExcludeFromAnswers,
            restriction: "never include in proxy output".into(),
            requires_human_confirmation: true,
        }],
        sensitive_topics: vec!["credentials".into()],
        default_refusal_rules: vec![
            DefaultRefusalRule {
                rule_id: "refusal-no-impersonation".into(),
                statement: "cannot impersonate owner".into(),
            },
            DefaultRefusalRule {
                rule_id: "refusal-no-irreversible-approval".into(),
                statement: "cannot approve irreversible actions".into(),
            },
            DefaultRefusalRule {
                rule_id: "refusal-no-sensitive-disclosure".into(),
                statement: "cannot disclose sensitive data".into(),
            },
            DefaultRefusalRule {
                rule_id: "refusal-no-invented-evidence".into(),
                statement: "cannot invent evidence".into(),
            },
            DefaultRefusalRule {
                rule_id: "refusal-no-outside-authority".into(),
                statement: "cannot answer outside authority".into(),
            },
        ],
        evidence_policy: EvidencePolicy {
            answer_without_evidence: false,
            require_evidence_for_claims: true,
            expose_limitations: true,
            cite_source_kinds: vec![EvidenceSourceKind::FilePath, EvidenceSourceKind::WorkEvent],
            unsupported_claim_behavior: UnsupportedClaimBehavior::SayUnknown,
        },
        limitations: vec![
            "local policy profile metadata only".into(),
            "no answering runtime in 0.1.4".into(),
        ],
        created_at: timestamp.clone(),
        last_updated_at: timestamp,
        profile_version: WORK_PROXY_PROFILE_VERSION.to_string(),
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProfileValidationError {
    #[error("unsupported profile_version {found}; accepted version is 1.0")]
    UnsupportedProfileVersion { found: String },
    #[error("profile_id is empty after trim")]
    EmptyProfileId,
    #[error("profile_id exceeds the {max}-byte bound")]
    ProfileIdTooLong { max: usize },
    #[error("workspace_id is empty after trim")]
    EmptyWorkspaceId,
    #[error("owner_label is empty after trim")]
    EmptyOwnerLabel,
    #[error("owner_label exceeds the {max}-byte bound")]
    OwnerLabelTooLong { max: usize },
    #[error("role_label exceeds the {max}-byte bound")]
    RoleLabelTooLong { max: usize },
    #[error("text field exceeds the {max}-byte bound")]
    TextTooLong { max: usize },
    #[error("timestamp is invalid: {0}")]
    InvalidTimestamp(String),
    #[error("created_at must be <= last_updated_at")]
    CreatedAfterLastUpdated,
    #[error("at least one authority rule is required")]
    MissingAuthorityRules,
    #[error("at least one default refusal rule is required")]
    MissingDefaultRefusalRules,
    #[error("limitations must not be empty")]
    EmptyLimitations,
    #[error("limitations exceed the {max} entry bound")]
    TooManyLimitations { max: usize },
    #[error("list exceeds the {max} entry bound")]
    TooManyListItems { max: usize },
    #[error("profile contains an impersonation claim")]
    ImpersonationClaim,
    #[error("profile contains a secret-like value")]
    SecretLikeValue,
    #[error("authority rule is invalid: {0}")]
    InvalidAuthorityRule(String),
    #[error("privacy rule is invalid: {0}")]
    InvalidPrivacyRule(String),
    #[error("evidence policy is invalid: {0}")]
    InvalidEvidencePolicy(String),
    #[error("conflicting profile policy: {0}")]
    ConflictingProfilePolicy(String),
    #[error("profile must include a no-impersonation default refusal rule")]
    MissingNoImpersonationRefusal,
    #[error("irreversible action scope requires human confirmation")]
    IrreversibleActionWithoutConfirmation,
    #[error("secret or sensitive privacy rule requires an explicit restriction")]
    SecretTopicWithoutRestriction,
}

/// Validate a single `AuthorityRule`.
pub fn validate_authority_rule(rule: &AuthorityRule) -> Result<(), ProfileValidationError> {
    validate_profile_id_field(&rule.rule_id, "rule_id")?;
    validate_bounded_text(&rule.scope, MAX_PROFILE_TEXT_BYTES)?;
    if let Some(description) = &rule.description {
        validate_bounded_text(description, MAX_PROFILE_TEXT_BYTES)?;
    }
    if rule.conditions.len() > MAX_PROFILE_LIST_ITEMS {
        return Err(ProfileValidationError::TooManyListItems {
            max: MAX_PROFILE_LIST_ITEMS,
        });
    }
    for condition in &rule.conditions {
        validate_bounded_text(condition, MAX_PROFILE_TEXT_BYTES)?;
    }
    if rule.limitations.len() > MAX_PROFILE_LIMITATIONS {
        return Err(ProfileValidationError::TooManyLimitations {
            max: MAX_PROFILE_LIMITATIONS,
        });
    }
    for limitation in &rule.limitations {
        validate_profile_limitation_text(limitation)?;
    }
    Ok(())
}

/// Validate a single `PrivacyRule`.
pub fn validate_privacy_rule(rule: &PrivacyRule) -> Result<(), ProfileValidationError> {
    validate_profile_id_field(&rule.rule_id, "rule_id")?;
    validate_bounded_text(&rule.topic, MAX_PROFILE_TEXT_BYTES)?;
    validate_bounded_text(&rule.restriction, MAX_PROFILE_TEXT_BYTES)?;
    if contains_secret_like_value(&rule.topic) || contains_secret_like_value(&rule.restriction) {
        return Err(ProfileValidationError::SecretLikeValue);
    }
    Ok(())
}

/// Validate an `EvidencePolicy`.
pub fn validate_evidence_policy(policy: &EvidencePolicy) -> Result<(), ProfileValidationError> {
    if policy.require_evidence_for_claims && policy.answer_without_evidence {
        return Err(ProfileValidationError::InvalidEvidencePolicy(
            "require_evidence_for_claims cannot be true when answer_without_evidence is true"
                .into(),
        ));
    }
    if policy.cite_source_kinds.len() > MAX_PROFILE_LIST_ITEMS {
        return Err(ProfileValidationError::TooManyListItems {
            max: MAX_PROFILE_LIST_ITEMS,
        });
    }
    Ok(())
}

/// Shared semantic validation for Work Proxy Profile records.
pub fn validate_work_proxy_profile(
    profile: &WorkProxyProfile,
) -> Result<(), ProfileValidationError> {
    if !is_supported_work_proxy_profile_version(&profile.profile_version) {
        return Err(ProfileValidationError::UnsupportedProfileVersion {
            found: profile.profile_version.clone(),
        });
    }
    validate_profile_id_field(&profile.profile_id, "profile_id")?;
    if profile.workspace_id.trim().is_empty() {
        return Err(ProfileValidationError::EmptyWorkspaceId);
    }
    if profile.owner_label.trim().is_empty() {
        return Err(ProfileValidationError::EmptyOwnerLabel);
    }
    if profile.owner_label.len() > MAX_PROFILE_LABEL_BYTES {
        return Err(ProfileValidationError::OwnerLabelTooLong {
            max: MAX_PROFILE_LABEL_BYTES,
        });
    }
    if profile.role_label.len() > MAX_PROFILE_LABEL_BYTES {
        return Err(ProfileValidationError::RoleLabelTooLong {
            max: MAX_PROFILE_LABEL_BYTES,
        });
    }
    validate_bounded_text(&profile.working_style, MAX_PROFILE_TEXT_BYTES)?;
    validate_bounded_text(&profile.communication_style, MAX_PROFILE_TEXT_BYTES)?;
    validate_communication_preferences(&profile.communication_preferences)?;
    validate_decision_preferences(&profile.decision_preferences)?;

    if profile.authority_rules.is_empty() {
        return Err(ProfileValidationError::MissingAuthorityRules);
    }
    if profile.authority_rules.len() > MAX_PROFILE_RULES {
        return Err(ProfileValidationError::TooManyListItems {
            max: MAX_PROFILE_RULES,
        });
    }
    for rule in &profile.authority_rules {
        validate_authority_rule(rule)?;
    }

    if profile.privacy_rules.len() > MAX_PROFILE_RULES {
        return Err(ProfileValidationError::TooManyListItems {
            max: MAX_PROFILE_RULES,
        });
    }
    for rule in &profile.privacy_rules {
        validate_privacy_rule(rule)?;
    }

    if profile.sensitive_topics.len() > MAX_PROFILE_LIST_ITEMS {
        return Err(ProfileValidationError::TooManyListItems {
            max: MAX_PROFILE_LIST_ITEMS,
        });
    }
    for topic in &profile.sensitive_topics {
        validate_bounded_text(topic, MAX_PROFILE_TEXT_BYTES)?;
        if contains_secret_like_value(topic) {
            return Err(ProfileValidationError::SecretLikeValue);
        }
    }

    if profile.default_refusal_rules.is_empty() {
        return Err(ProfileValidationError::MissingDefaultRefusalRules);
    }
    if profile.default_refusal_rules.len() > MAX_PROFILE_RULES {
        return Err(ProfileValidationError::TooManyListItems {
            max: MAX_PROFILE_RULES,
        });
    }
    for rule in &profile.default_refusal_rules {
        validate_profile_id_field(&rule.rule_id, "refusal rule_id")?;
        validate_bounded_text(&rule.statement, MAX_PROFILE_TEXT_BYTES)?;
    }

    validate_evidence_policy(&profile.evidence_policy)?;

    if profile.limitations.is_empty() {
        return Err(ProfileValidationError::EmptyLimitations);
    }
    if profile.limitations.len() > MAX_PROFILE_LIMITATIONS {
        return Err(ProfileValidationError::TooManyLimitations {
            max: MAX_PROFILE_LIMITATIONS,
        });
    }
    for limitation in &profile.limitations {
        validate_profile_limitation_text(limitation)?;
    }

    validate_utc_timestamp(&profile.created_at)
        .map_err(ProfileValidationError::InvalidTimestamp)?;
    validate_utc_timestamp(&profile.last_updated_at)
        .map_err(ProfileValidationError::InvalidTimestamp)?;
    let created = chrono::DateTime::parse_from_rfc3339(&profile.created_at)
        .map_err(|err| ProfileValidationError::InvalidTimestamp(err.to_string()))?;
    let updated = chrono::DateTime::parse_from_rfc3339(&profile.last_updated_at)
        .map_err(|err| ProfileValidationError::InvalidTimestamp(err.to_string()))?;
    if created > updated {
        return Err(ProfileValidationError::CreatedAfterLastUpdated);
    }

    if contains_impersonation_claim(&profile.owner_label)
        || contains_impersonation_claim(&profile.role_label)
    {
        return Err(ProfileValidationError::ImpersonationClaim);
    }
    for field in [
        &profile.working_style,
        &profile.communication_style,
        &profile.communication_preferences.tone,
        &profile.communication_preferences.detail_level,
        &profile.decision_preferences.decision_style,
    ] {
        if contains_impersonation_claim(field) {
            return Err(ProfileValidationError::ImpersonationClaim);
        }
        if contains_secret_like_value(field) {
            return Err(ProfileValidationError::SecretLikeValue);
        }
    }

    Ok(())
}

/// Resolve the most specific authority rule for `topic` (longest matching scope prefix).
pub fn effective_authority_for(
    topic: &str,
    rules: &[AuthorityRule],
) -> Option<ProxyAuthorityLevel> {
    rules
        .iter()
        .filter(|rule| topic.starts_with(&rule.scope) || rule.scope == "*")
        .max_by_key(|rule| rule.scope.len())
        .map(|rule| rule.authority)
}

pub(crate) fn validate_communication_preferences(
    prefs: &CommunicationPreferences,
) -> Result<(), ProfileValidationError> {
    validate_bounded_text(&prefs.tone, MAX_PROFILE_TEXT_BYTES)?;
    validate_bounded_text(&prefs.detail_level, MAX_PROFILE_TEXT_BYTES)?;
    validate_bounded_text(&prefs.async_preference, MAX_PROFILE_TEXT_BYTES)?;
    validate_bounded_text(&prefs.correction_preference, MAX_PROFILE_TEXT_BYTES)?;
    Ok(())
}

pub(crate) fn validate_decision_preferences(
    prefs: &DecisionPreferences,
) -> Result<(), ProfileValidationError> {
    validate_bounded_text(&prefs.decision_style, MAX_PROFILE_TEXT_BYTES)?;
    validate_bounded_text(&prefs.escalation_preference, MAX_PROFILE_TEXT_BYTES)?;
    Ok(())
}

fn validate_profile_id_field(value: &str, label: &str) -> Result<(), ProfileValidationError> {
    if value.trim().is_empty() {
        return Err(if label == "profile_id" {
            ProfileValidationError::EmptyProfileId
        } else {
            ProfileValidationError::InvalidAuthorityRule(format!("{label} is empty"))
        });
    }
    if value.len() > MAX_PROFILE_ID_BYTES {
        return Err(ProfileValidationError::ProfileIdTooLong {
            max: MAX_PROFILE_ID_BYTES,
        });
    }
    Ok(())
}

pub(crate) fn validate_bounded_text(value: &str, max: usize) -> Result<(), ProfileValidationError> {
    if value.len() > max {
        return Err(ProfileValidationError::TextTooLong { max });
    }
    Ok(())
}

fn validate_profile_limitation_text(limitation: &str) -> Result<(), ProfileValidationError> {
    if limitation.trim().is_empty() {
        return Err(ProfileValidationError::EmptyLimitations);
    }
    if limitation.len() > MAX_PROFILE_LIMITATION_BYTES {
        return Err(ProfileValidationError::TextTooLong {
            max: MAX_PROFILE_LIMITATION_BYTES,
        });
    }
    Ok(())
}

pub(crate) fn contains_impersonation_claim(text: &str) -> bool {
    let normalized = text.trim().to_ascii_lowercase();
    [
        "i am the human",
        "i am the owner",
        "speak as the human",
        "speak as the owner",
        "this proxy is the human",
        "this proxy is the owner",
        "impersonate the owner",
        "impersonate the human",
    ]
    .iter()
    .any(|phrase| normalized.contains(phrase))
}

pub(crate) fn contains_secret_like_value(text: &str) -> bool {
    let normalized = text.trim().to_ascii_lowercase();
    [
        "api_key=",
        "apikey=",
        "password=",
        "secret=",
        "token=",
        "bearer ",
        "sk-live-",
        "sk-test-",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}
