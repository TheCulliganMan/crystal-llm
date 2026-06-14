# Godot Client

Native Godot client scaffold for the Pokemon Crystal LLM toolkit.

## Current Slice

- Fixed-step runtime shell
- Per-frame input latching
- Atomic local save/load scaffolding
- Repo-aware asset discovery for exported JSON, PNG, and MP3 outputs
- GB tile/palette decoder utilities for future parity work

## Open In Godot

Open `apps/godot/project.godot` in Godot 4.6 or newer, then run `scenes/main.tscn`.

## Data Sources

The client reads exported runtime data from `apps/web/assets/` and treats the TypeScript runtime as the oracle during migration.

## Notes

- Generated Godot editor state lives under `apps/godot/.godot/` and `apps/godot/.import/`.
- This is not the full game yet.
- The current scene is a debugging shell that proves the runtime boundary.
