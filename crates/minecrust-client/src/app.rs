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
    last_dt: f64,
    sys: sysinfo::System,
    pid: sysinfo::Pid,

    // Sub-components
    state: AppState,
    settings: AppSettings,
    game: GameSession,
    audio: AudioManager,
    lang: LangManager,
    loader: AssetLoader,
    lan_discoverer: crate::lan::LanServerDiscoverer,
    connect_addr: String,
    main_menu_steve: Option<minecrust_engine::renderer::RenderMesh>,
    main_menu_alex: Option<minecrust_engine::renderer::RenderMesh>,
}

impl MinecrustApp {
    pub fn new() -> Self {
        let (server_tx, server_rx) = minecrust_server::IntegratedServer::start(12345, None);
        Self {
            renderer: None,
            camera: Camera {
                eye: glam::Vec3::new(8.0, 40.0, 8.0),
                yaw: 0.0,
                pitch: -std::f32::consts::FRAC_PI_2,
                aspect: 16.0 / 9.0,
                fovy: std::f32::consts::FRAC_PI_4,
                znear: 0.1,
                zfar: 1000.0,
            },
            camera_uniform: CameraUniform::new(),
            time: 0.0,
            last_dt: 0.016,
            sys: sysinfo::System::new(),
            pid: sysinfo::get_current_pid().unwrap_or(sysinfo::Pid::from_u32(0)),
            state: AppState::MainMenu,
            settings: AppSettings::default(),
            game: GameSession::new(server_tx, server_rx),
            audio: AudioManager::new(),
            lang: LangManager::new(),
            loader: AssetLoader::new(),
            lan_discoverer: crate::lan::LanServerDiscoverer::new(),
            connect_addr: "127.0.0.1:25565".to_string(),
            main_menu_steve: None,
            main_menu_alex: None,
        }
    }
}

impl EngineApp for MinecrustApp {
    fn on_init(&mut self, window: Arc<Window>) {
        let mut builder = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"));
        builder.filter_module("wgpu_core", log::LevelFilter::Warn);
        builder.filter_module("wgpu_hal", log::LevelFilter::Warn);
        builder.filter_module("naga", log::LevelFilter::Warn);
        let _ = builder.try_init();
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

            renderer.load_atlas_bytes(&pack.atlas_png, &pack.atlas_normal_png, &pack.atlas_specular_png, 1024, 1024);

            let (steve_v, steve_i) = crate::steve::build_steve_vertices(glam::Vec3::new(6.0, 100.0, 8.0), &pack, crate::steve::PlayerModelType::Steve);
            self.main_menu_steve = Some(renderer.create_render_mesh(&steve_v, &steve_i));

            let (alex_v, alex_i) = crate::steve::build_steve_vertices(glam::Vec3::new(10.0, 100.0, 8.0), &pack, crate::steve::PlayerModelType::Alex);
            self.main_menu_alex = Some(renderer.create_render_mesh(&alex_v, &alex_i));

            let arc_pack = Arc::new(pack);
            
            let bones = [
                "steve_head", "steve_body", "steve_right_arm", "steve_left_arm", "steve_right_leg", "steve_left_leg",
            ];
            for bone in bones {
                let (vertices, indices) = crate::steve::build_steve_bone_vertices(&arc_pack, self.settings.player_model, bone);
                renderer.mesh_registry.insert(bone.to_string(), Arc::new(renderer.create_render_mesh(&vertices, &indices)));
            }
            
            self.game.asset_pack = Some(arc_pack);
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
        self.last_dt = dt;

        let in_game_play = self.state == AppState::InGame;
        if in_game_play {
            self.game.update(dt, self.time, self.settings.render_distance, self.settings.player_model, self.renderer.as_ref());
        }

        // Update Camera Eye and Target
        match self.state {
            AppState::InGame | AppState::InGameMenu | AppState::Settings { from_in_game: true } => {
                let (eye, yaw, pitch) = {
                    use minecrust_shared::ecs::player::Player;
                    use minecrust_shared::ecs::transform::LocalTransform;
                    use minecrust_engine::systems::player::get_camera_vectors;
                    
                    if let Ok(player) = self.game.world_manager.ecs.get::<&Player>(self.game.local_player_entity) {
                        if let Ok(transform) = self.game.world_manager.ecs.get::<&LocalTransform>(self.game.local_player_entity) {
                            get_camera_vectors(&player, &transform, &self.game.world_manager.chunk_manager)
                        } else {
                            (glam::Vec3::ZERO, 0.0, 0.0)
                        }
                    } else {
                        (glam::Vec3::ZERO, 0.0, 0.0)
                    }
                };
                self.camera.eye = eye;
                self.camera.yaw = yaw;
                self.camera.pitch = pitch;
            }
            AppState::MainMenu | AppState::MultiplayerMenu | AppState::Settings { from_in_game: false } => {
                let center = glam::Vec3::new(8.0, 101.5, 8.0);
                self.camera.eye = center + glam::Vec3::new(0.0, 0.0, 4.0); // look from Z=12 to Z=8
                let forward = (center - self.camera.eye).normalize();
                self.camera.yaw = forward.z.atan2(forward.x);
                self.camera.pitch = forward.y.asin();
            }
        }

        self.camera_uniform.update_view_proj(&self.camera);
        self.camera_uniform.update_time(self.game.world_time);
        self.camera_uniform.update_frame_index();
        self.camera_uniform.update_settings(self.settings.enable_raytracing);

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
                AppState::MultiplayerMenu => AppState::MainMenu,
            };
            self.transition_state(next_state);
        }

        if key == Key::Named(NamedKey::F4) && state == ElementState::Pressed {
            self.settings.show_debug_info = !self.settings.show_debug_info;
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

        let mut action_trigger = None;

        if let Some(renderer) = &mut self.renderer {
            let mut all_meshes = Vec::new();
            if self.state == AppState::MainMenu {
                if let Some(m) = &self.main_menu_steve {
                    all_meshes.push(m);
                }
                if let Some(m) = &self.main_menu_alex {
                    all_meshes.push(m);
                }
            } else {
                for m in self.game.chunk_meshes.values() {
                    all_meshes.push(m);
                }
                for m in self.game.lod_meshes.values() {
                    all_meshes.push(m);
                }
            }
            
            let mut extra_entities = Vec::new();
            if self.state != AppState::MainMenu {
                for p in self.game.other_players.values() {
                    if let Some(m) = &p.mesh {
                        // Needs a transform matrix. For now, use identity since player mesh has position baked in vertices
                        // Wait, build_steve_vertices bakes position into the vertices. So Mat4::IDENTITY is correct.
                        extra_entities.push((m, glam::Mat4::IDENTITY));
                    }
                }
            }
            
            // Map extra_entities to references of Mat4
            let ref_extra_entities: Vec<_> = extra_entities.iter().map(|(m, mat)| (*m, mat)).collect();

            // Extract hardware info to avoid borrow conflicts in closure
            let adapter_name = renderer.adapter_info.name.clone();
            let backend_name = format!("{:?}", renderer.adapter_info.backend);
            let display_res = format!("{}x{}", renderer.size.width, renderer.size.height);

            match renderer.draw_world(window, &self.game.world_manager.ecs, all_meshes.into_iter(), ref_extra_entities.into_iter(), |ctx| {
                if !in_game {
                    exit_requested = ui::render_menus(
                        ctx,
                        &mut self.state,
                        &mut self.settings,
                        &self.lang,
                        &self.lan_discoverer,
                        &mut self.connect_addr,
                        &mut action_trigger,
                    );
                }
                
                if self.settings.show_debug_info {
                    self.sys.refresh_process(self.pid);
                    let mut process_mem_mb = 0;
                    let mut process_cpu = 0.0;
                    if let Some(process) = self.sys.process(self.pid) {
                        process_mem_mb = process.memory() / 1024 / 1024;
                        process_cpu = process.cpu_usage();
                    }
                    
                    let fps = 1.0 / self.last_dt;
                    let mut px = 0.0;
                    let mut py = 0.0;
                    let mut pz = 0.0;
                    let mut yaw = 0.0;
                    let mut pitch = 0.0;
                    if let Ok(transform) = self.game.world_manager.ecs.get::<&minecrust_shared::ecs::transform::LocalTransform>(self.game.local_player_entity) {
                        px = transform.translation.x;
                        py = transform.translation.y;
                        pz = transform.translation.z;
                    }
                    if let Ok(player) = self.game.world_manager.ecs.get::<&minecrust_shared::ecs::player::Player>(self.game.local_player_entity) {
                        yaw = player.yaw;
                        pitch = player.pitch;
                    }
                    
                    let cx = (px / minecrust_engine::world::CHUNK_WIDTH as f32).floor() as i32;
                    let cz = (pz / minecrust_engine::world::CHUNK_DEPTH as f32).floor() as i32;
                    let bx = px.floor() as i32;
                    let by = py.floor() as i32;
                    let bz = pz.floor() as i32;
                    
                    let dir = if yaw > -std::f32::consts::FRAC_PI_4 && yaw <= std::f32::consts::FRAC_PI_4 {
                        "East (Towards +X)"
                    } else if yaw > std::f32::consts::FRAC_PI_4 && yaw <= 3.0 * std::f32::consts::FRAC_PI_4 {
                        "South (Towards +Z)"
                    } else if yaw < -std::f32::consts::FRAC_PI_4 && yaw >= -3.0 * std::f32::consts::FRAC_PI_4 {
                        "North (Towards -Z)"
                    } else {
                        "West (Towards -X)"
                    };
                    
                    let w_time = self.game.world_time;
                    let days = (w_time / 24000.0).floor() as i32;
                    let hours = ((w_time / 1000.0) + 6.0) as i32 % 24;
                    let mins = ((w_time % 1000.0) / 1000.0 * 60.0) as i32;
                    
                    let rendered_chunks = self.game.chunk_meshes.len();
                    let total_entities = self.game.other_players.len() + 1;
                    
                    // Hardware info
                    let os_name = std::env::consts::OS;
                    let arch_name = std::env::consts::ARCH;
                    
                    // Left side
                    egui::Area::new("debug_info_left".into())
                        .fixed_pos(egui::pos2(5.0, 5.0))
                        .interactable(false)
                        .show(ctx, |ui| {
                            let bg_color = egui::Color32::from_black_alpha(120);
                            let text_color = egui::Color32::WHITE;
                            let font_id = egui::FontId::proportional(16.0);
                            
                            egui::Frame::none().fill(bg_color).inner_margin(4.0).show(ui, |ui| {
                                ui.label(egui::RichText::new("Minecrust 1.21.1").color(text_color).font(font_id.clone()).strong());
                                ui.label(egui::RichText::new(format!("{} fps", fps.round() as i32)).color(text_color).font(font_id.clone()));
                                ui.label(egui::RichText::new(format!("E: {}/{}   C: {}/{}", total_entities, total_entities, rendered_chunks, rendered_chunks)).color(text_color).font(font_id.clone()));
                                ui.add_space(8.0);
                                ui.label(egui::RichText::new(format!("XYZ: {:.3} / {:.5} / {:.3}", px, py, pz)).color(text_color).font(font_id.clone()));
                                ui.label(egui::RichText::new(format!("Block: {} {} {}", bx, by, bz)).color(text_color).font(font_id.clone()));
                                ui.label(egui::RichText::new(format!("Chunk: {} {} {}", cx, by / 16, cz)).color(text_color).font(font_id.clone()));
                                ui.label(egui::RichText::new(format!("Facing: {} ({:.1} / {:.1})", dir, yaw.to_degrees(), pitch.to_degrees())).color(text_color).font(font_id.clone()));
                                ui.add_space(8.0);
                                ui.label(egui::RichText::new(format!("Day {}, Time: {:.0} ticks ({:02}:{:02})", days, w_time, hours, mins)).color(text_color).font(font_id.clone()));
                            });
                        });
                        
                    // Right side
                    egui::Area::new("debug_info_right".into())
                        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-5.0, 5.0))
                        .interactable(false)
                        .show(ctx, |ui| {
                            let bg_color = egui::Color32::from_black_alpha(120);
                            let text_color = egui::Color32::WHITE;
                            let font_id = egui::FontId::proportional(16.0);
                            
                            egui::Frame::none().fill(bg_color).inner_margin(4.0).show(ui, |ui| {
                                ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
                                    ui.label(egui::RichText::new(format!("OS: {} ({})", os_name, arch_name)).color(text_color).font(font_id.clone()));
                                    ui.label(egui::RichText::new(format!("CPU: {:.1}%", process_cpu)).color(text_color).font(font_id.clone()));
                                    ui.label(egui::RichText::new(format!("Mem: {}MB", process_mem_mb)).color(text_color).font(font_id.clone()));
                                    ui.add_space(8.0);
                                    ui.label(egui::RichText::new(format!("Display: {} ({})", display_res, backend_name)).color(text_color).font(font_id.clone()));
                                    ui.label(egui::RichText::new(format!("{}", adapter_name)).color(text_color).font(font_id.clone()));
                                });
                            });
                        });
                }
            }) {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => renderer.resize(renderer.size),
                Err(wgpu::SurfaceError::OutOfMemory) => log::error!("Out of memory!"),
                Err(e) => log::error!("{:?}", e),
            }
        }

        if let Some(action) = action_trigger {
            match action {
                ui::MultiplayerAction::JoinSingleplayer => {
                    log::info!("Starting game in singleplayer mode...");
                    let (server_tx, server_rx) = minecrust_server::IntegratedServer::start(12345, None);
                    let mut new_game = GameSession::new(server_tx, server_rx);
                    new_game.asset_pack = self.game.asset_pack.take();
                    self.game = new_game;
                    self.transition_state(AppState::InGame);
                }
                ui::MultiplayerAction::JoinAddress(addr_str) => {
                    if let Ok(addr) = addr_str.parse::<std::net::SocketAddr>() {
                        log::info!("Connecting to multiplayer server: {}...", addr);
                        if let Ok((server_tx, server_rx)) = crate::lan::connect_multiplayer(addr, "Player".to_string()) {
                            let mut new_game = GameSession::new(server_tx, server_rx);
                            new_game.asset_pack = self.game.asset_pack.take();
                            self.game = new_game;
                            self.transition_state(AppState::InGame);
                        } else {
                            log::error!("Failed to connect to {}", addr);
                        }
                    } else {
                        log::error!("Invalid socket address format: {}", addr_str);
                    }
                }
                ui::MultiplayerAction::HostLan => {
                    log::info!("Hosting LAN multiplayer server on random port...");
                    let bind_addr = "0.0.0.0:0".parse::<std::net::SocketAddr>().unwrap();
                    let (server_tx, server_rx) = minecrust_server::IntegratedServer::start(12345, Some(bind_addr));
                    let mut new_game = GameSession::new(server_tx, server_rx);
                    new_game.asset_pack = self.game.asset_pack.take();
                    self.game = new_game;
                    self.transition_state(AppState::InGame);
                }
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
