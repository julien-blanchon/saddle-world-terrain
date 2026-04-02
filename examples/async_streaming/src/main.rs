use saddle_world_terrain_example_common as common;

use bevy::prelude::*;

fn main() {
    let mut app = App::new();
    common::configure_app(&mut app, "Terrain Async Streaming");
    app.add_systems(Startup, setup);
    app.add_systems(Update, (common::animate_focus, common::follow_focus));
    app.run();
}

fn setup(mut commands: Commands, mut debug: ResMut<saddle_world_terrain::TerrainDebugConfig>) {
    let mut config = common::default_config();
    config.size = Vec2::new(1024.0, 1024.0);
    config.streaming.max_builds_per_frame = 2;
    config.streaming.visual_radius = 240.0;
    config.lod.hysteresis = 16.0;
    let terrain = common::spawn_terrain(&mut commands, config);
    commands.insert_resource(saddle_world_terrain::TerrainFocusPoints(vec![
        saddle_world_terrain::TerrainFocusPoint {
            terrain: Some(terrain),
            position: Vec3::new(760.0, 0.0, 260.0),
            visual_radius_bias: -40.0,
            collider_radius_bias: -70.0,
        },
    ]));
    common::spawn_scene(&mut commands, terrain);
    common::enable_debug(&mut debug, saddle_world_terrain::TerrainDebugColorMode::Natural);
}
