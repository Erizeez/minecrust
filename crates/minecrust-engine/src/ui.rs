use std::sync::Arc;
use winit::window::Window;
use winit::event::WindowEvent;
use wgpu::{Device, Queue, TextureFormat};

pub struct EngineUi {
    pub context: egui::Context,
    pub state: egui_winit::State,
    pub renderer: egui_wgpu::Renderer,
}

impl EngineUi {
    pub fn new(
        device: &Device,
        output_color_format: TextureFormat,
        output_depth_format: Option<TextureFormat>,
        msaa_samples: u32,
        window: Arc<Window>,
    ) -> Self {
        let context = egui::Context::default();
        let viewport_id = context.viewport_id();
        let state = egui_winit::State::new(
            context.clone(),
            viewport_id,
            &window,
            Some(window.scale_factor() as f32),
            None,
            Some(2 * 1024 * 1024), // 2MB max texture size
        );
        let renderer = egui_wgpu::Renderer::new(device, output_color_format, output_depth_format, msaa_samples, false);

        Self {
            context,
            state,
            renderer,
        }
    }

    pub fn on_window_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        let response = self.state.on_window_event(window, event);
        response.consumed
    }

    pub fn register_native_texture(
        &mut self,
        device: &wgpu::Device,
        texture_view: &wgpu::TextureView,
    ) -> egui::TextureId {
        self.renderer.register_native_texture(device, texture_view, wgpu::FilterMode::Nearest)
    }
}
