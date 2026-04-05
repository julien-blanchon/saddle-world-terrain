//! Async streaming example — large terrain with throttled chunk builds.
//!
//! Demonstrates streaming configuration on a 1024x1024 terrain:
//!   - `max_builds_per_frame = 2` to throttle chunk builds (simulates a budget)
//!   - `visual_radius = 240.0` for a wide view distance
//!   - `lod.hysteresis = 16.0` to prevent LOD thrashing at boundaries
//!   - A secondary `TerrainFocusPoint` to show multi-focus streaming

use saddle_world_terrain_example_common as common;

use bevy::prelude::*;
use saddle_world_terrain::{
    TerrainBundle, TerrainDebugColorMode, TerrainDebugConfig, TerrainFocus, TerrainFocusPoint,
    TerrainFocusPoints, TerrainPlugin,
};

#[derive(Component)]
struct ExampleFocus;

#[derive(Component)]
struct ExampleCamera;

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.80, 0.88, 0.95)));
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Terrain — Async Streaming".into(),
            resolution: (1440, 900).into(),
            ..default()
        }),
        ..default()
    }));
    app.add_plugins(TerrainPlugin::default());
    app.add_systems(Startup, setup);
    app.add_systems(Update, (animate_focus, follow_focus));
    app.run();
}

fn setup(mut commands: Commands, mut debug: ResMut<TerrainDebugConfig>) {
    // --------------- Large terrain with throttled streaming ---------------
    let mut config = common::default_config();
    config.size = Vec2::new(1024.0, 1024.0); // larger world
    config.streaming.max_builds_per_frame = 2; // throttle to 2 chunks/frame
    config.streaming.visual_radius = 240.0; // wide view distance
    config.lod.hysteresis = 16.0; // prevent LOD thrashing

    // Spawn terrain
    let terrain = commands
        .spawn(TerrainBundle::new(
            common::generated_dataset(UVec2::new(257, 257)),
            config,
        ))
        .id();

    // --------------- Secondary focus point (resource-based) ---------------
    // This adds a second streaming origin far from the primary focus,
    // with custom radius biases.
    commands.insert_resource(TerrainFocusPoints(vec![TerrainFocusPoint {
        terrain: Some(terrain),
        position: Vec3::new(760.0, 0.0, 260.0),
        visual_radius_bias: -40.0,
        collider_radius_bias: -70.0,
    }]));

    // --------------- Primary focus entity ---------------
    commands.spawn((
        Name::new("Terrain Focus"),
        ExampleFocus,
        TerrainFocus {
            terrain: Some(terrain),
            ..default()
        },
        Transform::from_xyz(320.0, 0.0, 320.0),
        GlobalTransform::default(),
    ));

    // Camera
    commands.spawn((
        Name::new("Example Camera"),
        ExampleCamera,
        Camera3d::default(),
        Transform::from_xyz(180.0, 140.0, 220.0).looking_at(Vec3::new(320.0, 20.0, 320.0), Vec3::Y),
    ));

    // Lighting
    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.7, 0.45, 0.0)),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 200.0,
        affects_lightmapped_meshes: true,
    });

    // Debug
    debug.show_chunk_bounds = true;
    debug.show_focus_rings = true;
    debug.color_mode = TerrainDebugColorMode::Natural;
}

fn animate_focus(time: Res<Time>, mut q: Query<&mut Transform, With<ExampleFocus>>) {
    let t = time.elapsed_secs() * 0.18;
    for mut transform in &mut q {
        transform.translation.x = 320.0 + 210.0 * t.cos();
        transform.translation.z = 320.0 + 180.0 * (t * 0.7).sin();
    }
}

fn follow_focus(
    focus: Query<&Transform, With<ExampleFocus>>,
    mut camera: Query<&mut Transform, (With<ExampleCamera>, Without<ExampleFocus>)>,
) {
    let Ok(focus) = focus.single() else { return };
    let Ok(mut cam) = camera.single_mut() else {
        return;
    };
    cam.look_at(focus.translation + Vec3::new(0.0, 28.0, 0.0), Vec3::Y);
}
