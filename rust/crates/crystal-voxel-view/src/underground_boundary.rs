//! Shared side-wall volumes in Crystal's Underground atlas.
//!
//! Blocks `$0c/$0e` place a repeated 16-pixel-wide dark rail beside ordinary
//! `$10` floor. The rail is the horizontal cap of a boundary wall, not four
//! rows of flat floor and not four independent upright cards. This rule is
//! scoped to maps that use the drawing as their room boundary; Rocket Base
//! owns a separate maze-wall network for the same atlas cells.

use crystal_render_api::VisualTileSource;

use crate::profile::{CellShape, SolidKind};

const WALL_HEIGHT: f32 = 16.0;

fn uses_shared_boundary(map_id: &str) -> bool {
    matches!(
        map_id,
        "GoldenrodDeptStoreB1F"
            | "GoldenrodUndergroundWarehouse"
            | "OlivinePortPassage"
            | "SaffronGym"
            | "UndergroundPath"
            | "VermilionPortPassage"
    )
}

pub(crate) fn shape(map_id: &str, source: &VisualTileSource) -> Option<CellShape> {
    if !uses_shared_boundary(map_id) || source.tileset_id.as_ref() != "underground" {
        return None;
    }
    let boundary_cell = match source.metatile_id {
        0x0c => source.subtile_column < 2,
        0x0e => source.subtile_column >= 2,
        _ => false,
    };
    boundary_cell.then_some(CellShape::RaisedTop {
        height: WALL_HEIGHT,
        solid: SolidKind::Bank,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn source(metatile: u16, column: u8, row: u8, tile: u16) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("underground"),
            metatile_id: metatile,
            subtile_column: column,
            subtile_row: row,
            tile_index: tile,
        }
    }

    #[test]
    fn exact_boundary_halves_raise_as_one_wall_course() {
        for map in [
            "GoldenrodDeptStoreB1F",
            "GoldenrodUndergroundWarehouse",
            "OlivinePortPassage",
            "SaffronGym",
            "UndergroundPath",
            "VermilionPortPassage",
        ] {
            for row in 0..4 {
                for column in 0..2 {
                    assert_eq!(
                        shape(map, &source(0x0c, column, row, 0x0c + u16::from(column))),
                        Some(CellShape::RaisedTop {
                            height: WALL_HEIGHT,
                            solid: SolidKind::Bank,
                        })
                    );
                }
                for column in 2..4 {
                    assert_eq!(
                        shape(map, &source(0x0e, column, row, 0x0a + u16::from(column))),
                        Some(CellShape::RaisedTop {
                            height: WALL_HEIGHT,
                            solid: SolidKind::Bank,
                        })
                    );
                }
            }
        }
    }

    #[test]
    fn adjacent_floor_and_rocket_maze_are_not_claimed() {
        assert_eq!(shape("UndergroundPath", &source(0x0c, 2, 0, 0x10)), None);
        assert_eq!(shape("UndergroundPath", &source(0x0e, 1, 0, 0x10)), None);
        assert_eq!(shape("TeamRocketBaseB1F", &source(0x0c, 0, 0, 0x0c)), None);
        let mut wrong_tileset = source(0x0c, 0, 0, 0x0c);
        wrong_tileset.tileset_id = Arc::from("cave");
        assert_eq!(shape("UndergroundPath", &wrong_tileset), None);
    }
}
