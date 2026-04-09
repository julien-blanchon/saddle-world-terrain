use std::collections::HashSet;

use bevy::prelude::*;
use saddle_world_terrain::{TerrainChunk, TerrainColliderData, TerrainDebugColorMode};

pub fn entity_by_name(world: &mut World, name: &str) -> Option<Entity> {
    let mut query = world.query::<(Entity, &Name)>();
    query
        .iter(world)
        .find_map(|(entity, entity_name)| (entity_name.as_str() == name).then_some(entity))
}

pub fn diagnostics(world: &World) -> saddle_world_terrain::TerrainDiagnostics {
    world
        .resource::<saddle_world_terrain::TerrainDiagnostics>()
        .clone()
}

pub fn overlay_text(world: &mut World) -> Option<String> {
    let overlay = entity_by_name(world, "Overlay")?;
    world.get::<Text>(overlay).map(|text| text.0.clone())
}

pub fn focus_stats(world: &mut World) -> Option<saddle_world_terrain::TerrainProbeSample> {
    let focus = entity_by_name(world, "Lab Focus")?;
    world
        .get::<saddle_world_terrain::TerrainProbeSample>(focus)
        .cloned()
}

pub fn chunk_keys(world: &mut World) -> HashSet<(IVec2, u8)> {
    let mut query = world.query::<&TerrainChunk>();
    query
        .iter(world)
        .map(|chunk| (chunk.key.coord, chunk.key.lod))
        .collect()
}

pub fn lod_levels(world: &mut World) -> HashSet<u8> {
    let mut query = world.query::<&TerrainChunk>();
    query.iter(world).map(|chunk| chunk.key.lod).collect()
}

pub fn lod_count(world: &mut World, lod: u8) -> usize {
    let mut query = world.query::<&TerrainChunk>();
    query
        .iter(world)
        .filter(|chunk| chunk.key.lod == lod)
        .count()
}

pub fn collider_chunk_coords(world: &mut World) -> HashSet<IVec2> {
    let mut query = world.query::<(&TerrainChunk, &TerrainColliderData)>();
    query
        .iter(world)
        .map(|(chunk, _)| chunk.key.coord)
        .collect()
}

pub fn set_focus_position(world: &mut World, position: Vec3) {
    let focus = entity_by_name(world, "Lab Focus").expect("Lab Focus entity should exist");
    let transform = Transform::from_translation(position);
    let global_transform = GlobalTransform::from(Transform::from_translation(position));
    world.entity_mut(focus).insert(transform);
    world.entity_mut(focus).insert(global_transform);
}

pub fn set_debug_color_mode(world: &mut World, mode: TerrainDebugColorMode) {
    world.resource_mut::<saddle_world_terrain::TerrainDebugConfig>().color_mode = mode;
}
