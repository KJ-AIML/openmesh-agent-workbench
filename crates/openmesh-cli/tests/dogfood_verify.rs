// Checkpoint E — dogfood verifier (approved plan §17.E/§18/§19.H, Correction 6).
//
// Ignored by default: real dogfood data only exists after the human-run
// dogfood setup produces a real `.openmesh/signals/pending` inbox. Explicit
// invocation:
//   OPENMESH_DOGFOOD_PROJECT=<path> cargo test -p openmesh-cli --test dogfood_verify -- --ignored
//
// This test only reads and classifies what the real Reporter Skill ->
// `openmesh-cli` -> `write_signal` path already produced on disk. It never
// constructs a `WorkSignal`, never calls `write_signal`, and is not a
// producer of any kind.

use openmesh_core::signals::{process_pending, replay, ReplayReport};

#[test]
#[ignore]
fn dogfood_process_and_replay_are_consistent() {
    let project_path = std::env::var("OPENMESH_DOGFOOD_PROJECT")
        .expect("set OPENMESH_DOGFOOD_PROJECT to the real dogfood project path");

    let summary = process_pending(&project_path).expect("process_pending failed");

    println!("=== process_pending summary ===");
    println!("valid:       {}", summary.valid.len());
    for path in &summary.valid {
        println!("  valid: {}", path.display());
    }
    println!("duplicates:  {}", summary.duplicates.len());
    for (path, classification) in &summary.duplicates {
        println!("  duplicate: {} ({classification:?})", path.display());
    }
    println!("quarantined: {}", summary.quarantined.len());
    for (path, classification) in &summary.quarantined {
        println!("  quarantined: {} ({classification:?})", path.display());
    }
    println!("move_failed: {}", summary.move_failed.len());
    for failure in &summary.move_failed {
        println!(
            "  move_failed: {} ({:?})",
            failure.source.display(),
            failure.classification
        );
    }

    let first = replay(&project_path).expect("first replay failed");
    let second = replay(&project_path).expect("second replay failed");

    println!("=== replay (run 1): {} records ===", first.records.len());
    for record in &first.records {
        println!("  {} -> {:?}", record.path.display(), record.classification);
    }
    println!("=== replay (run 2): {} records ===", second.records.len());
    for record in &second.records {
        println!("  {} -> {:?}", record.path.display(), record.classification);
    }

    fn as_comparable(report: &ReplayReport) -> Vec<(std::path::PathBuf, String)> {
        report
            .records
            .iter()
            .map(|r| (r.path.clone(), format!("{:?}", r.classification)))
            .collect()
    }

    assert_eq!(
        as_comparable(&first),
        as_comparable(&second),
        "replay is not idempotent: two consecutive replay() calls produced different results"
    );

    println!(
        "replay is idempotent: {} records identical across both runs",
        first.records.len()
    );
}
