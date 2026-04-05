use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task, futures::check_ready},
};

use crate::{
    TerrainChunk, TerrainChunkBounds, TerrainChunkReady, TerrainChunkRemoved, TerrainChunkState,
    TerrainColliderData, TerrainColliderPatch, TerrainColliderReady, TerrainDebugColorMode,
    TerrainDiagnostics, TerrainFocus, TerrainFocusPoints, TerrainProbe, TerrainProbeSample,
    TerrainRoot, TerrainRootStats, TerrainSourceHandle, TerrainTextureMaterial,
    chunking::{TerrainChunkKey, chunk_center_local, chunk_coords_in_radius, chunk_origin_local},
    config::TerrainConfig,
    debug::TerrainDebugConfig,
    lod::select_lod_with_hysteresis,
    meshing::TerrainBuildArtifact,
    sampling::sample_terrain,
};

#[derive(Resource, Default)]
pub(crate) struct TerrainRuntimeState {
    active: bool,
    frame: u64,
}

#[derive(Component, Default)]
pub(crate) struct TerrainRootRuntime {
    pub revision: u64,
    pub material: Option<TerrainRootMaterialHandle>,
    pub mesh_color_mode: TerrainDebugColorMode,
}

#[derive(Clone)]
pub(crate) enum TerrainRootMaterialHandle {
    Standard(Handle<StandardMaterial>),
    Textured(Handle<TerrainTextureMaterial>),
}

#[derive(Component, Default)]
pub(crate) struct TerrainChunkRuntime {
    pub build_generation: u64,
    pub needs_collider: bool,
    pub collider_patch: Option<Arc<TerrainColliderPatch>>,
    pub cache_hits: u64,
}

#[derive(Component)]
pub(crate) struct TerrainBuildTask {
    pub generation: u64,
    pub task: Task<Result<TerrainBuildArtifact, String>>,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct TerrainCacheKey {
    terrain: Entity,
    revision: u64,
    key: TerrainChunkKey,
    color_mode: TerrainDebugColorMode,
}

#[derive(Clone)]
struct TerrainCacheEntry {
    mesh: Handle<Mesh>,
    bounds: TerrainChunkBounds,
    collider_patch: Option<Arc<TerrainColliderPatch>>,
    last_used: u64,
}

#[derive(Resource, Default)]
pub(crate) struct TerrainChunkCache {
    entries: HashMap<TerrainCacheKey, TerrainCacheEntry>,
    tick: u64,
}

pub(crate) fn activate_runtime(mut runtime: ResMut<TerrainRuntimeState>) {
    runtime.active = true;
}

pub(crate) fn deactivate_runtime(
    mut commands: Commands,
    mut runtime: ResMut<TerrainRuntimeState>,
    chunks: Query<Entity, With<TerrainChunk>>,
) {
    runtime.active = false;
    for entity in &chunks {
        commands.entity(entity).despawn();
    }
}

pub(crate) fn runtime_is_active(runtime: Res<TerrainRuntimeState>) -> bool {
    runtime.active
}

pub(crate) fn advance_runtime_frame(
    mut runtime: ResMut<TerrainRuntimeState>,
    mut cache: ResMut<TerrainChunkCache>,
) {
    runtime.frame = runtime.frame.wrapping_add(1);
    cache.tick = runtime.frame;
}

pub(crate) fn sync_root_materials(
    mut commands: Commands,
    debug: Res<TerrainDebugConfig>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut textured_materials: ResMut<Assets<TerrainTextureMaterial>>,
    mut roots: Query<
        (
            Entity,
            Ref<TerrainConfig>,
            Ref<TerrainSourceHandle>,
            Option<&mut TerrainRootRuntime>,
        ),
        With<TerrainRoot>,
    >,
    mut chunks: Query<
        (
            Entity,
            &TerrainChunk,
            &mut TerrainChunkState,
            &mut TerrainChunkRuntime,
        ),
        With<TerrainChunk>,
    >,
) {
    let mut changed_roots = HashSet::new();

    for (entity, config, source, runtime) in &mut roots {
        let mesh_color_mode = mesh_color_mode(debug.color_mode);
        let Some(mut runtime) = runtime else {
            commands.entity(entity).insert(TerrainRootRuntime {
                revision: 1,
                material: Some(build_root_material(
                    &config,
                    mesh_color_mode,
                    &mut materials,
                    &mut textured_materials,
                )),
                mesh_color_mode,
            });
            changed_roots.insert(entity);
            continue;
        };

        let changed = runtime.material.is_none()
            || config.is_changed()
            || source.is_changed()
            || runtime.mesh_color_mode != mesh_color_mode;

        if changed {
            runtime.revision = runtime.revision.wrapping_add(1);
            runtime.mesh_color_mode = mesh_color_mode;
            runtime.material = Some(build_root_material(
                &config,
                mesh_color_mode,
                &mut materials,
                &mut textured_materials,
            ));
            changed_roots.insert(entity);
        }
    }

    if changed_roots.is_empty() {
        return;
    }

    for (entity, chunk, mut state, mut runtime) in &mut chunks {
        if changed_roots.contains(&chunk.terrain) {
            *state = TerrainChunkState::Queued;
            runtime.build_generation = runtime.build_generation.wrapping_add(1);
            runtime.collider_patch = None;
            commands.entity(entity).remove::<TerrainBuildTask>();
            commands.entity(entity).remove::<TerrainColliderData>();
        }
    }
}

pub(crate) fn refresh_chunk_targets(
    mut commands: Commands,
    focus_points: Res<TerrainFocusPoints>,
    focuses: Query<(&GlobalTransform, &TerrainFocus)>,
    mut roots: Query<
        (
            Entity,
            &TerrainConfig,
            &GlobalTransform,
            &mut TerrainRootStats,
            Option<&TerrainRootRuntime>,
        ),
        With<TerrainRoot>,
    >,
    mut chunk_queries: ParamSet<(
        Query<(Entity, &TerrainChunk)>,
        Query<(
            Entity,
            &mut TerrainChunk,
            &mut TerrainChunkState,
            &mut TerrainChunkRuntime,
            &mut Transform,
        )>,
    )>,
    mut removed_writer: MessageWriter<TerrainChunkRemoved>,
) {
    let existing_chunks: HashMap<Entity, HashMap<IVec2, (Entity, TerrainChunkKey)>> = chunk_queries
        .p0()
        .iter()
        .fold(HashMap::new(), |mut map, (entity, chunk)| {
            map.entry(chunk.terrain)
                .or_default()
                .insert(chunk.key.coord, (entity, chunk.key));
            map
        });

    for (terrain, config, terrain_transform, mut stats, runtime) in &mut roots {
        if runtime.is_none() {
            continue;
        }

        let local_focuses =
            gather_focuses(terrain, config, terrain_transform, &focus_points, &focuses);
        stats.focus_count = local_focuses.len() as u32;

        let mut desired = HashMap::<IVec2, DesiredChunkState>::new();
        let previous = existing_chunks.get(&terrain);

        for focus in &local_focuses {
            let coords = chunk_coords_in_radius(focus.local_position, focus.visual_radius, config);
            for coord in coords {
                let center = chunk_center_local(coord, config);
                let distance = center.distance(focus.local_position);
                let previous_lod = previous
                    .and_then(|chunks| chunks.get(&coord))
                    .map(|(_, key)| key.lod);
                let lod = select_lod_with_hysteresis(distance, previous_lod, &config.lod);
                let needs_collider = distance <= focus.collider_radius;

                desired
                    .entry(coord)
                    .and_modify(|entry| {
                        entry.lod = entry.lod.min(lod);
                        entry.needs_collider |= needs_collider;
                    })
                    .or_insert(DesiredChunkState {
                        lod,
                        needs_collider,
                    });
            }
        }

        let previous_coords = previous
            .map(|entries| entries.keys().copied().collect::<HashSet<_>>())
            .unwrap_or_default();

        for coord in &previous_coords {
            if desired.contains_key(coord) {
                continue;
            }
            if let Some((entity, key)) = previous.and_then(|entries| entries.get(coord).copied()) {
                removed_writer.write(TerrainChunkRemoved {
                    terrain,
                    chunk: entity,
                    key,
                });
                commands.entity(entity).despawn();
            }
        }

        let mut active_visual = 0_u32;
        let mut active_collider = 0_u32;
        let mut max_visible_lod = 0_u8;

        for (coord, desired_state) in desired {
            active_visual += 1;
            if desired_state.needs_collider {
                active_collider += 1;
            }
            max_visible_lod = max_visible_lod.max(desired_state.lod);

            if let Some((entity, current_key)) =
                previous.and_then(|entries| entries.get(&coord).copied())
            {
                if let Ok((_, mut chunk, mut state, mut runtime, mut transform)) =
                    chunk_queries.p1().get_mut(entity)
                {
                    transform.translation = chunk_transform(coord, config);
                    if current_key.lod != desired_state.lod
                        || runtime.needs_collider != desired_state.needs_collider
                    {
                        chunk.key = TerrainChunkKey {
                            coord,
                            lod: desired_state.lod,
                        };
                        *state = TerrainChunkState::Queued;
                        runtime.build_generation = runtime.build_generation.wrapping_add(1);
                        runtime.needs_collider = desired_state.needs_collider;
                        runtime.collider_patch = None;
                        commands.entity(entity).remove::<TerrainBuildTask>();
                        commands.entity(entity).remove::<TerrainColliderData>();
                        commands
                            .entity(entity)
                            .insert(Name::new(chunk_name(chunk.key)));
                    } else {
                        runtime.needs_collider = desired_state.needs_collider;
                    }
                }
                continue;
            }

            let key = TerrainChunkKey {
                coord,
                lod: desired_state.lod,
            };
            let entity = commands
                .spawn((
                    Name::new(chunk_name(key)),
                    TerrainChunk { terrain, key },
                    TerrainChunkState::Queued,
                    TerrainChunkRuntime {
                        build_generation: 1,
                        needs_collider: desired_state.needs_collider,
                        ..default()
                    },
                    Transform::from_translation(chunk_transform(coord, config)),
                    GlobalTransform::default(),
                    Visibility::Visible,
                    InheritedVisibility::VISIBLE,
                    ViewVisibility::default(),
                ))
                .id();
            commands.entity(terrain).add_child(entity);
        }

        stats.active_visual_chunks = active_visual;
        stats.active_collider_chunks = active_collider;
        stats.max_visible_lod = max_visible_lod;
    }
}

pub(crate) fn queue_chunk_builds(
    mut commands: Commands,
    mut cache: ResMut<TerrainChunkCache>,
    roots: Query<(&TerrainConfig, &TerrainSourceHandle, &TerrainRootRuntime), With<TerrainRoot>>,
    mut chunks: Query<(
        Entity,
        &TerrainChunk,
        &mut TerrainChunkState,
        &mut TerrainChunkRuntime,
        Option<&TerrainBuildTask>,
    )>,
    ready_writer: MessageWriter<TerrainChunkReady>,
) {
    let mut ready_writer = ready_writer;
    let mut queued_by_root = HashMap::<Entity, usize>::new();
    let mut work: Vec<_> = chunks
        .iter_mut()
        .filter_map(|(entity, chunk, state, runtime, task)| {
            if *state != TerrainChunkState::Queued || task.is_some() {
                return None;
            }
            Some((entity, *chunk, runtime.build_generation))
        })
        .collect();
    work.sort_by_key(|(_, chunk, _)| (chunk.key.lod, chunk.key.coord.y, chunk.key.coord.x));

    for (entity, chunk, generation) in work {
        let Ok((config, source, root_runtime)) = roots.get(chunk.terrain) else {
            continue;
        };
        let Some(material) = root_runtime.material.clone() else {
            continue;
        };
        let builds = queued_by_root.entry(chunk.terrain).or_default();
        if *builds >= config.streaming.max_builds_per_frame {
            continue;
        }

        let cache_key = TerrainCacheKey {
            terrain: chunk.terrain,
            revision: root_runtime.revision,
            key: chunk.key,
            color_mode: root_runtime.mesh_color_mode,
        };

        let tick = cache.tick;
        if let Some(entry) = cache.entries.get_mut(&cache_key) {
            entry.last_used = tick;
            if let Ok((_, _, mut state, mut runtime, _)) = chunks.get_mut(entity) {
                *state = TerrainChunkState::Ready;
                runtime.collider_patch = entry.collider_patch.clone();
                runtime.cache_hits = runtime.cache_hits.wrapping_add(1);
            }
            insert_chunk_material(
                &mut commands,
                entity,
                material,
                entry.mesh.clone(),
                entry.bounds,
            );
            ready_writer.write(TerrainChunkReady {
                terrain: chunk.terrain,
                chunk: entity,
                key: chunk.key,
                from_cache: true,
            });
            continue;
        }

        let source = source.0.clone();
        let config = config.clone();
        let color_mode = root_runtime.mesh_color_mode;
        let task = AsyncComputeTaskPool::get().spawn(async move {
            crate::meshing::build_chunk_artifact(source.as_ref(), &config, chunk.key, color_mode)
        });
        if let Ok((_, _, mut state, _, _)) = chunks.get_mut(entity) {
            *state = TerrainChunkState::Building;
        }
        commands
            .entity(entity)
            .insert(TerrainBuildTask { generation, task });
        *builds += 1;
    }
}

pub(crate) fn poll_chunk_builds(
    mut commands: Commands,
    mut cache: ResMut<TerrainChunkCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    roots: Query<(&TerrainRootRuntime,), With<TerrainRoot>>,
    mut chunks: Query<(
        Entity,
        &TerrainChunk,
        &mut TerrainChunkState,
        &mut TerrainChunkRuntime,
        &mut TerrainBuildTask,
    )>,
    mut ready_writer: MessageWriter<TerrainChunkReady>,
) {
    for (entity, chunk, mut state, mut runtime, mut task) in &mut chunks {
        let Some(result) = check_ready(&mut task.task) else {
            continue;
        };
        commands.entity(entity).remove::<TerrainBuildTask>();

        if task.generation != runtime.build_generation {
            *state = TerrainChunkState::Queued;
            continue;
        }

        let Ok((root_runtime,)) = roots.get(chunk.terrain) else {
            continue;
        };
        let Some(material) = root_runtime.material.clone() else {
            continue;
        };

        match result {
            Ok(artifact) => {
                let mesh = meshes.add(artifact.mesh);
                let cache_key = TerrainCacheKey {
                    terrain: chunk.terrain,
                    revision: root_runtime.revision,
                    key: chunk.key,
                    color_mode: root_runtime.mesh_color_mode,
                };
                let tick = cache.tick;
                cache.entries.insert(
                    cache_key,
                    TerrainCacheEntry {
                        mesh: mesh.clone(),
                        bounds: artifact.bounds,
                        collider_patch: artifact.collider_patch.clone(),
                        last_used: tick,
                    },
                );

                runtime.collider_patch = artifact.collider_patch;
                *state = TerrainChunkState::Ready;
                insert_chunk_material(&mut commands, entity, material, mesh, artifact.bounds);
                ready_writer.write(TerrainChunkReady {
                    terrain: chunk.terrain,
                    chunk: entity,
                    key: chunk.key,
                    from_cache: false,
                });
            }
            Err(_) => {
                *state = TerrainChunkState::Failed;
            }
        }
    }
}

fn build_root_material(
    config: &TerrainConfig,
    color_mode: TerrainDebugColorMode,
    standard_materials: &mut Assets<StandardMaterial>,
    textured_materials: &mut Assets<TerrainTextureMaterial>,
) -> TerrainRootMaterialHandle {
    if color_mode == TerrainDebugColorMode::Natural {
        if let Some(material) = crate::textured_material::build_textured_material(&config.material)
        {
            return TerrainRootMaterialHandle::Textured(textured_materials.add(material));
        }
    }

    TerrainRootMaterialHandle::Standard(standard_materials.add(config.material.standard_material()))
}

fn insert_chunk_material(
    commands: &mut Commands,
    entity: Entity,
    material: TerrainRootMaterialHandle,
    mesh: Handle<Mesh>,
    bounds: TerrainChunkBounds,
) {
    let mut entity_commands = commands.entity(entity);
    entity_commands.insert((Mesh3d(mesh), bounds));
    match material {
        TerrainRootMaterialHandle::Standard(handle) => {
            entity_commands.remove::<MeshMaterial3d<TerrainTextureMaterial>>();
            entity_commands.insert(MeshMaterial3d(handle));
        }
        TerrainRootMaterialHandle::Textured(handle) => {
            entity_commands.remove::<MeshMaterial3d<StandardMaterial>>();
            entity_commands.insert(MeshMaterial3d(handle));
        }
    }
}

pub(crate) fn sync_chunk_colliders(
    mut commands: Commands,
    roots: Query<&TerrainConfig, With<TerrainRoot>>,
    chunks: Query<
        (
            Entity,
            &TerrainChunk,
            &TerrainChunkState,
            &TerrainChunkRuntime,
            Option<&TerrainColliderData>,
        ),
        With<TerrainChunk>,
    >,
    mut writer: MessageWriter<TerrainColliderReady>,
) {
    for (entity, chunk, state, runtime, collider) in &chunks {
        let Ok(config) = roots.get(chunk.terrain) else {
            continue;
        };

        let wants_collider =
            config.collider.enabled && runtime.needs_collider && *state == TerrainChunkState::Ready;
        match (wants_collider, collider, runtime.collider_patch.clone()) {
            (true, None, Some(patch)) => {
                commands.entity(entity).insert(TerrainColliderData(patch));
                writer.write(TerrainColliderReady {
                    terrain: chunk.terrain,
                    chunk: entity,
                    key: chunk.key,
                });
            }
            (false, Some(_), _) => {
                commands.entity(entity).remove::<TerrainColliderData>();
            }
            _ => {}
        }
    }
}

pub(crate) fn update_probe_samples(
    mut commands: Commands,
    roots: Query<
        (
            Entity,
            &GlobalTransform,
            &TerrainConfig,
            &TerrainSourceHandle,
        ),
        With<TerrainRoot>,
    >,
    mut probes: Query<(
        Entity,
        &GlobalTransform,
        &TerrainProbe,
        Option<&mut TerrainProbeSample>,
    )>,
) {
    for (entity, probe_transform, probe, sample_component) in &mut probes {
        let world_position = probe_transform.translation() + probe.world_offset;
        let mut best: Option<(Entity, crate::TerrainSample)> = None;

        for (terrain_entity, terrain_transform, config, source) in &roots {
            if probe.terrain.is_some() && probe.terrain != Some(terrain_entity) {
                continue;
            }
            if let Some(sample) =
                sample_terrain(world_position, terrain_transform, config, source.0.as_ref())
            {
                best = Some((terrain_entity, sample));
                if probe.terrain.is_some() {
                    break;
                }
            }
        }

        match best {
            Some((_, sample)) => {
                let reading = TerrainProbeSample {
                    height: sample.height,
                    world_position: sample.world_position,
                    normal: sample.normal,
                    slope_degrees: sample.slope_degrees,
                    dominant_layer: sample.layers.dominant_layer,
                };

                if let Some(mut current) = sample_component {
                    *current = reading;
                } else {
                    commands.entity(entity).insert(reading);
                }
            }
            None => {
                if sample_component.is_some() {
                    commands.entity(entity).remove::<TerrainProbeSample>();
                }
            }
        }
    }
}

pub(crate) fn update_diagnostics(
    cache: Res<TerrainChunkCache>,
    focus_points: Res<TerrainFocusPoints>,
    focuses: Query<&TerrainFocus>,
    chunks: Query<
        (
            &TerrainChunk,
            &TerrainChunkState,
            &TerrainChunkRuntime,
            Option<&TerrainColliderData>,
        ),
        With<TerrainChunk>,
    >,
    mut roots: Query<(Entity, &TerrainConfig, &mut TerrainRootStats), With<TerrainRoot>>,
    mut diagnostics: ResMut<TerrainDiagnostics>,
) {
    let mut by_root = HashMap::<Entity, (u32, u32, u64)>::new();

    diagnostics.active_roots = roots.iter().count() as u32;
    diagnostics.total_chunks = chunks.iter().count() as u32;
    diagnostics.pending_chunks = chunks
        .iter()
        .filter(|(_, state, _, _)| {
            matches!(
                **state,
                TerrainChunkState::Queued | TerrainChunkState::Building
            )
        })
        .count() as u32;
    diagnostics.ready_chunks = chunks
        .iter()
        .filter(|(_, state, _, _)| **state == TerrainChunkState::Ready)
        .count() as u32;
    diagnostics.collider_chunks = chunks
        .iter()
        .filter(|(_, _, _, collider)| collider.is_some())
        .count() as u32;
    diagnostics.cache_entries = cache.entries.len() as u32;
    diagnostics.focus_points = focus_points.0.len() as u32 + focuses.iter().count() as u32;

    let root_configs: HashMap<Entity, &TerrainConfig> = roots
        .iter()
        .map(|(entity, config, _)| (entity, config))
        .collect();

    let mut total_vertices = 0_u64;
    let mut total_triangles = 0_u64;

    for (chunk, state, runtime, _) in &chunks {
        let entry = by_root.entry(chunk.terrain).or_insert((0, 0, 0));
        match state {
            TerrainChunkState::Queued | TerrainChunkState::Building => entry.0 += 1,
            TerrainChunkState::Ready => {
                entry.1 += 1;
                if let Some(config) = root_configs.get(&chunk.terrain) {
                    let resolution =
                        crate::meshing::resolution_for_lod(config, chunk.key.lod);
                    let verts = (resolution + 1) as u64 * (resolution + 1) as u64;
                    let tris = resolution as u64 * resolution as u64 * 2;
                    total_vertices += verts;
                    total_triangles += tris;
                }
            }
            TerrainChunkState::Failed => {}
        }
        entry.2 = entry.2.saturating_add(runtime.cache_hits);
    }

    diagnostics.estimated_vertex_count = total_vertices;
    diagnostics.estimated_triangle_count = total_triangles;

    for (entity, _, mut stats) in &mut roots {
        let (pending, ready, cache_hits) = by_root.get(&entity).copied().unwrap_or((0, 0, 0));
        stats.pending_chunks = pending;
        stats.ready_chunks = ready;
        stats.cache_hits = cache_hits;
    }
}

pub(crate) fn prune_cache(
    mut cache: ResMut<TerrainChunkCache>,
    roots: Query<&TerrainConfig, With<TerrainRoot>>,
) {
    let max_entries = roots
        .iter()
        .map(|config| config.cache.max_entries)
        .max()
        .unwrap_or(0);
    if max_entries == 0 || cache.entries.len() <= max_entries {
        return;
    }

    let mut entries: Vec<_> = cache
        .entries
        .iter()
        .map(|(key, value)| (*key, value.last_used))
        .collect();
    entries.sort_by_key(|(_, last_used)| *last_used);
    let remove_count = cache.entries.len().saturating_sub(max_entries);
    for (key, _) in entries.into_iter().take(remove_count) {
        cache.entries.remove(&key);
    }
}

#[derive(Clone, Copy)]
struct DesiredChunkState {
    lod: u8,
    needs_collider: bool,
}

#[derive(Clone, Copy)]
struct LocalFocusPoint {
    local_position: Vec2,
    visual_radius: f32,
    collider_radius: f32,
}

fn gather_focuses(
    terrain: Entity,
    config: &TerrainConfig,
    terrain_transform: &GlobalTransform,
    explicit_points: &TerrainFocusPoints,
    focuses: &Query<(&GlobalTransform, &TerrainFocus)>,
) -> Vec<LocalFocusPoint> {
    let mut points = Vec::new();
    let inverse = terrain_transform.affine().inverse();

    for point in &explicit_points.0 {
        if point.terrain.is_some() && point.terrain != Some(terrain) {
            continue;
        }
        let local = inverse.transform_point3(point.position);
        points.push(LocalFocusPoint {
            local_position: local.xz(),
            visual_radius: (config.streaming.visual_radius + point.visual_radius_bias).max(0.0),
            collider_radius: (config.streaming.collider_radius + point.collider_radius_bias)
                .max(0.0),
        });
    }

    for (transform, focus) in focuses.iter() {
        if focus.terrain.is_some() && focus.terrain != Some(terrain) {
            continue;
        }
        let local = inverse.transform_point3(transform.translation());
        points.push(LocalFocusPoint {
            local_position: local.xz(),
            visual_radius: (config.streaming.visual_radius + focus.visual_radius_bias).max(0.0),
            collider_radius: (config.streaming.collider_radius + focus.collider_radius_bias)
                .max(0.0),
        });
    }

    if points.is_empty() {
        points.push(LocalFocusPoint {
            local_position: config.size * 0.5,
            visual_radius: config.streaming.visual_radius,
            collider_radius: config.streaming.collider_radius,
        });
    }

    points
}

fn chunk_name(key: TerrainChunkKey) -> String {
    format!(
        "Terrain Chunk ({}, {}) LOD {}",
        key.coord.x, key.coord.y, key.lod
    )
}

fn chunk_transform(coord: IVec2, config: &TerrainConfig) -> Vec3 {
    let origin = chunk_origin_local(coord, config);
    Vec3::new(origin.x, 0.0, origin.y)
}

fn mesh_color_mode(mode: TerrainDebugColorMode) -> TerrainDebugColorMode {
    match mode {
        TerrainDebugColorMode::ByChunkState => TerrainDebugColorMode::Natural,
        other => other,
    }
}

#[cfg(test)]
#[path = "systems_tests.rs"]
mod tests;
