//! Continuous authored north wall for Crystal's shared ordinary houses.

use super::*;

const STANDARD_COURSE: &[u16] = &[0x04, 0x1e, 0x05, 0x1d];
const DAY_CARE_COURSE: &[u16] = &[0x04, 0x0e, 0x05, 0x1c, 0x0a];
const FAN_CLUB_COURSE: &[u16] = &[0x04, 0x03, 0x05, 0x29, 0x29];
const SOUL_HOUSE_COURSE: &[u16] = &[0x05, 0x03, 0x05, 0x03, 0x05];
const MR_FUJI_COURSE: &[u16] = &[0x04, 0x05, 0x05, 0x1d, 0x1a];
const TRAINER_HOUSE_COURSE: &[u16] = &[0x05, 0x1c, 0x29, 0x29, 0x05];
const FLOWER_SHOP_COURSE: &[u16] = &[0x0e, 0x32, 0x16, 0x32];
const COURSE_VARIANTS: [&[u16]; 7] = [
    STANDARD_COURSE,
    DAY_CARE_COURSE,
    FAN_CLUB_COURSE,
    SOUL_HOUSE_COURSE,
    MR_FUJI_COURSE,
    TRAINER_HOUSE_COURSE,
    FLOWER_SHOP_COURSE,
];
const COURSE_HEIGHT: usize = 4;

pub(super) fn append_north_wall_courses(
    mesh: &mut TerrainMeshData,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    claimed: &mut [bool],
) -> Result<(), TerrainMeshError> {
    for (column, row, width) in course_origins(cells, geometry) {
        append_course(mesh, cells, geometry, claimed, column, row, width)?;
    }
    Ok(())
}

pub(super) fn course_origins(
    cells: &[&VisualTile],
    geometry: &GridGeometry,
) -> Vec<(usize, usize, usize)> {
    let mut origins = Vec::new();
    for row in 0..geometry.height.saturating_sub(COURSE_HEIGHT - 1) {
        for column in 0..geometry.width {
            let matched_width = COURSE_VARIANTS.iter().find_map(|variant| {
                let width = variant.len() * 4;
                if column + width > geometry.width {
                    return None;
                }
                variant
                    .iter()
                    .enumerate()
                    .all(|(block, expected)| {
                        (0..4).all(|local_row| {
                            (0..4).all(|local_column| {
                                let tile = cells[(row + local_row) * geometry.width
                                    + column
                                    + block * 4
                                    + local_column];
                                tile.source.tileset_id.as_ref() == "house"
                                    && tile.source.metatile_id == *expected
                                    && usize::from(tile.source.subtile_column) == local_column
                                    && usize::from(tile.source.subtile_row) == local_row
                            })
                        })
                    })
                    .then_some(width)
            });
            if let Some(width) = matched_width {
                origins.push((column, row, width));
            }
        }
    }
    origins
}

fn append_course(
    mesh: &mut TerrainMeshData,
    cells: &[&VisualTile],
    geometry: &GridGeometry,
    claimed: &mut [bool],
    column: usize,
    row: usize,
    width: usize,
) -> Result<(), TerrainMeshError> {
    let ground_index = cells
        .iter()
        .position(|tile| {
            tile.source.tileset_id.as_ref() == "house"
                && tile.source.tile_index == crate::house::HOUSE_FLOOR_TILE
        })
        .ok_or(TerrainMeshError::MissingGroundSample {
            column: column as u32,
            row: row as u32,
            tile_index: crate::house::HOUSE_FLOOR_TILE,
        })?;
    let ground_uv = geometry.uv(ground_index % geometry.width, ground_index / geometry.width);
    let plane_z = geometry.origin_z + (row + COURSE_HEIGHT) as f32 * geometry.tile_height;
    for local_row in 0..COURSE_HEIGHT {
        let band_bottom = (COURSE_HEIGHT - local_row - 1) as f32 * geometry.tile_height;
        let band_top = band_bottom + geometry.tile_height;
        for local_column in 0..width {
            let source_column = column + local_column;
            let source_row = row + local_row;
            let index = source_row * geometry.width + source_column;
            claimed[index] = true;
            let (x0, x1, z0, z1) = geometry.bounds(source_column, source_row);
            append_top(&mut mesh.textured, [x0, x1, z0, z1], 0.0, ground_uv);
            let (u0, u1, v0, v1) = geometry.uv(source_column, source_row);
            append_quad(
                &mut mesh.textured,
                [
                    [x1, band_bottom, plane_z],
                    [x1, band_top, plane_z],
                    [x0, band_top, plane_z],
                    [x0, band_bottom, plane_z],
                ],
                [0.0, 0.0, 1.0],
                [[u1, v1], [u1, v0], [u0, v0], [u0, v1]],
                TEXTURED_SHADE,
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_house_courses_cover_standard_and_extended_rooms() {
        assert_eq!(STANDARD_COURSE.len() * 4, 16);
        assert_eq!(DAY_CARE_COURSE.len() * 4, 20);
        assert_eq!(FAN_CLUB_COURSE.len() * 4, 20);
        assert_eq!(COURSE_VARIANTS.len(), 7);
        assert_eq!(COURSE_HEIGHT, 4);
    }
}
