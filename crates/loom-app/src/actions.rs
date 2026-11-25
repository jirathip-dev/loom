use std::collections::BTreeMap;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

const DEAD_ZONE: f32 = 0.15;

/// One direction of a mouse drag.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Axis {
    X,
    Y,
}

/// A gamepad button, by its place on the pad.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PadButton {
    South,
    East,
    West,
    North,
    LeftBumper,
    RightBumper,
    LeftTrigger,
    RightTrigger,
    Select,
    Start,
    LeftStick,
    RightStick,
    Up,
    Down,
    Left,
    Right,
}

/// One axis of a gamepad stick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PadAxis {
    LeftX,
    LeftY,
    RightX,
    RightY,
}

/// Where an action's value comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Source {
    Key(KeyCode),
    Mouse(MouseButton),
    Wheel,
    Drag(MouseButton, Axis),
    Pad(PadButton),
    Stick(PadAxis),
}

/// A source and the factor its value is multiplied by.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bind {
    pub from: Source,
    pub scale: f32,
}

/// The caller's actions, each with the sources that drive it.
#[derive(Clone, Debug, PartialEq)]
pub struct Binds<A>(pub Vec<(A, Bind)>);

/// One raw input event, as a device reports it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Input {
    Key(KeyCode, bool),
    Mouse(MouseButton, bool),
    Wheel(f32),
    Moved(f32, f32),
    Pad(PadButton, f32),
    Stick(PadAxis, f32),
}

/// The caller's actions, folded from raw input once a frame.
#[derive(Clone, Debug)]
pub struct Actions<A> {
    binds: Vec<(A, Bind)>,
    levels: BTreeMap<Source, f32>,
    wheel: f32,
    moved: (f32, f32),
    now: BTreeMap<A, f32>,
    before: BTreeMap<A, f32>,
}

fn dead_zone(value: f32) -> f32 {
    if value.abs() <= DEAD_ZONE {
        0.0
    } else {
        value.signum() * (value.abs() - DEAD_ZONE) / (1.0 - DEAD_ZONE)
    }
}

impl<A: Copy + Ord> Actions<A> {
    #[must_use]
    pub fn new(binds: Binds<A>) -> Self {
        Self {
            binds: binds.0,
            levels: BTreeMap::new(),
            wheel: 0.0,
            moved: (0.0, 0.0),
            now: BTreeMap::new(),
            before: BTreeMap::new(),
        }
    }

    /// Takes one raw event; its effect shows from the next `step`.
    pub fn feed(&mut self, input: Input) {
        let (source, level) = match input {
            Input::Key(key, down) => (Source::Key(key), f32::from(u8::from(down))),
            Input::Mouse(button, down) => (Source::Mouse(button), f32::from(u8::from(down))),
            Input::Pad(button, value) => (Source::Pad(button), value),
            Input::Stick(axis, value) => (Source::Stick(axis), value),
            Input::Wheel(lines) => return self.wheel += lines,
            Input::Moved(x, y) => return self.moved = (self.moved.0 + x, self.moved.1 + y),
        };
        self.levels.insert(source, level);
    }

    /// Folds the input fed since the last step into this frame's actions.
    pub fn step(&mut self) {
        let mut now = BTreeMap::new();
        for (action, bind) in &self.binds {
            *now.entry(*action).or_insert(0.0) += self.source(bind.from) * bind.scale;
        }
        for value in now.values_mut() {
            *value = f32::clamp(*value, -1.0, 1.0);
        }
        self.before = std::mem::replace(&mut self.now, now);
        self.wheel = 0.0;
        self.moved = (0.0, 0.0);
    }

    fn level(&self, source: Source) -> f32 {
        self.levels.get(&source).copied().unwrap_or(0.0)
    }

    fn source(&self, source: Source) -> f32 {
        match source {
            Source::Wheel => self.wheel,
            Source::Drag(button, axis) if self.level(Source::Mouse(button)) > 0.0 => match axis {
                Axis::X => self.moved.0,
                Axis::Y => self.moved.1,
            },
            Source::Drag(..) => 0.0,
            Source::Stick(_) => dead_zone(self.level(source)),
            Source::Key(_) | Source::Mouse(_) | Source::Pad(_) => self.level(source),
        }
    }

    /// The sum of the action's sources, each times its scale, clamped to -1 to 1.
    pub fn value(&self, action: A) -> f32 {
        self.now.get(&action).copied().unwrap_or(0.0)
    }

    pub fn held(&self, action: A) -> bool {
        self.value(action) != 0.0
    }

    /// True on the frame the action's value leaves 0.
    pub fn pressed(&self, action: A) -> bool {
        self.held(action) && self.before.get(&action).copied().unwrap_or(0.0) == 0.0
    }
}
