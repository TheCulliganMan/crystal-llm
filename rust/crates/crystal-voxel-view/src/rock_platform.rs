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
    // is not elevation syntax. Adjacent copies therefore remain at one
    // universal course instead of escalating into a staircase wall.
    for (tile, shape) in cells.iter().zip(shapes.iter_mut()) {
        if !is_rock_platform(tile) {
            continue;
        }
        *shape = CellShape::RaisedTop {
            height: MOUNTAIN_LEDGE_HEIGHT,
            solid: SolidKind::Bank,
        };
    }

    // Ice Path `$19` is one unique 4x4 rock-mass drawing. Promote it only
    // when all sixteen cells are present; a group clipped by the source halo
    // stays faithful flat art instead of turning into a partial crate.
    let height = cells.len() / width;
    for row in 0..height {
        for column in 0..width {
            let index = row * width + column;
            if crate::ice_path::rock_mass_local(&cells[index].source) != Some((0, 0))
                || column + 4 > width
                || row + 4 > height
            {
                continue;
            }
            let complete = (0..4).all(|local_row| {
                (0..4).all(|local_column| {
                    crate::ice_path::rock_mass_local(
                        &cells[(row + local_row) * width + column + local_column].source,
                    ) == Some((local_column as u8, local_row as u8))
                })
            });
            if !complete {
                continue;
            }
            for local_row in 0..4 {
                for local_column in 0..4 {
                    shapes[(row + local_row) * width + column + local_column] =
                        CellShape::RaisedTop {
                            height: MOUNTAIN_LEDGE_HEIGHT,
                            solid: SolidKind::Bank,
                        };
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crystal_render_api::{VisualTile, VisualTileSource};

    use super::*;

    fn tile(tileset: &str, metatile_id: u16) -> VisualTile {
        VisualTile {
            column: 0,
            row: 0,
            source: VisualTileSource {
                tileset_id: Arc::from(tileset),
                metatile_id,
                subtile_column: 0,
                subtile_row: 0,
                tile_index: 0,
            },
            texture: Default::default(),
            priority: false,
        }
    }

    #[test]
    fn ice_path_mass_is_one_universal_platform_course() {
        let mut tiles = Vec::new();
        for row in 0..4 {
            for column in 0..4 {
                let mut tile = tile("ice_path", 0x19);
                tile.source.subtile_column = column;
                tile.source.subtile_row = row;
                tile.source.tile_index = 0x84 + u16::from(row) * 0x10 + u16::from(column);
                tiles.push(tile);
            }
        }
        let cells = tiles.iter().collect::<Vec<_>>();
        let mut shapes = vec![CellShape::Flat; 16];
        resolve_rock_platform_tiers(&cells, &mut shapes, 4);
        assert_eq!(
            shapes,
            vec![
                CellShape::RaisedTop {
                    height: MOUNTAIN_LEDGE_HEIGHT,
                    solid: SolidKind::Bank,
                };
                16
            ]
        );

        let clipped = cells[..12].to_vec();
        let mut clipped_shapes = vec![CellShape::Flat; 12];
        resolve_rock_platform_tiers(&clipped, &mut clipped_shapes, 4);
        assert_eq!(clipped_shapes, vec![CellShape::Flat; 12]);
    }
}
