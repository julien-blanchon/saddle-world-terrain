#[cfg(feature = "e2e")]
mod e2e;
#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;

use bevy::{
    asset::RenderAssetUsages, mesh::Indices, pbr::MeshMaterial3d, prelude::*,
    render::render_resource::PrimitiveTopology,
};
use grass::{GrassConfig, GrassPatch, GrassPlugin, GrassSurface, GrassWindBridge};
use saddle_camera_orbit_camera::{OrbitCamera, OrbitCameraInputTarget, OrbitCameraPlugin};
use saddle_pane::prelude::*;
use saddle_procgen_noise::{Fbm, FractalConfig, NoiseSeed, NoiseSource, Perlin, signed_to_unit};
use saddle_world_foliage::{
    FoliageLayer, FoliageLod, FoliagePlugin, FoliagePrototype, FoliageSurface, FoliageSurfaceStats,
    FoliageViewer,
};
use saddle_world_sky::{SkyCamera, SkyConfig, SkyPlugin, SkyTimeOfDay};
use saddle_world_terrain::{
    TerrainBlendRange, TerrainBundle, TerrainConfig, TerrainDataset, TerrainFocus, TerrainLayer,
    TerrainMaterialProfile, TerrainPlugin, TerrainSource,
};
use saddle_world_wind::{
    WindAffected, WindBacklight, WindBlendMode, WindConfig, WindMaterial, WindPlugin, WindProfile,
    WindResponse, WindSampling, WindZone, WindZoneFalloff, WindZoneShape, build_wind_material,
};

const TERRAIN_DIMENSIONS: UVec2 = UVec2::new(257, 257);
const TERRAIN_SIZE: Vec2 = Vec2::new(640.0, 640.0);
const PATCH_CENTER: Vec2 = Vec2::new(320.0, 312.0);
const PATCH_SIZE: Vec2 = Vec2::new(260.0, 220.0);
const PATCH_RESOLUTION: UVec2 = UVec2::new(96, 82);
const PATCH_ELEVATION_BIAS: f32 = 0.08;

#[derive(Component)]
struct ShowcaseOverlay;

#[derive(Component)]
struct DraftZone {
    center: Vec3,
    span: Vec3,
    speed: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TerrainSnapshot {
    height_scale: f32,
    height_offset: f32,
}

#[derive(Resource, Clone, Debug, Pane)]
#[pane(title = "Open World", position = "top-right")]
struct OpenWorldPane {
    #[pane(tab = "Terrain", slider, min = 42.0, max = 140.0, step = 1.0)]
    terrain_height_scale: f32,
    #[pane(tab = "Terrain", slider, min = -24.0, max = 24.0, step = 0.5)]
    terrain_height_offset: f32,
    #[pane(tab = "Atmosphere", slider, min = 5.0, max = 20.0, step = 0.1)]
    time_of_day_hours: f32,
    #[pane(tab = "Atmosphere", slider, min = 0.0, max = 1.0, step = 0.01)]
    cloud_coverage: f32,
    #[pane(tab = "Wind", slider, min = 0.5, max = 18.0, step = 0.1)]
    wind_speed: f32,
    #[pane(tab = "Wind", slider, min = 0.0, max = 2.0, step = 0.02)]
    wind_intensity: f32,
    #[pane(tab = "Vegetation", slider, min = 12.0, max = 42.0, step = 1.0)]
    grass_density: f32,
    #[pane(tab = "Vegetation", slider, min = 0.01, max = 0.09, step = 0.005)]
    canopy_density: f32,
    #[pane(tab = "Vegetation", slider, min = 0.5, max = 2.5, step = 0.05)]
    grass_sway_scale: f32,
    #[pane(tab = "Runtime", monitor)]
    visible_grass_blades: u32,
    #[pane(tab = "Runtime", monitor)]
    foliage_instances: u32,
}

impl Default for OpenWorldPane {
    fn default() -> Self {
        Self {
            terrain_height_scale: 96.0,
            terrain_height_offset: -10.0,
            time_of_day_hours: 8.7,
            cloud_coverage: 0.34,
            wind_speed: 6.4,
            wind_intensity: 0.92,
            grass_density: 24.0,
            canopy_density: 0.045,
            grass_sway_scale: 1.35,
            visible_grass_blades: 0,
            foliage_instances: 0,
        }
    }
}

#[derive(Resource)]
struct OpenWorldScene {
    dataset: TerrainDataset,
    terrain_entity: Entity,
    terrain_cover_mesh: Handle<Mesh>,
    foliage_surface_entity: Entity,
    foliage_layer_entity: Entity,
    grass_patch_entity: Entity,
    terrain_snapshot: TerrainSnapshot,
}

fn main() {
    let mut sky_config = SkyConfig::default();
    sky_config.time_of_day.paused = true;
    sky_config
        .time_of_day
        .set_hours(OpenWorldPane::default().time_of_day_hours);

    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.70, 0.83, 0.95)))
        .insert_resource(OpenWorldPane::default())
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 220.0,
            affects_lightmapped_meshes: true,
        })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "terrain open_world_showcase".into(),
                resolution: (1600, 920).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            TerrainPlugin::default(),
            OrbitCameraPlugin::default(),
            SkyPlugin::default().with_config(sky_config),
            WindPlugin::default().with_config(WindProfile::Breeze.config()),
            FoliagePlugin::default(),
            GrassPlugin::default(),
            bevy_flair::FlairPlugin,
            bevy_input_focus::InputDispatchPlugin,
            bevy_ui_widgets::UiWidgetsPlugins,
            bevy_input_focus::tab_navigation::TabNavigationPlugin,
            PanePlugin,
        ));
    #[cfg(feature = "e2e")]
    app.add_plugins(e2e::OpenWorldShowcaseE2EPlugin);
    app.register_pane::<OpenWorldPane>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                sync_open_world_pane,
                animate_draft_zone,
                update_overlay.after(saddle_world_wind::WindSystems::SampleWind),
                update_monitors.after(saddle_world_wind::WindSystems::SampleWind),
            ),
        );
    app.run();
}

fn setup(
    mut commands: Commands,
    pane: Res<OpenWorldPane>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut wind_materials: ResMut<Assets<WindMaterial>>,
) {
    let dataset = build_dataset();
    let terrain_config = terrain_config(pane.terrain_height_scale, pane.terrain_height_offset);
    let terrain_entity = commands
        .spawn(TerrainBundle::new(dataset.clone(), terrain_config.clone()))
        .id();

    let cover_mesh = meshes.add(build_cover_mesh(&dataset, &terrain_config));
    let terrain_cover_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.26, 0.33, 0.19),
        perceptual_roughness: 0.94,
        ..default()
    });
    let tree_material = wind_materials.add(build_wind_material(
        StandardMaterial {
            base_color: Color::srgb(0.28, 0.52, 0.24),
            perceptual_roughness: 0.84,
            cull_mode: None,
            ..default()
        },
        WindResponse::tree(),
        WindBacklight {
            strength: 0.34,
            thickness: 0.07,
            ..default()
        },
    ));
    let shrub_material = wind_materials.add(build_wind_material(
        StandardMaterial {
            base_color: Color::srgb(0.24, 0.68, 0.30),
            perceptual_roughness: 0.88,
            cull_mode: None,
            ..default()
        },
        WindResponse::shrub(),
        WindBacklight {
            strength: 0.26,
            thickness: 0.05,
            ..default()
        },
    ));

    let tree_mesh = meshes.add(Cone::new(3.4, 9.0).mesh().resolution(10));
    let shrub_mesh = meshes.add(Sphere::new(1.2).mesh().uv(12, 10));
    let rock_mesh = meshes.add(Sphere::new(1.6).mesh().uv(16, 12));

    let foliage_surface_entity = commands
        .spawn((
            Name::new("Open World Foliage Surface"),
            Mesh3d(cover_mesh.clone()),
            MeshMaterial3d(terrain_cover_material),
            Transform::default(),
            FoliageSurface {
                seed: 31,
                chunk_size: Vec2::splat(18.0),
                ..default()
            },
        ))
        .id();

    let foliage_layer_entity = commands
        .spawn((
            Name::new("Windy Canopies"),
            FoliageLayer {
                order: 1,
                density_per_square_unit: pane.canopy_density,
                min_spacing: 3.8,
                occupancy_radius: 1.8,
                max_instances_per_chunk: 96,
                sample_budget_per_chunk: 280,
                lods: vec![
                    FoliageLod {
                        max_distance: 48.0,
                        density_scale: 1.0,
                        prototype_lod: 0,
                        fade_distance: 4.0,
                    },
                    FoliageLod {
                        max_distance: 90.0,
                        density_scale: 0.55,
                        prototype_lod: 0,
                        fade_distance: 6.0,
                    },
                ],
                ..default()
            },
        ))
        .id();
    commands
        .entity(foliage_surface_entity)
        .add_child(foliage_layer_entity);

    let rock_layer = commands
        .spawn((
            Name::new("Boulders"),
            FoliageLayer {
                order: 0,
                density_per_square_unit: 0.012,
                min_spacing: 8.0,
                occupancy_radius: 3.0,
                max_instances_per_chunk: 20,
                sample_budget_per_chunk: 64,
                lods: vec![FoliageLod {
                    max_distance: 100.0,
                    density_scale: 1.0,
                    prototype_lod: 0,
                    fade_distance: 8.0,
                }],
                ..default()
            },
        ))
        .id();
    commands
        .entity(foliage_surface_entity)
        .add_child(rock_layer);

    commands.entity(rock_layer).with_children(|parent| {
        parent.spawn((
            Name::new("Rock Prototype"),
            FoliagePrototype {
                occupancy_radius: 2.8,
                ..default()
            },
            Mesh3d(rock_mesh),
            MeshMaterial3d(materials.add(Color::srgb(0.44, 0.42, 0.38))),
            Transform::from_xyz(0.0, 1.1, 0.0).with_scale(Vec3::new(1.8, 1.0, 1.3)),
        ));
    });

    commands
        .entity(foliage_layer_entity)
        .with_children(|parent| {
            parent.spawn((
                Name::new("Tree Prototype"),
                FoliagePrototype {
                    occupancy_radius: 2.2,
                    weight: 0.55,
                    ..default()
                },
                Mesh3d(tree_mesh.clone()),
                MeshMaterial3d(tree_material.clone()),
                WindAffected::tree(),
                WindSampling {
                    local_offset: Vec3::new(0.0, 3.6, 0.0),
                },
                Transform::from_xyz(0.0, 3.2, 0.0),
            ));
            parent.spawn((
                Name::new("Shrub Prototype"),
                FoliagePrototype {
                    occupancy_radius: 1.2,
                    weight: 1.0,
                    ..default()
                },
                Mesh3d(shrub_mesh),
                MeshMaterial3d(shrub_material),
                WindAffected::shrub(),
                WindSampling {
                    local_offset: Vec3::new(0.0, 0.85, 0.0),
                },
                Transform::from_xyz(0.0, 0.95, 0.0).with_scale(Vec3::new(1.3, 0.8, 1.2)),
            ));
        });

    let grass_patch_entity = commands
        .spawn((
            Name::new("Meadow Grass"),
            GrassPatch {
                seed: 17,
                half_size: PATCH_SIZE * 0.5,
                surface: GrassSurface::Mesh(foliage_surface_entity),
                ..default()
            },
            GrassConfig {
                density_per_square_unit: pane.grass_density,
                cast_shadows: false,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        Name::new("Sky Terrain Camera"),
        OrbitCamera::looking_at(
            Vec3::new(PATCH_CENTER.x, 40.0, PATCH_CENTER.y),
            Vec3::new(PATCH_CENTER.x - 180.0, 140.0, PATCH_CENTER.y + 210.0),
        ),
        OrbitCameraInputTarget,
        SkyCamera::default(),
        FoliageViewer,
        TerrainFocus {
            terrain: Some(terrain_entity),
            ..default()
        },
    ));

    commands.spawn((
        Name::new("Valley Draft"),
        DraftZone {
            center: Vec3::new(PATCH_CENTER.x, 14.0, PATCH_CENTER.y),
            span: Vec3::new(PATCH_SIZE.x * 0.42, 0.0, PATCH_SIZE.y * 0.28),
            speed: 0.22,
        },
        WindZone {
            shape: WindZoneShape::Box {
                half_extents: Vec3::new(26.0, 8.0, 54.0),
            },
            blend_mode: WindBlendMode::DirectionalBias,
            falloff: WindZoneFalloff::SmoothStep,
            direction: Vec3::new(1.0, 0.0, 0.28),
            speed: 8.5,
            intensity: 0.95,
            turbulence_multiplier: 1.2,
            gust_multiplier: 1.1,
            priority: 6,
            ..default()
        },
        Transform::from_xyz(PATCH_CENTER.x - 70.0, 10.0, PATCH_CENTER.y),
        GlobalTransform::default(),
    ));

    spawn_landmarks(
        &mut commands,
        &dataset,
        &terrain_config,
        &mut meshes,
        &mut materials,
    );

    commands.spawn((
        Name::new("Open World Overlay"),
        ShowcaseOverlay,
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
            padding: UiRect::all(px(14.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.07, 0.10, 0.76)),
    ));

    commands.insert_resource(OpenWorldScene {
        dataset,
        terrain_entity,
        terrain_cover_mesh: cover_mesh,
        foliage_surface_entity,
        foliage_layer_entity,
        grass_patch_entity,
        terrain_snapshot: TerrainSnapshot {
            height_scale: pane.terrain_height_scale,
            height_offset: pane.terrain_height_offset,
        },
    });
}

fn build_dataset() -> TerrainDataset {
    let continental = Fbm::new(
        Perlin::new(NoiseSeed(13)),
        FractalConfig {
            octaves: 5,
            base_frequency: 0.65,
            lacunarity: 2.0,
            gain: 0.54,
            ..default()
        },
    );
    let ridge = Fbm::new(
        Perlin::new(NoiseSeed(41)),
        FractalConfig {
            octaves: 6,
            base_frequency: 1.55,
            lacunarity: 2.2,
            gain: 0.48,
            ..default()
        },
    );
    let details = Fbm::new(
        Perlin::new(NoiseSeed(77)),
        FractalConfig {
            octaves: 4,
            base_frequency: 3.2,
            lacunarity: 2.0,
            gain: 0.45,
            ..default()
        },
    );

    let mut heights = Vec::with_capacity((TERRAIN_DIMENSIONS.x * TERRAIN_DIMENSIONS.y) as usize);
    let mut weights = Vec::with_capacity((TERRAIN_DIMENSIONS.x * TERRAIN_DIMENSIONS.y) as usize);

    for y in 0..TERRAIN_DIMENSIONS.y {
        for x in 0..TERRAIN_DIMENSIONS.x {
            let uv = Vec2::new(
                x as f32 / (TERRAIN_DIMENSIONS.x - 1) as f32,
                y as f32 / (TERRAIN_DIMENSIONS.y - 1) as f32,
            );
            let point = uv * Vec2::new(7.0, 6.0);
            let warped = point
                + Vec2::new(
                    ridge.sample(point * 0.42) * 0.35,
                    details.sample(point * 0.34) * 0.25,
                );

            let continental_shape = signed_to_unit(continental.sample(warped * 0.62));
            let ridge_shape = 1.0 - (ridge.sample(warped * 0.95)).abs();
            let detail_shape = signed_to_unit(details.sample(warped * 1.7));
            let basin = (1.0 - ((uv - Vec2::new(0.34, 0.58)).length() * 2.0)).clamp(0.0, 1.0);
            let valley = (1.0 - ((uv.x - 0.48).abs() * 6.0)).clamp(0.0, 1.0)
                * (1.0 - ((uv.y - 0.46).abs() * 4.4)).clamp(0.0, 1.0);

            let height = (continental_shape * 0.52
                + ridge_shape * 0.28
                + detail_shape * 0.14
                + basin * 0.10
                - valley * 0.18)
                .clamp(0.0, 1.0);
            heights.push(height);

            let meadow =
                ((1.0 - (height - 0.46).abs() * 3.6) * (0.3 + basin * 0.7)).clamp(0.0, 1.0);
            let river = valley.clamp(0.0, 1.0);
            let trail = (1.0 - ((uv.y - 0.62).abs() * 10.0)).clamp(0.0, 1.0)
                * (1.0 - ((uv.x - 0.52).abs() * 2.2)).clamp(0.0, 1.0);
            weights.push([river, meadow, trail, 0.0]);
        }
    }

    TerrainDataset::from_heights(TERRAIN_DIMENSIONS, heights)
        .expect("noise heights should match dimensions")
        .with_weight_map(
            saddle_world_terrain::TerrainWeightMap::from_rgba(TERRAIN_DIMENSIONS, weights)
                .expect("noise weights should match dimensions"),
        )
}

fn terrain_config(height_scale: f32, height_offset: f32) -> TerrainConfig {
    TerrainConfig {
        size: TERRAIN_SIZE,
        chunk_size: Vec2::new(40.0, 40.0),
        vertex_resolution: 64,
        height_scale,
        height_offset,
        skirt_depth: 10.0,
        streaming: saddle_world_terrain::TerrainStreamingConfig {
            visual_radius: 210.0,
            collider_radius: 120.0,
            max_builds_per_frame: 8,
        },
        material: TerrainMaterialProfile {
            layers: vec![
                TerrainLayer::tinted("river", Color::srgb(0.14, 0.34, 0.58)).with_weight_channel(0),
                TerrainLayer::tinted("meadow", Color::srgb(0.28, 0.56, 0.24))
                    .with_weight_channel(1)
                    .with_height_range(TerrainBlendRange::new(0.10, 0.60)),
                TerrainLayer::tinted("trail", Color::srgb(0.46, 0.35, 0.24))
                    .with_weight_channel(2)
                    .with_height_range(TerrainBlendRange::new(0.15, 0.78)),
                TerrainLayer::tinted("rock", Color::srgb(0.54, 0.54, 0.56)).with_slope_range(
                    TerrainBlendRange {
                        start: 22.0,
                        end: 72.0,
                        falloff: 0.16,
                    },
                ),
                TerrainLayer::tinted("snow", Color::srgb(0.93, 0.95, 0.98)).with_height_range(
                    TerrainBlendRange {
                        start: 0.74,
                        end: 1.0,
                        falloff: 0.10,
                    },
                ),
            ],
            ..default()
        },
        ..default()
    }
}

fn build_cover_mesh(dataset: &TerrainDataset, config: &TerrainConfig) -> Mesh {
    let min = PATCH_CENTER - PATCH_SIZE * 0.5;
    let columns = PATCH_RESOLUTION.x as usize;
    let rows = PATCH_RESOLUTION.y as usize;
    let step = Vec2::new(
        PATCH_SIZE.x / (PATCH_RESOLUTION.x - 1) as f32,
        PATCH_SIZE.y / (PATCH_RESOLUTION.y - 1) as f32,
    );

    let mut positions = Vec::with_capacity(columns * rows);
    let mut normals = Vec::with_capacity(columns * rows);
    let mut uvs = Vec::with_capacity(columns * rows);
    let mut indices = Vec::with_capacity((columns - 1) * (rows - 1) * 6);

    for row in 0..rows {
        for column in 0..columns {
            let uv = Vec2::new(
                column as f32 / (columns - 1) as f32,
                row as f32 / (rows - 1) as f32,
            );
            let world = Vec2::new(min.x + PATCH_SIZE.x * uv.x, min.y + PATCH_SIZE.y * uv.y);
            let terrain_uv = Vec2::new(world.x / config.size.x, world.y / config.size.y);
            let height =
                config.height_offset + dataset.sample_height(terrain_uv) * config.height_scale;

            positions.push([world.x, height + PATCH_ELEVATION_BIAS, world.y]);
            normals.push(sample_patch_normal(dataset, config, world, step).to_array());
            uvs.push([uv.x, uv.y]);
        }
    }

    for row in 0..rows - 1 {
        for column in 0..columns - 1 {
            let base = (row * columns + column) as u32;
            indices.extend_from_slice(&[
                base,
                base + columns as u32,
                base + 1,
                base + 1,
                base + columns as u32,
                base + columns as u32 + 1,
            ]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn sample_patch_normal(
    dataset: &TerrainDataset,
    config: &TerrainConfig,
    world: Vec2,
    step: Vec2,
) -> Vec3 {
    let left = sample_world_height(dataset, config, world - Vec2::new(step.x, 0.0));
    let right = sample_world_height(dataset, config, world + Vec2::new(step.x, 0.0));
    let down = sample_world_height(dataset, config, world - Vec2::new(0.0, step.y));
    let up = sample_world_height(dataset, config, world + Vec2::new(0.0, step.y));
    let tangent_x = Vec3::new(step.x * 2.0, right - left, 0.0);
    let tangent_z = Vec3::new(0.0, up - down, step.y * 2.0);
    tangent_z.cross(tangent_x).normalize_or_zero()
}

fn sample_world_height(dataset: &TerrainDataset, config: &TerrainConfig, world: Vec2) -> f32 {
    let clamped = Vec2::new(
        world.x.clamp(0.0, config.size.x),
        world.y.clamp(0.0, config.size.y),
    );
    let uv = Vec2::new(clamped.x / config.size.x, clamped.y / config.size.y);
    config.height_offset + dataset.sample_height(uv) * config.height_scale
}

fn spawn_landmarks(
    commands: &mut Commands,
    dataset: &TerrainDataset,
    config: &TerrainConfig,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let arch_mesh = meshes.add(Cuboid::new(12.0, 18.0, 4.0));
    let pillar_mesh = meshes.add(Cuboid::new(4.0, 20.0, 4.0));
    let stone_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.52, 0.50, 0.46),
        perceptual_roughness: 0.88,
        ..default()
    });

    for (name, world, scale) in [
        (
            "North Arch",
            Vec2::new(268.0, 224.0),
            Vec3::new(1.0, 1.0, 1.0),
        ),
        (
            "South Arch",
            Vec2::new(380.0, 404.0),
            Vec3::new(0.8, 0.9, 1.2),
        ),
    ] {
        let height = sample_world_height(dataset, config, world);
        commands.spawn((
            Name::new(name),
            Mesh3d(arch_mesh.clone()),
            MeshMaterial3d(stone_material.clone()),
            Transform::from_xyz(world.x, height + 9.0, world.y).with_scale(scale),
        ));
    }

    for (name, world) in [
        ("Signal Pillar A", Vec2::new(300.0, 276.0)),
        ("Signal Pillar B", Vec2::new(348.0, 292.0)),
        ("Signal Pillar C", Vec2::new(404.0, 330.0)),
    ] {
        let height = sample_world_height(dataset, config, world);
        commands.spawn((
            Name::new(name),
            Mesh3d(pillar_mesh.clone()),
            MeshMaterial3d(stone_material.clone()),
            Transform::from_xyz(world.x, height + 10.0, world.y),
        ));
    }
}

fn sync_open_world_pane(
    pane: Res<OpenWorldPane>,
    mut scene: ResMut<OpenWorldScene>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut terrain: Query<&mut TerrainConfig>,
    mut grass_query: Query<&mut GrassConfig>,
    mut foliage_layers: Query<&mut FoliageLayer>,
    mut sky_config: ResMut<SkyConfig>,
    mut time_of_day: ResMut<SkyTimeOfDay>,
    mut wind_config: ResMut<WindConfig>,
    mut grass_bridge: ResMut<GrassWindBridge>,
) {
    let snapshot = TerrainSnapshot {
        height_scale: pane.terrain_height_scale,
        height_offset: pane.terrain_height_offset,
    };

    if snapshot != scene.terrain_snapshot {
        scene.terrain_snapshot = snapshot;
        if let Ok(mut terrain_config) = terrain.get_mut(scene.terrain_entity) {
            terrain_config.height_scale = snapshot.height_scale;
            terrain_config.height_offset = snapshot.height_offset;
            let rebuilt = build_cover_mesh(&scene.dataset, &terrain_config);
            if let Some(mesh) = meshes.get_mut(&scene.terrain_cover_mesh) {
                *mesh = rebuilt;
            }
        }
    }

    if let Ok(mut grass) = grass_query.get_mut(scene.grass_patch_entity) {
        if (grass.density_per_square_unit - pane.grass_density).abs() > 0.001 {
            grass.density_per_square_unit = pane.grass_density;
        }
    }
    if let Ok(mut layer) = foliage_layers.get_mut(scene.foliage_layer_entity) {
        if (layer.density_per_square_unit - pane.canopy_density).abs() > 0.000_1 {
            layer.density_per_square_unit = pane.canopy_density;
        }
    }

    if !sky_config.time_of_day.paused {
        sky_config.time_of_day.paused = true;
    }
    if (sky_config.time_of_day.hours - pane.time_of_day_hours).abs() > 0.001 {
        sky_config.time_of_day.set_hours(pane.time_of_day_hours);
    }
    if let Some(primary) = sky_config.cloud_layers.first_mut() {
        if (primary.coverage - pane.cloud_coverage).abs() > 0.001 {
            primary.coverage = pane.cloud_coverage;
        }
        let desired_opacity = 0.55 + pane.cloud_coverage * 0.25;
        if (primary.opacity - desired_opacity).abs() > 0.001 {
            primary.opacity = desired_opacity;
        }
    }
    if !time_of_day.paused {
        time_of_day.paused = true;
    }
    if (time_of_day.hours - pane.time_of_day_hours).abs() > 0.001 {
        time_of_day.set_hours(pane.time_of_day_hours);
    }

    if (wind_config.speed - pane.wind_speed).abs() > 0.001 {
        wind_config.speed = pane.wind_speed;
    }
    if (wind_config.intensity - pane.wind_intensity).abs() > 0.001 {
        wind_config.intensity = pane.wind_intensity;
    }
    if (grass_bridge.sway_strength_scale - pane.grass_sway_scale).abs() > 0.001 {
        grass_bridge.sway_strength_scale = pane.grass_sway_scale;
    }
}

fn animate_draft_zone(
    time: Res<Time>,
    mut zones: Query<(&DraftZone, &mut Transform, &mut WindZone)>,
) {
    for (draft, mut transform, mut zone) in &mut zones {
        let t = time.elapsed_secs() * draft.speed;
        transform.translation.x = draft.center.x + draft.span.x * t.sin();
        transform.translation.z = draft.center.z + draft.span.z * (t * 0.7).cos();
        zone.direction = Vec3::new(1.0, 0.0, 0.28 + 0.22 * (t * 0.6).sin()).normalize_or_zero();
    }
}

fn update_overlay(
    pane: Res<OpenWorldPane>,
    grass_diagnostics: Res<grass::GrassDiagnostics>,
    foliage: Query<&FoliageSurfaceStats>,
    scene: Res<OpenWorldScene>,
    mut overlay: Query<&mut Text, With<ShowcaseOverlay>>,
) {
    let Ok(mut text) = overlay.single_mut() else {
        return;
    };
    let foliage_instances = foliage
        .get(scene.foliage_surface_entity)
        .map(|stats| stats.active_instances)
        .unwrap_or_default();

    text.0 = format!(
        "Open-world integration showcase\nterrain + procgen noise + sky + wind + foliage + grass\nOrbit: left drag  Pan: middle drag  Zoom: wheel\nHeight {:.0} / offset {:.1}  Time {:04.1}h  Clouds {:.2}\nWind {:.1} m/s  intensity {:.2}  Grass density {:.0}  Canopy {:.3}\nVisible grass blades {}  Foliage instances {}  World-wind bridge {}",
        pane.terrain_height_scale,
        pane.terrain_height_offset,
        pane.time_of_day_hours,
        pane.cloud_coverage,
        pane.wind_speed,
        pane.wind_intensity,
        pane.grass_density,
        pane.canopy_density,
        grass_diagnostics.visible_blades,
        foliage_instances,
        if grass_diagnostics.using_world_wind {
            "on"
        } else {
            "off"
        },
    );
}

fn update_monitors(
    grass_diagnostics: Res<grass::GrassDiagnostics>,
    foliage: Query<&FoliageSurfaceStats>,
    scene: Res<OpenWorldScene>,
    mut pane: ResMut<OpenWorldPane>,
) {
    pane.visible_grass_blades = grass_diagnostics.visible_blades;
    pane.foliage_instances = foliage
        .get(scene.foliage_surface_entity)
        .map(|stats| stats.active_instances)
        .unwrap_or_default();
}
