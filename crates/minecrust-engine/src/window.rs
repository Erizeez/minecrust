use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};
use std::sync::Arc;

pub trait EngineApp {
    fn on_init(&mut self, window: Arc<Window>);
    fn on_update(&mut self, dt: f64);
    fn on_render(&mut self, window: &Window);
    fn on_resize(&mut self, width: u32, height: u32);
    fn on_keyboard(&mut self, key: Key, state: ElementState) {}
    fn on_mouse_move(&mut self, dx: f64, dy: f64) {}
    fn on_mouse_click(&mut self, state: ElementState, button: winit::event::MouseButton) {}
    fn on_window_event(&mut self, window: &Window, event: &WindowEvent) -> bool { false }
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
        if let Some(window) = &self.window {
            if self.app.on_window_event(window, &event) {
                return;
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.app.on_keyboard(event.logical_key.clone(), event.state);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.app.on_mouse_click(state, button);
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
                if let Some(window) = &self.window {
                    self.app.on_render(window);
                    window.request_redraw();
                }
            }
            _ => (),
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let winit::event::DeviceEvent::MouseMotion { delta } = event {
            self.app.on_mouse_move(delta.0, delta.1);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
