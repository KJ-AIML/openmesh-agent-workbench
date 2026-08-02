//! Dev Track 0.1.6 Checkpoint B — question identity tests.

use openmesh_core::domain::{
    validate_proxy_question, validate_proxy_question_id, ProxyQuestionValidationError,
    PROXY_QUESTION_PROTOCOL_VERSION,
};
use openmesh_core::proxy_question::{
    create_proxy_question, ProcessLocalRequestIdentityProvider, ProxyQuestionConstructionError,
    ProxyQuestionIdentityError, ProxyRequestIdentityProvider,
};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

struct TestSequenceIdentityProvider {
    sequence: AtomicU64,
}

impl TestSequenceIdentityProvider {
    fn new() -> Self {
        Self {
            sequence: AtomicU64::new(1),
        }
    }
}

impl ProxyRequestIdentityProvider for TestSequenceIdentityProvider {
    fn next_question_id(&self) -> Result<String, ProxyQuestionIdentityError> {
        let value = self.sequence.fetch_add(1, Ordering::SeqCst);
        Ok(format!("proxy-q-deadbeef-{value:04x}-00"))
    }
}

struct FailingIdentityProvider;

impl ProxyRequestIdentityProvider for FailingIdentityProvider {
    fn next_question_id(&self) -> Result<String, ProxyQuestionIdentityError> {
        Err(ProxyQuestionIdentityError::ClockBeforeUnixEpoch)
    }
}

fn question_id_counter_segment(question_id: &str) -> u64 {
    let remainder = question_id.strip_prefix("proxy-q-").expect("prefix");
    let counter = remainder.rsplit('-').next().expect("counter");
    u64::from_str_radix(counter, 16).expect("hex counter")
}

#[test]
fn process_local_counter_is_shared_across_provider_instances() {
    // Process-local counter is global; other parallel tests may interleave, so
    // assert monotonic increase across instances rather than exact +1.
    let first_provider = ProcessLocalRequestIdentityProvider::new();
    let second_provider = ProcessLocalRequestIdentityProvider::new();
    let first_id = first_provider.next_question_id().expect("first");
    let second_id = second_provider.next_question_id().expect("second");
    let first = question_id_counter_segment(&first_id);
    let second = question_id_counter_segment(&second_id);
    assert!(
        second > first,
        "shared process counter must advance across instances: first={first} second={second}"
    );
}

#[test]
fn many_ids_across_multiple_provider_instances_are_unique() {
    let mut ids = BTreeSet::new();
    for _ in 0..4 {
        let provider = ProcessLocalRequestIdentityProvider::new();
        for _ in 0..8 {
            let id = provider.next_question_id().expect("id");
            assert!(ids.insert(id));
        }
    }
    assert_eq!(ids.len(), 32);
}

#[test]
fn concurrent_process_local_generation_produces_unique_valid_ids() {
    let handles: Vec<_> = (0..16)
        .map(|_| {
            thread::spawn(|| {
                let provider = ProcessLocalRequestIdentityProvider::new();
                let mut ids = Vec::new();
                for _ in 0..8 {
                    let id = provider.next_question_id().expect("id");
                    validate_proxy_question_id(&id).expect("valid id");
                    ids.push(id);
                }
                ids
            })
        })
        .collect();
    let mut all = Vec::new();
    for handle in handles {
        all.extend(handle.join().expect("thread"));
    }
    assert_eq!(all.len(), 128);
    assert_eq!(all.iter().collect::<BTreeSet<_>>().len(), all.len());
}

#[test]
fn process_local_counter_does_not_require_timestamp_differences() {
    let first_provider = ProcessLocalRequestIdentityProvider::new();
    let second_provider = ProcessLocalRequestIdentityProvider::new();
    let first_id = first_provider.next_question_id().expect("first");
    let second_id = second_provider.next_question_id().expect("second");
    assert_ne!(first_id, second_id);
    assert_ne!(
        question_id_counter_segment(&first_id),
        question_id_counter_segment(&second_id)
    );
}

#[test]
fn process_local_provider_generates_valid_question_id() {
    let provider = ProcessLocalRequestIdentityProvider::new();
    let id = provider.next_question_id().expect("valid id");
    assert!(id.starts_with("proxy-q-"));
    let segments: Vec<_> = id.strip_prefix("proxy-q-").unwrap().split('-').collect();
    assert_eq!(segments.len(), 3);
}

#[test]
fn repeated_requests_receive_different_ids() {
    let provider = ProcessLocalRequestIdentityProvider::new();
    let first = provider.next_question_id().expect("first");
    let second = provider.next_question_id().expect("second");
    assert_ne!(first, second);
}

#[test]
fn same_question_receives_different_ids() {
    let provider = ProcessLocalRequestIdentityProvider::new();
    let first = create_proxy_question("What changed?", &provider).expect("first");
    let second = create_proxy_question("What changed?", &provider).expect("second");
    assert_ne!(first.question_id, second.question_id);
}

#[test]
fn question_id_contains_no_question_fragment() {
    let provider = ProcessLocalRequestIdentityProvider::new();
    let question = create_proxy_question("super-secret-fragment-xyz", &provider).expect("question");
    assert!(!question.question_id.contains("secret"));
    assert!(!question.question_id.contains("fragment"));
}

#[test]
fn question_id_contains_no_context_pack_hash() {
    let provider = ProcessLocalRequestIdentityProvider::new();
    let question = create_proxy_question("status?", &provider).expect("question");
    assert!(!question.question_id.contains("fnv1a"));
    assert!(!question.question_id.contains("context-pack"));
}

#[test]
fn question_id_is_not_persisted() {
    let provider = ProcessLocalRequestIdentityProvider::new();
    let _question = create_proxy_question("status?", &provider).expect("question");
    let source = std::include_str!("../src/proxy_question.rs");
    assert!(!source.contains("fs::write"));
    assert!(!source.contains("read_to_string"));
}

#[test]
fn process_local_provider_is_not_described_as_cryptographic() {
    let docs = std::include_str!("../src/proxy_question.rs");
    let lowered = docs.to_ascii_lowercase();
    assert!(!lowered.contains("is cryptographically secure"));
    assert!(lowered.contains("not claimed to be cryptographically secure"));
}

#[test]
fn deterministic_test_provider_implements_public_trait() {
    let provider = TestSequenceIdentityProvider::new();
    let first = provider.next_question_id().expect("first");
    let second = provider.next_question_id().expect("second");
    assert_eq!(first, "proxy-q-deadbeef-0001-00");
    assert_eq!(second, "proxy-q-deadbeef-0002-00");
}

#[test]
fn create_proxy_question_uses_provider_id() {
    let provider = TestSequenceIdentityProvider::new();
    let question = create_proxy_question("hello", &provider).expect("question");
    assert_eq!(question.question_id, "proxy-q-deadbeef-0001-00");
}

#[test]
fn create_proxy_question_validates_complete_contract() {
    let provider = TestSequenceIdentityProvider::new();
    let question = create_proxy_question("  complete contract  ", &provider).expect("question");
    validate_proxy_question(&question).expect("valid question");
    assert_eq!(question.protocol_version, PROXY_QUESTION_PROTOCOL_VERSION);
}

#[test]
fn create_proxy_question_accepts_thai_utf8() {
    let provider = TestSequenceIdentityProvider::new();
    let question = create_proxy_question("สถานะปัจจุบันคืออะไร", &provider).expect("question");
    validate_proxy_question(&question).expect("Thai question validates");
}

#[test]
fn identity_failure_returns_safe_typed_error() {
    let provider = FailingIdentityProvider;
    let err = create_proxy_question("hello", &provider).expect_err("identity failure");
    assert!(matches!(
        err,
        ProxyQuestionConstructionError::IdentityGenerationFailed(
            ProxyQuestionIdentityError::ClockBeforeUnixEpoch
        )
    ));
    let message = err.to_string().to_ascii_lowercase();
    assert!(!message.contains("secret"));
    assert!(!message.contains("fnv1a"));
}

#[test]
fn create_proxy_question_rejects_empty_text_before_identity() {
    let provider = TestSequenceIdentityProvider::new();
    let err = create_proxy_question("   ", &provider).expect_err("empty");
    assert!(matches!(
        err,
        ProxyQuestionConstructionError::InvalidText(ProxyQuestionValidationError::EmptyText)
    ));
    assert_eq!(
        provider.next_question_id().unwrap(),
        "proxy-q-deadbeef-0001-00"
    );
}
