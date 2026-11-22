use loom_core::{Seed, StreamName, Streams};

const DRIFT: StreamName = StreamName("30_motion/10_drift");
const CHURN: StreamName = StreamName("20_life/10_churn");

#[test]
fn a_streams_numbers_do_not_change_when_another_stream_is_added() {
    let mut alone = Streams::new(Seed(42));
    let drift_alone: Vec<u64> = (0..64).map(|_| alone.of(DRIFT).bits()).collect();

    let mut shared = Streams::new(Seed(42));
    let mut drift_shared = Vec::new();
    let mut churn = Vec::new();
    for _ in 0..64 {
        churn.push(shared.of(CHURN).bits());
        drift_shared.push(shared.of(DRIFT).bits());
    }

    assert_eq!(drift_alone, drift_shared);
    assert_ne!(churn, drift_shared);
}

#[test]
fn a_saved_stream_resumes_where_it_stopped() -> Result<(), serde_json::Error> {
    let mut straight = Streams::new(Seed(7));
    let mut resumed = Streams::new(Seed(7));
    for _ in 0..10 {
        straight.of(DRIFT).bits();
        resumed.of(DRIFT).bits();
    }
    let mut resumed: Streams = serde_json::from_str(&serde_json::to_string(&resumed)?)?;
    for _ in 0..10 {
        assert_eq!(straight.of(DRIFT).bits(), resumed.of(DRIFT).bits());
        assert_eq!(straight.of(CHURN).bits(), resumed.of(CHURN).bits());
    }
    Ok(())
}

#[test]
fn draws_stay_in_their_ranges() {
    let mut streams = Streams::new(Seed(1));
    let rng = streams.of(DRIFT);
    for _ in 0..10_000 {
        assert!(rng.below(6) < 6);
        let unit = rng.unit();
        assert!((0.0..1.0).contains(&unit));
    }
    assert_eq!(rng.below(0), 0);
    assert!(!rng.chance(0.0));
    assert!(rng.chance(1.0));
}
