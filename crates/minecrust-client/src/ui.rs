use crate::state::{AppState, AppSettings};
use crate::lang::LangManager;
use minecrust_engine::egui::{self, Color32, Frame, RichText};

#[derive(Debug, Clone)]
pub enum MultiplayerAction {
    JoinSingleplayer,
    JoinAddress(String),
    HostLan,
}

/// Renders the UI and returns true if the application should exit.
pub fn render_menus(
    ctx: &egui::Context,
    state: &mut AppState,
    settings: &mut AppSettings,
    lang: &LangManager,
    discoverer: &crate::lan::LanServerDiscoverer,
    connect_addr: &mut String,
    action_trigger: &mut Option<MultiplayerAction>,
    block_registry: Option<&minecrust_shared::world::block::BlockRegistry>,
    asset_pack: Option<&minecrust_shared::AssetPack>,
    atlas_texture: Option<egui::TextureId>,
) -> bool {
    let mut exit_requested = false;
    let current_state = *state;

    let bg_color = if current_state == AppState::MainMenu {
        Color32::TRANSPARENT
    } else {
        Color32::from_black_alpha(150)
    };

    egui::CentralPanel::default()
        .frame(Frame::default().fill(bg_color))
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);

                match current_state {
                    AppState::MainMenu => {
                        ui.heading(RichText::new("MINECRUST").size(60.0).strong());
                        ui.add_space(20.0);

                        ui.label(RichText::new("Select Character:").color(Color32::WHITE).size(20.0));
                        ui.add_space(250.0); // Push the buttons down under the 3D models

                        ui.columns(2, |columns| {
                            columns[0].vertical_centered(|ui| {
                                if ui.selectable_value(&mut settings.player_model, crate::steve::PlayerModelType::Steve, "Steve (Wide)").clicked() {}
                            });
                            columns[1].vertical_centered(|ui| {
                                if ui.selectable_value(&mut settings.player_model, crate::steve::PlayerModelType::Alex, "Alex (Slim)").clicked() {}
                            });
                        });
                        
                        ui.add_space(40.0);

                        if ui.add_sized([220.0, 40.0], egui::Button::new(lang.get("menu.singleplayer"))).clicked() {
                            *action_trigger = Some(MultiplayerAction::JoinSingleplayer);
                        }
                        ui.add_space(10.0);
                        
                        if ui.add_sized([220.0, 40.0], egui::Button::new(lang.get("menu.multiplayer"))).clicked() {
                            *state = AppState::MultiplayerMenu;
                        }
                        ui.add_space(10.0);
                        
                        if ui.add_sized([220.0, 40.0], egui::Button::new(lang.get("menu.options"))).clicked() {
                            *state = AppState::Settings { from_in_game: false };
                        }
                        ui.add_space(10.0);
                        
                        if ui.add_sized([220.0, 40.0], egui::Button::new(lang.get("menu.quit"))).clicked() {
                            exit_requested = true;
                        }
                    }
                    AppState::MultiplayerMenu => {
                        ui.heading(RichText::new(lang.get("menu.multiplayer")).size(40.0).strong());
                        ui.add_space(20.0);

                        // Direct Connect input
                        ui.horizontal(|ui| {
                            ui.add_space(ui.available_width() / 2.0 - 150.0);
                            ui.label("服务器地址:");
                            ui.add(egui::TextEdit::singleline(connect_addr).desired_width(180.0));
                        });
                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            ui.add_space(ui.available_width() / 2.0 - 155.0);
                            if ui.add_sized([150.0, 36.0], egui::Button::new("🔍 直连加入")).clicked() {
                                if !connect_addr.trim().is_empty() {
                                    *action_trigger = Some(MultiplayerAction::JoinAddress(connect_addr.clone()));
                                }
                            }
                            if ui.add_sized([150.0, 36.0], egui::Button::new("🌐 开启局域网主机")).clicked() {
                                *action_trigger = Some(MultiplayerAction::HostLan);
                            }
                        });
                        ui.add_space(30.0);

                        // LAN Servers List
                        ui.label(RichText::new("=== 局域网活动服务器 ===").size(20.0).strong().color(Color32::LIGHT_GREEN));
                        ui.add_space(10.0);

                        let servers = discoverer.get_servers();
                        if servers.is_empty() {
                            ui.label(RichText::new("正在搜寻局域网世界...").italics().color(Color32::GRAY));
                        } else {
                            egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                                for srv in servers {
                                    ui.horizontal(|ui| {
                                        ui.add_space(ui.available_width() / 2.0 - 200.0);
                                        let btn_label = format!("🎮 {} [{}]", srv.motd, srv.address);
                                        if ui.add_sized([400.0, 32.0], egui::Button::new(btn_label)).clicked() {
                                            *action_trigger = Some(MultiplayerAction::JoinAddress(srv.address));
                                        }
                                    });
                                    ui.add_space(5.0);
                                }
                            });
                        }

                        ui.add_space(40.0);
                        if ui.add_sized([200.0, 40.0], egui::Button::new("返回主菜单")).clicked() {
                            *state = AppState::MainMenu;
                        }
                    }
                    AppState::InGameMenu => {
                        ui.heading(RichText::new(lang.get("menu.game")).size(40.0).strong());
                        ui.add_space(50.0);

                        if ui.add_sized([200.0, 40.0], egui::Button::new(lang.get("menu.returnToGame"))).clicked() {
                            *state = AppState::InGame;
                        }
                        ui.add_space(10.0);
                        if ui.add_sized([200.0, 40.0], egui::Button::new(lang.get("menu.options"))).clicked() {
                            *state = AppState::Settings { from_in_game: true };
                        }
                        ui.add_space(10.0);
                        if ui.add_sized([200.0, 40.0], egui::Button::new(lang.get("menu.returnToMenu"))).clicked() {
                            *state = AppState::MainMenu;
                        }
                    }
                    AppState::Settings { from_in_game } => {
                        ui.heading(RichText::new(lang.get("menu.options")).size(40.0).strong());
                        ui.add_space(30.0);

                        ui.add_sized(
                            [200.0, 40.0],
                            egui::Slider::new(&mut settings.render_distance, 1..=16).text(lang.get("options.renderDistance")),
                        );
                        ui.add_space(10.0);

                        let vsync_label = format!(
                            "{}: {}",
                            lang.get("options.vsync"),
                            if settings.vsync { lang.get("options.on") } else { lang.get("options.off") }
                        );
                        if ui.add_sized([200.0, 40.0], egui::Button::new(vsync_label)).clicked() {
                            settings.vsync = !settings.vsync;
                        }
                        ui.add_space(10.0);

                        let fs_label = format!(
                            "{}: {}",
                            lang.get("options.fullscreen"),
                            if settings.fullscreen { lang.get("options.on") } else { lang.get("options.off") }
                        );
                        if ui.add_sized([200.0, 40.0], egui::Button::new(fs_label)).clicked() {
                            settings.fullscreen = !settings.fullscreen;
                        }
                        ui.add_space(10.0);

                        let rt_label = format!(
                            "Ray Tracing: {}",
                            if settings.enable_raytracing { "ON" } else { "OFF" }
                        );
                        if ui.add_sized([200.0, 40.0], egui::Button::new(rt_label)).clicked() {
                            settings.enable_raytracing = !settings.enable_raytracing;
                        }
                        ui.add_space(10.0);

                        // Language Toggle Button
                        let current_lang_name = match settings.language.as_str() {
                            "zh_cn" => "简体中文",
                            "en_us" => "English",
                            "ja_jp" => "日本語",
                            _ => "Unknown",
                        };
                        let clean_lang_key = lang.get("options.language").replace("…", "").replace("...", "");
                        let lang_label = format!("{}: {}", clean_lang_key, current_lang_name);
                        if ui.add_sized([200.0, 40.0], egui::Button::new(lang_label)).clicked() {
                            settings.language = match settings.language.as_str() {
                                "zh_cn" => "en_us".to_string(),
                                "en_us" => "ja_jp".to_string(),
                                "ja_jp" => "zh_cn".to_string(),
                                _ => "zh_cn".to_string(),
                            };
                        }
                        ui.add_space(30.0);

                        if ui.add_sized([200.0, 40.0], egui::Button::new(lang.get("gui.done"))).clicked() {
                            *state = if from_in_game {
                                AppState::InGameMenu
                            } else {
                                AppState::MainMenu
                            };
                        }
                    }
                    AppState::Inventory => {
                        ui.heading(RichText::new("物品栏 / Inventory").size(40.0).strong().color(Color32::WHITE));
                        ui.add_space(20.0);
                        
                        if let (Some(registry), Some(pack), Some(atlas_id)) = (block_registry, asset_pack, atlas_texture) {
                            egui::ScrollArea::vertical().max_height(ui.available_height() - 50.0).show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    // Ignore Air (id 0)
                                    let mut items_rendered = 0;
                                    for id in 1..=registry.id_to_name.len() as u16 {
                                        if let Some(name) = registry.get_name(id) {
                                            if let Some(render_data) = pack.block_dict.get(name) {
                                                // We can get UV from North face (index 0) or Top face (index 4)
                                                // Let's use North face
                                                let uv = render_data.uv_faces[0];
                                                let uv_rect = egui::Rect::from_min_max(
                                                    egui::pos2(uv[0], uv[1]),
                                                    egui::pos2(uv[2], uv[3]),
                                                );
                                                
                                                let is_selected = settings.selected_block_id == id;
                                                
                                                ui.allocate_ui(egui::vec2(64.0, 80.0), |ui| {
                                                    ui.vertical_centered(|ui| {
                                                        let img = egui::Image::new(egui::load::SizedTexture::new(atlas_id, [48.0, 48.0])).uv(uv_rect);
                                                        let mut btn = ui.add(egui::ImageButton::new(img));
                                                        
                                                        if is_selected {
                                                            btn = btn.highlight();
                                                        }
                                                        
                                                        if btn.clicked() {
                                                            settings.selected_block_id = id;
                                                            *state = AppState::InGame; // Auto-close after selection
                                                        }
                                                        
                                                        let clean_name = name.replace("minecraft:", "").replace("_", " ");
                                                        ui.label(RichText::new(clean_name).size(10.0).color(if is_selected { Color32::YELLOW } else { Color32::LIGHT_GRAY }));
                                                    });
                                                });
                                                
                                                items_rendered += 1;
                                            }
                                        }
                                    }
                                    
                                    if items_rendered == 0 {
                                        ui.label("正在加载方块资产...");
                                    }
                                });
                            });
                        } else {
                            ui.label("资源尚未准备就绪，请稍后...");
                        }
                    }
                    AppState::InGame => {}
                }
            });
        });

    exit_requested
}
