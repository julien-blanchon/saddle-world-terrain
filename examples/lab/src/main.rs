#[cfg(feature = "e2e")]
mod e2e;
#[cfg(feature = "e2e")]
mod scenarios;

use std::sync::{Arc, Mutex};

use saddle_world_terrain_example_common as common;

use bevy::{
    prelude::*,
    remote::{RemotePlugin, http::RemoteHttpPlugin},
};
use saddle_camera_orbit_camera::{OrbitCamera, OrbitCameraInputTarget, OrbitCameraPlugin};
use saddle_world_terrain::{
    TerrainBlendRange, TerrainBundle, TerrainConfig, TerrainDebugColorMode, TerrainDebugConfig,
    TerrainDiagnostics, TerrainFocus, TerrainFocusPoint, TerrainFocusPoints, TerrainLayer,
    TerrainMaterialProfile, TerrainPlugin, TerrainProbe, TerrainProbeSample, TerrainRootStats,
    TerrainSource, TerrainSourceHandle, TerrainStreamingConfig,
};

const ISLAND_SEA_LEVEL: f32 = 2.0;
const SCULPT_TERRAIN_SIZE: Vec2 = Vec2::new(256.0, 256.0);
const SCULPT_TERRAIN_CENTER: Vec2 = Vec2::new(128.0, 128.0);
const SCULPT_DIMENSIONS: UVec2 = UVec2::new(129, 129);
const SCULPT_BRUSH_RADIUS: f32 = 0.08;
#[cfg_attr(not(feature = "e2e"), allow(dead_code))]
const SCULPT_BRUSH_STRENGTH: f32 = 0.012;

#[cfg_attr(not(feature = "e2e"), allow(dead_code))]
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum LabSceneKind {
    #[default]
    Lab,
    Basic,
    ClipmapDebug,
    SplatLayers,
    AsyncStreaming,
    PhysicsColliders,
    Island,
    MountainRange,
    TerrainSculpting,
}

#[derive(Resource, Clone, Copy)]
pub(crate) struct LabSceneInfo {
    pub name: &'static str,
}

#[derive(Component)]
pub(crate) struct LabFocus;

#[derive(Component)]
pub(crate) struct LabCamera;

#[derive(Component)]
struct LabOverlay;

#[derive(Component)]
struct LabFocusMarker;

#[derive(Component)]
struct LabSculptBrushMarker;

#[derive(Resource)]
struct LabFocusControl {
    auto_roam: bool,
    move_speed: f32,
}

#[derive(Resource, Clone, Copy)]
struct LabTerrainExtent(Vec2);

#[cfg_attr(not(feature = "e2e"), allow(dead_code))]
#[derive(Resource)]
pub(crate) struct LabSculptState {
    pub stroke_count: u32,
    pub brush_uv: Vec2,
    heights: Arc<Mutex<Vec<f32>>>,
    original_heights: Vec<f32>,
    terrain_entity: Entity,
}

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

struct LabSceneRuntime {
    label: &'static str,
    terrain: Entity,
    terrain_extent: Vec2,
    pane: common::TerrainExamplePane,
    focus_position: Vec3,
    camera_target: Vec3,
    camera_eye: Vec3,
    move_speed: f32,
    sun_rotation: Quat,
    ambient_color: Color,
    ambient_brightness: f32,
}

fn main() {
    let mut app = App::new();
    app.init_resource::<LabSceneKind>();
    app.insert_resource(ClearColor(Color::srgb(0.78, 0.86, 0.94)));
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Terrain Lab".into(),
            resolution: (1520, 920).into(),
            ..default()
        }),
        ..default()
    }));
    app.add_plugins((
        OrbitCameraPlugin::default(),
        RemotePlugin::default(),
        TerrainPlugin::default(),
    ));
    common::install_terrain_example_debug_ui(&mut app);
    #[cfg(all(feature = "dev", not(target_arch = "wasm32")))]
    app.add_plugins(bevy_brp_extras::BrpExtrasPlugin::with_http_plugin(
        RemoteHttpPlugin::default(),
    ));
    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::E2EPlugin);
    app.add_systems(Startup, setup);
    app.add_systems(
        Update,
        (
            move_focus,
            handle_debug_keys,
            sync_orbit_focus,
            sync_focus_marker.after(saddle_world_terrain::TerrainSystems::UpdateMaterials),
            sync_sculpt_brush_marker
                .after(saddle_world_terrain::TerrainSystems::UpdateMaterials)
                .run_if(resource_exists::<LabSculptState>),
            update_overlay.after(saddle_world_terrain::TerrainSystems::UpdateMaterials),
        ),
    );
    app.run();
}

fn setup(
    mut commands: Commands,
    scene: Res<LabSceneKind>,
    mut debug: ResMut<TerrainDebugConfig>,
    mut pane: ResMut<common::TerrainExamplePane>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let runtime = spawn_scene(
        *scene,
        &mut commands,
        &mut debug,
        &mut meshes,
        &mut materials,
    );
    *pane = runtime.pane.clone();

    commands.insert_resource(LabTerrainExtent(runtime.terrain_extent));
    commands.insert_resource(LabFocusControl {
        auto_roam: false,
        move_speed: runtime.move_speed,
    });
    commands.insert_resource(LabSceneInfo {
        name: runtime.label,
    });

    commands.spawn((
        Name::new("Lab Focus"),
        LabFocus,
        TerrainFocus {
            terrain: Some(runtime.terrain),
            ..default()
        },
        TerrainProbe {
            terrain: Some(runtime.terrain),
            ..default()
        },
        TerrainProbeSample::default(),
        Transform::from_translation(runtime.focus_position),
        GlobalTransform::default(),
    ));

    commands.spawn((
        Name::new("Lab Camera"),
        LabCamera,
        OrbitCamera::looking_at(runtime.camera_target, runtime.camera_eye),
        OrbitCameraInputTarget,
        Camera3d::default(),
    ));

    commands.spawn((
        Name::new("Focus Marker"),
        LabFocusMarker,
        Mesh3d(meshes.add(Sphere::new(3.6).mesh().uv(20, 14))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.96, 0.60, 0.28),
            unlit: true,
            ..default()
        })),
        Transform::from_translation(runtime.focus_position + Vec3::Y * 6.0),
    ));

    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(runtime.sun_rotation),
    ));

    commands.insert_resource(GlobalAmbientLight {
        color: runtime.ambient_color,
        brightness: runtime.ambient_brightness,
        ..default()
    });

    commands.spawn((
        Name::new("Overlay"),
        LabOverlay,
        Node {
            position_type: PositionType::Absolute,
            left: px(16.0),
            top: px(16.0),
            width: px(620.0),
            padding: UiRect::all(px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.07, 0.10, 0.80)),
        Text::default(),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}

fn spawn_scene(
    scene: LabSceneKind,
    commands: &mut Commands,
    debug: &mut TerrainDebugConfig,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> LabSceneRuntime {
    commands.insert_resource(TerrainFocusPoints::default());
    *debug = TerrainDebugConfig::default();

    match scene {
        LabSceneKind::Lab => {
            let mut config = common::default_config();
            config.size = Vec2::new(960.0, 960.0);
            config.chunk_size = Vec2::new(48.0, 48.0);
            config.streaming.visual_radius = 260.0;
            config.streaming.collider_radius = 110.0;
            config.collider.enabled = true;
            config.collider.resolution_divisor = 4;
            let size = config.size;
            let pane = common::terrain_example_pane(&config, debug);

            let terrain = commands
                .spawn(TerrainBundle::new(
                common::generated_dataset(UVec2::new(257, 257)),
                config,
            ))
                .id();
            spawn_lab_landmarks(commands, meshes, materials);

            LabSceneRuntime {
                label: "Terrain Lab",
                terrain,
                terrain_extent: size,
                pane,
                focus_position: Vec3::new(440.0, 0.0, 440.0),
                camera_target: Vec3::new(440.0, 16.0, 440.0),
                camera_eye: Vec3::new(620.0, 170.0, 620.0),
                move_speed: 120.0,
                sun_rotation: Quat::from_euler(EulerRot::XYZ, -0.78, 0.52, 0.0),
                ambient_color: Color::WHITE,
                ambient_brightness: 24.0,
            }
        }
        LabSceneKind::Basic => {
            let config = common::default_config();
            let size = config.size;
            debug.show_chunk_bounds = true;
            debug.show_focus_rings = true;
            let pane = common::terrain_example_pane(&config, debug);
            let terrain = commands
                .spawn(TerrainBundle::new(
                common::generated_dataset(UVec2::new(257, 257)),
                config,
            ))
                .id();

            LabSceneRuntime {
                label: "Terrain Basic Example",
                terrain,
                terrain_extent: size,
                pane,
                focus_position: Vec3::new(320.0, 0.0, 320.0),
                camera_target: Vec3::new(320.0, 20.0, 320.0),
                camera_eye: Vec3::new(180.0, 140.0, 220.0),
                move_speed: 120.0,
                sun_rotation: Quat::from_euler(EulerRot::XYZ, -0.7, 0.45, 0.0),
                ambient_color: Color::WHITE,
                ambient_brightness: 200.0,
            }
        }
        LabSceneKind::ClipmapDebug => {
            let config = common::default_config();
            let size = config.size;
            debug.show_chunk_bounds = true;
            debug.show_focus_rings = true;
            debug.color_mode = TerrainDebugColorMode::ByLod;
            let pane = common::terrain_example_pane(&config, debug);
            let terrain = commands
                .spawn(TerrainBundle::new(
                common::generated_dataset(UVec2::new(257, 257)),
                config,
            ))
                .id();

            LabSceneRuntime {
                label: "Terrain Clipmap Debug Example",
                terrain,
                terrain_extent: size,
                pane,
                focus_position: Vec3::new(320.0, 0.0, 320.0),
                camera_target: Vec3::new(320.0, 20.0, 320.0),
                camera_eye: Vec3::new(180.0, 140.0, 220.0),
                move_speed: 120.0,
                sun_rotation: Quat::from_euler(EulerRot::XYZ, -0.7, 0.45, 0.0),
                ambient_color: Color::WHITE,
                ambient_brightness: 200.0,
            }
        }
        LabSceneKind::SplatLayers => {
            let mut config = common::default_config();
            config.streaming.visual_radius = 220.0;
            let size = config.size;
            debug.show_chunk_bounds = true;
            debug.show_focus_rings = true;
            debug.color_mode = TerrainDebugColorMode::ByLayerDominance;
            let pane = common::terrain_example_pane(&config, debug);
            let terrain = commands
                .spawn(TerrainBundle::new(
                common::generated_dataset(UVec2::new(257, 257)),
                config,
            ))
                .id();

            LabSceneRuntime {
                label: "Terrain Splat Layers Example",
                terrain,
                terrain_extent: size,
                pane,
                focus_position: Vec3::new(320.0, 0.0, 320.0),
                camera_target: Vec3::new(320.0, 20.0, 320.0),
                camera_eye: Vec3::new(180.0, 140.0, 220.0),
                move_speed: 120.0,
                sun_rotation: Quat::from_euler(EulerRot::XYZ, -0.7, 0.45, 0.0),
                ambient_color: Color::WHITE,
                ambient_brightness: 200.0,
            }
        }
        LabSceneKind::AsyncStreaming => {
            let mut config = common::default_config();
            config.size = Vec2::new(1024.0, 1024.0);
            config.streaming.max_builds_per_frame = 2;
            config.streaming.visual_radius = 240.0;
            config.lod.hysteresis = 16.0;
            let size = config.size;
            debug.show_chunk_bounds = true;
            debug.show_focus_rings = true;
            let pane = common::terrain_example_pane(&config, debug);
            let terrain = commands
                .spawn(TerrainBundle::new(
                    common::generated_dataset(UVec2::new(257, 257)),
                    config,
                ))
                .id();
            commands.insert_resource(TerrainFocusPoints(vec![TerrainFocusPoint {
                terrain: Some(terrain),
                position: Vec3::new(760.0, 0.0, 260.0),
                visual_radius_bias: -40.0,
                collider_radius_bias: -70.0,
            }]));

            LabSceneRuntime {
                label: "Terrain Async Streaming Example",
                terrain,
                terrain_extent: size,
                pane,
                focus_position: Vec3::new(320.0, 0.0, 320.0),
                camera_target: Vec3::new(320.0, 20.0, 320.0),
                camera_eye: Vec3::new(180.0, 140.0, 220.0),
                move_speed: 120.0,
                sun_rotation: Quat::from_euler(EulerRot::XYZ, -0.7, 0.45, 0.0),
                ambient_color: Color::WHITE,
                ambient_brightness: 200.0,
            }
        }
        LabSceneKind::PhysicsColliders => {
            let mut config = common::default_config();
            config.collider.enabled = true;
            config.collider.resolution_divisor = 4;
            config.streaming.collider_radius = 70.0;
            let size = config.size;
            debug.show_chunk_bounds = true;
            debug.show_focus_rings = true;
            debug.show_collider_bounds = true;
            let pane = common::terrain_example_pane(&config, debug);
            let terrain = commands
                .spawn(TerrainBundle::new(
                common::generated_dataset(UVec2::new(257, 257)),
                config,
            ))
                .id();

            LabSceneRuntime {
                label: "Terrain Physics Colliders Example",
                terrain,
                terrain_extent: size,
                pane,
                focus_position: Vec3::new(320.0, 0.0, 320.0),
                camera_target: Vec3::new(320.0, 20.0, 320.0),
                camera_eye: Vec3::new(180.0, 140.0, 220.0),
                move_speed: 120.0,
                sun_rotation: Quat::from_euler(EulerRot::XYZ, -0.7, 0.45, 0.0),
                ambient_color: Color::WHITE,
                ambient_brightness: 200.0,
            }
        }
        LabSceneKind::Island => {
            let config = common::island_config();
            let size = config.size;
            let pane = common::terrain_example_pane(&config, debug);
            let terrain = commands
                .spawn(TerrainBundle::new(common::island_dataset(), config))
                .id();
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
                Transform::from_xyz(size.x * 0.5, ISLAND_SEA_LEVEL, size.y * 0.5),
            ));

            LabSceneRuntime {
                label: "Terrain Island Example",
                terrain,
                terrain_extent: size,
                pane,
                focus_position: Vec3::new(size.x * 0.5, 0.0, size.y * 0.5),
                camera_target: Vec3::new(size.x * 0.5, 20.0, size.y * 0.5),
                camera_eye: Vec3::new(size.x * 0.5 + 180.0, 120.0, size.y * 0.5 + 200.0),
                move_speed: 110.0,
                sun_rotation: Quat::from_euler(EulerRot::XYZ, -0.65, 0.5, 0.0),
                ambient_color: Color::WHITE,
                ambient_brightness: 180.0,
            }
        }
        LabSceneKind::MountainRange => {
            let config = common::mountain_config();
            let size = config.size;
            let pane = common::terrain_example_pane(&config, debug);
            let terrain = commands
                .spawn(TerrainBundle::new(common::mountain_dataset(), config))
                .id();

            LabSceneRuntime {
                label: "Terrain Mountain Range Example",
                terrain,
                terrain_extent: size,
                pane,
                focus_position: Vec3::new(size.x * 0.5, 0.0, size.y * 0.5),
                camera_target: Vec3::new(size.x * 0.5, 60.0, size.y * 0.5),
                camera_eye: Vec3::new(size.x * 0.5 - 220.0, 180.0, size.y * 0.5 + 260.0),
                move_speed: 120.0,
                sun_rotation: Quat::from_euler(EulerRot::XYZ, -0.45, 0.65, 0.0),
                ambient_color: Color::srgb(0.85, 0.88, 0.96),
                ambient_brightness: 140.0,
            }
        }
        LabSceneKind::TerrainSculpting => {
            let heights = generate_base_heights();
            let original_heights = heights.clone();
            let shared_heights = Arc::new(Mutex::new(heights));
            let source = MutableHeightSource {
                dimensions: SCULPT_DIMENSIONS,
                heights: shared_heights.clone(),
            };
            let config = sculpt_config();
            let pane = common::terrain_example_pane(&config, debug);
            let terrain = commands.spawn(TerrainBundle::new(source, config)).id();
            commands.insert_resource(LabSculptState {
                stroke_count: 0,
                brush_uv: Vec2::new(0.5, 0.5),
                heights: shared_heights,
                original_heights,
                terrain_entity: terrain,
            });
            commands.spawn((
                Name::new("Brush Marker"),
                LabSculptBrushMarker,
                Mesh3d(
                    meshes.add(
                        Sphere::new(SCULPT_BRUSH_RADIUS * SCULPT_TERRAIN_SIZE.x * 0.5)
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
                Transform::from_xyz(SCULPT_TERRAIN_CENTER.x, 30.0, SCULPT_TERRAIN_CENTER.y),
            ));

            LabSceneRuntime {
                label: "Terrain Sculpting Example",
                terrain,
                terrain_extent: SCULPT_TERRAIN_SIZE,
                pane,
                focus_position: Vec3::new(SCULPT_TERRAIN_CENTER.x, 0.0, SCULPT_TERRAIN_CENTER.y),
                camera_target: Vec3::new(SCULPT_TERRAIN_CENTER.x, 20.0, SCULPT_TERRAIN_CENTER.y),
                camera_eye: Vec3::new(
                    SCULPT_TERRAIN_CENTER.x + 100.0,
                    90.0,
                    SCULPT_TERRAIN_CENTER.y + 120.0,
                ),
                move_speed: 90.0,
                sun_rotation: Quat::from_euler(EulerRot::XYZ, -0.7, 0.45, 0.0),
                ambient_color: Color::WHITE,
                ambient_brightness: 200.0,
            }
        }
    }
}

fn spawn_lab_landmarks(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    for (index, translation, scale, color) in [
        (
            1_u32,
            Vec3::new(260.0, 18.0, 320.0),
            Vec3::new(12.0, 32.0, 12.0),
            Color::srgb(0.92, 0.68, 0.26),
        ),
        (
            2_u32,
            Vec3::new(610.0, 20.0, 540.0),
            Vec3::new(18.0, 40.0, 18.0),
            Color::srgb(0.24, 0.44, 0.72),
        ),
        (
            3_u32,
            Vec3::new(760.0, 14.0, 260.0),
            Vec3::new(26.0, 28.0, 26.0),
            Color::srgb(0.68, 0.34, 0.28),
        ),
    ] {
        commands.spawn((
            Name::new(format!("Landmark {index}")),
            Mesh3d(meshes.add(Cuboid::new(scale.x, scale.y, scale.z))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.84,
                ..default()
            })),
            Transform::from_translation(translation),
        ));
    }
}

fn move_focus(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    bounds: Res<LabTerrainExtent>,
    mut control: ResMut<LabFocusControl>,
    mut focus: Query<&mut Transform, With<LabFocus>>,
) {
    if input.just_pressed(KeyCode::Digit5) {
        control.auto_roam = !control.auto_roam;
    }

    let Ok(mut focus) = focus.single_mut() else {
        return;
    };

    if control.auto_roam {
        let t = time.elapsed_secs() * 0.16;
        let center = bounds.0 * 0.5;
        let roam_x = (bounds.0.x * 0.27).min(center.x.max(32.0));
        let roam_z = (bounds.0.y * 0.23).min(center.y.max(32.0));
        focus.translation.x = center.x + roam_x * t.cos();
        focus.translation.z = center.y + roam_z * (t * 0.72).sin();
    } else {
        let mut intent = Vec2::ZERO;
        if input.pressed(KeyCode::KeyA) || input.pressed(KeyCode::ArrowLeft) {
            intent.x -= 1.0;
        }
        if input.pressed(KeyCode::KeyD) || input.pressed(KeyCode::ArrowRight) {
            intent.x += 1.0;
        }
        if input.pressed(KeyCode::KeyW) || input.pressed(KeyCode::ArrowUp) {
            intent.y -= 1.0;
        }
        if input.pressed(KeyCode::KeyS) || input.pressed(KeyCode::ArrowDown) {
            intent.y += 1.0;
        }

        if intent != Vec2::ZERO {
            let sprint = if input.pressed(KeyCode::ShiftLeft) || input.pressed(KeyCode::ShiftRight)
            {
                1.8
            } else {
                1.0
            };
            let delta = intent.normalize() * control.move_speed * sprint * time.delta_secs();
            focus.translation.x = (focus.translation.x + delta.x).clamp(0.0, bounds.0.x);
            focus.translation.z = (focus.translation.z + delta.y).clamp(0.0, bounds.0.y);
        }
    }

    focus.translation.y = 0.0;
}

fn sync_orbit_focus(
    focus: Query<&Transform, With<LabFocus>>,
    mut cameras: Query<&mut OrbitCamera, With<LabCamera>>,
) {
    let Ok(focus) = focus.single() else {
        return;
    };
    for mut orbit in &mut cameras {
        orbit.focus = focus.translation;
    }
}

fn sync_focus_marker(
    focus: Query<(&Transform, Option<&TerrainProbeSample>), With<LabFocus>>,
    mut markers: Query<&mut Transform, (With<LabFocusMarker>, Without<LabFocus>)>,
) {
    let Ok((focus, sample)) = focus.single() else {
        return;
    };
    let Ok(mut marker) = markers.single_mut() else {
        return;
    };

    marker.translation = sample
        .map(|sample| sample.world_position + Vec3::Y * 6.0)
        .unwrap_or(focus.translation + Vec3::Y * 6.0);
}

fn sync_sculpt_brush_marker(
    state: Res<LabSculptState>,
    mut markers: Query<&mut Transform, With<LabSculptBrushMarker>>,
) {
    let Ok(mut marker) = markers.single_mut() else {
        return;
    };
    marker.translation.x = state.brush_uv.x * SCULPT_TERRAIN_SIZE.x;
    marker.translation.z = state.brush_uv.y * SCULPT_TERRAIN_SIZE.y;
}

fn handle_debug_keys(input: Res<ButtonInput<KeyCode>>, mut debug: ResMut<TerrainDebugConfig>) {
    if input.just_pressed(KeyCode::Digit1) {
        debug.color_mode = match debug.color_mode {
            TerrainDebugColorMode::Natural => TerrainDebugColorMode::ByLod,
            TerrainDebugColorMode::ByLod => TerrainDebugColorMode::ByChunkState,
            TerrainDebugColorMode::ByChunkState => TerrainDebugColorMode::ByLayerDominance,
            TerrainDebugColorMode::ByLayerDominance => TerrainDebugColorMode::BySlopeBand,
            TerrainDebugColorMode::BySlopeBand => TerrainDebugColorMode::Natural,
        };
    }
    if input.just_pressed(KeyCode::Digit2) {
        debug.show_chunk_bounds = !debug.show_chunk_bounds;
    }
    if input.just_pressed(KeyCode::Digit3) {
        debug.show_collider_bounds = !debug.show_collider_bounds;
    }
    if input.just_pressed(KeyCode::Digit4) {
        debug.show_focus_rings = !debug.show_focus_rings;
    }
}

fn update_overlay(
    scene_info: Res<LabSceneInfo>,
    debug: Res<TerrainDebugConfig>,
    control: Res<LabFocusControl>,
    diagnostics: Res<TerrainDiagnostics>,
    sculpt: Option<Res<LabSculptState>>,
    focus: Query<(&Transform, Option<&TerrainProbeSample>), With<LabFocus>>,
    roots: Query<&TerrainRootStats>,
    mut overlay: Query<&mut Text, With<LabOverlay>>,
) {
    let Ok(mut overlay) = overlay.single_mut() else {
        return;
    };
    let Ok((focus_transform, probe)) = focus.single() else {
        return;
    };
    let stats = roots.iter().next().cloned().unwrap_or_default();
    let probe_height = probe.map(|sample| sample.height).unwrap_or_default();
    let probe_slope = probe.map(|sample| sample.slope_degrees).unwrap_or_default();
    let dominant_layer = probe.and_then(|sample| sample.dominant_layer);
    let probe_position = probe
        .map(|sample| sample.world_position)
        .unwrap_or(focus_transform.translation);
    let sculpt_line = sculpt.map_or(String::new(), |sculpt| {
        format!(
            "\nsculpt strokes={} brush_uv=({:.2}, {:.2})",
            sculpt.stroke_count, sculpt.brush_uv.x, sculpt.brush_uv.y
        )
    });

    overlay.0 = format!(
        "{}\nLMB orbit  MMB pan  wheel zoom\n1 color mode  2 chunk bounds  3 collider bounds  4 focus rings  5 auto roam\nWASD / arrows move focus  Shift sprint\nmode={:?} auto_roam={} focus_xz=({:.1}, {:.1})\nchunks total={} ready={} pending={} collider={}\nroot visual={} collider={} max_lod={} cache_hits={}\nprobe surface_y={:.2} slope={:.2} dominant={:?}\nprobe position={:.1}, {:.1}, {:.1}{}",
        scene_info.name,
        debug.color_mode,
        control.auto_roam,
        focus_transform.translation.x,
        focus_transform.translation.z,
        diagnostics.total_chunks,
        diagnostics.ready_chunks,
        diagnostics.pending_chunks,
        diagnostics.collider_chunks,
        stats.active_visual_chunks,
        stats.active_collider_chunks,
        stats.max_visible_lod,
        stats.cache_hits,
        probe_height,
        probe_slope,
        dominant_layer,
        probe_position.x,
        probe_position.y,
        probe_position.z,
        sculpt_line,
    );
}

fn generate_base_heights() -> Vec<f32> {
    let count = SCULPT_DIMENSIONS.x as usize * SCULPT_DIMENSIONS.y as usize;
    let mut heights = Vec::with_capacity(count);
    for y in 0..SCULPT_DIMENSIONS.y {
        for x in 0..SCULPT_DIMENSIONS.x {
            let uv = Vec2::new(
                x as f32 / (SCULPT_DIMENSIONS.x - 1) as f32,
                y as f32 / (SCULPT_DIMENSIONS.y - 1) as f32,
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

fn sculpt_config() -> TerrainConfig {
    TerrainConfig {
        size: SCULPT_TERRAIN_SIZE,
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

#[cfg_attr(not(feature = "e2e"), allow(dead_code))]
pub(crate) fn apply_sculpt_brush(world: &mut World, brush_uv: Vec2, direction: f32) -> bool {
    let Some(mut state) = world.get_resource_mut::<LabSculptState>() else {
        return false;
    };

    state.brush_uv = brush_uv;
    {
        let mut heights = state.heights.lock().unwrap();
        for y in 0..SCULPT_DIMENSIONS.y {
            for x in 0..SCULPT_DIMENSIONS.x {
                let uv = Vec2::new(
                    x as f32 / (SCULPT_DIMENSIONS.x - 1) as f32,
                    y as f32 / (SCULPT_DIMENSIONS.y - 1) as f32,
                );
                let dist = (uv - brush_uv).length();
                if dist < SCULPT_BRUSH_RADIUS {
                    let falloff = 1.0 - (dist / SCULPT_BRUSH_RADIUS);
                    let influence = falloff * falloff * SCULPT_BRUSH_STRENGTH * direction;
                    let idx = (y * SCULPT_DIMENSIONS.x + x) as usize;
                    heights[idx] = (heights[idx] + influence).clamp(0.0, 1.0);
                }
            }
        }
    }

    state.stroke_count += 1;
    let heights = state.heights.clone();
    let terrain_entity = state.terrain_entity;
    drop(state);

    let mut terrains = world.query::<&mut TerrainSourceHandle>();
    let Ok(mut handle) = terrains.get_mut(world, terrain_entity) else {
        return false;
    };
    *handle = TerrainSourceHandle::new(MutableHeightSource {
        dimensions: SCULPT_DIMENSIONS,
        heights,
    });
    true
}

#[cfg_attr(not(feature = "e2e"), allow(dead_code))]
pub(crate) fn reset_sculpt_heights(world: &mut World) -> bool {
    let Some(mut state) = world.get_resource_mut::<LabSculptState>() else {
        return false;
    };

    {
        let mut heights = state.heights.lock().unwrap();
        heights.copy_from_slice(&state.original_heights);
    }
    state.stroke_count = 0;
    let heights = state.heights.clone();
    let terrain_entity = state.terrain_entity;
    drop(state);

    let mut terrains = world.query::<&mut TerrainSourceHandle>();
    let Ok(mut handle) = terrains.get_mut(world, terrain_entity) else {
        return false;
    };
    *handle = TerrainSourceHandle::new(MutableHeightSource {
        dimensions: SCULPT_DIMENSIONS,
        heights,
    });
    true
}
