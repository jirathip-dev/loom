use crate::pad::Pads;
use crate::{Actions, App, Binds, Gpu, Input, Next, Turn, WindowSpec};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

const PIXELS_A_LINE: f32 = 20.0;

struct Live<G: App> {
    window: Arc<Window>,
    gpu: Gpu,
    app: G,
    actions: Actions<G::Action>,
    last: Instant,
    cursor: Option<PhysicalPosition<f32>>,
    occluded: bool,
}

struct Runner<G: App, S> {
    spec: WindowSpec,
    boot: Option<(Binds<G::Action>, S)>,
    live: Option<Live<G>>,
    pads: Pads,
    failed: bool,
}

/// Opens the window, makes the app with `start` once the GPU is ready, then calls its `frame` each display refresh.
pub fn run<G: App>(
    window: WindowSpec,
    binds: Binds<G::Action>,
    start: impl FnOnce(&mut Gpu) -> G,
) -> ExitCode {
    let mut runner = Runner {
        spec: window,
        boot: Some((binds, start)),
        live: None,
        pads: Pads::new(),
        failed: false,
    };
    let ran = EventLoop::new().and_then(|events| events.run_app(&mut runner));
    if let Err(e) = ran {
        runner.fail(&e);
    }
    if runner.failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

impl<G: App, S: FnOnce(&mut Gpu) -> G> Runner<G, S> {
    #[allow(
        clippy::print_stderr,
        reason = "run is the binary's edge: a failed start is told once"
    )]
    fn fail(&mut self, why: &dyn std::fmt::Display) {
        eprintln!("loom-app: {why}");
        self.failed = true;
    }

    fn boot(&mut self, events: &ActiveEventLoop) -> Result<(), Box<dyn std::error::Error>> {
        let Some((binds, start)) = self.boot.take() else {
            return Ok(());
        };
        let attributes = Window::default_attributes()
            .with_title(self.spec.title.clone())
            .with_inner_size(PhysicalSize::new(self.spec.size.0, self.spec.size.1));
        let window = Arc::new(events.create_window(attributes)?);
        let size = window.inner_size();
        let mut gpu = Gpu::on_window(Arc::clone(&window), (size.width, size.height))?;
        let app = start(&mut gpu);
        self.live = Some(Live {
            window,
            gpu,
            app,
            actions: Actions::new(binds),
            last: Instant::now(),
            cursor: None,
            occluded: false,
        });
        Ok(())
    }
}

impl<G: App> Live<G> {
    fn shown(&self) -> bool {
        let size = self.window.inner_size();
        !self.occluded && size.width > 0 && size.height > 0
    }

    fn input(&mut self, event: &WindowEvent) {
        let input = match event {
            WindowEvent::KeyboardInput { event, .. } => match event.physical_key {
                PhysicalKey::Code(key) => Input::Key(key, event.state.is_pressed()),
                PhysicalKey::Unidentified(_) => return,
            },
            WindowEvent::MouseInput { state, button, .. } => {
                Input::Mouse(*button, state.is_pressed())
            }
            WindowEvent::MouseWheel { delta, .. } => Input::Wheel(match delta {
                MouseScrollDelta::LineDelta(_, lines) => *lines,
                MouseScrollDelta::PixelDelta(at) => at.cast::<f32>().y / PIXELS_A_LINE,
            }),
            WindowEvent::CursorMoved { position, .. } => {
                let at = position.cast::<f32>();
                let from = self.cursor.replace(at).unwrap_or(at);
                Input::Moved(at.x - from.x, at.y - from.y)
            }
            _ => return,
        };
        self.actions.feed(input);
    }

    fn frame(&mut self, pads: &mut Pads) -> Next {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f32();
        self.last = now;
        pads.poll(&mut self.actions);
        self.actions.step();
        let size = self.window.inner_size();
        let turn = Turn {
            actions: &self.actions,
            dt,
            size: (size.width, size.height),
        };
        self.app.frame(&turn, &mut self.gpu)
    }
}

impl<G: App, S: FnOnce(&mut Gpu) -> G> ApplicationHandler for Runner<G, S> {
    fn resumed(&mut self, events: &ActiveEventLoop) {
        if let Err(e) = self.boot(events) {
            self.fail(&e);
            events.exit();
        }
    }

    fn window_event(&mut self, events: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(live) = &mut self.live else { return };
        let was_shown = live.shown();
        match event {
            WindowEvent::CloseRequested => events.exit(),
            WindowEvent::Resized(size) => live.gpu.resize((size.width, size.height)),
            WindowEvent::Occluded(occluded) => live.occluded = occluded,
            WindowEvent::RedrawRequested => {
                if live.frame(&mut self.pads) == Next::Quit {
                    events.exit();
                }
            }
            other => live.input(&other),
        }
        if !was_shown && live.shown() {
            live.last = Instant::now();
        }
    }

    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        if let Some(live) = &self.live
            && live.shown()
        {
            live.window.request_redraw();
        }
    }
}
