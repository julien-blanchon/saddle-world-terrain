use saddle_world_terrain_example_common as common;

use bevy::prelude::*;

fn main() {
    let mut app = App::new();
    common::configure_app(&mut app, "Terrain Collider Payload");
    app.add_systems(Startup, setup);
    app.add_systems(Update, (common::animate_focus, common::follow_focus));
    app.run();
}

fn setup(mut commands: Commands, mut debug: ResMut<saddle_world_terrain::TerrainDebugConfig>) {
    let mut config = common::default_config();
    config.collider.enabled = true;
    config.collider.resolution_divisor = 4;
    config.streaming.collider_radius = 70.0;
    let terrain = common::spawn_terrain(&mut commands, config);
    common::spawn_scene(&mut commands, terrain);
    common::enable_debug(
        &mut debug,
        saddle_world_terrain::TerrainDebugColorMode::Natural,
    );
    debug.show_collider_bounds = true;
}
