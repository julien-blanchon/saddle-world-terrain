use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use saddle_procgen_noise::{
    Fbm, FractalConfig, NoiseSeed, NoiseSource, Perlin, Ridged, RidgedConfig, signed_to_unit,
};
use saddle_pane::prelude::*;
use saddle_world_terrain::{
    TerrainBlendRange, TerrainConfig, TerrainDataset, TerrainDebugColorMode, TerrainDebugConfig,
    TerrainDiagnostics, TerrainLayer, TerrainMaterialProfile, TerrainRoot, TerrainRootStats,
};

#[derive(Resource, Clone, Pane)]
#[pane(title = "Terrain Controls", position = "top-right")]
pub struct TerrainExamplePane {
    #[pane(folder = "Debug")]
    pub show_chunk_bounds: bool,

    #[pane(folder = "Debug")]
    pub show_focus_rings: bool,

    #[pane(folder = "Debug")]
    pub show_collider_bounds: bool,

    #[pane(folder = "Debug")]
    pub show_sample_probes: bool,

    #[pane(
        folder = "Debug",
        label = "Color Mode",
        select(options = ["Natural", "LOD", "Chunk State", "Layer Dominance", "Slope Band"])
    )]
    pub color_mode: usize,

    #[pane(folder = "Streaming", slider, min = 64.0, max = 384.0, step = 8.0)]
    pub visual_radius: f32,

    #[pane(folder = "Streaming", slider, min = 0.0, max = 192.0, step = 4.0)]
    pub collider_radius: f32,

    #[pane(folder = "Streaming", label = "Builds / Frame", slider, min = 1.0, max = 16.0, step = 1.0)]
    pub max_builds_per_frame: u32,

    #[pane(folder = "Mesh", label = "Vertex Resolution", slider, min = 16.0, max = 128.0, step = 1.0)]
    pub vertex_resolution: u32,

    #[pane(folder = "Mesh", slider, min = 0.0, max = 24.0, step = 0.5)]
    pub skirt_depth: f32,

    #[pane(folder = "Shape", label = "Height Scale", slider, min = 16.0, max = 220.0, step = 1.0)]
    pub height_scale: f32,

    #[pane(folder = "Shape", label = "Height Offset", slider, min = -32.0, max = 32.0, step = 0.5)]
    pub height_offset: f32,

    #[pane(folder = "LOD", label = "LOD Count", slider, min = 2.0, max = 8.0, step = 1.0)]
    pub lod_count: u32,

    #[pane(folder = "LOD", label = "Near Distance", slider, min = 16.0, max = 160.0, step = 4.0)]
    pub lod_near_distance: f32,

    #[pane(folder = "LOD", label = "Distance Multiplier", slider, min = 1.2, max = 3.5, step = 0.1)]
    pub lod_distance_multiplier: f32,

    #[pane(folder = "LOD", slider, min = 0.0, max = 32.0, step = 1.0)]
    pub lod_hysteresis: f32,

    #[pane(folder = "Collider")]
    pub collider_enabled: bool,

    #[pane(folder = "Collider", label = "Resolution Divisor", slider, min = 1.0, max = 8.0, step = 1.0)]
    pub collider_resolution_divisor: u32,
}

impl Default for TerrainExamplePane {
    fn default() -> Self {
        terrain_example_pane(&TerrainConfig::default(), &TerrainDebugConfig::default())
    }
}

impl TerrainExamplePane {
    pub fn debug_color_mode(&self) -> TerrainDebugColorMode {
        match self.color_mode {
            1 => TerrainDebugColorMode::ByLod,
            2 => TerrainDebugColorMode::ByChunkState,
            3 => TerrainDebugColorMode::ByLayerDominance,
            4 => TerrainDebugColorMode::BySlopeBand,
            _ => TerrainDebugColorMode::Natural,
        }
    }
}

#[derive(Resource, Default, Clone, Pane)]
#[pane(title = "Terrain Stats", position = "bottom-right")]
pub struct TerrainExampleStatsPane {
    #[pane(folder = "Scene", monitor)]
    pub fps: f32,

    #[pane(folder = "Scene", monitor)]
    pub active_roots: u32,

    #[pane(folder = "Chunks", monitor)]
    pub total_chunks: u32,

    #[pane(folder = "Chunks", monitor)]
    pub ready_chunks: u32,

    #[pane(folder = "Chunks", monitor)]
    pub pending_chunks: u32,

    #[pane(folder = "Chunks", monitor)]
    pub collider_chunks: u32,

    #[pane(folder = "Chunks", monitor)]
    pub active_visual_chunks: u32,

    #[pane(folder = "Chunks", monitor)]
    pub active_collider_chunks: u32,

    #[pane(folder = "Cache", monitor)]
    pub cache_entries: u32,

    #[pane(folder = "Cache", monitor)]
    pub cache_hits: u64,

    #[pane(folder = "Focus", monitor)]
    pub focus_points: u32,

    #[pane(folder = "Focus", label = "Max Visible LOD", monitor)]
    pub max_visible_lod: u32,

    #[pane(folder = "Geometry", monitor)]
    pub estimated_vertex_count: u64,

    #[pane(folder = "Geometry", monitor)]
    pub estimated_triangle_count: u64,
}

pub fn install_terrain_example_debug_ui(app: &mut App) {
    app.add_plugins((
        FrameTimeDiagnosticsPlugin::default(),
        bevy_flair::FlairPlugin,
        bevy_input_focus::InputDispatchPlugin,
        bevy_ui_widgets::UiWidgetsPlugins,
        bevy_input_focus::tab_navigation::TabNavigationPlugin,
        PanePlugin,
    ))
    .register_pane::<TerrainExamplePane>()
    .register_pane::<TerrainExampleStatsPane>()
    .add_systems(
        Update,
        (
            sync_terrain_example_pane,
            update_terrain_example_stats,
        ),
    );
}

pub fn terrain_example_pane(
    config: &TerrainConfig,
    debug: &TerrainDebugConfig,
) -> TerrainExamplePane {
    TerrainExamplePane {
        show_chunk_bounds: debug.show_chunk_bounds,
        show_focus_rings: debug.show_focus_rings,
        show_collider_bounds: debug.show_collider_bounds,
        show_sample_probes: debug.show_sample_probes,
        color_mode: match debug.color_mode {
            TerrainDebugColorMode::Natural => 0,
            TerrainDebugColorMode::ByLod => 1,
            TerrainDebugColorMode::ByChunkState => 2,
            TerrainDebugColorMode::ByLayerDominance => 3,
            TerrainDebugColorMode::BySlopeBand => 4,
        },
        visual_radius: config.streaming.visual_radius,
        collider_radius: config.streaming.collider_radius,
        max_builds_per_frame: config.streaming.max_builds_per_frame as u32,
        vertex_resolution: config.vertex_resolution,
        skirt_depth: config.skirt_depth,
        height_scale: config.height_scale,
        height_offset: config.height_offset,
        lod_count: config.lod.lod_count as u32,
        lod_near_distance: config.lod.near_distance,
        lod_distance_multiplier: config.lod.distance_multiplier,
        lod_hysteresis: config.lod.hysteresis,
        collider_enabled: config.collider.enabled,
        collider_resolution_divisor: config.collider.resolution_divisor,
    }
}

fn sync_terrain_example_pane(
    pane: Res<TerrainExamplePane>,
    mut debug: ResMut<TerrainDebugConfig>,
    mut roots: Query<&mut TerrainConfig, With<TerrainRoot>>,
) {
    if !pane.is_changed() {
        return;
    }

    debug.show_chunk_bounds = pane.show_chunk_bounds;
    debug.show_focus_rings = pane.show_focus_rings;
    debug.show_collider_bounds = pane.show_collider_bounds;
    debug.show_sample_probes = pane.show_sample_probes;
    debug.color_mode = pane.debug_color_mode();

    for mut config in &mut roots {
        config.streaming.visual_radius = pane.visual_radius.max(1.0);
        config.streaming.collider_radius = pane.collider_radius.max(0.0);
        config.streaming.max_builds_per_frame = pane.max_builds_per_frame.max(1) as usize;
        config.vertex_resolution = pane.vertex_resolution.max(2);
        config.skirt_depth = pane.skirt_depth.max(0.0);
        config.height_scale = pane.height_scale.max(1.0);
        config.height_offset = pane.height_offset;
        config.lod.lod_count = pane.lod_count.clamp(2, 8) as u8;
        config.lod.near_distance = pane.lod_near_distance.max(1.0);
        config.lod.distance_multiplier = pane.lod_distance_multiplier.max(1.05);
        config.lod.hysteresis = pane.lod_hysteresis.max(0.0);
        config.collider.enabled = pane.collider_enabled;
        config.collider.resolution_divisor = pane.collider_resolution_divisor.max(1);
    }
}

fn update_terrain_example_stats(
    mut stats: ResMut<TerrainExampleStatsPane>,
    terrain_diagnostics: Res<TerrainDiagnostics>,
    diagnostics_store: Res<DiagnosticsStore>,
    root_stats: Query<&TerrainRootStats, With<TerrainRoot>>,
) {
    if let Some(fps) = diagnostics_store
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|diagnostic| diagnostic.smoothed())
    {
        stats.fps = fps as f32;
    }

    stats.active_roots = terrain_diagnostics.active_roots;
    stats.total_chunks = terrain_diagnostics.total_chunks;
    stats.ready_chunks = terrain_diagnostics.ready_chunks;
    stats.pending_chunks = terrain_diagnostics.pending_chunks;
    stats.collider_chunks = terrain_diagnostics.collider_chunks;
    stats.cache_entries = terrain_diagnostics.cache_entries;
    stats.focus_points = terrain_diagnostics.focus_points;
    stats.estimated_vertex_count = terrain_diagnostics.estimated_vertex_count;
    stats.estimated_triangle_count = terrain_diagnostics.estimated_triangle_count;

    let mut active_visual_chunks = 0;
    let mut active_collider_chunks = 0;
    let mut max_visible_lod = 0;
    let mut cache_hits = 0;

    for root in &root_stats {
        active_visual_chunks += root.active_visual_chunks;
        active_collider_chunks += root.active_collider_chunks;
        max_visible_lod = max_visible_lod.max(root.max_visible_lod as u32);
        cache_hits += root.cache_hits;
    }

    stats.active_visual_chunks = active_visual_chunks;
    stats.active_collider_chunks = active_collider_chunks;
    stats.max_visible_lod = max_visible_lod;
    stats.cache_hits = cache_hits;
}

/// Generate a procedural heightmap dataset with weight-painted regions.
pub fn generated_dataset(dimensions: UVec2) -> TerrainDataset {
    let base = TerrainDataset::from_fn(dimensions, |_coord, uv| {
        let ridge = ((uv.x * std::f32::consts::TAU * 1.7).sin() * 0.35
            + (uv.y * std::f32::consts::TAU * 2.3).cos() * 0.25)
            * 0.5
            + 0.5;
        let crater = 1.0 - ((uv - Vec2::new(0.72, 0.28)).length() * 1.8).clamp(0.0, 1.0);
        let shelf = ((uv.x * 3.5).fract() * 0.6 + (uv.y * 2.0).fract() * 0.4).sin() * 0.1 + 0.5;
        (ridge * 0.55 + crater * 0.25 + shelf * 0.20).clamp(0.0, 1.0)
    })
    .unwrap();

    let weights = (0..dimensions.y)
        .flat_map(|y| {
            (0..dimensions.x).map(move |x| {
                let uv = Vec2::new(
                    x as f32 / (dimensions.x - 1).max(1) as f32,
                    y as f32 / (dimensions.y - 1).max(1) as f32,
                );
                let river = (1.0 - ((uv.x - 0.18).abs() * 8.0)).clamp(0.0, 1.0);
                let meadow = (1.0 - ((uv - Vec2::new(0.55, 0.58)).length() * 2.4)).clamp(0.0, 1.0);
                let road = (1.0 - ((uv.y - 0.44).abs() * 10.0)).clamp(0.0, 1.0);
                [river, meadow, road, 0.0]
            })
        })
        .collect();

    base.with_weight_map(
        saddle_world_terrain::TerrainWeightMap::from_rgba(dimensions, weights).unwrap(),
    )
}

/// A richly configured terrain for examples: 640x640 world units, 5 material
/// layers (water, meadow, dirt, rock, snow) with height and slope blending.
pub fn default_config() -> TerrainConfig {
    TerrainConfig {
        size: Vec2::new(640.0, 640.0),
        chunk_size: Vec2::new(40.0, 40.0),
        vertex_resolution: 64,
        height_scale: 82.0,
        height_offset: -6.0,
        skirt_depth: 8.0,
        streaming: saddle_world_terrain::TerrainStreamingConfig {
            visual_radius: 180.0,
            collider_radius: 90.0,
            max_builds_per_frame: 8,
        },
        material: TerrainMaterialProfile {
            layers: vec![
                TerrainLayer::tinted("water", Color::srgb(0.16, 0.34, 0.62)).with_weight_channel(0),
                TerrainLayer::tinted("meadow", Color::srgb(0.28, 0.56, 0.26))
                    .with_weight_channel(1)
                    .with_height_range(TerrainBlendRange::new(0.05, 0.65)),
                TerrainLayer::tinted("dirt", Color::srgb(0.47, 0.34, 0.24))
                    .with_weight_channel(2)
                    .with_height_range(TerrainBlendRange::new(0.10, 0.85)),
                TerrainLayer::tinted("rock", Color::srgb(0.56, 0.56, 0.58)).with_slope_range(
                    TerrainBlendRange {
                        start: 24.0,
                        end: 70.0,
                        falloff: 0.2,
                    },
                ),
                TerrainLayer::tinted("snow", Color::srgb(0.93, 0.95, 0.98)).with_height_range(
                    TerrainBlendRange {
                        start: 0.72,
                        end: 1.0,
                        falloff: 0.1,
                    },
                ),
            ],
            ..default()
        },
        ..default()
    }
}

pub fn island_dataset() -> TerrainDataset {
    let dims = UVec2::new(257, 257);
    let ridge_noise = Fbm::new(
        Perlin::new(NoiseSeed(7)),
        FractalConfig {
            octaves: 4,
            base_frequency: 2.0,
            lacunarity: 2.1,
            gain: 0.52,
            ..default()
        },
    );
    let mountain_noise = Ridged::new(
        Perlin::new(NoiseSeed(53)),
        RidgedConfig {
            fractal: FractalConfig {
                octaves: 3,
                base_frequency: 1.4,
                ..default()
            },
            ..default()
        },
    );
    let detail_noise = Fbm::new(
        Perlin::new(NoiseSeed(31)),
        FractalConfig {
            octaves: 3,
            base_frequency: 5.5,
            gain: 0.42,
            ..default()
        },
    );

    TerrainDataset::from_fn(dims, move |_coord, uv| {
        let center = Vec2::new(0.5, 0.5);
        let dist = (uv - center).length() * 2.0;
        let island_mask = (1.0 - dist.powf(1.8)).max(0.0);
        let ridge = signed_to_unit(ridge_noise.sample(uv * 4.0));
        let mountain = signed_to_unit(mountain_noise.sample(uv * 2.0));
        let detail = signed_to_unit(detail_noise.sample(uv * 4.0));

        let base = island_mask * (ridge * 0.45 + mountain * 0.35 + detail * 0.20);
        base.clamp(0.0, 1.0)
    })
    .expect("island heights should match dimensions")
}

pub fn island_config() -> TerrainConfig {
    TerrainConfig {
        size: Vec2::new(512.0, 512.0),
        chunk_size: Vec2::new(40.0, 40.0),
        vertex_resolution: 48,
        height_scale: 90.0,
        height_offset: -4.0,
        skirt_depth: 8.0,
        streaming: saddle_world_terrain::TerrainStreamingConfig {
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

pub fn mountain_dataset() -> TerrainDataset {
    let dims = UVec2::new(257, 257);
    let detail_noise = Fbm::new(
        Perlin::new(NoiseSeed(19)),
        FractalConfig {
            octaves: 4,
            base_frequency: 3.0,
            lacunarity: 2.1,
            gain: 0.48,
            ..default()
        },
    );
    let fine_noise = Fbm::new(
        Perlin::new(NoiseSeed(61)),
        FractalConfig {
            octaves: 2,
            base_frequency: 7.0,
            gain: 0.4,
            ..default()
        },
    );
    let ridge_detail = Ridged::new(
        Perlin::new(NoiseSeed(37)),
        RidgedConfig {
            fractal: FractalConfig {
                octaves: 3,
                base_frequency: 2.4,
                ..default()
            },
            ..default()
        },
    );

    TerrainDataset::from_fn(dims, move |_coord, uv| {
        let ridge_axis = (uv.x * 0.7 + uv.y * 0.3 - 0.5).abs();
        let ridge = (1.0 - ridge_axis * 3.2).max(0.0).powf(0.8);

        let peak1_dist = (uv - Vec2::new(0.3, 0.35)).length();
        let peak1 = (1.0 - peak1_dist * 2.5).max(0.0).powf(1.2);

        let peak2_dist = (uv - Vec2::new(0.65, 0.55)).length();
        let peak2 = (1.0 - peak2_dist * 2.2).max(0.0).powf(1.1);

        let peak3_dist = (uv - Vec2::new(0.5, 0.7)).length();
        let peak3 = (1.0 - peak3_dist * 2.8).max(0.0).powf(1.3);

        let noise1 = signed_to_unit(detail_noise.sample(uv * 4.0));
        let noise2 = fine_noise.sample(uv * 4.0) * 0.08;
        let ridge_crests = signed_to_unit(ridge_detail.sample(uv * 3.0));

        let valley_axis = ((uv.x - 0.48) * 2.0).abs();
        let valley = (1.0 - valley_axis * 3.0).max(0.0) * 0.15;

        let elevation = ridge * 0.35
            + peak1 * 0.30
            + peak2 * 0.25
            + peak3 * 0.20
            + noise1 * 0.12
            + noise2
            + ridge_crests * 0.06
            - valley;

        (elevation * 0.85 + 0.08).clamp(0.0, 1.0)
    })
    .expect("mountain heights should match dimensions")
}

pub fn mountain_config() -> TerrainConfig {
    TerrainConfig {
        size: Vec2::new(800.0, 800.0),
        chunk_size: Vec2::new(48.0, 48.0),
        vertex_resolution: 48,
        height_scale: 160.0,
        height_offset: -12.0,
        skirt_depth: 12.0,
        streaming: saddle_world_terrain::TerrainStreamingConfig {
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
