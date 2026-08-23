#[test]
fn town_map_markers_project_oam_coordinates_at_full_lcd_scale() {
    let player = crate::core::models::PokegearLandmark {
        id: 1,
        constant: "PLAYER".to_string(),
        label: "PLAYER".to_string(),
        name: "PLAYER".to_string(),
        region: "JOHTO".to_string(),
        x: 40,
        y: 56,
    };
    let cursor = crate::core::models::PokegearLandmark {
        id: 2,
        constant: "CURSOR".to_string(),
        label: "CURSOR".to_string(),
        name: "CURSOR".to_string(),
        region: "JOHTO".to_string(),
        x: 88,
        y: 96,
    };

    let rects = town_map_marker_rects(&player, &cursor);

    assert_eq!(rects.iter().filter(|rect| rect.kind == TownMapMarkerKind::Player).count(), 0);
    assert_eq!(
        rects
            .iter()
            .filter(|rect| rect.kind == TownMapMarkerKind::Cursor)
            .count(),
        8
    );
    assert_eq!(rects[0].center, Vec2::new(75.5, 73.0));
    assert_eq!(rects[0].size, Vec2::new(7.0, 2.0));
    assert_eq!(TILE_SIZE / SOURCE_TILE_SIZE as f32, 4.0);
}

#[test]
fn town_map_labels_wrap_at_the_asm_panel_width() {
    assert_eq!(town_map_label_lines("NATIONAL PARK"), ["NATIONAL", "PARK"]);
    assert_eq!(town_map_label_lines("NEW BARK TOWN"), ["NEW BARK", "TOWN"]);
    assert_eq!(town_map_label_lines("ROUTE 29"), ["ROUTE 29", ""]);
}

#[test]
fn standalone_town_map_frame_matches_the_asm_header_tiles() {
    let mut tilemap = vec![0; 20 * 18];
    apply_standalone_town_map_frame(&mut tilemap);

    assert_eq!(&tilemap[..8], &[0x06, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x17]);
    assert_eq!(tilemap[20 + 7], 0x16);
    assert_eq!(tilemap[40 + 7], 0x26);
    assert_eq!(&tilemap[40 + 8..40 + 20], &[0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x17]);
}

#[test]
fn town_map_frame_reserves_the_two_row_landmark_label_panel() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let palette_map: serde_json::Value = serde_json::from_slice(
        &crate::read_runtime_asset(
            asset_root
                .runtime_assets()
                .join("data/pokegear_town_map_palette_map.json"),
        )
        .expect("read Pokégear palette map"),
    )
    .expect("decode Pokégear palette map");
    let tokens = |key: &str| {
        palette_map[key]
            .as_array()
            .expect("palette token array")
            .iter()
            .map(|token| token.as_str().expect("palette token").to_string())
            .collect::<Vec<_>>()
    };
    let town = tokens("town_map");
    let pokegear = tokens("pokegear");
    let mut images = Assets::<Image>::default();

    let frame = load_town_map_frame(&asset_root, "johto", 0, &town, &pokegear, false, &mut images)
        .expect("render Johto Town Map");
    let image = images.get(&frame.handle).expect("Town Map image");
    let pixel = |x: usize, y: usize| {
        let offset = (y * 160 + x) * 4;
        &image.data[offset..offset + 4]
    };
    let panel = pixel(9 * 8, 8).to_vec();

    assert!((0..16).all(|y| (9 * 8..160).all(|x| pixel(x, y) == panel)));
    assert!(
        (0..8).any(|y| (8 * 8..9 * 8).any(|x| pixel(x, y) != panel)),
        "the map-pin icon must remain at tile (8, 0)"
    );
}

#[test]
fn pokegear_rle_decoder_expands_the_authored_clock_screen() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let bytes =
        crate::read_runtime_asset(repo_root.join("apps/web/assets/gfx/pokegear/clock.tilemap.rle"))
            .expect("read clock tilemap");

    let tilemap = decode_pokegear_card_tilemap(&bytes).expect("decode clock tilemap");

    assert_eq!(tilemap.len(), 20 * 18);
    assert_eq!(
        &tilemap[12..20],
        &[0x30, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x7f, 0x31]
    );
    assert_eq!(tilemap[4 * 20 + 2], 0x06);
    assert_eq!(tilemap[10 * 20 + 2], 0x26);
}

#[test]
fn pokegear_non_map_cards_render_their_distinct_authored_layouts() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let palette_map: serde_json::Value = serde_json::from_slice(
        &crate::read_runtime_asset(
            asset_root
                .runtime_assets()
                .join("data/pokegear_town_map_palette_map.json"),
        )
        .expect("read Pokégear palette map"),
    )
    .expect("decode Pokégear palette map");
    let tokens = |key: &str| {
        palette_map[key]
            .as_array()
            .expect("palette token array")
            .iter()
            .map(|token| token.as_str().expect("palette token").to_string())
            .collect::<Vec<_>>()
    };
    let town = tokens("town_map");
    let pokegear = tokens("pokegear");
    let mut images = Assets::<Image>::default();
    let mut signatures = Vec::new();

    for page in [
        PokegearPage::Clock,
        PokegearPage::Phone,
        PokegearPage::Radio,
    ] {
        let frame = load_pokegear_card_frame(
            &asset_root,
            page,
            0,
            0b1111,
            true,
            &town,
            &pokegear,
            &mut images,
        )
        .expect("render Pokégear card");
        let image = images.get(&frame.handle).expect("Pokégear card image");
        assert_eq!((image.width(), image.height()), (160, 144));
        assert!(
            image
                .data
                .chunks_exact(4)
                .collect::<std::collections::HashSet<_>>()
                .len()
                >= 5,
            "{page:?} rendered as a flat placeholder"
        );
        signatures.push(image.data.clone());
    }

    assert_ne!(signatures[0], signatures[1]);
    assert_ne!(signatures[1], signatures[2]);
    assert_ne!(signatures[0], signatures[2]);
}
