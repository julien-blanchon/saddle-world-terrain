//! Mountain range example — rugged high-elevation terrain with snow at peaks.
//!
//! Demonstrates:
//!   - Terrain with high `height_scale` for dramatic elevation changes
//!   - Slope-based rock layer on steep faces
//!   - Snow layer at high elevations
//!   - Multiple overlapping noise layers for realistic mountain shapes
//!   - Orbit camera starting from a dramatic viewpoint
//!
//! Controls:
//!   - Left mouse drag: orbit camera
//!   - Middle mouse drag: pan camera
//!   - Scroll wheel: zoom in/out

use bevy::prelude::*;
use saddle_camera_orbit_camera::{OrbitCamera, OrbitCameraInputTarget, OrbitCameraPlugin};
use saddle_world_terrain::{
    TerrainBlendRange, TerrainBundle, TerrainConfig, TerrainDataset, TerrainFocus, TerrainLayer,
    TerrainMaterialProfile, TerrainPlugin, TerrainStreamingConfig,
};

const TERRAIN_SIZE: Vec2 = Vec2::new(800.0, 800.0);
const TERRAIN_CENTER: Vec2 = Vec2::new(400.0, 400.0);

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.68, 0.78, 0.92)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Terrain — Mountain Range".into(),
                resolution: (1440, 900).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((TerrainPlugin::default(), OrbitCameraPlugin::default()))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    let dataset = build_mountain_dataset();
    let config = mountain_config();
    let terrain = commands.spawn(TerrainBundle::new(dataset, config)).id();

    // Camera — positioned to see the mountain range from a dramatic angle
    commands.spawn((
        Name::new("Mountain Camera"),
        OrbitCamera::looking_at(
            Vec3::new(TERRAIN_CENTER.x, 60.0, TERRAIN_CENTER.y),
            Vec3::new(TERRAIN_CENTER.x - 220.0, 180.0, TERRAIN_CENTER.y + 260.0),
        ),
        OrbitCameraInputTarget,
        TerrainFocus {
            terrain: Some(terrain),
            ..default()
        },
    ));

    // Lighting — low sun angle for dramatic shadows on mountain faces
    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.45, 0.65, 0.0)),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.85, 0.88, 0.96),
        brightness: 140.0,
        affects_lightmapped_meshes: true,
    });

    // HUD
    commands.spawn((
        Name::new("Mountain Overlay"),
        Text::new("Mountain Range\nOrbit: left drag  Pan: middle drag  Zoom: wheel\nHigh elevation with snow peaks, slope-based rock, and valley meadows"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: px(18.0),
            top: px(18.0),
            padding: UiRect::all(px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.07, 0.10, 0.76)),
    ));
}

fn build_mountain_dataset() -> TerrainDataset {
    let dims = UVec2::new(257, 257);
    TerrainDataset::from_fn(dims, |_coord, uv| {
        // Major ridge line running diagonally
        let ridge_axis = (uv.x * 0.7 + uv.y * 0.3 - 0.5).abs();
        let ridge = (1.0 - ridge_axis * 3.2).max(0.0).powf(0.8);

        // Secondary peaks
        let peak1_dist = (uv - Vec2::new(0.3, 0.35)).length();
        let peak1 = (1.0 - peak1_dist * 2.5).max(0.0).powf(1.2);

        let peak2_dist = (uv - Vec2::new(0.65, 0.55)).length();
        let peak2 = (1.0 - peak2_dist * 2.2).max(0.0).powf(1.1);

        let peak3_dist = (uv - Vec2::new(0.5, 0.7)).length();
        let peak3 = (1.0 - peak3_dist * 2.8).max(0.0).powf(1.3);

        // Noise detail layers
        let noise1 = ((uv.x * 12.5).sin() * (uv.y * 11.3).cos() * 0.3
            + (uv.x * 6.2 + uv.y * 8.1).sin() * 0.2)
            * 0.5
            + 0.5;
        let noise2 = ((uv.x * 28.0 + 1.5).sin() * (uv.y * 24.0 - 0.8).cos()) * 0.08;

        // Valley carved between peaks
        let valley_axis = ((uv.x - 0.48) * 2.0).abs();
        let valley = (1.0 - valley_axis * 3.0).max(0.0) * 0.15;

        let elevation =
            ridge * 0.35 + peak1 * 0.30 + peak2 * 0.25 + peak3 * 0.20 + noise1 * 0.12 + noise2
                - valley;

        // Base elevation so valleys aren't at zero
        (elevation * 0.85 + 0.08).clamp(0.0, 1.0)
    })
    .expect("mountain heights should match dimensions")
}

fn mountain_config() -> TerrainConfig {
    TerrainConfig {
        size: TERRAIN_SIZE,
        chunk_size: Vec2::new(48.0, 48.0),
        vertex_resolution: 48,
        height_scale: 160.0,
        height_offset: -12.0,
        skirt_depth: 12.0,
        streaming: TerrainStreamingConfig {
            visual_radius: 240.0,
            collider_radius: 100.0,
            max_builds_per_frame: 8,
        },
        material: TerrainMaterialProfile {
            layers: vec![
                TerrainLayer::tinted("valley_floor", Color::srgb(0.34, 0.28, 0.20))
                    .with_height_range(TerrainBlendRange::new(0.0, 0.20)),
                TerrainLayer::tinted("meadow", Color::srgb(0.30, 0.52, 0.22))
                    .with_height_range(TerrainBlendRange::new(0.12, 0.40)),
                TerrainLayer::tinted("alpine", Color::srgb(0.38, 0.44, 0.28))
                    .with_height_range(TerrainBlendRange::new(0.30, 0.58)),
                TerrainLayer::tinted("rock", Color::srgb(0.48, 0.46, 0.44))
                    .with_slope_range(TerrainBlendRange {
                        start: 20.0,
                        end: 65.0,
                        falloff: 0.14,
                    })
                    .with_height_range(TerrainBlendRange::new(0.25, 0.90))
                    .with_strength(1.3),
                TerrainLayer::tinted("snow", Color::srgb(0.95, 0.96, 0.98))
                    .with_height_range(TerrainBlendRange {
                        start: 0.58,
                        end: 1.0,
                        falloff: 0.08,
                    })
                    .with_slope_range(TerrainBlendRange {
                        start: 0.0,
                        end: 40.0,
                        falloff: 0.20,
                    }),
            ],
            ..default()
        },
        ..default()
    }
}
