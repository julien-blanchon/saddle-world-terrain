# Terrain Lab

Standalone validation app for the shared `saddle-world-terrain` crate.

## Purpose

- verify focus-driven chunk streaming in a real Bevy scene
- expose LOD state, pending builds, collider payload counts, and probe samples through an on-screen overlay
- mirror the shipped example debug surface with `saddle-pane` controls and runtime stats panes
- support BRP screenshots and chunk inspection through named entities
- keep targeted E2E scenarios for each shipped example plus terrain-specific checks for streaming, probes, debug modes, throttled rebuilds, slope bands, and chunk lifecycle behavior

## Status

Working

## Run

```bash
cargo run -p saddle-world-terrain-lab
```

Controls:

- `LMB`: orbit
- `MMB`: pan
- `Mouse wheel`: zoom
- `WASD` / arrow keys: move the terrain focus
- `Shift`: faster focus movement
- `1`: cycle debug color mode
- `2`: toggle chunk bounds
- `3`: toggle collider bounds
- `4`: toggle focus rings
- `5`: toggle focus auto-roam

Pane surface:

- top-right `Terrain Controls`: live tuning for debug flags, streaming radii, LOD, collider settings, and mesh scale
- bottom-right `Terrain Stats`: FPS plus terrain diagnostics mirrored from `TerrainDiagnostics` and `TerrainRootStats`

## E2E

Example-backed scenarios:

```bash
cargo run -p saddle-world-terrain-lab --features e2e -- example_basic_smoke
cargo run -p saddle-world-terrain-lab --features e2e -- example_clipmap_debug
cargo run -p saddle-world-terrain-lab --features e2e -- example_splat_layers
cargo run -p saddle-world-terrain-lab --features e2e -- example_async_streaming
cargo run -p saddle-world-terrain-lab --features e2e -- example_physics_colliders
cargo run -p saddle-world-terrain-lab --features e2e -- example_island
cargo run -p saddle-world-terrain-lab --features e2e -- example_mountain_range
cargo run -p saddle-world-terrain-lab --features e2e -- example_terrain_sculpting
```

Terrain-focused scenarios:

```bash
cargo run -p saddle-world-terrain-lab --features e2e -- terrain_smoke
cargo run -p saddle-world-terrain-lab --features e2e -- terrain_lod_transition
cargo run -p saddle-world-terrain-lab --features e2e -- terrain_material_layers
cargo run -p saddle-world-terrain-lab --features e2e -- terrain_collider_walk
cargo run -p saddle-world-terrain-lab --features e2e -- terrain_probe_sample
cargo run -p saddle-world-terrain-lab --features e2e -- terrain_debug_modes
cargo run -p saddle-world-terrain-lab --features e2e -- terrain_async_throttle
cargo run -p saddle-world-terrain-lab --features e2e -- terrain_slope_band
cargo run -p saddle-world-terrain-lab --features e2e -- terrain_chunk_lifecycle
```

## BRP

```bash
uv run --project .codex/skills/bevy-brp/script brp app launch saddle-world-terrain-lab
uv run --project .codex/skills/bevy-brp/script brp world query bevy_ecs::name::Name
uv run --project .codex/skills/bevy-brp/script brp extras screenshot /tmp/terrain_lab.png
uv run --project .codex/skills/bevy-brp/script brp extras shutdown
```
