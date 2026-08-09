/// Publish the already-rendered overworld as a read-only frame for optional
/// presentation plugins.  This is intentionally downstream of
/// `render_playfield`: it observes presentation state and entities only and
/// never reads or mutates authoritative movement, collision, scripts, or game
/// state.
fn publish_visual_world_frame(
    rendered: Res<RenderedViewport>,
    runtime_shell: Res<BevyRuntimeShell>,
    map_sprites: Query<
        (&Handle<Image>, &Transform),
        (With<PlayfieldTile>, Without<PlayfieldPriorityTile>),
    >,
    players: Query<(&Handle<Image>, &Sprite, &Transform), With<PlayerMarker>>,
    grass_rustles: Query<
        (&Handle<Image>, &Sprite, &Transform),
        (With<GrassRustleMarker>, Without<PlayerMarker>),
    >,
    objects: Query<(&VisibleObjectSprite, &Handle<Image>, &Sprite, &Transform), With<ObjectMarker>>,
    battle_entities: Query<
        (),
        Or<(
            With<BattleBattlerMarker>,
            With<BattleHudMarker>,
            With<BattleCommandMarker>,
            With<FixedBattleCanvasMarker>,
            With<BattleWindowFrameMarker>,
        )>,
    >,
    fullscreen_entities: Query<(), Or<(With<TitleScreenMarker>, With<VisibleIntroSurface>)>>,
    mut published: ResMut<crystal_render_api::VisualWorldFrame>,
) {
    // Clearing first is deliberate: consumers must never keep drawing a stale
    // overworld behind a title, battle, or malformed/incomplete render frame.
    *published = crystal_render_api::VisualWorldFrame::default();

    if rendered.title_active
        // The naming screen is committed through Commands later in the classic
        // render pass. Check its live state too: waiting for the presenter
        // entity to exist leaves one frame in which an optional world renderer
        // can draw the overworld over the naming LCD.
        || naming_screen_blocks_world_presentation(runtime_shell.pending_name_input.as_ref())
        || runtime_shell.battle_lcd_animation_active
        || !voxel_spatial_effects_supported(&runtime_shell)
        || battle_entities.iter().next().is_some()
        || fullscreen_entities.iter().next().is_some()
        || rendered.map_name.is_none()
        || !visual_tile_grid_is_complete(&rendered.visual_tiles)
    {
        return;
    }

    let (Some(map_id), Some(map_visual_key), Some(viewport_origin), Some(map_texture)) = (
        rendered.map_name.as_deref(),
        rendered.map_visual_key,
        rendered.viewport_origin,
        rendered.map_texture.as_ref(),
    ) else {
        return;
    };
    let terrain_revision = visual_terrain_revision(
        map_visual_key,
        viewport_origin,
        rendered.visual_tiles.as_slice(),
    );
    let Some((_, map_transform)) = map_sprites
        .iter()
        .find(|(texture, _)| texture.id() == map_texture.id())
    else {
        return;
    };
    let center = map_transform.translation.truncate();
    if !center.is_finite() {
        return;
    }
    #[cfg(feature = "voxel-view")]
    let (published_map_texture, published_grid_size) = {
        let Some(texture) = rendered.visual_world_texture.as_ref() else {
            return;
        };
        (texture.clone(), rendered.visual_world_grid_size)
    };
    #[cfg(not(feature = "voxel-view"))]
    let (published_map_texture, published_grid_size) = (
        map_texture.clone(),
        UVec2::new(VIEWPORT_TILES_X as u32, VIEWPORT_TILES_Y as u32),
    );

    let mut actors = Vec::with_capacity(
        players.iter().count() + objects.iter().count() + grass_rustles.iter().count(),
    );
    let mut player_iter = players.iter();
    if let Some((texture, sprite, transform)) = player_iter.next() {
        // A second player sprite is an incomplete deferred scene transition,
        // not a valid immutable frame to hand to a renderer mod.
        if player_iter.next().is_some() {
            return;
        }
        let Some(actor) = visual_actor(
            crystal_render_api::VisualActorId::Player,
            texture,
            sprite,
            transform,
            false,
        ) else {
            return;
        };
        actors.push(actor);
    }

    // ECS query order is not a presentation contract.  Publish objects in
    // stable ASM/object-slot order so renderer caches receive deterministic
    // input even when Bevy archetypes move.
    let mut visible_objects = objects.iter().collect::<Vec<_>>();
    visible_objects.sort_by_key(|(object, _, _, _)| object.object_index);
    if visible_objects
        .windows(2)
        .any(|pair| pair[0].0.object_index == pair[1].0.object_index)
    {
        return;
    }
    for (object, texture, sprite, transform) in visible_objects {
        let Ok(object_index) = u32::try_from(object.object_index) else {
            return;
        };
        let Some(actor) = visual_actor(
            crystal_render_api::VisualActorId::Object(object_index),
            texture,
            sprite,
            transform,
            object.above_priority,
        ) else {
            return;
        };
        actors.push(actor);
    }

    let mut rustle_iter = grass_rustles.iter();
    if let Some((texture, sprite, transform)) = rustle_iter.next() {
        if rustle_iter.next().is_some() {
            return;
        }
        let Some(effect) = visual_actor(
            crystal_render_api::VisualActorId::Effect(
                crystal_render_api::VisualEffectId::GrassRustle,
            ),
            texture,
            sprite,
            transform,
            true,
        ) else {
            return;
        };
        actors.push(effect);
    }

    let next = crystal_render_api::VisualWorldFrame {
        active: true,
        map_id: Arc::from(map_id),
        terrain_revision,
        map_texture: published_map_texture,
        center,
        viewport_size: Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT),
        tile_size: Vec2::splat(TILE_SIZE),
        grid_size: published_grid_size,
        tiles: rendered.visual_tiles.clone(),
        actors,
    };
    if next.validate().is_ok() {
        *published = next;
    }
}

fn naming_screen_blocks_world_presentation(input: Option<&PendingNameInput>) -> bool {
    input.is_some()
}

/// Effects in this list are still authored in classic screen coordinates.
/// Publishing the displaced actor together with an undisplaced ground point
/// would turn a jump into motion across the pitched floor, while leaving the
/// classic overlay above a 3D world would visibly detach it. Keep the exact 2D
/// renderer on screen for those short sequences until the render API exposes
/// explicit world anchors/elevation for them.
fn voxel_spatial_effects_supported(runtime_shell: &BevyRuntimeShell) -> bool {
    let scripted_actor_displacement =
        runtime_shell
            .visible_script_movement
            .as_ref()
            .is_some_and(|movement| {
                movement.active_jump_duration.is_some()
                    || movement.follower_active_jump_duration.is_some()
                    || movement.stationary_y_offset != 0
            });

    runtime_shell.visible_ledge_jump.is_none()
        && !scripted_actor_displacement
        && runtime_shell.visible_fishing_animation.is_none()
        && runtime_shell.visible_strength_boulder_dust.is_none()
        && runtime_shell.visible_overworld_emote.is_none()
}

fn visual_actor(
    id: crystal_render_api::VisualActorId,
    texture: &Handle<Image>,
    sprite: &Sprite,
    transform: &Transform,
    above_priority: bool,
) -> Option<crystal_render_api::VisualActor> {
    let size = sprite.custom_size?;
    let center = transform.translation.truncate();
    if !size.is_finite() || size.x <= 0.0 || size.y <= 0.0 || !center.is_finite() {
        return None;
    }
    Some(crystal_render_api::VisualActor {
        id,
        texture: texture.clone(),
        center,
        size,
        flip_x: sprite.flip_x,
        above_priority,
    })
}

fn visual_tile_grid_is_complete(tiles: &[crystal_render_api::VisualTile]) -> bool {
    let Ok(width) = usize::try_from(VISUAL_WORLD_TILES_X) else {
        return false;
    };
    let Ok(height) = usize::try_from(VISUAL_WORLD_TILES_Y) else {
        return false;
    };
    tiles.len() == width * height
        && tiles.iter().enumerate().all(|(index, tile)| {
            usize::try_from(tile.column).ok() == Some(index % width)
                && usize::try_from(tile.row).ok() == Some(index / width)
        })
}

fn visual_terrain_revision(
    map_visual_key: u64,
    viewport_origin: (i16, i16),
    tiles: &[crystal_render_api::VisualTile],
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    map_visual_key.hash(&mut hasher);
    viewport_origin.hash(&mut hasher);
    // Geometry is selected by stable source identity, not by the current
    // animation-frame image handle. Hashing live handles here caused water
    // animation to replace the async mesh request every frame, so a cave with
    // animated water could remain on the classic view forever.
    for tile in tiles {
        tile.column.hash(&mut hasher);
        tile.row.hash(&mut hasher);
        tile.source.hash(&mut hasher);
        tile.priority.hash(&mut hasher);
    }
    hasher.finish()
}

#[cfg(test)]
mod render_mod_tests {
    use super::*;

    fn complete_visual_grid() -> Vec<crystal_render_api::VisualTile> {
        (0..VISUAL_WORLD_TILES_Y as u32)
            .flat_map(|row| {
                (0..VISUAL_WORLD_TILES_X as u32).map(move |column| crystal_render_api::VisualTile {
                    column,
                    row,
                    source: crystal_render_api::VisualTileSource {
                        tileset_id: Arc::from("johto"),
                        metatile_id: 3,
                        subtile_column: u8::try_from(column % 4)
                            .expect("test subtile column fits u8"),
                        subtile_row: u8::try_from(row % 4).expect("test subtile row fits u8"),
                        tile_index: u16::try_from(row * VISUAL_WORLD_TILES_X as u32 + column)
                            .expect("test tile index fits u16"),
                    },
                    texture: Handle::weak_from_u128(
                        1 + u128::from(row * VISUAL_WORLD_TILES_X as u32 + column),
                    ),
                    priority: false,
                })
            })
            .collect()
    }

    #[test]
    fn visual_tile_grid_requires_one_row_major_cell_per_terrain_position() {
        let mut tiles = complete_visual_grid();
        assert!(visual_tile_grid_is_complete(&tiles));

        tiles.pop();
        assert!(!visual_tile_grid_is_complete(&tiles));

        let mut tiles = complete_visual_grid();
        tiles.swap(0, 1);
        assert!(!visual_tile_grid_is_complete(&tiles));
    }

    #[test]
    fn naming_screen_blocks_world_presentation_before_its_lcd_entity_is_spawned() {
        let input = PendingNameInput {
            label: "YOUR NAME?".to_string(),
            value: String::new(),
            max_length: 7,
            cursor_column: 0,
            cursor_row: 0,
            case: NameInputCase::Upper,
        };

        assert!(!naming_screen_blocks_world_presentation(None));
        assert!(naming_screen_blocks_world_presentation(Some(&input)));
    }

    #[test]
    fn visual_terrain_revision_tracks_viewport_and_source_art() {
        let tiles = complete_visual_grid();
        let baseline = visual_terrain_revision(7, (-4, 12), &tiles);

        assert_eq!(baseline, visual_terrain_revision(7, (-4, 12), &tiles));
        assert_ne!(baseline, visual_terrain_revision(8, (-4, 12), &tiles));
        assert_ne!(baseline, visual_terrain_revision(7, (-3, 12), &tiles));

        let mut changed_tiles = tiles;
        changed_tiles[0].source.tile_index += 1;
        assert_ne!(
            baseline,
            visual_terrain_revision(7, (-4, 12), &changed_tiles)
        );

        let mut changed_texture = complete_visual_grid();
        changed_texture[0].texture = Handle::weak_from_u128(10_000);
        assert_eq!(
            baseline,
            visual_terrain_revision(7, (-4, 12), &changed_texture)
        );
    }
}
