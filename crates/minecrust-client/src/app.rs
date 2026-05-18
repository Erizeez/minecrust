use crate::asset_loader::AssetLoader;
use crate::game::GameSession;
use crate::lang::LangManager;
use crate::state::{AppSettings, AppState};
use crate::ui;
use minecrust_engine::{egui, AudioManager, Camera, CameraUniform, EngineApp, Renderer};
use minecrust_shared::AssetPack;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use winit::event::ElementState;
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;

pub struct MinecrustApp {
    renderer: Option<Renderer>,
    camera: Camera,
    camera_uniform: CameraUniform,
    time: f64,

    // Sub-components
    state: AppState,
    settings: AppSettings,
    game: GameSession,
    audio: AudioManager,
    lang: LangManager,
    loader: AssetLoader,
}

impl MinecrustApp {
    pub fn new() -> Self {
        Self {
            renderer: None,
            camera: Camera {
                eye: glam::Vec3::new(8.0, 40.0, 8.0),
                target: glam::Vec3::new(8.0, 0.0, 8.0),
                up: glam::Vec3::Y,
                aspect: 16.0 / 9.0,
                fovy: std::f32::consts::FRAC_PI_4,
                znear: 0.1,
                zfar: 1000.0,
            },
            camera_uniform: CameraUniform::new(),
            time: 0.0,
            state: AppState::MainMenu,
            settings: AppSettings::default(),
            game: GameSession::new(),
            audio: AudioManager::new(),
            lang: LangManager::new(),
            loader: AssetLoader::new(),
        }
    }
}

impl EngineApp for MinecrustApp {
    fn on_init(&mut self, window: Arc<Window>) {
        env_logger::init();
        log::info!("Initializing Minecrust Client...");

        let _ = window.set_cursor_grab(winit::window::CursorGrabMode::Locked);
        window.set_cursor_visible(false);

        let mut renderer = pollster::block_on(Renderer::new(window));

        // Load AssetPack
        let mca_path = "assets/processed/assets.mca";
        log::info!("Loading assets from {}", mca_path);
        if let Ok(mut file) = File::open(mca_path) {
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).unwrap();
            let pack: AssetPack = bincode::deserialize(&bytes).unwrap();
            log::info!("AssetPack loaded. Atlas size: {} bytes", pack.atlas_png.len());

            renderer.load_atlas_bytes(&pack.atlas_png, 1024, 1024);

            self.game.asset_pack = Some(pack);
        } else {
            log::error!("Failed to load assets.mca! Run asset-cli first.");
        }

        self.lang.load(&self.settings.language, &self.loader);

        // Initialize egui custom fonts
        let mut font_defs = egui::FontDefinitions::default();
        let mut loaded_minecraft = false;

        // Load Minecraft custom English font
        let font_path = "assets/raw/font/MinecraftDefault-Regular.ttf";
        if let Ok(mut font_file) = File::open(font_path) {
            let mut font_bytes = Vec::new();
            if font_file.read_to_end(&mut font_bytes).is_ok() {
                font_defs.font_data.insert(
                    "minecraft".to_string(),
                    egui::FontData::from_owned(font_bytes),
                );
                
                // Add to proportional and monospace fonts at first position
                font_defs.families.get_mut(&egui::FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "minecraft".to_string());
                font_defs.families.get_mut(&egui::FontFamily::Monospace)
                    .unwrap()
                    .push("minecraft".to_string());
                
                loaded_minecraft = true;
            }
        }

        // Load GNU Unifont (Minecraft original CJK pixel font) as fallback
        let unifont_path = "assets/raw/font/unifont.ttf";
        if let Ok(mut unifont_file) = File::open(unifont_path) {
            let mut unifont_bytes = Vec::new();
            if unifont_file.read_to_end(&mut unifont_bytes).is_ok() {
                font_defs.font_data.insert(
                    "unifont".to_string(),
                    egui::FontData::from_owned(unifont_bytes),
                );
                
                font_defs.families.get_mut(&egui::FontFamily::Proportional)
                    .unwrap()
                    .push("unifont".to_string());
                font_defs.families.get_mut(&egui::FontFamily::Monospace)
                    .unwrap()
                    .push("unifont".to_string());
                
                println!("Minecraft original CJK pixel font (Unifont) loaded successfully!");
            }
        }

        if loaded_minecraft {
            renderer.ui.context.set_fonts(font_defs);
            println!("Minecraft custom pixel fonts configured successfully!");
        }

        self.renderer = Some(renderer);
        
        // Start Menu Music
        self.audio.play_music("assets/raw/minecraft/sounds/music/menu/mutation.ogg");
    }

    fn on_update(&mut self, dt: f64) {
        self.time += dt;

        let in_game_play = self.state == AppState::InGame;
        if in_game_play {
            self.game.update(dt, self.time, self.settings.render_distance, self.renderer.as_ref());
        }

        // Update Camera Eye and Target
        match self.state {
            AppState::InGame | AppState::InGameMenu | AppState::Settings { from_in_game: true } => {
                let (eye, target) = self.game.player.get_camera_vectors();
                self.camera.eye = eye;
                self.camera.target = target;
            }
            AppState::MainMenu | AppState::Settings { from_in_game: false } => {
                let radius = 50.0;
                let center = glam::Vec3::new(8.0, 60.0, 8.0);
                let speed = 0.05;
                let angle = self.time as f32 * speed;
                self.camera.eye = center + glam::Vec3::new(angle.cos() * radius, 20.0, angle.sin() * radius);
                self.camera.target = center;
            }
        }

        self.camera_uniform.update_view_proj(&self.camera);

        if let Some(renderer) = &mut self.renderer {
            renderer.update_camera(&self.camera_uniform);
        }
    }

    fn on_keyboard(&mut self, key: Key, state: ElementState) {
        if key == Key::Named(NamedKey::Escape) && state == ElementState::Pressed {
            let next_state = match self.state {
                AppState::InGame => AppState::InGameMenu,
                AppState::InGameMenu => AppState::InGame,
                AppState::Settings { from_in_game } => {
                    if from_in_game {
                        AppState::InGameMenu
                    } else {
                        AppState::MainMenu
                    }
                }
                AppState::MainMenu => AppState::MainMenu,
            };
            self.transition_state(next_state);
        }

        if self.state == AppState::InGame {
            self.game.input_manager.set_key(key, state == ElementState::Pressed);
        }
    }

    fn on_mouse_move(&mut self, dx: f64, dy: f64) {
        if self.state == AppState::InGame {
            self.game.input_manager.add_mouse_delta(dx, dy);
        }
    }

    fn on_render(&mut self, window: &Window) {
        let previous_state = self.state;
        let in_game = self.state == AppState::InGame;

        if in_game {
            let _ = window.set_cursor_grab(winit::window::CursorGrabMode::Locked);
            window.set_cursor_visible(false);
        } else {
            let _ = window.set_cursor_grab(winit::window::CursorGrabMode::None);
            window.set_cursor_visible(true);
        }

        let mut exit_requested = false;
        let mut new_vsync = self.settings.vsync;
        let mut new_fullscreen = self.settings.fullscreen;
        let prev_lang = self.settings.language.clone();

        if let Some(renderer) = &mut self.renderer {
            let meshes_iter = self.game.chunk_meshes.values()
                .map(|m| (&m.vertex_buffer, &m.index_buffer, m.index_count));

            match renderer.draw(window, meshes_iter, |ctx| {
                if !in_game {
                    exit_requested = ui::render_menus(ctx, &mut self.state, &mut self.settings, &self.lang);
                }
            }) {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => renderer.resize(renderer.size),
                Err(wgpu::SurfaceError::OutOfMemory) => log::error!("Out of memory!"),
                Err(e) => log::error!("{:?}", e),
            }
        }

        if self.settings.language != prev_lang {
            self.lang.load(&self.settings.language, &self.loader);
        }

        if self.state != previous_state {
            self.transition_state(self.state);
        }

        // Apply setting changes
        if new_fullscreen != self.settings.fullscreen {
            if self.settings.fullscreen {
                window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            } else {
                window.set_fullscreen(None);
            }
        }

        if new_vsync != self.settings.vsync {
            // TODO: Update renderer vsync config
        }

        if exit_requested {
            std::process::exit(0);
        }
    }

    fn on_resize(&mut self, width: u32, height: u32) {
        self.camera.aspect = width as f32 / height as f32;
        if let Some(renderer) = &mut self.renderer {
            renderer.resize(winit::dpi::PhysicalSize::new(width, height));
        }
    }

    fn on_window_event(&mut self, window: &Window, event: &winit::event::WindowEvent) -> bool {
        if let Some(renderer) = &mut self.renderer {
            let consumed = renderer.ui.on_window_event(window, event);
            if self.state != AppState::InGame {
                return consumed;
            }
        }
        false
    }
}

impl MinecrustApp {
    fn transition_state(&mut self, next_state: AppState) {
        if self.state != next_state {
            // Check if we are crossing the InGame / MainMenu boundary to change music
            let was_in_game_branch = matches!(self.state, AppState::InGame | AppState::InGameMenu | AppState::Settings { from_in_game: true });
            let is_in_game_branch = matches!(next_state, AppState::InGame | AppState::InGameMenu | AppState::Settings { from_in_game: true });

            if !was_in_game_branch && is_in_game_branch {
                self.audio.play_music("assets/raw/minecraft/sounds/music/game/clark.ogg");
            } else if was_in_game_branch && !is_in_game_branch {
                self.audio.play_music("assets/raw/minecraft/sounds/music/menu/mutation.ogg");
            }

            self.state = next_state;
        }
    }
}
