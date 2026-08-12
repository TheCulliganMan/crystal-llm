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
- `crystal-render-api`: read-only presentation snapshots shared with optional
  Bevy render mods.
- `crystal-bevy`: desktop game shell for rendering, input, and audio.
- `crystal-voxel-view`: optional clean-room 2.5D overworld renderer.

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

### Browser build

Build the WASM game with the optional 2.5D renderer, generate its JavaScript
bindings, and serve `rust/web-dist` with the Rust server:

```sh
cd rust
rustup target add wasm32-unknown-unknown
cargo build -p crystal-bevy --target wasm32-unknown-unknown --release --features voxel-view
wasm-bindgen --target web --out-dir web-dist --out-name crystal-bevy \
  target/wasm32-unknown-unknown/release/crystal-bevy.wasm
cargo run -p crystal-web-server -- --dir web-dist --port 8080
```

Open `http://127.0.0.1:8080`, click the game canvas, and press `F3` to switch
between the classic 2D renderer and the optional 2.5D renderer. Browser audio
is not enabled yet. The canonical compiled game pack is embedded in the WASM
binary, and browser saves are disabled until persistent web storage is wired.

### Optional 2.5D overworld mod

The normal build keeps the original 2D Game Boy presentation. To opt into the
experimental renderer mod, build the Bevy shell with the non-default
`voxel-view` feature:

```sh
cargo run -p crystal-bevy --features voxel-view -- \
  --pack /path/to/core-modular.crystalpack \
  --save-path /tmp/pokecrystal.crystalsave
```

The mod consumes a read-only render snapshot. It does not change movement,
collision, scripts, random state, battles, saves, or replay checksums. Menus,
dialog, fades, and battles continue to use the faithful 2D compositor. Its
clean-room shape profile is keyed by stable tileset/metatile artwork identity,
never gameplay collision. The startup setting and `F3` are the only ways to
change between 2D and 2.5D. Unsupported maps, incomplete frames, terrain
builds, and renderer errors report 2.5D as inactive without exposing the 2D
overworld.

For repeatable map screenshots and side-by-side 2D/2.5D inspection, see
[RENDER_AT_LOCATION.md](RENDER_AT_LOCATION.md).

## Verification

Verify the pinned ASM checkout and reference ROM before exporting:

```sh
npm run verify:asm
npm run verify:pack
npm run asm:boot
```

From the repository root, rebuild the definitive core pack:

```sh
./export
```

Run the Rust compile and test gates from this directory:

```sh
cargo test --workspace --no-run
cargo test -p crystal-bevy --bin crystal-bevy tests:: --features bevy-shell
```

Runtime diagnostics and scenario construction belong in library integration
tests or dedicated test targets, not in the production game executable.
