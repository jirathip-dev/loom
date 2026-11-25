use loom_app::Pacer;

const FRAME: f32 = 1.0 / 60.0;

fn second(speed: u32) -> u32 {
    let mut pacer = Pacer::new(20);
    let mut ticks = 0;
    for _ in 0..60 {
        ticks += pacer.add(FRAME, speed);
        assert!((0.0..=1.0).contains(&pacer.alpha()));
    }
    ticks
}

#[test]
fn a_second_at_speed_one_three_and_zero() {
    assert_eq!(second(1), 20);
    assert_eq!(second(3), 60);
    assert_eq!(second(0), 0);
}

#[test]
fn a_frame_hands_out_at_most_eight_ticks() {
    let mut pacer = Pacer::new(20);
    assert_eq!(pacer.add(1.0, 3), 8);
    assert!((0.0..=1.0).contains(&pacer.alpha()));
}

#[test]
fn a_five_second_gap_drops_the_time_owed() {
    let mut pacer = Pacer::new(20);
    assert_eq!(pacer.add(5.0, 1), 8);
    assert!((0.0..1.0).contains(&pacer.alpha()));
    assert_eq!(pacer.add(FRAME, 1), 0);
}

#[test]
fn alpha_is_how_far_into_the_next_tick() {
    let mut pacer = Pacer::new(20);
    assert_eq!(pacer.add(0.025, 1), 0);
    assert!((pacer.alpha() - 0.5).abs() < 1e-3);
}
