//! Elevation topology for Johto's self-contained 4x4 rock platforms.

use crystal_render_api::VisualTile;

use crate::profile::{CellShape, MOUNTAIN_LEDGE_HEIGHT, SolidKind};

fn is_rock_platform(tile: &VisualTile) -> bool {
    matches!(tile.source.tileset_id.as_ref(), "johto" | "johto_modern")
        && tile.source.metatile_id == 0x0a
}

pub(crate) fn resolve_rock_platform_tiers(
    cells: &[&VisualTile],
    shapes: &mut [CellShape],
    width: usize,
) {
    if cells.len() != shapes.len() || width == 0 {
        return;
    }
    // Repetition in the block map tiles one drawing across a rocky field; it
    // is not elevation syntax. Adjacent north/south copies therefore remain
    // at one universal course instead of escalating into a staircase wall.
    for (tile, shape) in cells.iter().zip(shapes.iter_mut()) {
        if !is_rock_platform(tile) {
            continue;
        }
        *shape = CellShape::RaisedTop {
            height: MOUNTAIN_LEDGE_HEIGHT,
            solid: SolidKind::Bank,
        };
    }
}
