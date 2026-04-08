# Debugging

## Built-In Debug Surface

Use `TerrainDebugConfig` to expose the runtime state without touching internals.

- `show_chunk_bounds`: draw chunk boxes in LOD colors
- `show_focus_rings`: draw visual and collider radii around each active focus entity and each explicit `TerrainFocusPoints` entry
- `show_collider_bounds`: highlight chunks that currently carry collider payloads even when chunk bounds are otherwise hidden
- `show_sample_probes`: draw terrain probe hit points and normals
- `color_mode`: switch between natural shading, LOD colors, chunk-state colors, dominant-layer colors, and slope-band colors

## BRP Workflow

The crate-local lab enables BRP through `bevy_brp_extras` in the `dev` feature.

```bash
uv run --project .codex/skills/bevy-brp/script brp app launch saddle-world-terrain-lab
uv run --project .codex/skills/bevy-brp/script brp world query bevy_ecs::name::Name
uv run --project .codex/skills/bevy-brp/script brp extras screenshot /tmp/saddle-world-terrain-lab.png
uv run --project .codex/skills/bevy-brp/script brp extras shutdown
```

Useful targets to inspect:

- the `Terrain Root` entity
- chunk entities named like `Terrain Chunk (x, y) LOD n`
- `terrain::debug::TerrainDiagnostics`
- `terrain::components::TerrainProbeSample`
- `terrain::components::TerrainFocusPoints`

## What To Look For

### Missing or duplicated chunks

- enable `show_chunk_bounds`
- move the focus slowly across a chunk boundary
- verify that despawned chunks disappear cleanly and no duplicate coordinates remain

### LOD instability

- set `color_mode = ByLod`
- enable `show_focus_rings`
- idle near a threshold and watch for repeated LOD churn
- if churn appears, raise `TerrainLodConfig::hysteresis`

### Material seams

- use `Natural` or `ByLayerDominance`
- inspect chunk borders where a strong weight-map gradient crosses a chunk edge
- if seams appear, check that both chunks sample the same source data and material profile

### Collider lag

- enable `show_collider_bounds`
- keep `collider_radius` noticeably smaller than `visual_radius`
- verify that collider chunks follow the focus but do not stay attached too far away

### Probe validation

- add an entity with `TerrainProbe`
- enable `show_sample_probes`
- compare the probe hit point with the visible surface and the normal arrow orientation
- remember that `TerrainProbeSample::height` is the world-space surface Y value, while `world_position` carries the full hit position

## Common Failure Modes

| Symptom | Likely cause | Fix |
| --- | --- | --- |
| Chunks stay coarse near the focus | `near_distance` too small or focus not attached to the terrain | increase `near_distance` or set `TerrainFocus::terrain` |
| Visible cracks at LOD boundaries | `skirt_depth` too small | raise `skirt_depth` |
| Heavy rebuild spikes | `vertex_resolution` too high or `max_builds_per_frame` too large | lower the build budget and rely on the cache |
| Collider payloads everywhere | `collider_radius` too close to `visual_radius` | shrink the collider radius |
| Probes return no hit | probe point is outside `TerrainConfig::size` after root transform inversion | verify root transform and local extent |

## E2E Checks

Each shipped example has a dedicated scenario in the lab harness:

- `basic` → `example_basic_smoke`
- `clipmap_debug` → `example_clipmap_debug`
- `splat_layers` → `example_splat_layers`
- `async_streaming` → `example_async_streaming`
- `physics_colliders` → `example_physics_colliders`
- `island` → `example_island`
- `mountain_range` → `example_mountain_range`
- `terrain_sculpting` → `example_terrain_sculpting`

Those example-backed scenarios also assert that the shared `saddle-pane` controls and stats
resources are present so the example debug surface stays in lockstep with the shipped scenes.

The lab scenarios currently cover:

- `example_basic_smoke`
- `example_clipmap_debug`
- `example_splat_layers`
- `example_async_streaming`
- `example_physics_colliders`
- `example_island`
- `example_mountain_range`
- `example_terrain_sculpting`
- `terrain_smoke`
- `terrain_lod_transition`
- `terrain_material_layers`
- `terrain_collider_walk`
- `terrain_probe_sample`
- `terrain_debug_modes`
- `terrain_async_throttle`
- `terrain_slope_band`
- `terrain_chunk_lifecycle`

Each scenario includes runtime assertions in addition to screenshots and logs.
