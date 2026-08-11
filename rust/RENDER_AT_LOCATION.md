# Render at Location

The `crystal-bevy` location tester renders the faithful 2D compositor and the
optional 2.5D mod at the same runtime map coordinate. It is a developer-only
example; the default game build remains faithful 2D.

Run commands from `rust/` and use an absolute pack path:

```sh
PACK=/absolute/path/to/core-modular.crystalpack
```

## Discover maps

```sh
cargo run -q -p crystal-bevy \
  --example render_at_location \
  --features location-tester -- \
  --pack "$PACK" --list-maps
```

This prints each exact map identifier, dimensions, tileset, and environment.

## Render one location

```sh
cargo run -q -p crystal-bevy \
  --example render_at_location \
  --features location-tester -- \
  --pack "$PACK" \
  --map Route26 \
  --view both \
  --screenshot /tmp/route26.png
```

`--view` accepts `2d`, `2.5d`, or `both`. With `both`, the command writes
`/tmp/route26-2d.png` and `/tmp/route26-2.5d.png` and verifies that both are
visible Bevy frames.

Without coordinates, the tester selects the walkable tile nearest the map
center. Select a specific saved-map tile with `--x` and `--y`:

```sh
cargo run -q -p crystal-bevy \
  --example render_at_location \
  --features location-tester -- \
  --pack "$PACK" \
  --map GoldenrodCity --x 14 --y 8 \
  --view both \
  --screenshot /tmp/goldenrod-gym.png
```

Unknown maps and out-of-bounds coordinates fail explicitly.

## Render a map grid

```sh
mkdir -p /tmp/crystal-map-grid
cargo run -q -p crystal-bevy \
  --example render_at_location \
  --features location-tester -- \
  --pack "$PACK" \
  --maps NewBarkTown,GoldenrodCity,Route26 \
  --view both \
  --output-dir /tmp/crystal-map-grid
```

Each map uses its nearest-center walkable tile. Use `--all-maps` instead of
`--maps ...` for the complete runtime catalog.

## Audit full-game 2.5D coverage

Run the source-identity auditor before choosing visual samples:

```sh
cargo run -q -p crystal-bevy \
  --example audit_voxel_coverage \
  --features location-tester \
  --target-dir /tmp/crystal-voxel-audit-target -- \
  --pack "$PACK" \
  --output /tmp/crystal-voxel-coverage.json
```

The report walks every compiled map and every 8x8 source cell. It classifies
cells using the same complete-building, grouped-tree, and authored per-cell
rules as the optional renderer. Each distinct source identity records its
frequency, all affected maps, and one representative runtime coordinate.
`coverage: "flat"` means the source reaches the renderer's faithful flat
baseline; it is an audit candidate, not automatically a defect. Review those
entries against the 2D render and the reference mod, then add an authored
profile only when the artwork depicts real geometry.

## Avoid artifact locks

When another task is compiling, reuse one dedicated Cargo target directory:

```sh
CARGO_BUILD_JOBS=2 cargo run -q -p crystal-bevy \
  --example render_at_location \
  --features location-tester \
  --target-dir /tmp/crystal-location-render-target -- \
  --pack "$PACK" \
  --map Route26 --view both \
  --screenshot /tmp/route26.png
```

This changes only build artifacts. It does not switch branches, create a Git
worktree, or alter source. Reuse the directory so Bevy stays warm. After one
successful build, invoke the binary directly for quick repeated captures:

```sh
/tmp/crystal-location-render-target/debug/examples/render_at_location \
  --pack "$PACK" \
  --map Route26 --view both \
  --screenshot /tmp/route26.png
```

## Comparison loop

For each object:

1. Render `both` at one coordinate.
2. Treat `-2d.png` as the exact source-art and placement reference.
3. Find the corresponding behavior in the clean-room reference mod.
4. Recreate it only in `crystal-voxel-view`.
5. Rerender and compare texel scale, depth, anchoring, occlusion, edges, and
   shadows.

The location renderer is visual tooling. It must not drive changes to core
collision, movement, scripts, saves, or other gameplay behavior.
