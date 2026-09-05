/// A source texel is four logical units in the existing retro compositor.
/// Choose whole physical pixels, keeping the UI about 480 CSS pixels wide on
/// desktops. Larger displays reveal terrain instead of enlarging the LCD.
fn fullscreen_pixels_per_world_unit(physical_size: Vec2, dpi: f32) -> f32 {
    let display = physical_size.max(Vec2::ONE);
    let available = Vec2::new(CLASSIC_SCROLL_WIDTH, CLASSIC_SCROLL_HEIGHT)
        - Vec2::splat(2.0 * f32::from(METATILE_WIDTH) * TILE_SIZE);
    let required = (display / available * 4.0).max_element().ceil().max(1.0);
    let fits_lcd = (display / Vec2::new(160.0, 144.0)).min_element();
    let preferred = (3.0 * dpi).round().max(required);
    let source_scale = if fits_lcd >= 1.0 {
        preferred.min(fits_lcd.floor())
    } else {
        fits_lcd
    };
    source_scale / (4.0 * dpi)
}

fn sync_fullscreen_scaling(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut cameras: Query<&mut OrthographicProjection, With<MainCameraMarker>>,
    mut overlays: Query<&mut Sprite, Or<(With<ScreenFadeOverlay>, With<PoisonFlashOverlay>)>>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    let physical_size = Vec2::new(
        window.physical_width() as f32,
        window.physical_height() as f32,
    );
    if physical_size.min_element() <= 0.0 {
        return;
    }
    let pixel_scale = fullscreen_pixels_per_world_unit(physical_size, window.scale_factor());
    for mut projection in &mut cameras {
        let mode = bevy::render::camera::ScalingMode::WindowSize(pixel_scale);
        if !matches!(projection.scaling_mode, bevy::render::camera::ScalingMode::WindowSize(current) if current == pixel_scale)
        {
            projection.scaling_mode = mode;
        }
    }
    let size = physical_size / window.scale_factor() / pixel_scale;
    for mut sprite in &mut overlays {
        if sprite.custom_size != Some(size) {
            sprite.custom_size = Some(size);
        }
    }
}

#[cfg(test)]
mod fullscreen_scaling_tests {
    use super::*;

    #[test]
    fn fullscreen_desktop_reveals_world_and_keeps_retro_ui_compact() {
        for dpi in [1.0, 2.0] {
            let scale = fullscreen_pixels_per_world_unit(Vec2::new(1920.0, 1080.0) * dpi, dpi);
            assert_eq!(640.0 * scale, 480.0);
            assert_eq!(4.0 * scale * dpi, (4.0 * scale * dpi).round());
            assert!(1920.0 / scale > PLAYFIELD_WIDTH * 3.0);
        }
    }

    #[test]
    fn fullscreen_resize_preserves_ui_and_scroll_margin() {
        for (width, height) in [
            (320.0, 568.0),
            (640.0, 576.0),
            (2560.0, 1440.0),
            (3840.0, 2160.0),
            (7680.0, 4320.0),
        ] {
            let display = Vec2::new(width, height);
            let scale = fullscreen_pixels_per_world_unit(display, 1.0);
            let view = display / scale;
            assert!(view.x >= PLAYFIELD_WIDTH && view.y >= PLAYFIELD_HEIGHT);
            assert!(view.x <= CLASSIC_SCROLL_WIDTH - 2.0 * f32::from(METATILE_WIDTH) * TILE_SIZE);
            assert!(view.y <= CLASSIC_SCROLL_HEIGHT - 2.0 * f32::from(METATILE_WIDTH) * TILE_SIZE);
            assert_eq!(4.0 * scale, (4.0 * scale).round());
        }
    }

    #[test]
    fn fullscreen_multiplayer_players_beyond_original_lcd_have_positions() {
        assert!(runtime_tile_playfield_position(TilePosition::new(20, 10), 0, 0).is_some());
    }

    #[test]
    fn fullscreen_actor_depth_orders_rows_beyond_the_lcd_below_roofs() {
        let north = overworld_entity_depth(TilePosition::new(8, 20), None, (0, 0));
        let south = overworld_entity_depth(TilePosition::new(8, 25), None, (0, 0));
        assert!(north < south);
        assert!(north > 0.9 && south < 2.4);
    }

    #[test]
    fn fullscreen_resize_updates_camera_and_whole_screen_effects() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_systems(Startup, setup_shell_view)
            .add_systems(Update, sync_fullscreen_scaling);
        app.world_mut().spawn((
            Window {
                resolution: WindowResolution::new(1920.0, 1080.0).with_scale_factor_override(1.0),
                ..default()
            },
            bevy::window::PrimaryWindow,
        ));
        app.update();
        let world = app.world_mut();
        let projection = world
            .query_filtered::<&OrthographicProjection, With<MainCameraMarker>>()
            .single(world);
        assert!(matches!(projection.scaling_mode,
            bevy::render::camera::ScalingMode::WindowSize(scale) if scale == 0.75));
        let overlay = world
            .query_filtered::<&Sprite, With<ScreenFadeOverlay>>()
            .single(world);
        assert_eq!(overlay.custom_size, Some(Vec2::new(2560.0, 1440.0)));
    }
}

#[derive(Component)]
struct FullscreenSceneBackdrop;
#[derive(Component)]
struct FullscreenBootDialogue;

#[derive(Component)]
enum FullscreenTitlePiece {
    Artwork,
    Clock,
}

fn setup_fullscreen_scene(mut commands: Commands) {
    commands.spawn((SpatialBundle::default(), FullscreenWorldRoot));
    commands.spawn((SpatialBundle::default(), FullscreenDialogRoot));
    commands.spawn((
        SpriteBundle {
            visibility: Visibility::Hidden,
            transform: Transform::from_xyz(0.0, 0.0, 5.8),
            ..default()
        },
        FullscreenBootDialogue,
    ));
    commands.spawn((
        SpriteBundle {
            transform: Transform::from_xyz(0.0, 0.0, PRESENTED_FULLSCREEN_BASE_Z - 0.01),
            visibility: Visibility::Hidden,
            ..default()
        },
        FullscreenSceneBackdrop,
    ));
}

fn spawn_fullscreen_title_artwork(
    commands: &mut Commands,
    runtime_shell: &BevyRuntimeShell,
    title: &TitleMenu,
    menu: &SpriteFrame,
    rendered_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
) -> Result<()> {
    let art = title_screen_frame_for_art(rendered_art, &runtime_shell.asset_root, title, images)
        .context("fullscreen menu requires the native title artwork")?;
    commands.spawn((
        SpriteBundle {
            texture: art.handle,
            transform: Transform::from_xyz(0.0, 0.0, PRESENTED_FULLSCREEN_BASE_Z),
            ..default()
        },
        TitleScreenMarker,
        FullscreenTitlePiece::Artwork,
    ));
    if title_continue_save_path(runtime_shell, title).is_some() {
        commands.spawn((
            SpriteBundle {
                texture: menu.handle.clone(),
                transform: Transform::from_xyz(0.0, 0.0, PRESENTED_FULLSCREEN_BASE_Z + 0.01),
                ..default()
            },
            TitleScreenMarker,
            FullscreenTitlePiece::Clock,
        ));
    }
    Ok(())
}

fn fullscreen_art_size(native: Vec2, available: Vec2, physical_pixels_per_unit: f32) -> Vec2 {
    let fit = (available.max(Vec2::ONE) / native).min_element() * physical_pixels_per_unit;
    let pixels = if fit >= 1.0 { fit.floor() } else { fit };
    native * pixels / physical_pixels_per_unit
}

struct FullscreenTitleLayout {
    art: Rect,
    menu: Vec2,
    clock: Vec2,
}

fn fullscreen_title_layout(
    view: Vec2,
    menu_size: Vec2,
    clock_height: f32,
) -> FullscreenTitleLayout {
    let padding = view.min_element() * 0.025;
    let gap = padding;
    let clock_space = if clock_height > 0.0 {
        clock_height + gap
    } else {
        0.0
    };
    if view.x > view.y * 1.35 && view.x > menu_size.x + 640.0 + padding * 3.0 {
        let right = view.x * 0.5 - padding;
        let column_width = if clock_height > 0.0 {
            menu_size.x.max(640.0)
        } else {
            menu_size.x
        };
        FullscreenTitleLayout {
            art: Rect::from_corners(
                Vec2::new(-view.x * 0.5, -view.y * 0.5 + padding),
                Vec2::new(right - column_width - gap, view.y * 0.5 - padding),
            ),
            menu: Vec2::new(right - menu_size.x * 0.5, 0.0),
            clock: Vec2::new(right - 320.0, -view.y * 0.5 + padding + clock_height * 0.5),
        }
    } else {
        let menu_y = -view.y * 0.5 + padding + clock_space + menu_size.y * 0.5;
        FullscreenTitleLayout {
            art: Rect::from_corners(
                Vec2::new(-view.x * 0.5, menu_y + menu_size.y * 0.5 + gap),
                Vec2::new(view.x * 0.5, view.y * 0.5 - padding),
            ),
            menu: Vec2::new(0.0, menu_y),
            clock: Vec2::new(0.0, -view.y * 0.5 + padding + clock_height * 0.5),
        }
    }
}

// The LCD presenter remains the source of truth for pixels and input. Only its
// display rectangle changes: title choices and the save clock become separate
// native-size panels, while artwork receives the remaining viewport space.
fn sync_fullscreen_scene_layout(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    runtime: Res<BevyRuntimeShell>,
    rendered: Res<RenderedViewport>,
    images: Res<Assets<Image>>,
    mut pieces: ParamSet<(
        Query<(&Handle<Image>, &mut Sprite, &mut Transform), With<VisibleIntroSurface>>,
        Query<(
            &FullscreenTitlePiece,
            &mut Sprite,
            &mut Transform,
            &mut Visibility,
        )>,
        Query<(&mut Sprite, &mut Visibility), With<FullscreenSceneBackdrop>>,
        Query<
            (
                &mut Handle<Image>,
                &mut Sprite,
                &mut Transform,
                &mut Visibility,
            ),
            With<FullscreenBootDialogue>,
        >,
    )>,
) {
    let Ok(window) = windows.get_single() else {
        return;
    };
    let physical = Vec2::new(
        window.physical_width() as f32,
        window.physical_height() as f32,
    );
    if physical.min_element() <= 0.0 {
        return;
    }
    let scale = fullscreen_pixels_per_world_unit(physical, window.scale_factor());
    let pixels_per_unit = scale * window.scale_factor();
    let view = physical / pixels_per_unit;
    let title_menu = runtime.title_menu.as_ref().filter(|title| {
        visible_title_main_menu_active(title)
            && !runtime.options_menu_open
            && runtime.visible_continue_screen.is_none()
    });
    let naming = runtime
        .pending_name_choice
        .as_ref()
        .filter(|choice| choice.player_phase == Some(VisiblePlayerNameChoicePhase::Menu));
    let mut boot_dialogue = None;
    let mut background = None;
    let mut layout = None;
    {
        let mut presenters = pieces.p0();
        if let Ok((texture, mut sprite, mut transform)) = presenters.get_single_mut() {
            sprite.rect = None;
            sprite.custom_size = Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT));
            transform.translation.x = 0.0;
            transform.translation.y = 0.0;
            if rendered.title_active {
                background = images
                    .get(texture)
                    .and_then(|image| image.data.get(..4))
                    .map(|p| Color::srgb_u8(p[0], p[1], p[2]));
                if let Some(choice) = naming {
                    if let Some(menu) = choice.player_menu.as_ref() {
                        let menu_size = Vec2::new(
                            (menu.right - menu.left + 1) as f32,
                            (menu.bottom - menu.top + 1) as f32,
                        ) * TILE_SIZE;
                        let positions =
                            fullscreen_title_layout(view - Vec2::new(0.0, 224.0), menu_size, 0.0);
                        let x = (6 + usize::from(
                            choice.motion_step.min(menu.motion_steps.saturating_sub(1)),
                        )) as f32
                            * 8.0;
                        sprite.rect = Some(Rect::from_corners(
                            Vec2::new(x, 32.0),
                            Vec2::new(x + 56.0, 88.0),
                        ));
                        sprite.custom_size = Some(fullscreen_art_size(
                            Vec2::splat(56.0),
                            positions.art.size(),
                            pixels_per_unit,
                        ));
                        transform.translation.x = positions.art.center().x;
                        transform.translation.y = positions.art.center().y + 112.0;
                        boot_dialogue = Some(texture.clone());
                    }
                } else if let Some(title) = title_menu {
                    let rect = Rect::from_corners(
                        Vec2::new(title.main_menu.left as f32, title.main_menu.top as f32)
                            * SOURCE_TILE_SIZE as f32,
                        Vec2::new(
                            (title.main_menu.right + 1) as f32,
                            (title.main_menu.bottom + 1) as f32,
                        ) * SOURCE_TILE_SIZE as f32,
                    );
                    let menu_size = rect.size() * 4.0;
                    let clock_height = if title_continue_save_path(&runtime, title).is_some() {
                        128.0
                    } else {
                        0.0
                    };
                    let positions = fullscreen_title_layout(view, menu_size, clock_height);
                    sprite.rect = Some(rect);
                    sprite.custom_size = Some(menu_size);
                    transform.translation.x = positions.menu.x;
                    transform.translation.y = positions.menu.y;
                    background = Some(Color::BLACK);
                    layout = Some(positions);
                } else if runtime.intro_screen.is_some()
                    || runtime
                        .title_menu
                        .as_ref()
                        .is_some_and(|title| !visible_title_main_menu_active(title))
                {
                    sprite.custom_size = Some(fullscreen_art_size(
                        Vec2::new(160.0, 144.0),
                        view,
                        pixels_per_unit,
                    ));
                }
            }
        }
    }
    for (kind, mut sprite, mut transform, mut visibility) in &mut pieces.p1() {
        *visibility = if layout.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let Some(layout) = layout.as_ref() else {
            continue;
        };
        let center = match kind {
            FullscreenTitlePiece::Artwork => {
                sprite.custom_size = Some(fullscreen_art_size(
                    Vec2::new(160.0, 144.0),
                    layout.art.size(),
                    pixels_per_unit,
                ));
                layout.art.center()
            }
            FullscreenTitlePiece::Clock => {
                sprite.rect = Some(Rect::from_corners(
                    Vec2::new(0.0, 112.0),
                    Vec2::new(160.0, 144.0),
                ));
                sprite.custom_size = Some(Vec2::new(640.0, 128.0));
                layout.clock
            }
        };
        transform.translation.x = center.x;
        transform.translation.y = center.y;
    }
    for (mut texture, mut sprite, mut transform, mut visibility) in &mut pieces.p3() {
        *visibility = if boot_dialogue.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let Some(handle) = boot_dialogue.as_ref() {
            *texture = handle.clone();
            sprite.rect = Some(Rect::from_corners(
                Vec2::new(0.0, 96.0),
                Vec2::new(160.0, 144.0),
            ));
            sprite.custom_size = Some(Vec2::new(640.0, 192.0));
            transform.translation.y = -view.y * 0.5 + 112.0;
        }
    }
    for (mut sprite, mut visibility) in &mut pieces.p2() {
        *visibility = if background.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let Some(color) = background {
            sprite.color = color;
            sprite.custom_size = Some(view);
        }
    }
}

#[cfg(test)]
mod responsive_world_tests {
    use super::*;

    #[test]
    fn fullscreen_small_room_uses_available_height_without_enlarging_dialogue() {
        let view = Vec2::new(2560.0, 1440.0);
        let (zoom, center) = fullscreen_world_layout(view, Vec2::new(640.0, 576.0));
        assert!(zoom > 1.8);
        assert!(576.0 * zoom <= view.y - 224.0);
        assert!(center.y > 0.0);
        let (outdoor_zoom, _) = fullscreen_world_layout(view, Vec2::splat(4000.0));
        assert_eq!(outdoor_zoom, 1.0);
    }
}

#[derive(Component)]
struct FullscreenWorldRoot;
#[derive(Component)]
struct FullscreenDialogRoot;

fn fullscreen_world_layout(view: Vec2, map: Vec2) -> (f32, Vec2) {
    // Reserve the original six-row textbox at the bottom, independently of
    // terrain magnification. Large maps retain the expanded native viewport.
    let available = Vec2::new(view.x - 64.0, view.y - 224.0).max(Vec2::ONE);
    let fit = (available / map).min_element();
    if fit > 1.0 {
        (fit, Vec2::new(0.0, 96.0))
    } else {
        (1.0, Vec2::ZERO)
    }
}

fn sync_fullscreen_world_layout(
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    runtime: Res<BevyRuntimeShell>,
    rendered: Res<RenderedViewport>,
    timer: Res<RuntimeTickTimer>,
    mut roots: Query<
        (&mut Transform, Option<&FullscreenWorldRoot>),
        Or<(With<FullscreenWorldRoot>, With<FullscreenDialogRoot>)>,
    >,
    world_root: Query<Entity, With<FullscreenWorldRoot>>,
    dialog_root: Query<Entity, With<FullscreenDialogRoot>>,
    world_entities: Query<
        Entity,
        (
            Without<Parent>,
            Or<(
                With<PlayfieldTile>,
                With<PlayerMarker>,
                With<MultiplayerGhost>,
                With<ObjectMarker>,
                With<JumpShadowMarker>,
                With<GrassRustleMarker>,
            )>,
        ),
    >,
    dialog_entities: Query<Entity, (Without<Parent>, With<SceneDialogMarker>)>,
    #[cfg(feature = "voxel-view")] voxel: Option<Res<crystal_voxel_view::VoxelViewSettings>>,
) {
    let (Ok(window), Ok(world_root), Ok(dialog_root)) = (
        windows.get_single(),
        world_root.get_single(),
        dialog_root.get_single(),
    ) else {
        return;
    };
    for entity in &world_entities {
        commands.entity(entity).set_parent(world_root);
    }
    for entity in &dialog_entities {
        commands.entity(entity).set_parent(dialog_root);
    }
    let physical = Vec2::new(
        window.physical_width() as f32,
        window.physical_height() as f32,
    );
    let view = physical
        / window.scale_factor()
        / fullscreen_pixels_per_world_unit(physical, window.scale_factor());
    let mut world_transform = Transform::IDENTITY;
    let mut dialog_transform = Transform::IDENTITY;
    let active = !rendered.title_active && rendered.map_name.is_some();
    #[cfg(feature = "voxel-view")]
    let active = active && !voxel.is_some_and(|settings| settings.enabled);
    if active {
        dialog_transform.translation.y = -view.y * 0.5 + PLAYFIELD_HEIGHT * 0.5 + 16.0;
        if let Some((width, height)) = rendered
            .map_name
            .as_deref()
            .and_then(|name| runtime.runtime.data().saved_map_tile_bounds(name))
        {
            let map = Vec2::new(width as f32, height as f32) * (TILE_SIZE * 2.0);
            let (zoom, center) = fullscreen_world_layout(view, map);
            if zoom > 1.0 {
                if let Some((x, y)) = rendered.viewport_origin {
                    let map_center = Vec2::new(
                        -PLAYFIELD_WIDTH * 0.5 + map.x * 0.5 - x as f32 * TILE_SIZE,
                        PLAYFIELD_HEIGHT * 0.5 - map.y * 0.5 + y as f32 * TILE_SIZE,
                    ) + visible_overworld_camera_offset(
                        &rendered,
                        &runtime,
                        timer.presentation_subframe(),
                    );
                    world_transform.scale = Vec3::new(zoom, zoom, 1.0);
                    world_transform.translation = (center - map_center * zoom).extend(0.0);
                }
            }
        }
    }
    if let Some(menu) = runtime
        .pending_name_choice
        .as_ref()
        .filter(|choice| choice.player_phase == Some(VisiblePlayerNameChoicePhase::Menu))
        .and_then(|choice| choice.player_menu.as_ref())
    {
        let size = Vec2::new(
            (menu.right - menu.left + 1) as f32,
            (menu.bottom - menu.top + 1) as f32,
        ) * TILE_SIZE;
        let position = fullscreen_title_layout(view - Vec2::new(0.0, 224.0), size, 0.0).menu
            + Vec2::new(0.0, 112.0);
        let (x, y) = field_window_center(
            menu.left as f32,
            menu.top as f32,
            size.x / TILE_SIZE,
            size.y / TILE_SIZE,
        );
        dialog_transform.translation = (position - Vec2::new(x, y)).extend(0.0);
    }
    for (mut transform, world) in &mut roots {
        *transform = if world.is_some() {
            world_transform
        } else {
            dialog_transform
        };
    }
}
