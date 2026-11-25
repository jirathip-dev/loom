#![forbid(unsafe_code)]

mod actions;
mod pacer;
mod pad;
mod run;

pub use actions::{Actions, Axis, Bind, Binds, Input, PadAxis, PadButton, Source};
pub use loom_render::Gpu;
pub use pacer::Pacer;
pub use run::run;
pub use winit::event::MouseButton;
pub use winit::keyboard::KeyCode;

/// A game's side of the loop: one call a displayed frame.
pub trait App {
    type Action: Copy + Ord;
    fn frame(&mut self, turn: &Turn<Self::Action>, gpu: &mut Gpu) -> Next;
}

/// What one frame hands the app: the actions, the seconds since the last frame and the window's size.
pub struct Turn<'a, A> {
    pub actions: &'a Actions<A>,
    pub dt: f32,
    pub size: (u32, u32),
}

/// Whether the loop goes on after a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    Go,
    Quit,
}

/// The window to open: its title and its inner size in pixels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowSpec {
    pub title: String,
    pub size: (u32, u32),
}
