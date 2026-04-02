# Saddle World Terrain

Reusable heightfield terrain streaming for Bevy. The crate focuses on CPU-truth terrain data, chunk residency around one or more focus points, stable distance-based LOD, runtime sampling, and debug-friendly chunk state.

## Quick Start

```toml
saddle-world-terrain = { git = "https://github.com/julien-blanchon/saddle-world-terrain" }
```

```rust
use bevy::prelude::*;
use saddle_world_terrain::{TerrainBundle, TerrainConfig, TerrainDataset, TerrainPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TerrainPlugin::always_on(Update))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    let dataset = TerrainDataset::from_fn(UVec2::new(257, 257), |_coord, uv| {
        ((uv.x * std::f32::consts::TAU).sin() * 0.25 + 0.5).clamp(0.0, 1.0)
    })
    .unwrap();

    let mut config = TerrainConfig::default();
    config.size = Vec2::new(512.0, 512.0);
    config.chunk_size = Vec2::new(32.0, 32.0);

    let terrain = commands.spawn(TerrainBundle::new(dataset, config)).id();

    commands.spawn((
        saddle_world_terrain::TerrainFocus {
            terrain: Some(terrain),
            ..default()
        },
        Transform::from_xyz(256.0, 0.0, 256.0),
        GlobalTransform::default(),
    ));
}
```

## Public API

- Plugin: `TerrainPlugin::new(activate_schedule, deactivate_schedule, update_schedule)` or `TerrainPlugin::always_on(update_schedule)`
- System sets: `TerrainSystems::{MaintainFocus, UpdateMaterials, StreamChunks, BuildMeshes, BuildColliders, Debug}`
- Bundles and components: `TerrainBundle`, `TerrainRoot`, `TerrainChunk`, `TerrainFocus`, `TerrainProbe`, `TerrainColliderData`
- Resources: `TerrainDebugConfig`, `TerrainDiagnostics`, `TerrainFocusPoints`
- Configuration: `TerrainConfig`, `TerrainStreamingConfig`, `TerrainLodConfig`, `TerrainColliderConfig`, `TerrainCacheConfig`
- Source data: `TerrainSource`, `TerrainDataset`, `TerrainWeightMap`
- Sampling helpers: `sample_terrain`, `sample_height`, `sample_normal`, `sample_layer_weights`
- Messages: `TerrainChunkReady`, `TerrainChunkRemoved`, `TerrainColliderReady`

## Source Data Workflow

`saddle-world-terrain` separates static dataset truth from runtime chunk entities.

- Build or load a `TerrainDataset` up front.
- Spawn one `TerrainBundle` per terrain root.
- Attach `TerrainFocus` to any entity that should drive chunk residency.
- Optionally add extra points with `TerrainFocusPoints` when a secondary view, spectator, or debug sampler should keep terrain resident without spawning another focus entity.
- Query the source data through the public sampling helpers instead of reading chunk meshes directly.

`TerrainDataset` currently supports:

- direct height arrays
- procedural generation from a function
- height images (`R8`, `R16`, `RGBA8`)
- explicit RGBA weight maps for splat-style blending

The `TerrainSource` trait keeps room for future tiled or procedural backends without changing the runtime chunk API.

## Sampling API

Use the helpers with a terrain root transform, `TerrainConfig`, and any `TerrainSource`:

```rust
let sample = saddle_world_terrain::sample_terrain(world_pos, &terrain_transform, &config, source.as_ref());
```

`TerrainSample` includes:

- world-space surface height (`height`) plus terrain-local surface height (`local_height`)
- local and world hit positions
- world-space surface normal
- slope in degrees
- normalized material-layer weights plus the dominant layer

## Material Layering

The first shipped renderer path stays `StandardMaterial` compatible by baking blended layer colors into mesh vertex colors.

Layer weights can come from:

- explicit weight channels in one or more `TerrainWeightMap`s
- height rules
- slope rules
- any combination of the three

This keeps the public API renderer-neutral while still giving a practical v1 shading path.

`TerrainColliderPatch` is chunk-local data. Its `origin` starts at zero for the owning chunk entity, while the owning `TerrainChunk` key and transform define where that patch lives in the full terrain.

## Examples

| Example | What it shows | Run |
| --- | --- | --- |
| `basic` | Minimal terrain root plus one animated focus | `cargo run -p saddle-world-terrain-example-basic` |
| `clipmap_debug` | LOD color mode, chunk bounds, focus radii | `cargo run -p saddle-world-terrain-example-clipmap-debug` |
| `splat_layers` | Dominant-layer debug coloring from weight, height, and slope blending | `cargo run -p saddle-world-terrain-example-splat-layers` |
| `async_streaming` | Tight build budget, continuous focus movement, and a secondary explicit focus point | `cargo run -p saddle-world-terrain-example-async-streaming` |
| `physics_colliders` | Backend-agnostic collider payload generation and collider debug bounds | `cargo run -p saddle-world-terrain-example-physics-colliders` |

The richer standalone validation app lives in [`examples/lab`](examples/lab/README.md).

## Limitations And Non-Goals

- v1 keeps chunk world size fixed and lowers far-field vertex density instead of shipping a full clipmap mesh reuse path.
- Crack prevention currently relies on shared sampling plus skirts, not geomorphing.
- Collider generation is backend-agnostic payload data, not a built-in physics integration.
- Debug chunk state is exposed through gizmo colors, named entities, and BRP-friendly diagnostics; the crate does not ship a font-dependent in-world text system.
- Runtime terrain editing and hole masks are intentionally deferred; see [`docs/architecture.md`](docs/architecture.md).
