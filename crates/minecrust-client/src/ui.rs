use crate::state::{AppState, AppSettings};
use minecrust_engine::egui::{self, Color32, Frame, RichText};

/// Renders the UI and returns true if the application should exit.
pub fn render_menus(ctx: &egui::Context, state: &mut AppState, settings: &mut AppSettings) -> bool {
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

                        if ui.add_sized([200.0, 40.0], egui::Button::new("Singleplayer")).clicked() {
                            *state = AppState::InGame;
                        }
                        ui.add_space(10.0);
                        ui.add_sized([200.0, 40.0], egui::Button::new("Multiplayer (WIP)"));
                        ui.add_space(10.0);
                        if ui.add_sized([200.0, 40.0], egui::Button::new("Settings")).clicked() {
                            *state = AppState::Settings { from_in_game: false };
                        }
                        ui.add_space(10.0);
                        if ui.add_sized([200.0, 40.0], egui::Button::new("Quit Game")).clicked() {
                            exit_requested = true;
                        }
                    }
                    AppState::InGameMenu => {
                        ui.heading(RichText::new("Game Menu").size(40.0).strong());
                        ui.add_space(50.0);

                        if ui.add_sized([200.0, 40.0], egui::Button::new("Back to Game")).clicked() {
                            *state = AppState::InGame;
                        }
                        ui.add_space(10.0);
                        if ui.add_sized([200.0, 40.0], egui::Button::new("Settings")).clicked() {
                            *state = AppState::Settings { from_in_game: true };
                        }
                        ui.add_space(10.0);
                        if ui.add_sized([200.0, 40.0], egui::Button::new("Save and Quit to Title")).clicked() {
                            *state = AppState::MainMenu;
                        }
                    }
                    AppState::Settings { from_in_game } => {
                        ui.heading(RichText::new("Settings").size(40.0).strong());
                        ui.add_space(50.0);

                        ui.add_sized(
                            [200.0, 40.0],
                            egui::Slider::new(&mut settings.render_distance, 1..=16).text("Render Distance"),
                        );
                        ui.add_space(10.0);

                        let vsync_text = if settings.vsync { "VSync: ON" } else { "VSync: OFF" };
                        if ui.add_sized([200.0, 40.0], egui::Button::new(vsync_text)).clicked() {
                            settings.vsync = !settings.vsync;
                        }
                        ui.add_space(10.0);

                        let fs_text = if settings.fullscreen {
                            "Fullscreen: ON"
                        } else {
                            "Fullscreen: OFF"
                        };
                        if ui.add_sized([200.0, 40.0], egui::Button::new(fs_text)).clicked() {
                            settings.fullscreen = !settings.fullscreen;
                        }
                        ui.add_space(30.0);

                        if ui.add_sized([200.0, 40.0], egui::Button::new("Done")).clicked() {
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
