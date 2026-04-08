//! Terrain sculpting example — runtime terrain modification demo.
//!
//! Demonstrates:
//!   - Replacing the `TerrainSourceHandle` at runtime to update terrain
//!   - Brush-based terrain raising/lowering by modifying the height dataset
//!   - The terrain system automatically rebuilds affected chunks
//!   - Orbit camera for free exploration
//!
//! Controls:
//!   - Left mouse drag: orbit camera
//!   - Middle mouse drag: pan camera
//!   - Scroll wheel: zoom in/out
//!   - Space: raise terrain at a roaming brush position
//!   - Shift+Space: lower terrain at the brush position
//!   - R: reset terrain to original heights

use std::sync::{Arc, Mutex};

use saddle_world_terrain_example_common as common;

use bevy::prelude::*;
use saddle_camera_orbit_camera::{OrbitCamera, OrbitCameraInputTarget, OrbitCameraPlugin};
use saddle_world_terrain::{
    TerrainBlendRange, TerrainBundle, TerrainConfig, TerrainFocus, TerrainLayer,
    TerrainDebugConfig, TerrainMaterialProfile, TerrainPlugin, TerrainSource, TerrainSourceHandle,
    TerrainStreamingConfig,
};

const TERRAIN_SIZE: Vec2 = Vec2::new(256.0, 256.0);
const TERRAIN_CENTER: Vec2 = Vec2::new(128.0, 128.0);
const DIMENSIONS: UVec2 = UVec2::new(129, 129);
const BRUSH_RADIUS: f32 = 0.08;
const BRUSH_STRENGTH: f32 = 0.012;

#[derive(Component)]
struct BrushMarker;

#[derive(Resource)]
struct SculptState {
    heights: Arc<Mutex<Vec<f32>>>,
    original_heights: Vec<f32>,
    brush_uv: Vec2,
    terrain_entity: Entity,
    stroke_count: u32,
}

/// A terrain source backed by a shared, mutable height buffer.
#[derive(Clone)]
struct MutableHeightSource {
    dimensions: UVec2,
    heights: Arc<Mutex<Vec<f32>>>,
}

impl TerrainSource for MutableHeightSource {
    fn height_dimensions(&self) -> UVec2 {
        self.dimensions
    }

    fn sample_height(&self, uv: Vec2) -> f32 {
        let clamped = uv.clamp(Vec2::ZERO, Vec2::ONE);
        let x = clamped.x * (self.dimensions.x.saturating_sub(1)) as f32;
        let y = clamped.y * (self.dimensions.y.saturating_sub(1)) as f32;
        let x0 = x.floor() as u32;
        let y0 = y.floor() as u32;
        let x1 = (x0 + 1).min(self.dimensions.x.saturating_sub(1));
        let y1 = (y0 + 1).min(self.dimensions.y.saturating_sub(1));
        let tx = x - x0 as f32;
        let ty = y - y0 as f32;

        let heights = self.heights.lock().unwrap();
        let get = |x: u32, y: u32| -> f32 { heights[(y * self.dimensions.x + x) as usize] };
        let a = get(x0, y0);
        let b = get(x1, y0);
        let c = get(x0, y1);
        let d = get(x1, y1);
        let ab = a + (b - a) * tx;
        let cd = c + (d - c) * tx;
        ab + (cd - ab) * ty
    }

    fn explicit_weight_channel_count(&self) -> usize {
        0
    }

    fn sample_explicit_weight(&self, _channel: usize, _uv: Vec2) -> f32 {
        0.0
    }
}

fn main() {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.72, 0.82, 0.90)));
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Terrain — Sculpting".into(),
            resolution: (1440, 900).into(),
            ..default()
        }),
        ..default()
    }));
    app.add_plugins((TerrainPlugin::default(), OrbitCameraPlugin::default()));
    common::install_terrain_example_debug_ui(&mut app);
    app.add_systems(Startup, setup);
    app.add_systems(Update, (move_brush, sculpt_terrain, update_overlay));
    app.run();
}

fn setup(
    mut commands: Commands,
    debug: Res<TerrainDebugConfig>,
    mut pane: ResMut<common::TerrainExamplePane>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let heights = generate_base_heights();
    let original_heights = heights.clone();
    let shared_heights = Arc::new(Mutex::new(heights));

    let source = MutableHeightSource {
        dimensions: DIMENSIONS,
        heights: shared_heights.clone(),
    };

    let config = sculpt_config();
    let pane_state = common::terrain_example_pane(&config, &debug);
    let terrain = commands.spawn(TerrainBundle::new(source, config)).id();
    *pane = pane_state;

    commands.insert_resource(SculptState {
        heights: shared_heights,
        original_heights,
        brush_uv: Vec2::new(0.5, 0.5),
        terrain_entity: terrain,
        stroke_count: 0,
    });

    // Brush indicator sphere
    commands.spawn((
        Name::new("Brush Marker"),
        BrushMarker,
        Mesh3d(
            meshes.add(
                Sphere::new(BRUSH_RADIUS * TERRAIN_SIZE.x * 0.5)
                    .mesh()
                    .uv(16, 10),
            ),
        ),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.96, 0.42, 0.28, 0.65),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(TERRAIN_CENTER.x, 30.0, TERRAIN_CENTER.y),
    ));

    // Camera
    commands.spawn((
        Name::new("Sculpt Camera"),
        OrbitCamera::looking_at(
            Vec3::new(TERRAIN_CENTER.x, 20.0, TERRAIN_CENTER.y),
            Vec3::new(TERRAIN_CENTER.x + 100.0, 90.0, TERRAIN_CENTER.y + 120.0),
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
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.7, 0.45, 0.0)),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 200.0,
        affects_lightmapped_meshes: true,
    });

    // HUD
    commands.spawn((
        Name::new("Sculpt Overlay"),
        SculptOverlay,
        Text::default(),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: px(18.0),
            top: px(18.0),
            width: px(520.0),
            padding: UiRect::all(px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.07, 0.10, 0.76)),
    ));
}

#[derive(Component)]
struct SculptOverlay;

fn generate_base_heights() -> Vec<f32> {
    let count = DIMENSIONS.x as usize * DIMENSIONS.y as usize;
    let mut heights = Vec::with_capacity(count);
    for y in 0..DIMENSIONS.y {
        for x in 0..DIMENSIONS.x {
            let uv = Vec2::new(
                x as f32 / (DIMENSIONS.x - 1) as f32,
                y as f32 / (DIMENSIONS.y - 1) as f32,
            );
            let noise = ((uv.x * 6.0).sin() * (uv.y * 5.0).cos() * 0.3
                + (uv.x * 13.0 + uv.y * 9.0).sin() * 0.1)
                * 0.5
                + 0.35;
            heights.push(noise.clamp(0.0, 1.0));
        }
    }
    heights
}

fn move_brush(
    time: Res<Time>,
    mut state: ResMut<SculptState>,
    mut markers: Query<&mut Transform, With<BrushMarker>>,
) {
    let t = time.elapsed_secs() * 0.3;
    state.brush_uv = Vec2::new(0.5 + 0.3 * t.cos(), 0.5 + 0.25 * (t * 0.7).sin());

    let world_x = state.brush_uv.x * TERRAIN_SIZE.x;
    let world_z = state.brush_uv.y * TERRAIN_SIZE.y;

    for mut transform in &mut markers {
        transform.translation.x = world_x;
        transform.translation.z = world_z;
    }
}

fn sculpt_terrain(
    input: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<SculptState>,
    mut terrains: Query<&mut TerrainSourceHandle>,
) {
    let raising = input.pressed(KeyCode::Space) && !input.pressed(KeyCode::ShiftLeft);
    let lowering = input.pressed(KeyCode::Space) && input.pressed(KeyCode::ShiftLeft);
    let reset = input.just_pressed(KeyCode::KeyR);

    if reset {
        {
            let mut heights = state.heights.lock().unwrap();
            heights.copy_from_slice(&state.original_heights);
        }
        state.stroke_count = 0;
        // Trigger rebuild by re-inserting the source handle
        if let Ok(mut handle) = terrains.get_mut(state.terrain_entity) {
            let new_source = MutableHeightSource {
                dimensions: DIMENSIONS,
                heights: state.heights.clone(),
            };
            *handle = TerrainSourceHandle::new(new_source);
        }
        return;
    }

    if !raising && !lowering {
        return;
    }

    let direction = if raising { 1.0 } else { -1.0 };
    let brush_uv = state.brush_uv;

    {
        let mut heights = state.heights.lock().unwrap();
        for y in 0..DIMENSIONS.y {
            for x in 0..DIMENSIONS.x {
                let uv = Vec2::new(
                    x as f32 / (DIMENSIONS.x - 1) as f32,
                    y as f32 / (DIMENSIONS.y - 1) as f32,
                );
                let dist = (uv - brush_uv).length();
                if dist < BRUSH_RADIUS {
                    let falloff = 1.0 - (dist / BRUSH_RADIUS);
                    let influence = falloff * falloff * BRUSH_STRENGTH * direction;
                    let idx = (y * DIMENSIONS.x + x) as usize;
                    heights[idx] = (heights[idx] + influence).clamp(0.0, 1.0);
                }
            }
        }
    }

    state.stroke_count += 1;

    // Re-insert the source to trigger chunk rebuilds
    if let Ok(mut handle) = terrains.get_mut(state.terrain_entity) {
        let new_source = MutableHeightSource {
            dimensions: DIMENSIONS,
            heights: state.heights.clone(),
        };
        *handle = TerrainSourceHandle::new(new_source);
    }
}

fn update_overlay(state: Res<SculptState>, mut overlay: Query<&mut Text, With<SculptOverlay>>) {
    let Ok(mut text) = overlay.single_mut() else {
        return;
    };
    text.0 = format!(
        "Terrain Sculpting Demo\nOrbit: left drag  Pan: middle drag  Zoom: wheel\nSpace: raise terrain  Shift+Space: lower  R: reset\nBrush UV: ({:.2}, {:.2})  Strokes: {}",
        state.brush_uv.x, state.brush_uv.y, state.stroke_count,
    );
}

fn sculpt_config() -> TerrainConfig {
    TerrainConfig {
        size: TERRAIN_SIZE,
        chunk_size: Vec2::new(32.0, 32.0),
        vertex_resolution: 32,
        height_scale: 60.0,
        height_offset: -4.0,
        skirt_depth: 6.0,
        streaming: TerrainStreamingConfig {
            visual_radius: 160.0,
            collider_radius: 60.0,
            max_builds_per_frame: 12,
        },
        material: TerrainMaterialProfile {
            layers: vec![
                TerrainLayer::tinted("low", Color::srgb(0.34, 0.54, 0.22))
                    .with_height_range(TerrainBlendRange::new(0.0, 0.45)),
                TerrainLayer::tinted("mid", Color::srgb(0.52, 0.46, 0.32))
                    .with_height_range(TerrainBlendRange::new(0.30, 0.70)),
                TerrainLayer::tinted("rock", Color::srgb(0.56, 0.54, 0.50)).with_slope_range(
                    TerrainBlendRange {
                        start: 22.0,
                        end: 68.0,
                        falloff: 0.16,
                    },
                ),
                TerrainLayer::tinted("high", Color::srgb(0.88, 0.86, 0.82)).with_height_range(
                    TerrainBlendRange {
                        start: 0.65,
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
