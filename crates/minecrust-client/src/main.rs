pub mod app;
pub mod asset_loader;
pub mod game;
pub mod lang;
pub mod state;
pub mod ui;
pub mod lan;
pub mod steve;
pub mod entity_meshes;

use app::MinecrustApp;
use minecrust_engine::EngineRunner;

fn main() -> anyhow::Result<()> {
    let app = MinecrustApp::new();
    let runner = EngineRunner::new(app);
    runner.run()
}
