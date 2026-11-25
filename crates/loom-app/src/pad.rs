use crate::Actions;

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub struct Pads(Option<gilrs::Gilrs>);

#[cfg(any(target_os = "macos", target_os = "windows"))]
impl Pads {
    pub fn new() -> Self {
        Self(gilrs::Gilrs::new().ok())
    }

    pub fn poll<A: Copy + Ord>(&mut self, actions: &mut Actions<A>) {
        use crate::{Input, PadAxis, PadButton};
        use gilrs::{Axis, Button, EventType};
        let Some(pads) = &mut self.0 else { return };
        while let Some(event) = pads.next_event() {
            let input = match event.event {
                EventType::ButtonChanged(button, value, _) => Input::Pad(
                    match button {
                        Button::South => PadButton::South,
                        Button::East => PadButton::East,
                        Button::West => PadButton::West,
                        Button::North => PadButton::North,
                        Button::LeftTrigger => PadButton::LeftBumper,
                        Button::RightTrigger => PadButton::RightBumper,
                        Button::LeftTrigger2 => PadButton::LeftTrigger,
                        Button::RightTrigger2 => PadButton::RightTrigger,
                        Button::Select => PadButton::Select,
                        Button::Start => PadButton::Start,
                        Button::LeftThumb => PadButton::LeftStick,
                        Button::RightThumb => PadButton::RightStick,
                        Button::DPadUp => PadButton::Up,
                        Button::DPadDown => PadButton::Down,
                        Button::DPadLeft => PadButton::Left,
                        Button::DPadRight => PadButton::Right,
                        _ => continue,
                    },
                    value,
                ),
                EventType::AxisChanged(axis, value, _) => Input::Stick(
                    match axis {
                        Axis::LeftStickX => PadAxis::LeftX,
                        Axis::LeftStickY => PadAxis::LeftY,
                        Axis::RightStickX => PadAxis::RightX,
                        Axis::RightStickY => PadAxis::RightY,
                        _ => continue,
                    },
                    value,
                ),
                _ => continue,
            };
            actions.feed(input);
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub struct Pads;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
impl Pads {
    pub fn new() -> Self {
        Self
    }

    pub fn poll<A: Copy + Ord>(&mut self, _: &mut Actions<A>) {}
}
