# Crystal Voxel View

`crystal-voxel-view` is an optional, presentation-only Bevy plugin for the
Rust port. It turns the existing 20x18 overworld viewport into a tilted 2.5D
scene while leaving `crystal-core` as the sole gameplay authority.

The renderer has a safe flat baseline for every map, with its first authored
relief profile covering New Bark's Johto artwork:

- stable tileset/metatile/subtile identity selects clean-room visual shapes;
- unknown artwork stays flat; no collision class is ever promoted to height;
- the live composed map supplies one native cell to each top or facade band;
- tree and sign cards compare their live 8x8 art with an authored ground tile
  and emit only differing pixel runs, avoiding opaque background rectangles;
- generated caps and sides use a separate solid-color material, so map art is
  never stretched down a wall;
- terrain meshing runs on Bevy's asynchronous compute pool and is keyed by the
  complete visual revision; stale jobs are replaced while the faithful 2D
  world remains available until authored geometry is ready;
- player and NPC textures are vertical cards anchored at their feet;
- tall-grass rustle is exported as a foot-anchored world card, so it follows
  the pitched scene instead of forcing a switch back to screen-space 2D;
- a dedicated reverse-depth material reveals a translucent player silhouette
  only through closer authored geometry, leaving visible player pixels intact;
- the optional 3D camera uses a focal-length-matched 45-degree perspective
  diorama angle and casts directional terrain/actor shadows; the classic
  camera then composites the exact 2D UI, dialog, and fades above it;
- the result is rendered below the existing 2D UI, dialog, fades, and battle
  compositor;
- the normal launch remains the unchanged 2D presentation even when the
  optional code is compiled; the tester or an explicit setting must enable
  2.5D, and disabling the Cargo feature removes it entirely.

The renderer cannot mutate movement, collision, scripts, RNG, saves, replay
journals, or checksums. It consumes only `crystal-render-api` snapshots.

## Clean-room scope

The architecture was informed by observing Gen1Recomp's public rendering
extension points and the behavior of Dramatic Shape Voxel Mod. No code,
profiles, models, textures, or other assets from Dramatic Shape are included;
the inspected project did not provide a project-wide reuse license. All
geometry and Bevy integration here are independently implemented for Crystal.

The initial clean-room profile covers New Bark's compact and large buildings,
trees, signs, and shallow water. Exact animated water cells are also recessed
in the modern Johto, Kanto, and cave tilesets without lowering shoreline art;
Kanto's exact two-cell tree bands form masked upright cards whenever their
authored background cell is present in the visual snapshot. A clipped profile
with missing art evidence stays flat instead of guessing or disabling 2.5D.
The cave tileset's exact two-row rock drawings likewise fold into masked
upright props while ambiguous wall contours remain flat pending a connected
profile; cave collision is never treated as visual height.
Building facade rows fold onto one shared front plane while their vacated cells
remain walkable ground. Profile metadata is mod-owned presentation data; it is
neither inferred from collision nor fed back into movement, scripts, saves,
replay state, or checksums.

## Render-at-location tester

The developer-only tester boots the real compiled runtime at any gameplay tile
and can start in either presentation. Press `F3` to toggle 2D/2.5D without
changing location:

```sh
cargo run -p crystal-bevy --example render_at_location \
  --features location-tester -- \
  --pack /path/to/game.crystalpack --list-maps

cargo run -p crystal-bevy --example render_at_location \
  --features location-tester -- \
  --pack /path/to/game.crystalpack \
  --map NewBarkTown --x 6 --y 8 --view 2.5d \
  --screenshot /tmp/new-bark-2.5d.png

# Render the identical location in both modes. This writes
# /tmp/new-bark-2d.png and /tmp/new-bark-2.5d.png.
cargo run -p crystal-bevy --example render_at_location \
  --features location-tester -- \
  --pack /path/to/game.crystalpack \
  --map NewBarkTown --x 6 --y 8 --view both \
  --screenshot /tmp/new-bark.png

# Batch-audit selected map centers in both presentations. Each map receives
# independently verified 2D and 2.5D screenshots in the output directory.
cargo run -p crystal-bevy --example render_at_location \
  --features location-tester -- \
  --pack /path/to/game.crystalpack \
  --maps NewBarkTown,UnionCaveB1F,CeladonCity --view both \
  --output-dir /tmp/crystal-render-audit

# Use --all-maps instead of --maps to audit every dimensioned runtime map.
```
