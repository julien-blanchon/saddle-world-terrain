# Terrain Lab

Standalone validation app for the shared `saddle-world-terrain` crate.

## Purpose

- verify focus-driven chunk streaming in a real Bevy scene
- expose LOD state, pending builds, collider payload counts, and probe samples through an on-screen overlay
- support BRP screenshots and chunk inspection through named entities
- keep targeted E2E scenarios for smoke coverage, LOD transitions, material layering, and collider streaming

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

## E2E

```bash
cargo run -p saddle-world-terrain-lab --features e2e -- terrain_smoke
cargo run -p saddle-world-terrain-lab --features e2e -- terrain_lod_transition
cargo run -p saddle-world-terrain-lab --features e2e -- terrain_material_layers
cargo run -p saddle-world-terrain-lab --features e2e -- terrain_collider_walk
```

## BRP

```bash
uv run --project .codex/skills/bevy-brp/script brp app launch saddle-world-terrain-lab
uv run --project .codex/skills/bevy-brp/script brp world query bevy_ecs::name::Name
uv run --project .codex/skills/bevy-brp/script brp extras screenshot /tmp/terrain_lab.png
uv run --project .codex/skills/bevy-brp/script brp extras shutdown
```
