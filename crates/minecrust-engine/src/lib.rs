pub mod window;
pub mod renderer;
pub mod camera;
pub mod core;
pub mod audio;
pub mod world;
pub mod physics;
pub mod input;
pub mod ui;
pub mod systems;
pub mod hecs_test;

#[cfg(all(target_os = "macos", feature = "rt-metal"))]
pub mod metal_rt;

// Re-export common types
pub use egui;
pub use window::{EngineApp, EngineRunner};
pub use renderer::{Renderer, Vertex};
pub use camera::{Camera, CameraUniform};
pub use audio::AudioManager;
