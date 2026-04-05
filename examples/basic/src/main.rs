//! Basic terrain example — a 640x640 procedural terrain with five material layers.
//!
//! Demonstrates the minimal setup for `TerrainPlugin`: add the plugin, spawn a
//! `TerrainBundle` with a dataset and config, place a `TerrainFocus` entity
//! (which drives streaming), and set up debug visualization.

use saddle_world_terrain_example_common as common;

use bevy::prelude::*;
use saddle_world_terrain::{
    TerrainBundle, TerrainDebugColorMode, TerrainDebugConfig, TerrainFocus, TerrainPlugin,
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
            title: "Terrain — Basic".into(),
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
    // --------------- Terrain config ---------------
    // default_config() provides a 640x640 terrain with 5 material layers:
    //   water, meadow, dirt, rock, snow — blended by height and slope.
    let config = common::default_config();

    // --------------- Spawn terrain ---------------
    let terrain = commands
        .spawn(TerrainBundle::new(
            common::generated_dataset(UVec2::new(257, 257)),
            config,
        ))
        .id();

    // --------------- Focus entity (drives chunk streaming) ---------------
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

    // --------------- Camera ---------------
    commands.spawn((
        Name::new("Example Camera"),
        ExampleCamera,
        Camera3d::default(),
        Transform::from_xyz(180.0, 140.0, 220.0).looking_at(Vec3::new(320.0, 20.0, 320.0), Vec3::Y),
    ));

    // --------------- Lighting ---------------
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

    // --------------- Debug visualization ---------------
    debug.show_chunk_bounds = true;
    debug.show_focus_rings = true;
    debug.color_mode = TerrainDebugColorMode::Natural;
}

/// Move the focus entity in an elliptical orbit so we see streaming in action.
fn animate_focus(time: Res<Time>, mut q: Query<&mut Transform, With<ExampleFocus>>) {
    let t = time.elapsed_secs() * 0.18;
    for mut transform in &mut q {
        transform.translation.x = 320.0 + 210.0 * t.cos();
        transform.translation.z = 320.0 + 180.0 * (t * 0.7).sin();
    }
}

/// Keep the camera looking at the focus point.
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
