use std::{collections::BTreeMap, path::Path};

use anyhow::{Context, Result, bail, ensure};
use crystal_assets::read_verified_compiled_game_pack;
use image::{Rgba, RgbaImage};

use crate::{Coordinate, H3CellPlan};

const TILE_SIZE: usize = 8;
const METATILE_WIDTH: usize = 4;
const METATILE_TILE_COUNT: usize = METATILE_WIDTH * METATILE_WIDTH;
const METATILE_SIZE: usize = TILE_SIZE * METATILE_WIDTH;

type Palette = [[u8; 3]; 4];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MosaicPlacement {
    left: i64,
    top: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MosaicTransportBridge {
    edge_id: String,
    first_cell: usize,
    first_gate: (u16, u16),
    second_cell: usize,
    second_gate: (u16, u16),
    /// Destination top-lefts for repeated, unscaled copies of the exact
    /// transport metatile. Consecutive blocks overlap or share one cardinal
    /// edge, so a composed preview cannot reveal terrain between the rooms.
    blocks: Vec<(i64, i64)>,
}

/// Render a map with the exact tiles, metatiles, palette map, and block ids
/// embedded in its compiled playable pack.
pub fn render_tile_preview(
    pack_path: impl AsRef<Path>,
    map_name: &str,
    output_path: impl AsRef<Path>,
) -> Result<()> {
    let pack_path = pack_path.as_ref();
    let output_path = output_path.as_ref();
    let pack = read_verified_compiled_game_pack(pack_path)
        .with_context(|| format!("load generated pack {}", pack_path.display()))?;
    let map = pack
        .data()
        .maps
        .get(map_name)
        .with_context(|| format!("compiled pack has no map named {map_name}"))?;
    let tileset_id = &map.attributes.tileset_name;
    let tileset = pack
        .data()
        .tilesets
        .get(tileset_id)
        .with_context(|| format!("compiled pack has no tileset named {tileset_id}"))?;
    let files = pack.runtime_files();
    let metatile_path = format!("data/tilesets/{tileset_id}_metatiles.bin");
    let tiles_path = format!("gfx/tilesets/{tileset_id}.png");
    let metatiles = files
        .get(&metatile_path)
        .with_context(|| format!("compiled pack is missing {metatile_path}"))?;
    ensure!(
        metatiles.len() % METATILE_TILE_COUNT == 0,
        "{metatile_path} has an invalid byte length"
    );
    let source = image::load_from_memory(
        files
            .get(&tiles_path)
            .with_context(|| format!("compiled pack is missing {tiles_path}"))?,
    )
    .with_context(|| format!("decode {tiles_path}"))?
    .to_rgba8();
    ensure!(
        source.width() % TILE_SIZE as u32 == 0 && source.height() % TILE_SIZE as u32 == 0,
        "{tiles_path} is not aligned to 8x8 Game Boy tiles"
    );
    let palettes = load_palette_bank(files, tileset_id)?;
    ensure!(!palettes.is_empty(), "tileset {tileset_id} has no palettes");
    ensure!(
        map.blocks.len() == usize::from(map.attributes.width) * usize::from(map.attributes.height),
        "map {map_name} block dimensions do not match its block data"
    );

    let output_width = usize::from(map.attributes.width) * METATILE_SIZE;
    let output_height = usize::from(map.attributes.height) * METATILE_SIZE;
    let mut output = RgbaImage::new(u32::try_from(output_width)?, u32::try_from(output_height)?);
    let source_width = source.width() as usize;
    let source_tile_count = (source_width / TILE_SIZE) * (source.height() as usize / TILE_SIZE);

    for (block_position, block) in map.blocks.iter().copied().enumerate() {
        let metatile_offset = usize::from(block) * METATILE_TILE_COUNT;
        let tile_ids = metatiles
            .get(metatile_offset..metatile_offset + METATILE_TILE_COUNT)
            .with_context(|| format!("map {map_name} references missing metatile {block:#04x}"))?;
        let block_x = block_position % usize::from(map.attributes.width);
        let block_y = block_position / usize::from(map.attributes.width);
        for (subtile_position, tile_id) in tile_ids.iter().copied().enumerate() {
            let palette_value = tileset
                .palette_map
                .get(usize::from(tile_id))
                .copied()
                .unwrap_or(0);
            let palette = palettes
                .get(usize::from(palette_value & 0x07))
                .unwrap_or(&palettes[0]);
            let source_tile = resolve_tileset_tile_index(
                source_tile_count,
                usize::from(tile_id),
                (palette_value >> 3) & 1,
            );
            blit_tile(
                &source,
                source_width,
                source_tile,
                palette,
                &mut output,
                block_x * METATILE_SIZE + subtile_position % METATILE_WIDTH * TILE_SIZE,
                block_y * METATILE_SIZE + subtile_position / METATILE_WIDTH * TILE_SIZE,
            );
        }
    }
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create preview directory {}", parent.display()))?;
    }
    output
        .save(output_path)
        .with_context(|| format!("write real-tile preview {}", output_path.display()))
}

/// Assemble exact per-cell tile renders into their geographic H3 topology.
/// Rectangular storage outside each H3 polygon is masked away, so the result
/// shows the connected faces rather than seven square screenshots.
pub fn render_h3_mosaic(
    cells: &[(H3CellPlan, std::path::PathBuf)],
    grid_width: u16,
    grid_height: u16,
    output_path: impl AsRef<Path>,
) -> Result<()> {
    ensure!(!cells.is_empty(), "H3 mosaic requires at least one cell");
    let origin = &cells[0].0;
    let images = cells
        .iter()
        .map(|(_, path)| {
            image::open(path)
                .with_context(|| format!("load H3 cell preview {}", path.display()))
                .map(|image| image.to_rgba8())
        })
        .collect::<Result<Vec<_>>>()?;
    let image_width = images[0].width();
    let image_height = images[0].height();
    ensure!(
        images
            .iter()
            .all(|image| image.dimensions() == (image_width, image_height)),
        "H3 cell previews have inconsistent dimensions"
    );
    ensure!(
        image_width % u32::from(grid_width) == 0 && image_height % u32::from(grid_height) == 0,
        "H3 cell previews do not contain an integral rendered block grid"
    );

    let origin_boundary = origin
        .boundary
        .iter()
        .map(|&point| local_mosaic_point(origin.center, point))
        .collect::<Vec<_>>();
    let world_width = origin_boundary
        .iter()
        .map(|point| point.0)
        .fold(f64::NEG_INFINITY, f64::max)
        - origin_boundary
            .iter()
            .map(|point| point.0)
            .fold(f64::INFINITY, f64::min);
    let world_height = origin_boundary
        .iter()
        .map(|point| point.1)
        .fold(f64::NEG_INFINITY, f64::max)
        - origin_boundary
            .iter()
            .map(|point| point.1)
            .fold(f64::INFINITY, f64::min);
    ensure!(
        world_width > 0.0 && world_height > 0.0,
        "H3 mosaic origin has a degenerate boundary"
    );
    let scale_x = f64::from(image_width) / world_width;
    let scale_y = f64::from(image_height) / world_height;
    let centers = cells
        .iter()
        .map(|(plan, _)| {
            let point = local_mosaic_point(origin.center, plan.center);
            (point.0 * scale_x, point.1 * scale_y)
        })
        .collect::<Vec<_>>();
    let half_width = f64::from(image_width) / 2.0;
    let half_height = f64::from(image_height) / 2.0;
    let min_x = centers
        .iter()
        .map(|center| center.0 - half_width)
        .fold(f64::INFINITY, f64::min);
    let max_x = centers
        .iter()
        .map(|center| center.0 + half_width)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = centers
        .iter()
        .map(|center| center.1 - half_height)
        .fold(f64::INFINITY, f64::min);
    let max_y = centers
        .iter()
        .map(|center| center.1 + half_height)
        .fold(f64::NEG_INFINITY, f64::max);
    let padding = 16_i64;
    let canvas_width = (max_x - min_x).ceil() as i64 + padding * 2;
    let canvas_height = (max_y - min_y).ceil() as i64 + padding * 2;
    let mut output = RgbaImage::from_pixel(
        u32::try_from(canvas_width)?,
        u32::try_from(canvas_height)?,
        Rgba([0, 0, 0, 0]),
    );

    let placements = centers
        .iter()
        .map(|center| {
            let block_width = i64::from(image_width / u32::from(grid_width));
            let block_height = i64::from(image_height / u32::from(grid_height));
            let raw_left = (center.0 - half_width - min_x).round() as i64 + padding;
            let raw_top = (center.1 - half_height - min_y).round() as i64 + padding;
            MosaicPlacement {
                left: ((raw_left as f64 / block_width as f64).round() as i64) * block_width,
                top: ((raw_top as f64 / block_height as f64).round() as i64) * block_height,
            }
        })
        .collect::<Vec<_>>();

    for (((plan, _), image), placement) in cells.iter().zip(&images).zip(&placements) {
        // Pixel-center rasterization otherwise leaves a sub-block transparent
        // gutter between two mathematically shared polygon edges. Expand by
        // half a block: reciprocal grid finalization and the three-cell POI
        // clearance make this overlap safe, while each face contributes only
        // half of the former one-block black join.
        let authored_polygon = plan.raster_polygon(grid_width, grid_height)?;
        let polygon = expand_preview_polygon(&authored_polygon, 0.5);
        let block_width = image_width / u32::from(grid_width);
        let block_height = image_height / u32::from(grid_height);
        for grid_y_cell in 0..u32::from(grid_height) {
            let grid_y = f64::from(grid_y_cell) + 0.5;
            for grid_x_cell in 0..u32::from(grid_width) {
                let grid_x = f64::from(grid_x_cell) + 0.5;
                if !point_in_preview_polygon(grid_x, grid_y, &polygon) {
                    continue;
                }
                let (sample_grid_x, sample_grid_y) =
                    if point_in_preview_polygon(grid_x, grid_y, &authored_polygon) {
                        (grid_x_cell, grid_y_cell)
                    } else {
                        let center_x = f64::from(grid_width) / 2.0;
                        let center_y = f64::from(grid_height) / 2.0;
                        let dx = center_x - grid_x;
                        let dy = center_y - grid_y;
                        let length = dx.hypot(dy);
                        let inset_x = grid_x + dx / length * 0.75;
                        let inset_y = grid_y + dy / length * 0.75;
                        (
                            (inset_x.floor() as u32).min(u32::from(grid_width) - 1),
                            (inset_y.floor() as u32).min(u32::from(grid_height) - 1),
                        )
                    };
                for pixel_y in 0..block_height {
                    for pixel_x in 0..block_width {
                        let output_x =
                            placement.left + i64::from(grid_x_cell * block_width + pixel_x);
                        let output_y =
                            placement.top + i64::from(grid_y_cell * block_height + pixel_y);
                        if output_x >= 0
                            && output_y >= 0
                            && output_x < i64::from(output.width())
                            && output_y < i64::from(output.height())
                        {
                            output.put_pixel(
                                output_x as u32,
                                output_y as u32,
                                *image.get_pixel(
                                    sample_grid_x * block_width + pixel_x,
                                    sample_grid_y * block_height + pixel_y,
                                ),
                            );
                        }
                    }
                }
            }
        }
    }

    // Independent H3 rasters quantize the two sides of one geographic edge
    // separately. Their exact runtime gates can consequently be more than one
    // rendered block apart after the north-up face images are translated into
    // the mosaic, even though both three-cell route bands pass their per-room
    // seam audit. Restore those selected joins *after* face composition with
    // unscaled copies of the real transport block already rendered at each
    // endpoint. No color or substitute art is synthesized, and natural/water
    // edges without a reciprocal regional connection are never touched.
    let bridges = plan_mosaic_transport_bridges(
        cells,
        &placements,
        grid_width,
        grid_height,
        image_width,
        image_height,
    )?;
    let block_width = image_width / u32::from(grid_width);
    let block_height = image_height / u32::from(grid_height);
    for bridge in &bridges {
        for (index, &destination) in bridge.blocks.iter().enumerate() {
            let (source_cell, source_gate) = if index + 1 == bridge.blocks.len() {
                (bridge.second_cell, bridge.second_gate)
            } else {
                (bridge.first_cell, bridge.first_gate)
            };
            copy_preview_block(
                &images[source_cell],
                source_gate,
                block_width,
                block_height,
                &mut output,
                destination,
            );
        }
    }
    fill_internal_mosaic_block_gaps(&mut output, block_width, block_height);
    let output_path = output_path.as_ref();
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create H3 mosaic directory {}", parent.display()))?;
    }
    output
        .save(output_path)
        .with_context(|| format!("write H3 mosaic {}", output_path.display()))
}

/// Fill only metatile-sized gaps bracketed by already rendered faces. The
/// source block is selected by a neighborhood vote over complete rendered
/// blocks, so no sprite or shoreline can be cut diagonally at a join.
fn fill_internal_mosaic_block_gaps(output: &mut RgbaImage, block_width: u32, block_height: u32) {
    let columns = output.width() / block_width;
    let rows = output.height() / block_height;
    for _ in 0..4 {
        let snapshot = output.clone();
        let mut fills = Vec::new();
        for y in 1..rows.saturating_sub(1) {
            for x in 1..columns.saturating_sub(1) {
                if block_is_visible(&snapshot, x, y, block_width, block_height) {
                    continue;
                }
                let neighbors = [
                    (x - 1, y),
                    (x + 1, y),
                    (x, y - 1),
                    (x, y + 1),
                    (x - 1, y - 1),
                    (x + 1, y - 1),
                    (x - 1, y + 1),
                    (x + 1, y + 1),
                ];
                let visible = neighbors
                    .into_iter()
                    .filter(|&(nx, ny)| {
                        block_is_visible(&snapshot, nx, ny, block_width, block_height)
                    })
                    .collect::<Vec<_>>();
                let bracketed = (block_is_visible(&snapshot, x - 1, y, block_width, block_height)
                    && block_is_visible(&snapshot, x + 1, y, block_width, block_height))
                    || (block_is_visible(&snapshot, x, y - 1, block_width, block_height)
                        && block_is_visible(&snapshot, x, y + 1, block_width, block_height));
                if !bracketed && visible.len() < 3 {
                    continue;
                }
                let source = visible
                    .iter()
                    .copied()
                    .max_by_key(|&candidate| {
                        visible
                            .iter()
                            .filter(|&&other| {
                                blocks_equal(&snapshot, candidate, other, block_width, block_height)
                            })
                            .count()
                    })
                    .expect("a bracketed mosaic gap has a visible neighbor");
                fills.push(((x, y), source));
            }
        }
        if fills.is_empty() {
            break;
        }
        for (destination, source) in fills {
            copy_image_block(output, source, destination, block_width, block_height);
        }
    }
}

fn block_is_visible(image: &RgbaImage, x: u32, y: u32, width: u32, height: u32) -> bool {
    image.get_pixel(x * width + width / 2, y * height + height / 2)[3] != 0
}

fn blocks_equal(
    image: &RgbaImage,
    first: (u32, u32),
    second: (u32, u32),
    width: u32,
    height: u32,
) -> bool {
    (0..height).all(|dy| {
        (0..width).all(|dx| {
            image.get_pixel(first.0 * width + dx, first.1 * height + dy)
                == image.get_pixel(second.0 * width + dx, second.1 * height + dy)
        })
    })
}

fn copy_image_block(
    image: &mut RgbaImage,
    source: (u32, u32),
    destination: (u32, u32),
    width: u32,
    height: u32,
) {
    let mut pixels = Vec::with_capacity((width * height) as usize);
    for dy in 0..height {
        for dx in 0..width {
            pixels.push(*image.get_pixel(source.0 * width + dx, source.1 * height + dy));
        }
    }
    for (index, pixel) in pixels.into_iter().enumerate() {
        let dx = index as u32 % width;
        let dy = index as u32 / width;
        image.put_pixel(
            destination.0 * width + dx,
            destination.1 * height + dy,
            pixel,
        );
    }
}

fn expand_preview_polygon(polygon: &[(f64, f64)], distance: f64) -> Vec<(f64, f64)> {
    let center = polygon
        .iter()
        .fold((0.0, 0.0), |sum, point| (sum.0 + point.0, sum.1 + point.1));
    let center = (
        center.0 / polygon.len() as f64,
        center.1 / polygon.len() as f64,
    );
    polygon
        .iter()
        .map(|&(x, y)| {
            let dx = x - center.0;
            let dy = y - center.1;
            let length = dx.hypot(dy);
            (x + dx / length * distance, y + dy / length * distance)
        })
        .collect()
}

fn plan_mosaic_transport_bridges(
    cells: &[(H3CellPlan, std::path::PathBuf)],
    placements: &[MosaicPlacement],
    grid_width: u16,
    grid_height: u16,
    image_width: u32,
    image_height: u32,
) -> Result<Vec<MosaicTransportBridge>> {
    ensure!(
        cells.len() == placements.len(),
        "H3 mosaic placement count does not match its cell count"
    );
    let by_cell = cells
        .iter()
        .enumerate()
        .map(|(index, (plan, _))| (plan.cell.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    ensure!(
        by_cell.len() == cells.len(),
        "H3 mosaic contains duplicate cell plans"
    );
    let mut by_edge = BTreeMap::<String, Vec<(usize, &crate::H3RegionalConnection)>>::new();
    for (cell_index, (plan, _)) in cells.iter().enumerate() {
        let Some(regional) = &plan.regional else {
            continue;
        };
        for connection in regional.connections.iter().filter(|connection| {
            !connection.boundary_exit && by_cell.contains_key(connection.neighbor.as_str())
        }) {
            by_edge
                .entry(connection.edge_id.clone())
                .or_default()
                .push((cell_index, connection));
        }
    }

    let block_width = image_width / u32::from(grid_width);
    let block_height = image_height / u32::from(grid_height);
    ensure!(
        block_width > 0
            && block_height > 0
            && block_width * u32::from(grid_width) == image_width
            && block_height * u32::from(grid_height) == image_height,
        "H3 mosaic transport bridges require an integral rendered block grid"
    );
    let block_width = i64::from(block_width);
    let block_height = i64::from(block_height);
    let mut bridges = Vec::with_capacity(by_edge.len());
    for (edge_id, mut endpoints) in by_edge {
        endpoints.sort_by_key(|(cell_index, _)| *cell_index);
        let [(first_cell, first), (second_cell, second)] = endpoints.as_slice() else {
            bail!(
                "H3 mosaic transport edge {edge_id} has {} endpoints instead of two",
                endpoints.len()
            );
        };
        let first_plan = &cells[*first_cell].0;
        let second_plan = &cells[*second_cell].0;
        ensure!(
            first.neighbor == second_plan.cell
                && second.neighbor == first_plan.cell
                && first.transport == second.transport
                && coordinates_match(first.coordinate, second.coordinate),
            "H3 mosaic transport edge {edge_id} is not an exact reciprocal pair"
        );
        let first_gate =
            crate::h3::h3_raster_landing(first_plan, grid_width, grid_height, first.coordinate)?;
        let second_gate =
            crate::h3::h3_raster_landing(second_plan, grid_width, grid_height, second.coordinate)?;
        let first_destination = (
            placements[*first_cell].left + i64::from(first_gate.0) * block_width,
            placements[*first_cell].top + i64::from(first_gate.1) * block_height,
        );
        let second_destination = (
            placements[*second_cell].left + i64::from(second_gate.0) * block_width,
            placements[*second_cell].top + i64::from(second_gate.1) * block_height,
        );
        let blocks = cardinal_bridge_blocks(
            first_destination,
            second_destination,
            block_width,
            block_height,
        );
        ensure!(
            blocks.first() == Some(&first_destination)
                && blocks.last() == Some(&second_destination)
                && blocks.windows(2).all(|pair| {
                    let dx = pair[0].0.abs_diff(pair[1].0);
                    let dy = pair[0].1.abs_diff(pair[1].1);
                    (dx == 0 && dy <= block_height as u64) || (dy == 0 && dx <= block_width as u64)
                }),
            "H3 mosaic transport edge {edge_id} has a non-cardinal or gapped bridge"
        );
        bridges.push(MosaicTransportBridge {
            edge_id,
            first_cell: *first_cell,
            first_gate,
            second_cell: *second_cell,
            second_gate,
            blocks,
        });
    }
    Ok(bridges)
}

fn cardinal_bridge_blocks(
    first: (i64, i64),
    second: (i64, i64),
    block_width: i64,
    block_height: i64,
) -> Vec<(i64, i64)> {
    let mut blocks = vec![first];
    let mut cursor = first;
    let horizontal_first = first.0.abs_diff(second.0) <= first.1.abs_diff(second.1);
    for horizontal in [horizontal_first, !horizontal_first] {
        while (horizontal && cursor.0 != second.0) || (!horizontal && cursor.1 != second.1) {
            if horizontal {
                let delta = second.0 - cursor.0;
                cursor.0 += delta.signum() * delta.abs().min(block_width);
            } else {
                let delta = second.1 - cursor.1;
                cursor.1 += delta.signum() * delta.abs().min(block_height);
            }
            if blocks.last() != Some(&cursor) {
                blocks.push(cursor);
            }
        }
    }
    blocks
}

fn copy_preview_block(
    source: &RgbaImage,
    gate: (u16, u16),
    block_width: u32,
    block_height: u32,
    output: &mut RgbaImage,
    destination: (i64, i64),
) {
    let source_left = u32::from(gate.0) * block_width;
    let source_top = u32::from(gate.1) * block_height;
    for y in 0..block_height {
        for x in 0..block_width {
            let output_x = destination.0 + i64::from(x);
            let output_y = destination.1 + i64::from(y);
            if output_x >= 0
                && output_y >= 0
                && output_x < i64::from(output.width())
                && output_y < i64::from(output.height())
            {
                output.put_pixel(
                    output_x as u32,
                    output_y as u32,
                    *source.get_pixel(source_left + x, source_top + y),
                );
            }
        }
    }
}

fn coordinates_match(first: Coordinate, second: Coordinate) -> bool {
    let longitude_delta = (first.lon - second.lon + 180.0).rem_euclid(360.0) - 180.0;
    (first.lat - second.lat).abs() <= 1e-10 && longitude_delta.abs() <= 1e-10
}

fn local_mosaic_point(origin: Coordinate, point: Coordinate) -> (f64, f64) {
    let longitude_delta = (point.lon - origin.lon + 540.0).rem_euclid(360.0) - 180.0;
    (
        longitude_delta.to_radians() * origin.lat.to_radians().cos(),
        -(point.lat - origin.lat).to_radians(),
    )
}

fn point_in_preview_polygon(x: f64, y: f64, polygon: &[(f64, f64)]) -> bool {
    let mut inside = false;
    let mut previous = polygon[polygon.len() - 1];
    for &current in polygon {
        if (current.1 > y) != (previous.1 > y)
            && x < (previous.0 - current.0) * (y - current.1) / (previous.1 - current.1) + current.0
        {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn load_palette_bank(files: &BTreeMap<String, Vec<u8>>, tileset_id: &str) -> Result<Vec<Palette>> {
    let tileset_palette = format!("gfx/tilesets/{tileset_id}.pal");
    if let Some(bytes) = files.get(&tileset_palette) {
        let palettes = parse_palette_file(std::str::from_utf8(bytes)?, None)?;
        if !palettes.is_empty() {
            return Ok(palettes.into_iter().take(8).collect());
        }
    }
    let content = std::str::from_utf8(
        files
            .get("gfx/tilesets/bg_tiles.pal")
            .context("compiled pack is missing gfx/tilesets/bg_tiles.pal")?,
    )?;
    for group in ["day", "morn", "indoor"] {
        let palettes = parse_palette_file(content, Some(group))?;
        if !palettes.is_empty() {
            return Ok(palettes.into_iter().take(8).collect());
        }
    }
    bail!("compiled pack background palette has no usable day palette")
}

fn blit_tile(
    source: &RgbaImage,
    source_width: usize,
    tile_index: usize,
    palette: &Palette,
    output: &mut RgbaImage,
    output_x: usize,
    output_y: usize,
) {
    let tiles_per_row = (source_width / TILE_SIZE).max(1);
    let source_x = tile_index % tiles_per_row * TILE_SIZE;
    let source_y = tile_index / tiles_per_row * TILE_SIZE;
    for row in 0..TILE_SIZE {
        for col in 0..TILE_SIZE {
            let pixel = source.get_pixel((source_x + col) as u32, (source_y + row) as u32);
            let palette_index = if pixel[3] == 0 {
                0
            } else {
                palette_index_from_gray(pixel[0])
            };
            let [red, green, blue] = palette[palette_index];
            output.put_pixel(
                (output_x + col) as u32,
                (output_y + row) as u32,
                Rgba([red, green, blue, 255]),
            );
        }
    }
}

fn resolve_tileset_tile_index(source_tile_count: usize, tile_index: usize, vram_bank: u8) -> usize {
    if source_tile_count == 0 {
        return 0;
    }
    if vram_bank == 1 && source_tile_count % 2 == 0 {
        let candidate = (tile_index & 0x7f) + source_tile_count / 2;
        if candidate < source_tile_count {
            return candidate;
        }
    }
    if vram_bank == 1 {
        let candidate = (tile_index & 0x7f) + 0x80;
        if candidate < source_tile_count {
            return candidate;
        }
    }
    if tile_index < source_tile_count {
        return tile_index;
    }
    if tile_index >= 0x80 && tile_index - 0x80 < source_tile_count {
        return tile_index - 0x80;
    }
    0
}

fn parse_palette_file(content: &str, group_filter: Option<&str>) -> Result<Vec<Palette>> {
    let mut current_group = "default".to_string();
    let mut palettes = Vec::new();
    let mut pending = Vec::new();
    for raw_line in content.lines() {
        let trimmed = raw_line.trim();
        if trimmed.starts_with(';') && !trimmed.to_ascii_uppercase().contains("RGB") {
            current_group = trimmed
                .trim_start_matches(';')
                .trim()
                .to_ascii_lowercase()
                .replace(' ', "_");
            pending.clear();
            continue;
        }
        let line = raw_line.split(';').next().unwrap_or("").trim();
        if !line.to_ascii_uppercase().starts_with("RGB")
            || group_filter.is_some_and(|group| current_group != group)
        {
            continue;
        }
        let values = parse_rgb_values(line)?;
        ensure!(values.len() % 3 == 0, "malformed RGB palette line {line:?}");
        for triplet in values.chunks(3) {
            pending.push(rgb_triplet_to_u8(triplet)?);
            if pending.len() == 4 {
                palettes.push([pending[0], pending[1], pending[2], pending[3]]);
                pending.clear();
            }
        }
    }
    Ok(palettes)
}

fn parse_rgb_values(line: &str) -> Result<Vec<u8>> {
    line.replace("RGB", "")
        .replace("rgb", "")
        .replace(',', " ")
        .split_whitespace()
        .map(|value| {
            value
                .parse::<u8>()
                .with_context(|| format!("parse palette component {value:?}"))
        })
        .collect()
}

fn rgb_triplet_to_u8(values: &[u8]) -> Result<[u8; 3]> {
    ensure!(values.len() == 3, "RGB triplet must contain three values");
    let normalize = |value: u8| {
        if value <= 31 {
            (value << 3) | (value >> 2)
        } else {
            value
        }
    };
    Ok([
        normalize(values[0]),
        normalize(values[1]),
        normalize(values[2]),
    ])
}

fn palette_index_from_gray(gray: u8) -> usize {
    match gray {
        213..=u8::MAX => 0,
        128..=212 => 1,
        43..=127 => 2,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Coordinate, FeatureKind, H3RegionalCellPlan, H3RegionalConnection, plan_h3_batch};

    #[test]
    fn parses_named_palette_group_like_runtime_renderer() {
        let palettes = parse_palette_file(
            "; morn\nRGB 1, 2, 3\nRGB 4, 5, 6\nRGB 7, 8, 9\nRGB 10, 11, 12\n; day\nRGB 31, 0, 0, 0, 31, 0, 0, 0, 31, 31, 31, 31\n",
            Some("day"),
        )
        .unwrap();
        assert_eq!(palettes.len(), 1);
        assert_eq!(palettes[0][0], [255, 0, 0]);
        assert_eq!(palettes[0][3], [255, 255, 255]);
    }

    #[test]
    fn resolves_second_vram_bank_like_runtime_renderer() {
        assert_eq!(resolve_tileset_tile_index(256, 7, 1), 135);
        assert_eq!(resolve_tileset_tile_index(128, 7, 1), 71);
    }

    #[test]
    fn nineteen_h3_previews_form_one_masked_geographic_mosaic() {
        let manifest = plan_h3_batch(
            Coordinate {
                lat: 44.9475196,
                lon: -93.3253477,
            },
            6,
            19,
        )
        .expect("nineteen H3 plans");
        let directory = tempfile::tempdir().expect("temporary mosaic directory");
        let cells = manifest
            .cells
            .iter()
            .map(|entry| {
                let path = directory.path().join(format!("{}.png", entry.plan.cell));
                RgbaImage::from_pixel(
                    64,
                    64,
                    Rgba([(entry.ordinal as u8).saturating_mul(29), 120, 200, 255]),
                )
                .save(&path)
                .expect("write cell fixture");
                (entry.plan.clone(), path)
            })
            .collect::<Vec<_>>();
        let output = directory.path().join("buckyball.png");
        render_h3_mosaic(&cells, 64, 64, &output).expect("render H3 mosaic");
        let mosaic = image::open(output).expect("load mosaic").to_rgba8();
        assert!(mosaic.width() > 256);
        assert!(mosaic.height() > 256);
        let opaque = mosaic.pixels().filter(|pixel| pixel[3] == 255).count();
        assert!(
            opaque > 19 * 2_000,
            "all nineteen masked faces should be visible"
        );
        assert!(
            mosaic.pixels().any(|pixel| pixel[3] == 0),
            "canvas corners should remain transparent"
        );
    }

    #[test]
    fn two_h3_faces_render_one_gapless_exact_tile_transport_join() {
        assert_gapless_regional_transport_mosaic(2);
    }

    #[test]
    fn three_h3_faces_render_every_transport_join_as_a_cardinal_pixel_chain() {
        assert_gapless_regional_transport_mosaic(3);
    }

    fn assert_gapless_regional_transport_mosaic(cell_count: usize) {
        const GRID: u16 = 64;
        const BLOCK: u32 = 4;
        const EDGE_COLORS: [Rgba<u8>; 3] = [
            Rgba([247, 31, 83, 255]),
            Rgba([255, 173, 28, 255]),
            Rgba([173, 64, 255, 255]),
        ];

        let manifest = plan_h3_batch(
            Coordinate {
                lat: 44.9475196,
                lon: -93.3253477,
            },
            6,
            cell_count,
        )
        .expect("regional mosaic fixture plans");
        let mut plans = manifest
            .cells
            .iter()
            .map(|entry| entry.plan.clone())
            .collect::<Vec<_>>();
        let by_cell = plans
            .iter()
            .enumerate()
            .map(|(ordinal, plan)| (plan.cell.clone(), ordinal))
            .collect::<BTreeMap<_, _>>();
        let mut edges = BTreeMap::<String, (usize, usize, Coordinate)>::new();
        for (ordinal, plan) in plans.iter().enumerate() {
            for portal in &plan.portals {
                let Some(&neighbor) = by_cell.get(&portal.neighbor) else {
                    continue;
                };
                if ordinal < neighbor {
                    edges.insert(portal.edge_id.clone(), (ordinal, neighbor, portal.midpoint));
                }
            }
        }
        assert_eq!(edges.len(), if cell_count == 2 { 1 } else { 3 });
        for (ordinal, plan) in plans.iter_mut().enumerate() {
            plan.regional = Some(H3RegionalCellPlan {
                ordinal,
                cell: plan.cell.clone(),
                building_count: 0,
                facilities: Vec::new(),
                connections: Vec::new(),
                closed_transport_crossings: Vec::new(),
            });
        }
        for (edge_id, &(first, second, coordinate)) in &edges {
            let first_cell = plans[first].cell.clone();
            let second_cell = plans[second].cell.clone();
            plans[first]
                .regional
                .as_mut()
                .expect("first regional fixture")
                .connections
                .push(H3RegionalConnection {
                    edge_id: edge_id.clone(),
                    neighbor: second_cell,
                    coordinate,
                    transport: FeatureKind::Road,
                    bridge: false,
                    authoritative: true,
                    boundary_exit: false,
                });
            plans[second]
                .regional
                .as_mut()
                .expect("second regional fixture")
                .connections
                .push(H3RegionalConnection {
                    edge_id: edge_id.clone(),
                    neighbor: first_cell,
                    coordinate,
                    transport: FeatureKind::Road,
                    bridge: false,
                    authoritative: true,
                    boundary_exit: false,
                });
        }

        let directory = tempfile::tempdir().expect("temporary transport mosaic directory");
        let color_by_edge = edges
            .keys()
            .enumerate()
            .map(|(index, edge)| (edge.clone(), EDGE_COLORS[index]))
            .collect::<BTreeMap<_, _>>();
        let cells = plans
            .into_iter()
            .enumerate()
            .map(|(ordinal, plan)| {
                let mut preview = RgbaImage::from_pixel(
                    u32::from(GRID) * BLOCK,
                    u32::from(GRID) * BLOCK,
                    Rgba([18 + ordinal as u8 * 24, 104, 66, 255]),
                );
                for connection in &plan
                    .regional
                    .as_ref()
                    .expect("regional fixture")
                    .connections
                {
                    let gate =
                        crate::h3::h3_raster_landing(&plan, GRID, GRID, connection.coordinate)
                            .expect("exact fixture gate");
                    let color = color_by_edge[&connection.edge_id];
                    for y in u32::from(gate.1) * BLOCK..u32::from(gate.1 + 1) * BLOCK {
                        for x in u32::from(gate.0) * BLOCK..u32::from(gate.0 + 1) * BLOCK {
                            preview.put_pixel(x, y, color);
                        }
                    }
                }
                let path = directory.path().join(format!("{}.png", plan.cell));
                preview.save(&path).expect("write transport face fixture");
                (plan, path)
            })
            .collect::<Vec<_>>();
        let output = directory.path().join("regional-transport.png");
        render_h3_mosaic(&cells, GRID, GRID, &output).expect("render regional transport mosaic");
        let mosaic = image::open(output)
            .expect("load regional transport mosaic")
            .to_rgba8();
        for (edge_id, color) in color_by_edge {
            let pixels = mosaic.pixels().filter(|pixel| **pixel == color).count();
            assert!(
                pixels >= (BLOCK * BLOCK * 2) as usize,
                "transport edge {edge_id} did not retain both exact endpoint blocks"
            );
            assert_eq!(
                matching_pixel_components(&mosaic, color),
                1,
                "transport edge {edge_id} contains a visible gap in the composed mosaic"
            );
        }
    }

    fn matching_pixel_components(image: &RgbaImage, wanted: Rgba<u8>) -> usize {
        let width = image.width() as usize;
        let height = image.height() as usize;
        let mut unseen = image
            .pixels()
            .map(|pixel| *pixel == wanted)
            .collect::<Vec<_>>();
        let mut components = 0;
        for start in 0..unseen.len() {
            if !unseen[start] {
                continue;
            }
            components += 1;
            unseen[start] = false;
            let mut frontier = vec![start];
            while let Some(index) = frontier.pop() {
                let x = index % width;
                let y = index / width;
                for (next_x, next_y) in [
                    (x.wrapping_sub(1), y),
                    (x + 1, y),
                    (x, y.wrapping_sub(1)),
                    (x, y + 1),
                ] {
                    if next_x >= width || next_y >= height {
                        continue;
                    }
                    let next = next_y * width + next_x;
                    if unseen[next] {
                        unseen[next] = false;
                        frontier.push(next);
                    }
                }
            }
        }
        components
    }
}
