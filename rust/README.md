# PokeCrystal Rust Port

This workspace is the Rust game port. It intentionally excludes the existing
MCP, web UI, CLI agent tooling, and TypeScript app surfaces.

## Crates

- `crystal-core`: deterministic game state, timing, input, battle/world rules,
  save state, and multiplayer-ready simulation boundaries.
- `crystal-assets`: loaders for ASM-derived data exported from
  `vendor/pokecrystal` and the existing TypeScript asset pipeline.
- `crystal-audio`: music, SFX, cries, and audio command playback data.
- `crystal-net`: transport-neutral multiplayer protocol types.
- `crystal-bevy`: desktop game shell for rendering, input, and audio.

The port should move file by file from the game runtime surfaces under
`packages/core/src`, `packages/assets/src`, `packages/exporters/src`, and
`vendor/pokecrystal`. Do not port MCP, web routes, agent workflows, or desktop
packaging.

## Play

The playable target is the Bevy shell. It reads one definitive compiled pack and
starts either a new game from a spawn id or an existing Rust save:

```sh
cargo run -p crystal-bevy -- \
  --repo /path/to/crystal-llm \
  --pack apps/web/assets/data/content-packs/core-modular.crystalpack \
  --spawn <spawn-id> \
  --save-path /tmp/pokecrystal.crystalsave
```

```sh
cargo run -p crystal-bevy -- \
  --repo /path/to/crystal-llm \
  --pack apps/web/assets/data/content-packs/core-modular.crystalpack \
  --load-save /tmp/pokecrystal.crystalsave \
  --save-path /tmp/pokecrystal.crystalsave
```

Use `--list-spawns` with the same `--pack` to print compiled spawn ids. The
launcher has no stdin command shell and no web, MCP, agent, or Electron surface.

## Verification

Run the Rust compile gate from this directory:

```sh
cargo test --workspace --no-run
```

From the repository root, rebuild the definitive core pack:

```sh
npm run export:core
```

Then smoke the compiled pack through Bevy without opening a long-running shell:

```sh
cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack apps/web/assets/data/content-packs/core-modular.crystalpack \
  --list-spawns

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack apps/web/assets/data/content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-save /tmp/core-modular-smoke.crystalsave \
  --smoke-script 'right;down;left;up'

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack apps/web/assets/data/content-packs/core-modular.crystalpack \
  --smoke-load-save /tmp/core-modular-smoke.crystalsave \
  --save-path /tmp/core-modular-smoke-roundtrip.crystalsave
```
