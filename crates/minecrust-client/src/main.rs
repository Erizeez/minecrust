pub mod app;
pub mod game;
pub mod state;
pub mod ui;

use app::MinecrustApp;
use minecrust_engine::EngineRunner;

fn main() -> anyhow::Result<()> {
    let app = MinecrustApp::new();
    let runner = EngineRunner::new(app);
    runner.run()
}
