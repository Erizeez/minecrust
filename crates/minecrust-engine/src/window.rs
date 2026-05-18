use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};
use std::sync::Arc;

pub trait EngineApp {
    fn on_init(&mut self, window: Arc<Window>);
    fn on_update(&mut self, dt: f64);
    fn on_render(&mut self);
    fn on_resize(&mut self, width: u32, height: u32);
}

pub struct EngineRunner<A: EngineApp> {
    app: A,
    window: Option<Arc<Window>>,
    last_time: std::time::Instant,
}

impl<A: EngineApp> EngineRunner<A> {
    pub fn new(app: A) -> Self {
        Self {
            app,
            window: None,
            last_time: std::time::Instant::now(),
        }
    }

    pub fn run(mut self) -> anyhow::Result<()> {
        let event_loop = EventLoop::new()?;
        event_loop.set_control_flow(ControlFlow::Poll);
        event_loop.run_app(&mut self)?;
        Ok(())
    }
}

impl<A: EngineApp> ApplicationHandler for EngineRunner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let attributes = Window::default_attributes()
                .with_title("Minecrust")
                .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0));
            
            let window = Arc::new(event_loop.create_window(attributes).unwrap());
            self.window = Some(window.clone());
            self.app.on_init(window);
            self.last_time = std::time::Instant::now();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                self.app.on_resize(physical_size.width, physical_size.height);
            }
            WindowEvent::RedrawRequested => {
                // Calculate dt
                let now = std::time::Instant::now();
                let dt = now.duration_since(self.last_time).as_secs_f64();
                self.last_time = now;

                self.app.on_update(dt);
                self.app.on_render();

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
