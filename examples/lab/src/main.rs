#[cfg(feature = "e2e")]
mod e2e;
#[cfg(feature = "e2e")]
mod scenarios;

use saddle_world_terrain_example_common as common;

use bevy::{
    prelude::*,
    remote::{RemotePlugin, http::RemoteHttpPlugin},
};
use saddle_camera_orbit_camera::{OrbitCamera, OrbitCameraInputTarget, OrbitCameraPlugin};
use saddle_world_terrain::{
    TerrainBundle, TerrainDebugColorMode, TerrainDebugConfig, TerrainDiagnostics, TerrainFocus,
    TerrainPlugin, TerrainProbe, TerrainProbeSample, TerrainRootStats,
};

#[derive(Component)]
pub(crate) struct LabFocus;

#[derive(Component)]
pub(crate) struct LabCamera;

#[derive(Component)]
struct LabOverlay;

#[derive(Component)]
struct LabFocusMarker;

#[derive(Resource)]
struct LabFocusControl {
    auto_roam: bool,
    move_speed: f32,
}

#[derive(Resource, Clone, Copy)]
struct LabTerrainExtent(Vec2);

fn main() {
    let mut app = App::new();
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
            update_overlay.after(saddle_world_terrain::TerrainSystems::UpdateMaterials),
        ),
    );
    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut config = common::default_config();
    config.size = Vec2::new(960.0, 960.0);
    config.chunk_size = Vec2::new(48.0, 48.0);
    config.streaming.visual_radius = 260.0;
    config.streaming.collider_radius = 110.0;
    config.collider.enabled = true;
    config.collider.resolution_divisor = 4;
    let terrain_extent = config.size;
    commands.insert_resource(LabTerrainExtent(terrain_extent));
    commands.insert_resource(LabFocusControl {
        auto_roam: false,
        move_speed: 120.0,
    });

    let terrain = commands
        .spawn(TerrainBundle::new(
            common::generated_dataset(UVec2::new(257, 257)),
            config,
        ))
        .id();

    commands.spawn((
        Name::new("Lab Focus"),
        LabFocus,
        TerrainFocus {
            terrain: Some(terrain),
            ..default()
        },
        TerrainProbe {
            terrain: Some(terrain),
            ..default()
        },
        TerrainProbeSample::default(),
        Transform::from_xyz(440.0, 0.0, 440.0),
        GlobalTransform::default(),
    ));

    commands.spawn((
        Name::new("Lab Camera"),
        LabCamera,
        OrbitCamera::looking_at(
            Vec3::new(440.0, 16.0, 440.0),
            Vec3::new(620.0, 170.0, 620.0),
        ),
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
        Transform::from_xyz(440.0, 10.0, 440.0),
    ));

    commands.spawn((
        Name::new("Sun"),
        DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.78, 0.52, 0.0)),
    ));

    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 24.0,
        ..default()
    });

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

    commands.spawn((
        Name::new("Overlay"),
        LabOverlay,
        Node {
            position_type: PositionType::Absolute,
            left: px(16.0),
            top: px(16.0),
            width: px(560.0),
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
        focus.translation.x = 440.0 + 260.0 * t.cos();
        focus.translation.z = 440.0 + 220.0 * (t * 0.72).sin();
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
            let sprint = if input.pressed(KeyCode::ShiftLeft) || input.pressed(KeyCode::ShiftRight) {
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
    debug: Res<TerrainDebugConfig>,
    control: Res<LabFocusControl>,
    diagnostics: Res<TerrainDiagnostics>,
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

    overlay.0 = format!(
        "Terrain Lab\nLMB orbit  MMB pan  wheel zoom\n1 color mode  2 chunk bounds  3 collider bounds  4 focus rings  5 auto roam\nWASD / arrows move focus  Shift sprint\nmode={:?} auto_roam={} focus_xz=({:.1}, {:.1})\nchunks total={} ready={} pending={} collider={}\nroot visual={} collider={} max_lod={} cache_hits={}\nprobe surface_y={:.2} slope={:.2} dominant={:?}\nprobe position={:.1}, {:.1}, {:.1}",
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
    );
}
