use saddle_world_terrain_example_common as common;

use bevy::prelude::*;
use saddle_world_terrain::TerrainDebugColorMode;

fn main() {
    let mut app = App::new();
    common::configure_app(&mut app, "Terrain Splat Layers");
    app.add_systems(Startup, setup);
    app.add_systems(Update, (common::animate_focus, common::follow_focus));
    app.run();
}

fn setup(mut commands: Commands, mut debug: ResMut<saddle_world_terrain::TerrainDebugConfig>) {
    let mut config = common::default_config();
    config.streaming.visual_radius = 220.0;
    let terrain = common::spawn_terrain(&mut commands, config);
    common::spawn_scene(&mut commands, terrain);
    common::enable_debug(&mut debug, TerrainDebugColorMode::ByLayerDominance);
}
