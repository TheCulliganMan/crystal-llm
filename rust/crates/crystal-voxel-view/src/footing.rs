//! Pure conversion from classic 2D visual coordinates to voxel footing.

use bevy::prelude::{UVec2, Vec2, Vec3};
use crystal_render_api::{VisualActor, VisualWorldFrame};

use crate::profile::support_height;

const FOOTING_SAMPLE_EPSILON: f32 = 0.01;

pub fn actor_foot(actor: &VisualActor) -> Vec2 {
    actor.center - Vec2::new(0.0, actor.size.y * 0.5)
}

pub fn tile_at_visual_point(frame: &VisualWorldFrame, point: Vec2) -> Option<UVec2> {
    if !point.is_finite() || !frame.center.is_finite() || !frame.viewport_size.is_finite() {
        return None;
    }
    if frame.tile_size.x <= 0.0 || frame.tile_size.y <= 0.0 {
        return None;
    }

    let left = frame.center.x - frame.viewport_size.x * 0.5;
    let top = frame.center.y + frame.viewport_size.y * 0.5;
    let relative_x = point.x - left;
    let relative_y = top - point.y;
    if relative_x < 0.0
        || relative_y < 0.0
        || relative_x >= frame.viewport_size.x
        || relative_y >= frame.viewport_size.y
    {
        return None;
    }

    let column = (relative_x / frame.tile_size.x).floor() as u32;
    let row = (relative_y / frame.tile_size.y).floor() as u32;
    (column < frame.grid_size.x && row < frame.grid_size.y).then_some(UVec2::new(column, row))
}

pub fn footing_height(frame: &VisualWorldFrame, foot: Vec2) -> Option<f32> {
    if !foot.is_finite() {
        return None;
    }
    // Sprite bottoms normally sit exactly on a tile boundary. Sample just
    // inside the sprite footprint while rendering at the exact bottom point.
    let Some(coordinate) = tile_at_visual_point(frame, foot + Vec2::Y * FOOTING_SAMPLE_EPSILON)
    else {
        // The classic renderer can retain a partially clipped actor whose foot
        // lies just outside the exact 20x18 sample. Treat that seam as ordinary
        // ground instead of disabling the whole optional renderer.
        return Some(0.0);
    };
    Some(
        frame
            .tiles
            .iter()
            .find(|tile| tile.column == coordinate.x && tile.row == coordinate.y)
            .map(|tile| support_height(&tile.source, frame.tile_size.y))
            .unwrap_or(0.0),
    )
}

pub fn visual_point_to_voxel(point: Vec2, height: f32) -> Vec3 {
    Vec3::new(point.x, height, -point.y)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bevy::prelude::{Handle, Image, UVec2};
    use crystal_render_api::{VisualActorId, VisualTile, VisualTileSource, VisualWorldFrame};

    use super::*;

    fn source(metatile_id: u16, row: u8) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from("johto"),
            metatile_id,
            subtile_column: 0,
            subtile_row: row,
            tile_index: 1,
        }
    }

    fn frame() -> VisualWorldFrame {
        VisualWorldFrame {
            active: true,
            map_id: Arc::from("NewBarkTown"),
            terrain_revision: 1,
            map_texture: Handle::<Image>::weak_from_u128(1),
            center: Vec2::new(-16.0, 16.0),
            viewport_size: Vec2::new(16.0, 8.0),
            tile_size: Vec2::splat(8.0),
            grid_size: UVec2::new(2, 1),
            tiles: vec![
                VisualTile {
                    column: 1,
                    row: 0,
                    source: source(0x54, 0),
                    texture: Handle::<Image>::weak_from_u128(2),
                    priority: true,
                },
                VisualTile {
                    column: 0,
                    row: 0,
                    source: source(0x01, 0),
                    texture: Handle::<Image>::weak_from_u128(3),
                    priority: false,
                },
            ],
            actors: Vec::new(),
        }
    }

    #[test]
    fn actor_card_is_anchored_at_the_bottom_center() {
        let actor = VisualActor {
            id: VisualActorId::Player,
            texture: Handle::weak_from_u128(2),
            center: Vec2::new(10.0, 20.0),
            size: Vec2::new(8.0, 16.0),
            flip_x: false,
            above_priority: false,
        };
        assert_eq!(actor_foot(&actor), Vec2::new(10.0, 12.0));
    }

    #[test]
    fn footing_uses_explicit_tile_coordinates_not_vector_order() {
        let frame = frame();
        let left_cell = Vec2::new(-20.0, 16.0);
        let right_cell = Vec2::new(-12.0, 16.0);

        assert_eq!(footing_height(&frame, left_cell), Some(0.0));
        assert_eq!(footing_height(&frame, right_cell), Some(0.0));
    }

    #[test]
    fn voxel_coordinates_preserve_absolute_presentation_position() {
        assert_eq!(
            visual_point_to_voxel(Vec2::new(-8.0, 12.0), 3.0),
            Vec3::new(-8.0, 3.0, -12.0),
        );
    }

    #[test]
    fn points_on_the_exclusive_right_edge_are_not_clamped() {
        let frame = frame();
        let right_edge = frame.center + Vec2::new(frame.viewport_size.x * 0.5, 0.0);
        assert_eq!(tile_at_visual_point(&frame, right_edge), None);
        assert_eq!(footing_height(&frame, right_edge), Some(0.0));
    }

    #[test]
    fn boundary_footing_samples_last_occupied_row_but_keeps_door_support_ground() {
        let mut frame = frame();
        frame.center = Vec2::ZERO;
        frame.viewport_size = Vec2::new(8.0, 16.0);
        frame.grid_size = UVec2::new(1, 2);
        frame.tiles = vec![
            VisualTile {
                column: 0,
                row: 0,
                source: source(0x16, 3),
                texture: Handle::<Image>::weak_from_u128(2),
                priority: true,
            },
            VisualTile {
                column: 0,
                row: 1,
                source: source(0x01, 0),
                texture: Handle::<Image>::weak_from_u128(3),
                priority: false,
            },
        ];

        assert_eq!(
            tile_at_visual_point(&frame, Vec2::ZERO),
            Some(UVec2::new(0, 1))
        );
        assert_eq!(footing_height(&frame, Vec2::ZERO), Some(0.0));
    }
}
