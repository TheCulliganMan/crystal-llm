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
  complete visual revision; stale jobs are replaced while the last complete
  2.5D terrain remains active;
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
- renderer readiness never changes presentation mode: startup configuration
  and the manual `F3` toggle are the only 2D/2.5D switches, and an unavailable
  2.5D frame cannot reveal the classic overworld;
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
Celadon Department Store is claimed from its native roof cap through both
rounded window courses and its entrance course as one exterior landmark. Only
the real cap is top-facing, so its roof cannot detach and lie flat behind the
six-storey facade. Its twelve upright source rows remain at full native
height, while generated sidewalls carry only the authored outer frame—not
the broad window fields—so windows cannot wrap around the upper corners.
The cave tileset's exact two-row rock drawings likewise fold into masked
upright props. Its authored `$2d/$2e/$2f` ceiling-rock family forms one
connected 16-pixel bank: the edge blocks retain their native boundary art,
the repeated interior suppresses internal faces, and only the true perimeter
is exposed. Other ambiguous contours remain flat pending an authored
connected profile; cave collision is never treated as visual height.
The closed `$31` cave shelf similarly keeps one topology: its north rim,
side courses, interior cap, and final `$25/$26/$27` south lip all meet the
same six-pixel shelf datum instead of leaving the bottom edge flat and open.
The matching `$38/$39/$3a` hop-down family uses that same single native lip:
left, repeating middle, and right blocks join one six-pixel shelf edge rather
than becoming disconnected flat strips or full rock-height walls.
The distinct barred shelf in `$13/$37` keeps its complete `$0e/$0f` over
`$1e/$1f` drawing as two native upright rock courses with the contextual
floor or ceiling cap. It is not segmented into four props, and the separate
loose-boulder drawings remain flat 2.5D cards rather than voxel blocks.
Loose cave-rock cards also remain on the actual ground datum. They no longer
inherit the six-pixel lit-shelf elevation merely because the same drawing is
used nearby; mixed block `$34` therefore stays two independent rocks above a
separate pale course instead of becoming a floating or merged plateau.
Mixed block `$35` likewise keeps its objects separate: the lower southeast
quadrant reuses the exact authored diagonal-corner prism from `$10`, while
the loose boulder above it remains its existing flat 2.5D card.
Blocks `$12/$30` are treated as one mirrored transition vocabulary rather
than gray decoration: one lower quadrant is the 16-pixel rock cap and the
opposite `$36/$37` pair supplies its two native east/west wall courses. The
same edge strips in `$36` retain that block's separate south-shelf role.
Cave-water corners `$3d/$3e` remain wholly on the lower cave-water datum.
Their reused `$15/$17` boundary pixels no longer inherit the lit shelf's
six-pixel height, preventing false ledges beside pools and waterfalls.
The lateral hop-edge pair `$3b/$3c` keeps the opposite topology: twelve
interior cells form the six-pixel shelf, while the four repeated outside
edge cells fold onto one continuous one-course west/east wall. They never
stack into four artificial elevation levels.
Compact interior fixtures use the same conservative source-art approach:
authored PCs, TVs, radios, bookcases, counters, shelves, displays, and
domestic furniture receive only shallow pixel relief, while unprofiled room
art remains flat. Shared house bookcases preserve their authored ten-pixel
lid/front-rim fold; small shelf and cupboard fields enclosed by the darkest
frame sit one source pixel behind that frame, matching the reference mod's
cabinet relief without inventing geometry for plain Crystal furniture.
Complete wall-appliance drawings keep their exact live front pixels and gain
only a one-source-pixel shallow casing; generic trees and other flat cards do
not inherit that thickness.
Interior depth ends at authored wall courses. The optional renderer does not
insert a solid full-width back-wall quad, avoiding gray horizontal bars that
have no counterpart in the Game Boy tilemap.
The active Oak and Elm laboratory maps also separate their exact framed
two-course north panels and complete four-course equipment bank from the
floor/table vocabulary. Each native row folds once onto a shared upright
plane; ordinary lab floor, chairs, and shallow worktables retain their
existing presentation roles.
Trainer House open books are reconstructed as their complete cross-metatile
two-by-two page drawing and remain paper-thin on the authored tabletop. Their
light boundary-touching pages use whole-drawing background separation rather
than a dark-outline-only mask, which would discard most of the book.
The two traditional gift shops keep each authored merchandise bank as one
four-by-four shelf: two source rows form its top, two form its front, and only
small display fields enclosed by the darkest frame recess one source pixel.
Goldenrod and Celadon's shared department-store interiors reuse one authored
fixture vocabulary across both stores. Their elevator cabins fold only the
north pair of complete wall blocks into one 16-pixel face; the exact door mat,
walkable cabin floor, and elevator mechanics remain flat and authoritative.
The shared 5F blocked fixture remains one complete half-cell-high top-view
surface instead of becoming either a hollow counter or a four-row wall.
Goldenrod's roof keeps its repeated south parapet as two native upright bands,
while each complete terrace display above it is an independent shallow
top-facing fixture; neither profile claims the surrounding roof floor.
The four 6F machine banks reuse the existing complete two-by-four rack/card
geometry through their exact `$0b` and `$20` metatile-half variants, so the
native machine drawing stands once without fusing neighboring banks or
raising the shop floor.
The shared Underground `$0c/$0e` boundary blocks now raise only their exact
two-column dark rail halves as one 16-pixel wall course across the department
store basement, warehouse, port passages, Saffron Gym, and Underground Path.
Their adjacent `$10` floor halves remain flat, and Team Rocket Base retains
its separate authored maze-wall network for the reused atlas cells.
The remaining regular-floor variants share those same systems: 2F's final
two-by-four rack is reconstructed across its `$2c/$2d` metatile boundary, and
the exact blocked counter-end quadrants on 1F/3F remain eight-pixel top-facing
surfaces. The full-game coverage audit reports zero suspicious-flat cells for
both stores' floors 1F through 6F and both elevator cabins.
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
