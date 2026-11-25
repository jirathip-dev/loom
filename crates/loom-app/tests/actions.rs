use loom_app::{
    Actions, Axis, Bind, Binds, Input, KeyCode, MouseButton, PadAxis, PadButton, Source,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Act {
    Jump,
    Steer,
    Look,
}

fn bind(from: Source) -> Bind {
    Bind { from, scale: 1.0 }
}

fn actions() -> Actions<Act> {
    Actions::new(Binds(vec![
        (Act::Jump, bind(Source::Key(KeyCode::Space))),
        (Act::Jump, bind(Source::Mouse(MouseButton::Left))),
        (Act::Jump, bind(Source::Pad(PadButton::South))),
        (Act::Steer, bind(Source::Stick(PadAxis::LeftX))),
        (
            Act::Look,
            Bind {
                from: Source::Drag(MouseButton::Right, Axis::X),
                scale: 0.01,
            },
        ),
    ]))
}

fn jumps_once(down: Input, up: Input) {
    let mut actions = actions();
    actions.step();
    assert!(!actions.held(Act::Jump));
    actions.feed(down);
    actions.step();
    assert_eq!(actions.value(Act::Jump), 1.0);
    assert!(actions.pressed(Act::Jump));
    actions.step();
    assert!(actions.held(Act::Jump));
    assert!(!actions.pressed(Act::Jump));
    actions.feed(up);
    actions.step();
    assert!(!actions.held(Act::Jump));
}

#[test]
fn a_key_a_mouse_button_and_a_pad_button_drive_one_action() {
    jumps_once(
        Input::Key(KeyCode::Space, true),
        Input::Key(KeyCode::Space, false),
    );
    jumps_once(
        Input::Mouse(MouseButton::Left, true),
        Input::Mouse(MouseButton::Left, false),
    );
    jumps_once(
        Input::Pad(PadButton::South, 1.0),
        Input::Pad(PadButton::South, 0.0),
    );
}

#[test]
fn sources_add_up_and_clamp_to_one() {
    let mut actions = actions();
    actions.feed(Input::Key(KeyCode::Space, true));
    actions.feed(Input::Pad(PadButton::South, 1.0));
    actions.step();
    assert_eq!(actions.value(Act::Jump), 1.0);
}

#[test]
fn a_stick_inside_its_dead_zone_is_zero() {
    let mut actions = actions();
    actions.feed(Input::Stick(PadAxis::LeftX, 0.1));
    actions.step();
    assert_eq!(actions.value(Act::Steer), 0.0);
    assert!(!actions.pressed(Act::Steer));
    actions.feed(Input::Stick(PadAxis::LeftX, -1.0));
    actions.step();
    assert_eq!(actions.value(Act::Steer), -1.0);
    assert!(actions.pressed(Act::Steer));
}

#[test]
fn a_drag_counts_only_while_its_button_is_held_and_only_for_its_frame() {
    let mut actions = actions();
    actions.feed(Input::Moved(50.0, 0.0));
    actions.step();
    assert_eq!(actions.value(Act::Look), 0.0);
    actions.feed(Input::Mouse(MouseButton::Right, true));
    actions.feed(Input::Moved(50.0, 7.0));
    actions.step();
    assert_eq!(actions.value(Act::Look), 0.5);
    actions.step();
    assert_eq!(actions.value(Act::Look), 0.0);
}
