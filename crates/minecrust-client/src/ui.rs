use crate::state::{AppState, AppSettings};
use crate::lang::LangManager;
use minecrust_engine::egui::{self, Color32, Frame, RichText};

/// Renders the UI and returns true if the application should exit.
pub fn render_menus(
    ctx: &egui::Context,
    state: &mut AppState,
    settings: &mut AppSettings,
    lang: &LangManager,
) -> bool {
    let mut exit_requested = false;
    let current_state = *state;

    egui::CentralPanel::default()
        .frame(Frame::default().fill(Color32::from_black_alpha(150)))
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);

                match current_state {
                    AppState::MainMenu => {
                        ui.heading(RichText::new("MINECRUST").size(60.0).strong());
                        ui.add_space(50.0);

                        if ui.add_sized([200.0, 40.0], egui::Button::new(lang.get("menu.singleplayer"))).clicked() {
                            *state = AppState::InGame;
                        }
                        ui.add_space(10.0);
                        ui.add_sized([200.0, 40.0], egui::Button::new(format!("{} (WIP)", lang.get("menu.multiplayer"))));
                        ui.add_space(10.0);
                        if ui.add_sized([200.0, 40.0], egui::Button::new(lang.get("menu.options"))).clicked() {
                            *state = AppState::Settings { from_in_game: false };
                        }
                        ui.add_space(10.0);
                        if ui.add_sized([200.0, 40.0], egui::Button::new(lang.get("menu.quit"))).clicked() {
                            exit_requested = true;
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
                        ui.add_space(50.0);

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
                    _ => {}
                }
            });
        });

    exit_requested
}
