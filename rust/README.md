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

The playable target is the Bevy shell. It reads one definitive compiled pack
and enters the title screen by default:

```sh
cargo run -p crystal-bevy -- \
  --pack /path/to/core-modular.crystalpack \
  --save-path /tmp/pokecrystal.crystalsave
```

Existing saves can be loaded directly:

```sh
cargo run -p crystal-bevy -- \
  --pack /path/to/core-modular.crystalpack \
  --load-save /tmp/pokecrystal.crystalsave \
  --save-path /tmp/pokecrystal.crystalsave
```

When `--pack` is omitted, `core-modular.crystalpack` must be beside the
executable. When `--save-path` is omitted, saves are written under `saves/`
beside the pack. The release executable exposes no spawn, smoke, script,
inventory, battle, or other state-mutation command line.

Keyboard controls are arrows for the D-pad, `Z` for A, `X` for B, `Enter` for
Start, and Right Shift for Select.

## Verification

Verify the pinned ASM checkout and reference ROM before exporting:

```sh
npm run verify:asm
npm run verify:pack
npm run asm:boot
```

From the repository root, rebuild the definitive core pack:

```sh
npm run export:core
```

Run the Rust compile and test gates from this directory:

```sh
cargo test --workspace --no-run
cargo test -p crystal-bevy --bin crystal-bevy tests:: --features bevy-shell
```

Runtime diagnostics and scenario construction belong in library integration
tests or dedicated test targets, not in the production game executable.
