#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    MainMenu,
    MultiplayerMenu,
    Settings { from_in_game: bool },
    InGame,
    InGameMenu,
}

#[derive(Debug, Clone)]
pub struct AppSettings {
    pub render_distance: i32,
    pub vsync: bool,
    pub fullscreen: bool,
    pub language: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            render_distance: 4,
            vsync: true,
            fullscreen: false,
            language: "zh_cn".to_string(),
        }
    }
}
