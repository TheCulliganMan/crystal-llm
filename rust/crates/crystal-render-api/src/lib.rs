#![forbid(unsafe_code)]

//! Read-only presentation data shared by the classic world renderer and
//! optional world-rendering mods.
//!
//! The game runtime remains authoritative. Render mods should consume
//! [`VisualWorldFrame`] through `Res<VisualWorldFrame>` during
//! [`WorldRenderSet::RenderSync`] and must not feed presentation state back
//! into simulation.

use std::{collections::HashSet, sync::Arc};

use bevy::prelude::{
    App, Handle, Image, IntoSystemSetConfigs, Plugin, Resource, SystemSet, UVec2, Update, Vec2,
};

/// Ordered phases for publishing and consuming one visual world frame.
///
/// [`WorldRenderSet::ClassicWorld`] updates the faithful 2D presentation,
/// [`WorldRenderSet::PresentationExtract`] publishes its read-only frame, and
/// [`WorldRenderSet::RenderSync`] lets optional renderers synchronize their
/// own presentation from that frame.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorldRenderSet {
    ClassicWorld,
    PresentationExtract,
    RenderSync,
}

/// Installs the render-phase ordering into Bevy's `Update` schedule.
pub fn configure_world_render_sets(app: &mut App) -> &mut App {
    app.configure_sets(
        Update,
        (
            WorldRenderSet::ClassicWorld,
            WorldRenderSet::PresentationExtract,
            WorldRenderSet::RenderSync,
        )
            .chain(),
    )
}

/// Initializes the visual frame resource and installs the render-phase order.
#[derive(Clone, Copy, Debug, Default)]
pub struct VisualWorldRenderPlugin;

impl Plugin for VisualWorldRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VisualWorldFrame>();
        configure_world_render_sets(app);
    }
}

/// Exact source identity for one 8x8 tile in Crystal's 4x4 metatile layout.
///
/// Optional renderers may use this stable, presentation-only identity to look
/// up an authored shape profile. The host deliberately does not derive shape
/// or height from gameplay collision permissions.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VisualTileSource {
    pub tileset_id: Arc<str>,
    pub metatile_id: u16,
    pub subtile_column: u8,
    pub subtile_row: u8,
    pub tile_index: u16,
}

/// One tile in the visual map grid.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VisualTile {
    pub column: u32,
    pub row: u32,
    pub source: VisualTileSource,
    /// Exact live 8x8 image used by the classic renderer for this cell. This
    /// gives optional renderers non-stretched top/edge source art without
    /// exposing asset-root paths or gameplay data.
    pub texture: Handle<Image>,
    /// Whether the tile belongs to the classic map foreground-priority layer.
    /// This is compositing metadata, not a height or shape signal.
    pub priority: bool,
}

/// Stable identity for an actor across visual frame publications.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VisualActorId {
    Player,
    Object(u32),
    /// Presentation-only card emitted by the faithful world renderer. These
    /// IDs never enter gameplay state or object-slot identity.
    Effect(VisualEffectId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VisualEffectId {
    GrassRustle,
}

/// Presentation state for one actor visible in the world pass.
#[derive(Clone, Debug, PartialEq)]
pub struct VisualActor {
    pub id: VisualActorId,
    pub texture: Handle<Image>,
    /// Actor center in the same world-pixel coordinate system as the map.
    pub center: Vec2,
    pub size: Vec2,
    pub flip_x: bool,
    /// Whether this actor is drawn above foreground-priority map tiles.
    pub above_priority: bool,
}

/// Immutable-by-convention snapshot consumed by optional world renderers.
///
/// An inactive frame is intentionally empty and valid. When `active` is true,
/// producers must publish a real map texture and valid finite geometry before
/// optional renderers consume the resource.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct VisualWorldFrame {
    pub active: bool,
    /// Stable compiled map identifier for presentation-profile selection.
    pub map_id: Arc<str>,
    /// Changes whenever the visible tile sources or their live art change.
    pub terrain_revision: u64,
    pub map_texture: Handle<Image>,
    /// Center of the composited map surface / visual viewport in world pixels.
    pub center: Vec2,
    /// Visible world extent in world pixels.
    pub viewport_size: Vec2,
    /// Width and height of a visual tile in world pixels.
    pub tile_size: Vec2,
    pub grid_size: UVec2,
    pub tiles: Vec<VisualTile>,
    pub actors: Vec<VisualActor>,
}

impl VisualWorldFrame {
    /// Checks the geometry and identity invariants required by render mods.
    ///
    /// Inactive frames are always accepted because consumers must ignore all
    /// remaining fields when `active` is false.
    pub fn validate(&self) -> Result<(), VisualWorldFrameError> {
        if !self.active {
            return Ok(());
        }

        if self.map_id.is_empty() {
            return Err(VisualWorldFrameError::EmptyMapId);
        }
        if self.map_texture == Handle::<Image>::default() {
            return Err(VisualWorldFrameError::MissingMapTexture);
        }
        if !self.center.is_finite() {
            return Err(VisualWorldFrameError::NonFiniteCenter);
        }
        if !is_positive_finite(self.viewport_size) {
            return Err(VisualWorldFrameError::InvalidViewportSize);
        }
        if !is_positive_finite(self.tile_size) {
            return Err(VisualWorldFrameError::InvalidTileSize);
        }
        if self.grid_size.x == 0 || self.grid_size.y == 0 {
            return Err(VisualWorldFrameError::EmptyGrid);
        }
        if self.viewport_size != self.tile_size * self.grid_size.as_vec2() {
            return Err(VisualWorldFrameError::ViewportGridMismatch);
        }

        let expected_tile_count = u64::from(self.grid_size.x) * u64::from(self.grid_size.y);
        let actual_tile_count = self.tiles.len() as u64;
        if actual_tile_count != expected_tile_count {
            return Err(VisualWorldFrameError::TileCountMismatch {
                expected: expected_tile_count,
                actual: actual_tile_count,
            });
        }

        let mut tile_coordinates = HashSet::with_capacity(self.tiles.len());
        for tile in &self.tiles {
            if tile.column >= self.grid_size.x || tile.row >= self.grid_size.y {
                return Err(VisualWorldFrameError::TileOutsideGrid {
                    column: tile.column,
                    row: tile.row,
                });
            }
            if !tile_coordinates.insert((tile.column, tile.row)) {
                return Err(VisualWorldFrameError::DuplicateTile {
                    column: tile.column,
                    row: tile.row,
                });
            }
            if tile.texture == Handle::<Image>::default() {
                return Err(VisualWorldFrameError::MissingTileTexture {
                    column: tile.column,
                    row: tile.row,
                });
            }
            if tile.source.tileset_id.is_empty() {
                return Err(VisualWorldFrameError::EmptyTileSourceTileset {
                    column: tile.column,
                    row: tile.row,
                });
            }
            if tile.source.subtile_column >= 4 || tile.source.subtile_row >= 4 {
                return Err(VisualWorldFrameError::TileSourceOutsideMetatile {
                    column: tile.column,
                    row: tile.row,
                    subtile_column: tile.source.subtile_column,
                    subtile_row: tile.source.subtile_row,
                });
            }
        }

        let mut actor_ids = HashSet::with_capacity(self.actors.len());
        for actor in &self.actors {
            if actor.texture == Handle::<Image>::default() {
                return Err(VisualWorldFrameError::MissingActorTexture(actor.id));
            }
            if !actor.center.is_finite() {
                return Err(VisualWorldFrameError::NonFiniteActorCenter(actor.id));
            }
            if !is_positive_finite(actor.size) {
                return Err(VisualWorldFrameError::InvalidActorSize(actor.id));
            }
            if !actor_ids.insert(actor.id) {
                return Err(VisualWorldFrameError::DuplicateActor(actor.id));
            }
        }

        Ok(())
    }
}

fn is_positive_finite(value: Vec2) -> bool {
    value.is_finite() && value.x > 0.0 && value.y > 0.0
}

/// Why an active [`VisualWorldFrame`] cannot be consumed safely.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisualWorldFrameError {
    EmptyMapId,
    MissingMapTexture,
    NonFiniteCenter,
    InvalidViewportSize,
    InvalidTileSize,
    EmptyGrid,
    ViewportGridMismatch,
    TileCountMismatch {
        expected: u64,
        actual: u64,
    },
    TileOutsideGrid {
        column: u32,
        row: u32,
    },
    DuplicateTile {
        column: u32,
        row: u32,
    },
    MissingTileTexture {
        column: u32,
        row: u32,
    },
    EmptyTileSourceTileset {
        column: u32,
        row: u32,
    },
    TileSourceOutsideMetatile {
        column: u32,
        row: u32,
        subtile_column: u8,
        subtile_row: u8,
    },
    MissingActorTexture(VisualActorId),
    NonFiniteActorCenter(VisualActorId),
    InvalidActorSize(VisualActorId),
    DuplicateActor(VisualActorId),
}

#[cfg(test)]
mod tests {
    use bevy::prelude::{App, IntoSystemConfigs, ResMut, Resource, Update};

    use super::*;

    fn visual_tile(column: u32, row: u32, tile_index: u16) -> VisualTile {
        VisualTile {
            column,
            row,
            source: VisualTileSource {
                tileset_id: Arc::from("johto"),
                metatile_id: 3,
                subtile_column: u8::try_from(column).expect("test column fits u8"),
                subtile_row: u8::try_from(row).expect("test row fits u8"),
                tile_index,
            },
            texture: Handle::weak_from_u128(100 + u128::from(tile_index)),
            priority: false,
        }
    }

    fn active_frame() -> VisualWorldFrame {
        VisualWorldFrame {
            active: true,
            map_id: Arc::from("NewBarkTown"),
            terrain_revision: 7,
            map_texture: Handle::weak_from_u128(1),
            center: Vec2::new(8.0, 4.0),
            viewport_size: Vec2::new(16.0, 8.0),
            tile_size: Vec2::splat(8.0),
            grid_size: UVec2::new(2, 1),
            tiles: vec![
                visual_tile(0, 0, 7),
                VisualTile {
                    priority: true,
                    ..visual_tile(1, 0, 9)
                },
            ],
            actors: vec![VisualActor {
                id: VisualActorId::Player,
                texture: Handle::weak_from_u128(2),
                center: Vec2::new(8.0, 4.0),
                size: Vec2::splat(16.0),
                flip_x: false,
                above_priority: false,
            }],
        }
    }

    #[test]
    fn default_frame_is_inactive_and_valid() {
        let frame = VisualWorldFrame::default();

        assert!(!frame.active);
        assert!(frame.map_id.is_empty());
        assert_eq!(frame.terrain_revision, 0);
        assert_eq!(frame.map_texture, Handle::<Image>::default());
        assert!(frame.tiles.is_empty());
        assert!(frame.actors.is_empty());
        assert_eq!(frame.validate(), Ok(()));
    }

    #[test]
    fn active_frame_with_valid_geometry_is_accepted() {
        assert_eq!(active_frame().validate(), Ok(()));
    }

    #[test]
    fn active_frame_requires_a_stable_map_identity() {
        let mut frame = active_frame();
        frame.map_id = Arc::from("");

        assert_eq!(frame.validate(), Err(VisualWorldFrameError::EmptyMapId));
    }

    #[test]
    fn active_frame_requires_viewport_to_match_tile_grid_extent() {
        let mut frame = active_frame();
        frame.viewport_size.x += 1.0;

        assert_eq!(
            frame.validate(),
            Err(VisualWorldFrameError::ViewportGridMismatch)
        );
    }

    #[test]
    fn active_frame_rejects_tiles_outside_the_grid() {
        let mut frame = active_frame();
        frame.tiles[1].column = frame.grid_size.x;

        assert_eq!(
            frame.validate(),
            Err(VisualWorldFrameError::TileOutsideGrid { column: 2, row: 0 })
        );
    }

    #[test]
    fn active_frame_requires_complete_unique_tile_coverage() {
        let mut incomplete = active_frame();
        incomplete.tiles.pop();
        assert_eq!(
            incomplete.validate(),
            Err(VisualWorldFrameError::TileCountMismatch {
                expected: 2,
                actual: 1,
            })
        );

        let mut duplicate = active_frame();
        duplicate.tiles[1].column = duplicate.tiles[0].column;
        assert_eq!(
            duplicate.validate(),
            Err(VisualWorldFrameError::DuplicateTile { column: 0, row: 0 })
        );
    }

    #[test]
    fn active_frame_rejects_default_texture_handles() {
        let mut missing_map = active_frame();
        missing_map.map_texture = Handle::default();
        assert_eq!(
            missing_map.validate(),
            Err(VisualWorldFrameError::MissingMapTexture)
        );

        let mut missing_actor = active_frame();
        missing_actor.actors[0].texture = Handle::default();

        assert_eq!(
            missing_actor.validate(),
            Err(VisualWorldFrameError::MissingActorTexture(
                VisualActorId::Player
            ))
        );

        let mut missing_tile = active_frame();
        missing_tile.tiles[0].texture = Handle::default();
        assert_eq!(
            missing_tile.validate(),
            Err(VisualWorldFrameError::MissingTileTexture { column: 0, row: 0 })
        );
    }

    #[test]
    fn active_frame_rejects_incomplete_source_identity() {
        let mut empty_tileset = active_frame();
        empty_tileset.tiles[0].source.tileset_id = Arc::from("");
        assert_eq!(
            empty_tileset.validate(),
            Err(VisualWorldFrameError::EmptyTileSourceTileset { column: 0, row: 0 })
        );

        let mut outside_metatile = active_frame();
        outside_metatile.tiles[0].source.subtile_column = 4;
        assert_eq!(
            outside_metatile.validate(),
            Err(VisualWorldFrameError::TileSourceOutsideMetatile {
                column: 0,
                row: 0,
                subtile_column: 4,
                subtile_row: 0,
            })
        );
    }

    #[test]
    fn active_frame_rejects_duplicate_stable_actor_ids() {
        let mut frame = active_frame();
        frame.actors.push(VisualActor {
            id: VisualActorId::Player,
            texture: Handle::weak_from_u128(3),
            center: Vec2::new(96.0, 72.0),
            size: Vec2::splat(16.0),
            flip_x: true,
            above_priority: true,
        });

        assert_eq!(
            frame.validate(),
            Err(VisualWorldFrameError::DuplicateActor(VisualActorId::Player))
        );
    }

    #[test]
    fn presentation_effect_identity_is_distinct_and_stable() {
        let mut frame = active_frame();
        let grass_rustle = VisualActor {
            id: VisualActorId::Effect(VisualEffectId::GrassRustle),
            texture: Handle::weak_from_u128(3),
            center: Vec2::new(8.0, 4.0),
            size: Vec2::splat(16.0),
            flip_x: false,
            above_priority: true,
        };
        frame.actors.push(grass_rustle.clone());

        assert_eq!(frame.validate(), Ok(()));

        frame.actors.push(grass_rustle);
        assert_eq!(
            frame.validate(),
            Err(VisualWorldFrameError::DuplicateActor(
                VisualActorId::Effect(VisualEffectId::GrassRustle)
            ))
        );
    }

    #[derive(Resource, Default)]
    struct PhaseLog(Vec<WorldRenderSet>);

    fn record_classic(mut log: ResMut<PhaseLog>) {
        log.0.push(WorldRenderSet::ClassicWorld);
    }

    fn record_extract(mut log: ResMut<PhaseLog>) {
        log.0.push(WorldRenderSet::PresentationExtract);
    }

    fn record_sync(mut log: ResMut<PhaseLog>) {
        log.0.push(WorldRenderSet::RenderSync);
    }

    #[test]
    fn plugin_orders_render_phases_and_initializes_frame() {
        let mut app = App::new();
        app.add_plugins(VisualWorldRenderPlugin)
            .init_resource::<PhaseLog>()
            .add_systems(Update, record_sync.in_set(WorldRenderSet::RenderSync))
            .add_systems(Update, record_classic.in_set(WorldRenderSet::ClassicWorld))
            .add_systems(
                Update,
                record_extract.in_set(WorldRenderSet::PresentationExtract),
            );

        app.update();

        assert!(!app.world().resource::<VisualWorldFrame>().active);
        assert_eq!(
            app.world().resource::<PhaseLog>().0,
            vec![
                WorldRenderSet::ClassicWorld,
                WorldRenderSet::PresentationExtract,
                WorldRenderSet::RenderSync,
            ]
        );
    }
}
