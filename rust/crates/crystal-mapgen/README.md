# Crystal coordinate map generator

`crystal-mapgen` is a separate Rust pipeline that turns any latitude/longitude
into a deterministic, playable Crystal modpack. It fetches a square from
OpenStreetMap, compresses water, parks, buildings, roads, trails, and rail into
authored Pokemon Crystal rooms, adds a verified runtime map/spawn to the base
pack, and emits both the `.crystalpack` and a QA preview. The generator uses
exact Crystal metatiles and a derived `johto_modern_generated` tileset; its
outdoor benches, National Park fountain, and interactive trash-can art are
extracted from canonical Park and Lab assets rather than approximated sprites.

Every generated town includes real-door residential facades, a functional
Pokemon Center, a functional Mart with the canonical Violet inventory, readable
OSM-derived signs, wandering residents, compact encounter fields, tree belts,
flowers, fences, rocks, relief, and collision-correct shore access when water is
present. Mapped roads are addressed on a global metatile lattice before the
requested window is cropped, so overlapping half-mile moves retain the same
east-west and north-south corridors instead of re-snapping them around the new
center.

From `rust/`:

```sh
cargo run -p crystal-mapgen -- \
  --lat 44.9475196 \
  --lon -93.3253477 \
  --miles 1 \
  --output-dir output/minneapolis-map \
  --base-pack ../content-packs/core-modular.crystalpack
```

The output contains `neighborhood.crystalpack`, `preview.png` rendered from the
exact Crystal tiles embedded in that pack, the normalized
`source.json`, exact generated `grid.json`, automated quality checks in
`audit.json`, and launch metadata in
`modpack.json`. New Game starts at the requested coordinate. For repeatable
offline regeneration, pass the emitted normalized source back with
`--source output/minneapolis-map/source.json`.

Render or play the generated map with the existing location tester:

```sh
cargo run -p crystal-bevy --example render_at_location \
  --features location-tester -- \
  --pack output/minneapolis-map/neighborhood.crystalpack \
  --map GeneratedNeighborhood \
  --x 65 --y 65 --view 2d --hour 12 \
  --screenshot output/minneapolis-map/gameplay/home-day.png
```

## H3 projection and buckyball proofs

Pass a coordinate and any H3 resolution to generate the exact owning cell in a
cell-centered tangent projection (including across UTM zones, the antimeridian,
and polar latitudes). Lower H3 resolution numbers produce larger geographic
faces; the 96-block raster below gives a resolution-6 face enough internal
detail for a dense city, wild rooms, lakes, and broad climbable cliff systems:

```sh
cargo run -p crystal-mapgen -- \
  --lat 44.9475196 --lon -93.3253477 \
  --h3-res 6 --grid 96 \
  --output-dir output/minneapolis-h3 \
  --base-pack ../content-packs/core-modular.crystalpack
```

Plan the first 5,000 connected cells without fetching or rendering them:

```sh
cargo run -p crystal-mapgen -- \
  --lat 44.9475196 --lon -93.3253477 \
  --h3-res 6 --h3-plan-cells 5000 --grid 96 \
  --output-dir output/minneapolis-h3-5k
```

This topology-only run writes `h3-manifest.json`, a hard-gated
`h3-topology-audit.json`, and `h3-connections.json`. Every internal edge in the
connection file contains both cell IDs, opposite presentation sides, and the
two exact block-level portal gates. Each successive manifest prefix is connected.

Generate and audit a small real-map neighborhood, then assemble its exact tiles
as an H3 buckyball image:

```sh
cargo run -p crystal-mapgen -- \
  --lat 44.9475196 --lon -93.3253477 \
  --h3-res 6 --h3-generate-cells 7 --h3-render-proof --grid 96 \
  --source output/minneapolis-source.json \
  --base-pack ../content-packs/core-modular.crystalpack \
  --output-dir output/minneapolis-h3-buckyball
```

The proof retains only per-cell grids, audits, seams, exact PNG previews, and
`h3-buckyball.png`; temporary full packs used to render each face are removed.
Only real source geometry may open a transport crossing, and every internal
crossing is audited from both cells. Atomic houses, facilities, fields, ledges,
and cliffs must fit wholly inside their owning face, so a stitched view cannot
contain half structures.
