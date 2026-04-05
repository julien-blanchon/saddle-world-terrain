//! Island example — terrain shaped as an island surrounded by water.
//!
//! Demonstrates:
//!   - Custom heightmap generation using radial falloff for an island shape
//!   - Multiple terrain layers: sand, grass, rock, snow at peaks
//!   - A flat water plane at sea level
//!   - Orbit camera for free exploration
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

const TERRAIN_SIZE: Vec2 = Vec2::new(512.0, 512.0);
const TERRAIN_CENTER: Vec2 = Vec2::new(256.0, 256.0);
const SEA_LEVEL: f32 = 2.0;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.52, 0.72, 0.88)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Terrain — Island".into(),
                resolution: (1440, 900).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((TerrainPlugin::default(), OrbitCameraPlugin::default()))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dataset = build_island_dataset();
    let config = island_config();
    let terrain = commands.spawn(TerrainBundle::new(dataset, config)).id();

    // Water plane at sea level
    commands.spawn((
        Name::new("Ocean Surface"),
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(600.0)).mesh())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.08, 0.28, 0.52, 0.82),
            perceptual_roughness: 0.15,
            metallic: 0.1,
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_xyz(TERRAIN_CENTER.x, SEA_LEVEL, TERRAIN_CENTER.y),
    ));

    // Camera
    commands.spawn((
        Name::new("Island Camera"),
        OrbitCamera::looking_at(
            Vec3::new(TERRAIN_CENTER.x, 20.0, TERRAIN_CENTER.y),
            Vec3::new(TERRAIN_CENTER.x + 180.0, 120.0, TERRAIN_CENTER.y + 200.0),
        ),
        OrbitCameraInputTarget,
        TerrainFocus {
            terrain: Some(terrain),
            ..default()
        },
    ));

    // Lighting
    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.65, 0.5, 0.0)),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 180.0,
        affects_lightmapped_meshes: true,
    });

    // HUD
    commands.spawn((
        Name::new("Island Overlay"),
        Text::new("Island Terrain\nOrbit: left drag  Pan: middle drag  Zoom: wheel\nIsland shaped by radial falloff + noise — 5 material layers"),
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

fn build_island_dataset() -> TerrainDataset {
    let dims = UVec2::new(257, 257);
    TerrainDataset::from_fn(dims, |_coord, uv| {
        let center = Vec2::new(0.5, 0.5);
        let dist = (uv - center).length() * 2.0;

        // Radial falloff — creates the island shape
        let island_mask = (1.0 - dist.powf(1.8)).max(0.0);

        // Noise layers (simple procedural without external crate)
        let ridge = ((uv.x * 8.3 + uv.y * 2.1).sin() * 0.4 + (uv.y * 7.7 - uv.x * 3.5).cos() * 0.3)
            * 0.5
            + 0.5;
        let detail = ((uv.x * 22.0).sin() * (uv.y * 19.0).cos() * 0.15) * 0.5 + 0.5;
        let mountain = ((uv.x * 4.2 + 0.3).sin() * (uv.y * 3.8 + 0.7).cos()).abs();

        let base = island_mask * (ridge * 0.45 + mountain * 0.35 + detail * 0.20);
        base.clamp(0.0, 1.0)
    })
    .expect("island heights should match dimensions")
}

fn island_config() -> TerrainConfig {
    TerrainConfig {
        size: TERRAIN_SIZE,
        chunk_size: Vec2::new(40.0, 40.0),
        vertex_resolution: 48,
        height_scale: 90.0,
        height_offset: -4.0,
        skirt_depth: 8.0,
        streaming: TerrainStreamingConfig {
            visual_radius: 200.0,
            collider_radius: 80.0,
            max_builds_per_frame: 8,
        },
        material: TerrainMaterialProfile {
            layers: vec![
                TerrainLayer::tinted("sand", Color::srgb(0.82, 0.74, 0.56))
                    .with_height_range(TerrainBlendRange::new(0.0, 0.15)),
                TerrainLayer::tinted("grass", Color::srgb(0.26, 0.54, 0.22))
                    .with_height_range(TerrainBlendRange::new(0.08, 0.55)),
                TerrainLayer::tinted("forest", Color::srgb(0.16, 0.38, 0.14))
                    .with_height_range(TerrainBlendRange::new(0.30, 0.65)),
                TerrainLayer::tinted("rock", Color::srgb(0.52, 0.50, 0.48)).with_slope_range(
                    TerrainBlendRange {
                        start: 25.0,
                        end: 70.0,
                        falloff: 0.18,
                    },
                ),
                TerrainLayer::tinted("snow", Color::srgb(0.94, 0.96, 0.98)).with_height_range(
                    TerrainBlendRange {
                        start: 0.72,
                        end: 1.0,
                        falloff: 0.12,
                    },
                ),
            ],
            ..default()
        },
        ..default()
    }
}
