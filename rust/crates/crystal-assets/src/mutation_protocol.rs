#[cfg(test)]
fn read_compiled_game_pack(path: impl AsRef<Path>) -> Result<CompiledGamePack> {
    let (_, _, pack) = read_loaded_compiled_game_pack(path)?.into_parts();
    Ok(pack)
}

fn read_loaded_compiled_game_pack(path: impl AsRef<Path>) -> Result<LoadedCompiledGamePack> {
    let path = path.as_ref();
    validate_compiled_game_pack_path(path)?;
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let pack = decode_compiled_game_pack(&bytes, path)?;
    Ok(LoadedCompiledGamePack {
        path: path.to_path_buf(),
        bytes,
        pack,
    })
}

pub fn read_verified_compiled_game_pack(path: impl AsRef<Path>) -> Result<CompiledGamePack> {
    let (_, _, pack) = read_loaded_verified_compiled_game_pack(path)?.into_parts();
    Ok(pack)
}

pub fn read_loaded_verified_compiled_game_pack(
    path: impl AsRef<Path>,
) -> Result<LoadedCompiledGamePack> {
    let loaded = read_loaded_compiled_game_pack(path)?;
    verify_compiled_game_pack_for_runtime(loaded.pack())?;
    Ok(loaded)
}

pub fn load_verified_compiled_game_pack_bytes(
    path: impl Into<PathBuf>,
    bytes: Vec<u8>,
) -> Result<LoadedCompiledGamePack> {
    let path = path.into();
    validate_compiled_game_pack_path(&path)?;
    let pack = decode_compiled_game_pack(&bytes, &path)?;
    verify_compiled_game_pack_for_runtime(&pack)?;
    Ok(LoadedCompiledGamePack { path, bytes, pack })
}

/// A complete, already-materialized runtime map addition for an existing
/// compiled pack. This is the production boundary for generators and other
/// tools that add maps without teaching the faithful runtime how they were
/// produced.
#[derive(Debug, Clone)]
pub struct CompiledMapExtension {
    pub manifest_id: String,
    pub map_name: String,
    pub map_constant: String,
    pub module: MapModule,
    pub metadata: RuntimeMapMetadata,
    pub spawn_key: String,
    pub spawn: RuntimeSpawnPoint,
    pub wild_encounters: Option<WildEncounterData>,
    pub start_new_game_here: bool,
}

/// A complete tileset addition for an existing compiled pack.
///
/// Tileset behavior lives in `definition`; the exact renderer inputs remain
/// embedded runtime files. Keeping both sides in one typed mutation prevents a
/// generated map from naming collision data whose metatile or pixel art was
/// never shipped in the playable pack.
#[derive(Debug, Clone)]
pub struct CompiledTilesetExtension {
    pub manifest_id: String,
    pub tileset_id: String,
    /// Existing tileset whose block-replacement behavior this derived tileset
    /// inherits (Cut/Whirlpool). Art and collision remain independently owned.
    pub behavior_source_tileset_id: Option<String>,
    pub definition: TilesetDefinition,
    pub metatiles: Vec<u8>,
    pub tile_graphics_2bpp: Vec<u8>,
    pub tile_graphics_png: Vec<u8>,
}

impl CompiledGamePack {
    /// Adds one complete tileset and recalculates the definitive pack identity.
    /// Existing tilesets and runtime files remain byte-for-byte unchanged.
    pub fn with_tileset_extension(&self, extension: CompiledTilesetExtension) -> Result<Self> {
        verify_compiled_game_pack_for_runtime(self)?;
        validate_compiled_tileset_extension(&extension)?;
        let mut pack = self.clone();
        insert_compiled_tileset_extension(&mut pack, extension)?;
        pack.identity = derive_compiled_game_pack_identity_from_manifest(
            pack.format_version,
            &pack.data,
            &pack.audio_manifest,
            &pack.runtime_files,
            &pack.report,
        )?;
        verify_compiled_game_pack_for_runtime(&pack)?;
        Ok(pack)
    }

    /// Adds one standalone map and recalculates the definitive pack identity.
    /// Existing maps and catalogs remain byte-for-byte unchanged.
    pub fn with_map_extension(&self, extension: CompiledMapExtension) -> Result<Self> {
        self.with_map_extensions([extension])
    }

    /// Adds a mutually-referencing set of complete maps atomically.
    ///
    /// This is required for honest generated buildings with interiors: the
    /// outdoor door and the indoor return warp must both exist before runtime
    /// verification resolves either target. Each map is still validated at
    /// the same production boundary, and the pack identity is derived once
    /// from the complete verified result.
    pub fn with_map_extensions(
        &self,
        extensions: impl IntoIterator<Item = CompiledMapExtension>,
    ) -> Result<Self> {
        verify_compiled_game_pack_for_runtime(self)?;
        let extensions = extensions.into_iter().collect::<Vec<_>>();
        if extensions.is_empty() {
            anyhow::bail!("compiled map extension set cannot be empty");
        }
        for extension in &extensions {
            validate_compiled_map_extension(extension)?;
        }
        let mut pack = self.clone();
        for extension in extensions {
            insert_compiled_map_extension(&mut pack, extension)?;
        }
        pack.report.maps = pack.data.maps.len();
        pack.report.reachable_maps.sort();
        pack.report.solvable_maps.sort();
        pack.identity = derive_compiled_game_pack_identity_from_manifest(
            pack.format_version,
            &pack.data,
            &pack.audio_manifest,
            &pack.runtime_files,
            &pack.report,
        )?;
        verify_compiled_game_pack_for_runtime(&pack)?;
        Ok(pack)
    }

    /// Writes a verified pack without changing its existing embedded/sidecar
    /// audio storage mode. This is required when extending an already compiled
    /// pack because its PCM payloads may already be compressed.
    pub fn write_preserving_storage(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        validate_compiled_game_pack_path(path)?;
        verify_compiled_game_pack_for_runtime(self)?;
        write_serialized_compiled_game_pack(path, self)
    }
}

fn insert_compiled_tileset_extension(
    pack: &mut CompiledGamePack,
    extension: CompiledTilesetExtension,
) -> Result<()> {
    if pack.data.tilesets.contains_key(&extension.tileset_id) {
        anyhow::bail!(
            "compiled pack already contains tileset '{}'",
            extension.tileset_id
        );
    }
    if let Some(source) = extension.behavior_source_tileset_id.as_deref() {
        if !pack.data.tilesets.contains_key(source) {
            anyhow::bail!("compiled pack has no behavior-source tileset '{source}'");
        }
        clone_block_replacement_behavior(
            &mut pack.data.field_moves.cut.replacements,
            source,
            &extension.tileset_id,
        )?;
        clone_block_replacement_behavior(
            &mut pack.data.field_moves.whirlpool.replacements,
            source,
            &extension.tileset_id,
        )?;
    }

    let metatile_path = format!("data/tilesets/{}_metatiles.bin", extension.tileset_id);
    let collision_path = format!("data/tilesets/{}.json", extension.tileset_id);
    let palette_path = format!("data/tilesets/{}_palette_map.json", extension.tileset_id);
    let graphics_2bpp_path = format!("gfx/tilesets/{}.2bpp", extension.tileset_id);
    let graphics_png_path = format!("gfx/tilesets/{}.png", extension.tileset_id);
    for path in [
        &metatile_path,
        &collision_path,
        &palette_path,
        &graphics_2bpp_path,
        &graphics_png_path,
    ] {
        if pack.runtime_files.contains_key(path) {
            anyhow::bail!("compiled pack already contains runtime file '{path}'");
        }
    }

    let collision_json = serde_json::to_vec(&extension.definition.collision)
        .context("encode compiled tileset collision JSON")?;
    let palette_json = serde_json::to_vec(&extension.definition.palette_map)
        .context("encode compiled tileset palette-map JSON")?;
    insert_keyed_tileset_definition(
        &mut pack.data.tilesets,
        extension.tileset_id,
        extension.definition,
    )?;
    pack.runtime_files
        .insert(metatile_path, extension.metatiles);
    pack.runtime_files.insert(collision_path, collision_json);
    pack.runtime_files.insert(palette_path, palette_json);
    pack.runtime_files
        .insert(graphics_2bpp_path, extension.tile_graphics_2bpp);
    pack.runtime_files
        .insert(graphics_png_path, extension.tile_graphics_png);
    pack.report.manifests.push(extension.manifest_id);
    Ok(())
}

fn validate_compiled_tileset_extension(extension: &CompiledTilesetExtension) -> Result<()> {
    const METATILE_BYTES: usize = 16;
    const TILE_BYTES_2BPP: usize = 16;
    const MAX_METATILES: usize = 256;
    const MAX_TILES: usize = 256;

    if !is_exact_manifest_id_token(&extension.manifest_id) {
        anyhow::bail!("compiled tileset extension manifest id is invalid");
    }
    if !is_exact_tileset_id(&extension.tileset_id) {
        anyhow::bail!(
            "compiled tileset extension id '{}' must be an exact asset id",
            extension.tileset_id
        );
    }
    if let Some(source) = extension.behavior_source_tileset_id.as_deref() {
        if !is_exact_tileset_id(source) || source == extension.tileset_id {
            anyhow::bail!(
                "compiled tileset extension behavior source '{source}' must be a distinct exact asset id"
            );
        }
    }
    if extension.metatiles.is_empty() || !extension.metatiles.len().is_multiple_of(METATILE_BYTES) {
        anyhow::bail!(
            "compiled tileset extension '{}' metatile data must be nonempty and divisible by {METATILE_BYTES}",
            extension.tileset_id
        );
    }
    let metatile_count = extension.metatiles.len() / METATILE_BYTES;
    if metatile_count > MAX_METATILES {
        anyhow::bail!(
            "compiled tileset extension '{}' has {metatile_count} metatiles but Crystal block ids are bytes",
            extension.tileset_id
        );
    }
    let collisions =
        tileset_collision_from_definition(&extension.tileset_id, &extension.definition)
            .with_context(|| {
                format!(
                    "validate compiled tileset extension '{}' collision data",
                    extension.tileset_id
                )
            })?;
    if collisions.metatiles.len() != metatile_count {
        anyhow::bail!(
            "compiled tileset extension '{}' has {metatile_count} art metatiles but {} collision metatiles",
            extension.tileset_id,
            collisions.metatiles.len()
        );
    }

    if extension.tile_graphics_2bpp.is_empty()
        || !extension
            .tile_graphics_2bpp
            .len()
            .is_multiple_of(TILE_BYTES_2BPP)
    {
        anyhow::bail!(
            "compiled tileset extension '{}' 2bpp data must be nonempty and divisible by {TILE_BYTES_2BPP}",
            extension.tileset_id
        );
    }
    let source_tile_count = extension.tile_graphics_2bpp.len() / TILE_BYTES_2BPP;
    if source_tile_count > MAX_TILES || !source_tile_count.is_multiple_of(2) {
        anyhow::bail!(
            "compiled tileset extension '{}' must contain an even number of at most {MAX_TILES} packed VRAM tiles, found {source_tile_count}",
            extension.tileset_id
        );
    }
    let (png_width, png_height) =
        png_dimensions(&extension.tile_graphics_png).with_context(|| {
            format!(
                "validate compiled tileset extension '{}' PNG",
                extension.tileset_id
            )
        })?;
    if png_width % 8 != 0 || png_height % 8 != 0 {
        anyhow::bail!(
            "compiled tileset extension '{}' PNG dimensions {png_width}x{png_height} are not aligned to 8x8 tiles",
            extension.tileset_id
        );
    }
    let png_tile_count = usize::try_from(png_width / 8)? * usize::try_from(png_height / 8)?;
    if png_tile_count != source_tile_count {
        anyhow::bail!(
            "compiled tileset extension '{}' PNG has {png_tile_count} tiles but its 2bpp data has {source_tile_count}",
            extension.tileset_id
        );
    }

    let tiles_per_bank = source_tile_count / 2;
    for &tile_id in &extension.metatiles {
        let palette_value = extension
            .definition
            .palette_map
            .get(usize::from(tile_id))
            .copied()
            .with_context(|| {
                format!(
                    "compiled tileset extension '{}' metatile art references tile {tile_id:#04x} beyond its palette map",
                    extension.tileset_id
                )
            })?;
        if palette_value > 0x0f {
            anyhow::bail!(
                "compiled tileset extension '{}' referenced tile {tile_id:#04x} has invalid palette/bank value {palette_value:#04x}",
                extension.tileset_id
            );
        }
        let bank = usize::from((palette_value >> 3) & 1);
        let tile_in_bank = usize::from(tile_id & 0x7f);
        if tile_in_bank >= tiles_per_bank {
            anyhow::bail!(
                "compiled tileset extension '{}' referenced tile {tile_id:#04x} addresses missing VRAM bank {bank} tile {tile_in_bank:#04x}",
                extension.tileset_id
            );
        }
    }
    Ok(())
}

fn clone_block_replacement_behavior<T: Clone>(
    replacements: &mut BTreeMap<String, T>,
    source: &str,
    target: &str,
) -> Result<()> {
    let Some(source_rules) = replacements.get(source).cloned() else {
        return Ok(());
    };
    if replacements.insert(target.to_string(), source_rules).is_some() {
        anyhow::bail!("field-move behavior already exists for tileset '{target}'");
    }
    Ok(())
}

fn png_dimensions(bytes: &[u8]) -> Result<(u32, u32)> {
    const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() < 24 || &bytes[..8] != PNG_SIGNATURE || &bytes[12..16] != b"IHDR" {
        anyhow::bail!("tile graphics are not a PNG with a leading IHDR chunk");
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into()?);
    if width == 0 || height == 0 {
        anyhow::bail!("tile graphics PNG dimensions must be nonzero");
    }
    Ok((width, height))
}

#[cfg(test)]
mod compiled_tileset_extension_tests {
    use super::*;

    fn canonical_pack() -> CompiledGamePack {
        let repository_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let asset_root = AssetRoot::new(repository_root);
        let data = asset_root
            .load_base_game_data()
            .expect("load canonical game data");
        let runtime_files = compile_runtime_files(&asset_root).expect("compile runtime files");
        let report = canonical_test_compile_report(&data, "tileset-extension-base");
        CompiledGamePack::new_unchecked_for_tests(data, report)
            .with_runtime_files_for_tests(runtime_files)
    }

    fn clone_johto_extension(pack: &CompiledGamePack) -> CompiledTilesetExtension {
        let files = pack.runtime_files();
        CompiledTilesetExtension {
            manifest_id: "tileset-extension-test".to_string(),
            tileset_id: "johto_generated_test".to_string(),
            behavior_source_tileset_id: Some("johto_modern".to_string()),
            definition: pack.data().tilesets["johto_modern"].clone(),
            metatiles: files["data/tilesets/johto_modern_metatiles.bin"].clone(),
            tile_graphics_2bpp: files["gfx/tilesets/johto_modern.2bpp"].clone(),
            tile_graphics_png: files["gfx/tilesets/johto_modern.png"].clone(),
        }
    }

    #[test]
    fn compiled_tileset_extension_adds_catalog_and_renderer_files_atomically() {
        let base = canonical_pack();
        let original_identity = base.identity().expect("base identity");
        let extension = clone_johto_extension(&base);

        let extended = base
            .with_tileset_extension(extension)
            .expect("extend compiled tileset");

        assert!(!base.data().tilesets.contains_key("johto_generated_test"));
        assert!(
            extended
                .data()
                .tilesets
                .contains_key("johto_generated_test")
        );
        for path in [
            "data/tilesets/johto_generated_test_metatiles.bin",
            "data/tilesets/johto_generated_test.json",
            "data/tilesets/johto_generated_test_palette_map.json",
            "gfx/tilesets/johto_generated_test.2bpp",
            "gfx/tilesets/johto_generated_test.png",
        ] {
            assert!(
                extended.runtime_files().contains_key(path),
                "missing {path}"
            );
            assert!(!base.runtime_files().contains_key(path));
        }
        assert_eq!(
            serde_json::from_slice::<BTreeMap<String, Vec<String>>>(
                &extended.runtime_files()["data/tilesets/johto_generated_test.json"]
            )
            .expect("collision JSON"),
            extended.data().tilesets["johto_generated_test"].collision
        );
        assert_eq!(
            extended.data().field_moves.cut.replacements["johto_generated_test"],
            extended.data().field_moves.cut.replacements["johto_modern"]
        );
        assert_ne!(
            extended.identity().expect("extended identity"),
            original_identity
        );
        verify_compiled_game_pack_for_runtime(&extended).expect("verified extended pack");
    }

    #[test]
    fn compiled_tileset_extension_rejects_art_collision_count_drift() {
        let base = canonical_pack();
        let mut extension = clone_johto_extension(&base);
        extension.metatiles.pop();

        let error = base
            .with_tileset_extension(extension)
            .expect_err("misaligned metatile art must fail");
        assert!(
            error
                .to_string()
                .contains("metatile data must be nonempty and divisible"),
            "{error:#}"
        );
    }
}

fn insert_compiled_map_extension(
    pack: &mut CompiledGamePack,
    extension: CompiledMapExtension,
) -> Result<()> {
    if pack.data.maps.contains_key(&extension.map_name) {
        anyhow::bail!(
            "compiled pack already contains map '{}'",
            extension.map_name
        );
    }
    if pack
        .data
        .runtime_map_metadata
        .contains_key(&extension.map_constant)
    {
        anyhow::bail!(
            "compiled pack already contains map constant '{}'",
            extension.map_constant
        );
    }
    if pack
        .data
        .runtime_spawn_points
        .contains_key(&extension.spawn_key)
    {
        anyhow::bail!(
            "compiled pack already contains spawn key '{}'",
            extension.spawn_key
        );
    }

    let blocks_label = extension
        .module
        .attributes
        .blocks_label
        .as_deref()
        .context("compiled map extension requires blocks_label")?
        .to_string();
    let scripts_label = extension
        .module
        .attributes
        .map_scripts_label
        .as_deref()
        .context("compiled map extension requires map_scripts_label")?
        .to_string();
    let events_label = extension
        .module
        .attributes
        .map_events_label
        .as_deref()
        .context("compiled map extension requires map_events_label")?
        .to_string();
    if pack.data.map_blocks.contains_key(&blocks_label) {
        anyhow::bail!("compiled pack already contains block label '{blocks_label}'");
    }
    for label in extension
        .module
        .scripts
        .keys()
        .chain(extension.module.script_text_bodies.keys())
        .chain([&scripts_label, &events_label])
    {
        if pack.data.map_scripts.contains_key(label) {
            anyhow::bail!("compiled pack already contains map script label '{label}'");
        }
    }
    if pack.data.npcs.contains_key(&extension.map_name) {
        anyhow::bail!(
            "compiled pack already contains NPC payload for map '{}'",
            extension.map_name
        );
    }
    let block_bytes = extension
        .module
        .blocks
        .iter()
        .map(|&block| {
            u8::try_from(block).with_context(|| {
                format!("compiled map extension block id {block} exceeds Crystal byte range")
            })
        })
        .collect::<Result<Vec<_>>>()?;
    pack.data
        .map_blocks
        .insert(blocks_label, encode_base64_bytes(&block_bytes));
    pack.data.map_dimensions.insert(
        extension.map_constant.clone(),
        serde_json::json!({
            "width": extension.module.attributes.width,
            "height": extension.module.attributes.height,
        }),
    );
    pack.data.map_attributes.insert(
        extension.map_name.clone(),
        extension.module.attributes.clone(),
    );
    insert_compiled_map_split_payloads(
        &mut pack.data.map_scripts,
        &mut pack.data.npcs,
        &extension.map_name,
        &scripts_label,
        &events_label,
        &extension.module,
    )?;
    pack.data
        .runtime_map_metadata
        .insert(extension.map_constant.clone(), extension.metadata);
    pack.data
        .runtime_spawn_points
        .insert(extension.spawn_key, extension.spawn.clone());
    pack.data
        .maps
        .insert(extension.map_name.clone(), extension.module);
    if let Some(encounters) = extension.wild_encounters.clone() {
        insert_wild_encounter_data(&mut pack.data.wild_encounters, encounters)?;
    }
    if extension.start_new_game_here {
        pack.data.story_event_script_constants.global.insert(
            "SPAWN_HOME".to_string(),
            i64::from(extension.spawn.identifier),
        );
    }

    pack.report.manifests.push(extension.manifest_id);
    if !pack.report.reachable_maps.contains(&extension.map_name) {
        pack.report.reachable_maps.push(extension.map_name.clone());
    }
    if !pack.report.solvable_maps.contains(&extension.map_name) {
        pack.report.solvable_maps.push(extension.map_name);
    }
    Ok(())
}

fn insert_compiled_map_split_payloads(
    map_scripts: &mut BTreeMap<String, Value>,
    npcs: &mut BTreeMap<String, Value>,
    map_name: &str,
    scripts_label: &str,
    events_label: &str,
    module: &MapModule,
) -> Result<()> {
    for (label, payload) in &module.scripts {
        map_scripts.insert(label.clone(), payload.clone());
    }
    // Some generated text is constructed directly as typed bodies instead of
    // beginning life as raw ASM-shaped JSON. Materialize those labels too so
    // a split-payload reload retains every sign and resident line.
    for (label, body) in &module.script_text_bodies {
        map_scripts.entry(label.clone()).or_insert_with(|| {
            Value::Array(
                body.commands
                    .iter()
                    .map(|command| {
                        serde_json::json!({
                            "command": command.command,
                            "args": command.args,
                        })
                    })
                    .collect(),
            )
        });
    }
    let script_section = module
        .map_script_section_commands
        .iter()
        .map(|command| {
            serde_json::json!({
                "command": command.command,
                "args": command.args,
            })
        })
        .collect();
    let event_section = module
        .map_event_section_commands
        .iter()
        .map(|command| {
            serde_json::json!({
                "command": command.command,
                "args": command.args,
            })
        })
        .collect();
    map_scripts.insert(scripts_label.to_string(), Value::Array(script_section));
    map_scripts.insert(events_label.to_string(), Value::Array(event_section));
    npcs.insert(
        map_name.to_string(),
        serde_json::to_value(&module.objects).context("encode compiled map NPC payload")?,
    );
    Ok(())
}

fn validate_compiled_map_extension(extension: &CompiledMapExtension) -> Result<()> {
    if !is_exact_manifest_id_token(&extension.manifest_id) {
        anyhow::bail!("compiled map extension manifest id is invalid");
    }
    if extension.module.id != extension.map_name {
        anyhow::bail!(
            "compiled map extension module id '{}' must match map name '{}'",
            extension.module.id,
            extension.map_name
        );
    }
    if extension.metadata.name != extension.map_name
        || extension.metadata.constant != extension.map_constant
    {
        anyhow::bail!("compiled map extension metadata must match its map name and constant");
    }
    if extension.spawn.map_name != extension.map_name
        || extension.spawn.map_constant != extension.map_constant
    {
        anyhow::bail!("compiled map extension spawn must target the added map");
    }
    if let Some(encounters) = &extension.wild_encounters {
        if encounters.map_name != extension.map_name {
            anyhow::bail!(
                "compiled map extension wild encounters target '{}' instead of '{}'",
                encounters.map_name,
                extension.map_name
            );
        }
        validate_wild_encounter_species_tokens(&extension.map_name, encounters)?;
    }
    if extension.spawn_key != extension.spawn.identifier.to_string() {
        anyhow::bail!("compiled map extension spawn key must equal its numeric identifier");
    }
    let expected =
        extension.module.attributes.width as usize * extension.module.attributes.height as usize;
    if extension.module.blocks.len() != expected {
        anyhow::bail!(
            "compiled map extension has {} blocks but dimensions require {expected}",
            extension.module.blocks.len()
        );
    }
    extension
        .module
        .attributes
        .validate()
        .map_err(anyhow::Error::msg)
}

fn encode_base64_bytes(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        encoded.push(ALPHABET[usize::from(first >> 2)] as char);
        encoded.push(ALPHABET[usize::from((first & 0x03) << 4 | second >> 4)] as char);
        encoded.push(if chunk.len() > 1 {
            ALPHABET[usize::from((second & 0x0f) << 2 | third >> 6)] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            ALPHABET[usize::from(third & 0x3f)] as char
        } else {
            '='
        });
    }
    encoded
}

pub fn verify_compiled_game_pack_for_runtime(pack: &CompiledGamePack) -> Result<()> {
    if pack.format_version != COMPILED_GAME_PACK_FORMAT_VERSION {
        anyhow::bail!(
            "compiled game pack has unsupported format version {}",
            pack.format_version
        );
    }
    validate_compiled_game_pack_identity(pack)?;
    validate_compiled_report_manifest_identity(&pack.report)?;
    validate_compiled_report_data_counts(&pack.report, &pack.data)?;
    validate_compiled_report_map_references(&pack.report, &pack.data)?;
    validate_compiled_report_progression_outputs(&pack.report, &pack.data)?;
    validate_compiled_runtime_game_data(&pack.data)?;
    validate_compiled_audio_payloads(pack)?;

    let mut diagnostics = pack
        .report
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == VerificationSeverity::Error)
        .cloned()
        .collect::<Vec<_>>();
    let mut runtime_diagnostics = Vec::new();
    verify_runtime_pack_data(&pack.data, &mut runtime_diagnostics);
    diagnostics.extend(
        runtime_diagnostics
            .into_iter()
            .filter(|diagnostic| diagnostic.severity == VerificationSeverity::Error),
    );
    if diagnostics.is_empty() {
        return Ok(());
    }

    let summary = diagnostics
        .into_iter()
        .take(8)
        .map(|diagnostic| {
            format!(
                "{:?} {} [{}]: {}",
                diagnostic.severity, diagnostic.subject, diagnostic.code, diagnostic.message
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    anyhow::bail!("compiled game pack is not verified for runtime: {summary}")
}

fn validate_compiled_game_pack_identity(pack: &CompiledGamePack) -> Result<()> {
    validate_compiled_runtime_files(&pack.runtime_files)?;
    let derived = if matches!(
        pack.audio_compression.as_deref(),
        Some(PACK_AUDIO_COMPRESSION_GZIP | PACK_AUDIO_COMPRESSION_MIDI)
    ) {
        derive_compiled_game_pack_identity_from_manifest(
            pack.format_version,
            &pack.data,
            &pack.audio_manifest,
            &pack.runtime_files,
            &pack.report,
        )?
    } else {
        derive_compiled_game_pack_identity(
            pack.format_version,
            &pack.data,
            &pack.compiled_audio,
            &pack.runtime_files,
            &pack.report,
        )?
    };
    if pack.identity != derived {
        anyhow::bail!(
            "compiled game pack identity {} does not match derived identity {}",
            pack.identity.content_hash,
            derived.content_hash
        );
    }
    Ok(())
}

pub fn validate_compiled_runtime_files(runtime_files: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    for key in runtime_files.keys() {
        validate_compiled_runtime_file_key(key)?;
    }
    if runtime_files.is_empty() {
        return Ok(());
    }
    for &key in REQUIRED_VENDOR_RUNTIME_FILE_KEYS {
        let bytes = runtime_files.get(key).with_context(|| {
            format!("compiled runtime file bundle is missing required vendor asset '{key}'")
        })?;
        if bytes.is_empty() {
            anyhow::bail!("compiled runtime vendor asset '{key}' must not be empty");
        }
    }
    Ok(())
}

pub fn validate_compiled_runtime_file_key(key: &str) -> Result<()> {
    if key.is_empty() {
        anyhow::bail!("compiled runtime file key must not be empty");
    }
    let path = Path::new(key);
    let bytes = key.as_bytes();
    let has_windows_prefix = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    if path.is_absolute() || key.starts_with('/') || key.starts_with('\\') || has_windows_prefix {
        anyhow::bail!("compiled runtime file key '{key}' must be relative");
    }
    let components = key
        .split(|character| character == '/' || character == '\\')
        .collect::<Vec<_>>();
    if components.contains(&"..") {
        anyhow::bail!("compiled runtime file key '{key}' must not traverse parent directories");
    }
    if components.contains(&".") {
        anyhow::bail!(
            "compiled runtime file key '{key}' must not include current-directory components"
        );
    }
    if components.contains(&"") {
        anyhow::bail!("compiled runtime file key '{key}' must not contain empty path components");
    }
    if key.contains('\\') {
        anyhow::bail!("compiled runtime file key '{key}' must use forward-slash separators");
    }
    Ok(())
}

fn validate_compiled_runtime_game_data(data: &GameDataSet) -> Result<()> {
    if data.pokemon.is_empty() {
        anyhow::bail!("compiled game pack has no Pokemon species data");
    }
    if data.moves.is_empty() {
        anyhow::bail!("compiled game pack has no move data");
    }
    if data.maps.is_empty() {
        anyhow::bail!("compiled game pack has no map modules");
    }
    if data.audio.is_empty() {
        anyhow::bail!("compiled game pack has no audio catalog");
    }
    if data.pokemon_cries.is_empty() {
        anyhow::bail!("compiled game pack has no Pokemon cry metadata");
    }
    for (map_name, module) in &data.maps {
        if module.attributes.width == 0 || module.attributes.height == 0 {
            anyhow::bail!("compiled game pack map '{map_name}' has empty dimensions");
        }
        let expected_blocks = module.attributes.width as usize * module.attributes.height as usize;
        if module.blocks.len() != expected_blocks {
            anyhow::bail!(
                "compiled game pack map '{map_name}' has {} blocks but dimensions require {expected_blocks}",
                module.blocks.len()
            );
        }
    }
    Ok(())
}

fn validate_compiled_report_manifest_identity(report: &ModpackCompileReport) -> Result<()> {
    compiled_game_pack_runtime_modpack_id(report)?;
    Ok(())
}

fn validate_compiled_report_data_counts(
    report: &ModpackCompileReport,
    data: &GameDataSet,
) -> Result<()> {
    validate_compiled_report_data_count("maps", report.maps, data.maps.len())?;
    validate_compiled_report_data_count("pokemon", report.pokemon, data.pokemon.len())?;
    validate_compiled_report_data_count("moves", report.moves, data.moves.len())?;
    validate_compiled_report_data_count("items", report.items, data.items.len())?;
    Ok(())
}

fn validate_compiled_audio_payloads(pack: &CompiledGamePack) -> Result<()> {
    if let Some(compression) = pack.audio_compression.as_deref()
        && compression != PACK_AUDIO_COMPRESSION_GZIP
        && compression != PACK_AUDIO_COMPRESSION_MIDI
    {
        anyhow::bail!("unsupported compiled audio storage mode '{compression}'");
    }
    if pack.audio_compression.as_deref() == Some(PACK_AUDIO_COMPRESSION_MIDI) {
        if !pack.compiled_audio.is_empty() {
            anyhow::bail!("MIDI audio pack must not embed PCM payloads");
        }
        let expected_manifest =
            ModpackAudioManifest::from_assets(&pack.data.audio, &BTreeMap::new())?;
        if pack.audio_manifest != expected_manifest {
            anyhow::bail!("MIDI audio manifest does not match definitive audio metadata");
        }
        for asset in &pack.data.audio {
            if !matches!(asset.source, ModpackAudioSource::Midi) {
                anyhow::bail!(
                    "MIDI audio pack contains non-MIDI audio asset '{}'",
                    asset.id
                );
            }
            let entry = match asset.kind {
                ModpackAudioKind::Music => pack.audio_manifest.music.get(&asset.id),
                ModpackAudioKind::SoundEffect => {
                    pack.audio_manifest.sound_effects.get(&asset.id)
                }
                ModpackAudioKind::Cry => pack.audio_manifest.cries.get(&asset.id),
            }
            .with_context(|| format!("MIDI audio pack is missing manifest '{}'", asset.id))?;
            entry.validate()?;
            asset
                .midi_program
                .as_ref()
                .with_context(|| format!("MIDI audio pack is missing program '{}'", asset.id))?
                .validate(&asset.id)?;
        }
        return Ok(());
    }
    if pack.audio_compression.as_deref() == Some(PACK_AUDIO_COMPRESSION_GZIP) {
        for asset in &pack.data.audio {
            if !pack.compiled_audio.contains_key(&asset.id) {
                anyhow::bail!(
                    "compiled game pack is missing embedded audio payload '{}'",
                    asset.id
                );
            }
            let entry = match asset.kind {
                ModpackAudioKind::Music => pack.audio_manifest.music.get(&asset.id),
                ModpackAudioKind::SoundEffect => pack.audio_manifest.sound_effects.get(&asset.id),
                ModpackAudioKind::Cry => pack.audio_manifest.cries.get(&asset.id),
            }
            .with_context(|| {
                format!(
                    "compiled game pack is missing audio manifest '{}'",
                    asset.id
                )
            })?;
            entry.validate()?;
        }
        return Ok(());
    }
    let declared_audio = pack
        .data
        .audio
        .iter()
        .map(|asset| asset.id.as_str())
        .collect::<BTreeSet<_>>();
    for asset in &pack.data.audio {
        match pack.compiled_audio.get(&asset.id) {
            Some(bytes) => validate_compiled_audio_payload(asset, bytes)?,
            None if matches!(asset.source, ModpackAudioSource::Pcm) => {
                let manifest =
                    ModpackAudioManifest::from_assets(&pack.data.audio, &pack.compiled_audio)?;
                let entry = manifest
                    .music
                    .get(&asset.id)
                    .or_else(|| manifest.sound_effects.get(&asset.id))
                    .or_else(|| manifest.cries.get(&asset.id))
                    .with_context(|| {
                        format!(
                            "compiled game pack missing PCM manifest entry '{}'",
                            asset.id
                        )
                    })?;
                entry.validate()?;
            }
            None => anyhow::bail!(
                "compiled game pack is missing embedded audio payload '{}'",
                asset.id
            ),
        }
    }
    for audio_id in pack.compiled_audio.keys() {
        if !declared_audio.contains(audio_id.as_str()) {
            anyhow::bail!(
                "compiled game pack includes embedded audio payload '{}' not declared by pack data",
                audio_id
            );
        }
    }
    Ok(())
}

fn validate_compiled_report_data_count(field: &str, reported: usize, actual: usize) -> Result<()> {
    if reported != actual {
        anyhow::bail!(
            "compiled game pack report {field} count {reported} does not match embedded data count {actual}"
        );
    }
    Ok(())
}

fn validate_compiled_report_map_references(
    report: &ModpackCompileReport,
    data: &GameDataSet,
) -> Result<()> {
    let map_names = data.maps.keys().cloned().collect::<BTreeSet<_>>();
    validate_compiled_report_map_list("reachable_maps", &report.reachable_maps, &map_names)?;
    validate_compiled_report_map_list("solvable_maps", &report.solvable_maps, &map_names)?;
    let reachable_maps: BTreeSet<&str> = report.reachable_maps.iter().map(String::as_str).collect();
    let granted_maps = declared_progression_maps(&data.playability);
    for map in &report.solvable_maps {
        if !reachable_maps.contains(map.as_str()) && !granted_maps.contains(map.as_str()) {
            anyhow::bail!(
                "compiled game pack report solvable_maps references map '{map}' that is neither reachable nor declared by embedded playability rules"
            );
        }
    }
    for edge in &report.graph_edges {
        validate_compiled_report_map_reference("graph_edges.from", &edge.from, &map_names)?;
        validate_compiled_report_map_reference("graph_edges.to", &edge.to, &map_names)?;
        validate_compiled_report_graph_edge_kind(&edge.kind)?;
    }
    Ok(())
}

fn validate_compiled_report_map_list(
    field: &str,
    maps: &[String],
    map_names: &BTreeSet<String>,
) -> Result<()> {
    let mut seen = BTreeSet::new();
    for map in maps {
        if !seen.insert(map) {
            anyhow::bail!("compiled game pack report {field} includes duplicate map '{map}'");
        }
        validate_compiled_report_map_reference(field, map, map_names)?;
    }
    Ok(())
}

fn validate_compiled_report_map_reference(
    field: &str,
    map: &str,
    map_names: &BTreeSet<String>,
) -> Result<()> {
    if !map_names.contains(map) {
        anyhow::bail!(
            "compiled game pack report {field} references map '{map}' that is not embedded in pack data"
        );
    }
    Ok(())
}

fn validate_compiled_report_graph_edge_kind(kind: &str) -> Result<()> {
    if kind.is_empty()
        || kind.trim() != kind
        || !kind
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        anyhow::bail!("compiled game pack report graph_edges.kind '{kind}' must be an exact token");
    }
    Ok(())
}

fn validate_compiled_report_progression_outputs(
    report: &ModpackCompileReport,
    data: &GameDataSet,
) -> Result<()> {
    validate_compiled_report_token_list("solvable_events", &report.solvable_events)?;
    validate_compiled_report_token_list("solvable_items", &report.solvable_items)?;
    let declared_events = declared_progression_events(&data.playability);
    for event in &report.solvable_events {
        if !declared_events.contains(event.as_str()) {
            anyhow::bail!(
                "compiled game pack report solvable_events references event '{event}' that is not declared by embedded playability rules"
            );
        }
    }
    let declared_items = declared_progression_items(&data.playability);
    for item in &report.solvable_items {
        if !data.items.contains_key(item) {
            anyhow::bail!(
                "compiled game pack report solvable_items references item '{item}' that is not embedded in pack data"
            );
        }
        if !declared_items.contains(item.as_str()) {
            anyhow::bail!(
                "compiled game pack report solvable_items references item '{item}' that is not declared by embedded playability rules"
            );
        }
    }
    Ok(())
}

fn declared_progression_maps(rules: &PlayabilityRules) -> BTreeSet<&str> {
    rules
        .progression_rules
        .iter()
        .flat_map(|rule| rule.grants.maps.iter().map(String::as_str))
        .collect()
}

fn declared_progression_events(rules: &PlayabilityRules) -> BTreeSet<&str> {
    rules
        .initial_events
        .iter()
        .map(String::as_str)
        .chain(
            rules
                .progression_rules
                .iter()
                .flat_map(|rule| rule.grants.events.iter().map(String::as_str)),
        )
        .collect()
}

fn declared_progression_items(rules: &PlayabilityRules) -> BTreeSet<&str> {
    rules
        .initial_items
        .iter()
        .map(String::as_str)
        .chain(
            rules
                .progression_rules
                .iter()
                .flat_map(|rule| rule.grants.items.iter().map(String::as_str)),
        )
        .collect()
}

fn validate_compiled_report_token_list(field: &str, values: &[String]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        if value.is_empty()
            || value.trim() != value
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b':'))
        {
            anyhow::bail!(
                "compiled game pack report {field} value '{value}' must be an exact token"
            );
        }
        if !seen.insert(value) {
            anyhow::bail!("compiled game pack report {field} includes duplicate value '{value}'");
        }
    }
    Ok(())
}

fn compiled_game_pack_runtime_modpack_id(report: &ModpackCompileReport) -> Result<String> {
    if report.manifests.is_empty() {
        anyhow::bail!("compiled game pack report must include at least one manifest id");
    }
    let mut seen = BTreeSet::new();
    for manifest_id in &report.manifests {
        if !is_exact_manifest_id_token(manifest_id) {
            anyhow::bail!(
                "compiled game pack report manifest id '{}' must be exact ASCII letters, numbers, underscores, hyphens, or dots",
                manifest_id
            );
        }
        if !seen.insert(manifest_id) {
            anyhow::bail!(
                "compiled game pack report includes duplicate manifest id '{}'",
                manifest_id
            );
        }
    }
    let id = report.manifests.join("+");
    SaveModpackIdentity::validate_id(&id)?;
    Ok(id)
}

fn decode_compiled_game_pack(bytes: &[u8], path: &Path) -> Result<CompiledGamePack> {
    if !bytes.starts_with(COMPILED_GAME_PACK_MAGIC) {
        anyhow::bail!("{} is not a compiled Crystal game pack", path.display());
    }
    if bytes.len() < COMPILED_GAME_PACK_HEADER_LEN {
        anyhow::bail!(
            "compiled game pack {} is shorter than the required header",
            path.display()
        );
    }
    let frame_version = u16::from_be_bytes([
        bytes[COMPILED_GAME_PACK_VERSION_OFFSET],
        bytes[COMPILED_GAME_PACK_VERSION_OFFSET + 1],
    ]);
    if frame_version != COMPILED_GAME_PACK_FORMAT_VERSION {
        anyhow::bail!(
            "compiled game pack {} uses unsupported frame format version {}",
            path.display(),
            frame_version
        );
    }
    let declared = u32::from_be_bytes([
        bytes[COMPILED_GAME_PACK_PAYLOAD_LENGTH_OFFSET],
        bytes[COMPILED_GAME_PACK_PAYLOAD_LENGTH_OFFSET + 1],
        bytes[COMPILED_GAME_PACK_PAYLOAD_LENGTH_OFFSET + 2],
        bytes[COMPILED_GAME_PACK_PAYLOAD_LENGTH_OFFSET + 3],
    ]) as usize;
    let actual = bytes.len() - COMPILED_GAME_PACK_HEADER_LEN;
    if declared != actual {
        anyhow::bail!(
            "compiled game pack {} payload length {} does not match actual {}",
            path.display(),
            declared,
            actual
        );
    }
    if declared == 0 {
        anyhow::bail!("compiled game pack {} payload is empty", path.display());
    }
    let expected_hash = u32::from_be_bytes([
        bytes[COMPILED_GAME_PACK_PAYLOAD_HASH_OFFSET],
        bytes[COMPILED_GAME_PACK_PAYLOAD_HASH_OFFSET + 1],
        bytes[COMPILED_GAME_PACK_PAYLOAD_HASH_OFFSET + 2],
        bytes[COMPILED_GAME_PACK_PAYLOAD_HASH_OFFSET + 3],
    ]);
    let payload = &bytes[COMPILED_GAME_PACK_HEADER_LEN..];
    let actual_hash = fnv1a32_bytes(payload);
    if actual_hash != expected_hash {
        anyhow::bail!(
            "compiled game pack {} payload hash {actual_hash:#010x} does not match declared {expected_hash:#010x}",
            path.display()
        );
    }
    let mut cursor = std::io::Cursor::new(payload);
    let pack: CompiledGamePack = ciborium::from_reader(&mut cursor)
        .with_context(|| format!("decode compiled game pack {}", path.display()))?;
    if cursor.position() as usize != payload.len() {
        anyhow::bail!(
            "compiled game pack {} has {} trailing bytes",
            path.display(),
            payload.len() - cursor.position() as usize
        );
    }
    if pack.format_version != COMPILED_GAME_PACK_FORMAT_VERSION {
        anyhow::bail!(
            "compiled game pack {} uses unsupported format version {}",
            path.display(),
            pack.format_version
        );
    }
    Ok(pack)
}

pub const PACK_AUDIO_COMPRESSION_GZIP: &str = "gzip";
pub const PACK_AUDIO_COMPRESSION_MIDI: &str = "midi_synth_v1";

fn compress_pack_audio(pack: &mut CompiledGamePack) -> Result<()> {
    let mut compressed = BTreeMap::new();
    for (id, bytes) in &pack.compiled_audio {
        let Some(asset) = pack.data.audio.iter().find(|asset| asset.id == *id) else {
            anyhow::bail!("compiled audio payload {id} has no declared asset");
        };
        if !matches!(asset.source, ModpackAudioSource::Pcm) {
            compressed.insert(id.clone(), bytes.clone());
            continue;
        }
        let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
        std::io::Write::write_all(&mut encoder, bytes)
            .with_context(|| format!("compress compiled audio payload {id}"))?;
        compressed.insert(
            id.clone(),
            encoder
                .finish()
                .with_context(|| format!("finish compressed audio payload {id}"))?,
        );
    }
    pack.compiled_audio = compressed;
    pack.audio_compression = Some(PACK_AUDIO_COMPRESSION_GZIP.to_string());
    Ok(())
}

pub(crate) fn write_compiled_game_pack_with_midi_audio(
    path: impl AsRef<Path>,
    pack: &CompiledGamePack,
) -> Result<()> {
    let path = path.as_ref();
    validate_compiled_game_pack_path(path)?;
    validate_compiled_game_pack_identity(pack)
        .with_context(|| format!("validate browser game pack identity {}", path.display()))?;
    let mut serialized_pack = pack.clone();
    for asset in &mut serialized_pack.data.audio {
        asset
            .midi_program
            .as_ref()
            .with_context(|| format!("browser audio asset '{}' has no MIDI program", asset.id))?
            .validate(&asset.id)?;
        asset.source = ModpackAudioSource::Midi;
        asset.path = asset.path.strip_suffix(".pcm").with_context(|| {
            format!("browser audio asset '{}' does not use a .pcm source path", asset.id)
        })?.to_string() + ".mid";
        asset.validate()?;
    }
    serialized_pack.compiled_audio.clear();
    serialized_pack.audio_manifest =
        ModpackAudioManifest::from_assets(&serialized_pack.data.audio, &BTreeMap::new())?;
    serialized_pack.audio_compression = Some(PACK_AUDIO_COMPRESSION_MIDI.to_string());
    serialized_pack.identity = derive_compiled_game_pack_identity_from_manifest(
        serialized_pack.format_version,
        &serialized_pack.data,
        &serialized_pack.audio_manifest,
        &serialized_pack.runtime_files,
        &serialized_pack.report,
    )?;
    let audio_directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .context("browser game pack path has no parent directory")?
        .join("audio");
    if audio_directory.is_dir() {
        for entry in std::fs::read_dir(&audio_directory)
            .with_context(|| format!("read PCM sidecar directory {}", audio_directory.display()))?
        {
            let entry = entry.with_context(|| {
                format!("read PCM sidecar entry in {}", audio_directory.display())
            })?;
            let stale_sidecar = entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.ends_with(".pcm.gz"))
                && entry
                    .file_type()
                    .with_context(|| format!("inspect PCM sidecar {}", entry.path().display()))?
                    .is_file();
            if stale_sidecar {
                std::fs::remove_file(entry.path()).with_context(|| {
                    format!("remove obsolete PCM sidecar {}", entry.path().display())
                })?;
            }
        }
    }
    write_serialized_compiled_game_pack(path, &serialized_pack)?;
    Ok(())
}

fn validate_content_pack_audio_metadata_entry(pack_id: &str, entry: &str) -> Result<()> {
    validate_content_pack_entry_segments(pack_id, entry)?;
    let extension = Path::new(entry)
        .extension()
        .and_then(|extension| extension.to_str());
    if extension != Some("json") {
        anyhow::bail!(
            "content pack {pack_id} audio entry {entry} must point to explicit audio metadata JSON"
        );
    }
    Ok(())
}

fn validate_content_pack_json_entry(
    pack_id: &str,
    category: ContentPackCategory,
    entry: &str,
) -> Result<()> {
    validate_content_pack_entry_segments(pack_id, entry)?;
    let extension = Path::new(entry)
        .extension()
        .and_then(|extension| extension.to_str());
    if extension != Some("json") {
        anyhow::bail!(
            "content pack {pack_id} category {} entry {entry} must point to explicit JSON data",
            category.as_str()
        );
    }
    Ok(())
}

fn validate_content_pack_compiled_entry(pack_id: &str, entry: &str) -> Result<()> {
    validate_content_pack_entry_segments(pack_id, entry)?;
    if Path::new(entry)
        .extension()
        .and_then(|extension| extension.to_str())
        != Some(COMPILED_GAME_PACK_EXTENSION)
    {
        anyhow::bail!(
            "content pack {pack_id} compiled entry {entry} must point to an explicit .{COMPILED_GAME_PACK_EXTENSION} artifact"
        );
    }
    Ok(())
}

fn validate_content_pack_entry_segments(pack_id: &str, entry: &str) -> Result<()> {
    if entry.split('/').any(|segment| segment == ".") {
        anyhow::bail!(
            "content pack {pack_id} path '{entry}' must not include current-directory components"
        );
    }
    if entry.split('/').any(|segment| segment == "..") {
        anyhow::bail!("content pack {pack_id} path '{entry}' must not traverse parent directories");
    }
    Ok(())
}

fn resolve_content_pack_compiled_game_pack_path(
    asset_root: &AssetRoot,
    pack_id: &str,
    entry: &str,
) -> Result<PathBuf> {
    validate_content_pack_compiled_entry(pack_id, entry)?;
    let path = Path::new(entry);
    let resolved = resolve_compiled_game_pack_data_path(asset_root, path)?;
    let mut components = path.components();
    let expected_pack_root = format!("content-packs/{pack_id}");
    let pack_root_matches = matches!(
        (components.next(), components.next()),
        (Some(Component::Normal(root)), Some(Component::Normal(id)))
            if root == OsStr::new("content-packs") && id == OsStr::new(pack_id)
    );
    if !pack_root_matches {
        anyhow::bail!(
            "content pack {pack_id} compiled entry {entry} must be under {expected_pack_root}"
        );
    }
    Ok(resolved)
}

fn resolve_content_pack_data_path(
    asset_root: &AssetRoot,
    pack_id: &str,
    entry: &str,
) -> Result<PathBuf> {
    let path = Path::new(entry);
    if path.is_absolute() {
        anyhow::bail!("content pack {pack_id} path '{entry}' must be relative to assets/data");
    }
    if entry.starts_with("assets/data/") {
        anyhow::bail!(
            "content pack {pack_id} path '{entry}' must not include the assets/data prefix"
        );
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        anyhow::bail!("content pack {pack_id} path '{entry}' must not traverse parent directories");
    }
    if path
        .components()
        .any(|component| matches!(component, Component::CurDir))
    {
        anyhow::bail!(
            "content pack {pack_id} path '{entry}' must not include current-directory components"
        );
    }
    let mut components = path.components();
    let expected_pack_root = format!("content-packs/{pack_id}");
    let pack_root_matches = matches!(
        (components.next(), components.next()),
        (Some(Component::Normal(root)), Some(Component::Normal(id)))
            if root == OsStr::new("content-packs") && id == OsStr::new(pack_id)
    );
    if !pack_root_matches {
        anyhow::bail!("content pack {pack_id} path '{entry}' must be under {expected_pack_root}");
    }
    asset_root.resolve_data_path(path)
}

fn resolve_compiled_game_pack_data_path(asset_root: &AssetRoot, entry: &Path) -> Result<PathBuf> {
    if entry.is_absolute() {
        anyhow::bail!(
            "compiled game pack path '{}' must be relative to assets/data",
            entry.display()
        );
    }
    let entry_text = entry.to_string_lossy();
    if entry_text.starts_with("assets/data/") {
        anyhow::bail!(
            "compiled game pack path '{entry_text}' must not include the assets/data prefix"
        );
    }
    if entry
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        anyhow::bail!(
            "compiled game pack path '{}' must not traverse parent directories",
            entry.display()
        );
    }
    if path_contains_current_directory_alias(entry) {
        anyhow::bail!(
            "compiled game pack path '{}' must not include current-directory components",
            entry.display()
        );
    }
    asset_root.resolve_data_path(entry)
}

fn validate_compiled_game_pack_path(path: &Path) -> Result<()> {
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        anyhow::bail!(
            "compiled game pack {} must not traverse parent directories",
            path.display()
        );
    }
    if path_contains_current_directory_alias(path) {
        anyhow::bail!(
            "compiled game pack {} must not include current-directory components",
            path.display()
        );
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "compiled game pack {} must have a file extension",
                path.display()
            )
        })?;
    if extension != COMPILED_GAME_PACK_EXTENSION {
        anyhow::bail!(
            "compiled game pack {} must use .{}",
            path.display(),
            COMPILED_GAME_PACK_EXTENSION
        );
    }
    Ok(())
}

fn path_contains_current_directory_alias(path: &Path) -> bool {
    let text = path.to_string_lossy();
    text == "."
        || text.starts_with("./")
        || text.ends_with("/.")
        || text.contains("/./")
        || text.starts_with(".\\")
        || text.ends_with("\\.")
        || text.contains("\\.\\")
}

fn parse_object_map<T>(payload: Value) -> Result<BTreeMap<String, T>>
where
    T: DeserializeOwned,
{
    parse_object_map_with_description(payload, "object-map payload")
}

fn parse_object_map_with_description<T>(
    payload: Value,
    description: &str,
) -> Result<BTreeMap<String, T>>
where
    T: DeserializeOwned,
{
    let parsed: BTreeMap<String, T> = serde_json::from_value(payload)
        .map_err(|error| anyhow::anyhow!("parse {description}: {error}"))?;
    if parsed.is_empty() {
        anyhow::bail!("{description} must contain at least one entry");
    }
    Ok(parsed)
}

fn parse_string_vec_payload(payload: Value, description: &str) -> Result<Vec<String>> {
    let parsed: Vec<String> =
        serde_json::from_value(payload).with_context(|| format!("parse {description} payload"))?;
    if parsed.is_empty() {
        anyhow::bail!("{description} payload must contain at least one entry");
    }
    Ok(parsed)
}

fn parse_learnsets(payload: Value) -> Result<SpeciesLearnsets> {
    let mut learnsets = SpeciesLearnsets::new();
    let Some(object) = payload.as_object() else {
        anyhow::bail!("learnset payload must be a species-keyed object");
    };
    if object.is_empty() {
        anyhow::bail!("learnset payload must contain at least one entry");
    }
    for (species, entry) in object {
        validate_modpack_payload_token(species, "learnset species")?;
        merge_keyed_learnset_entry(&mut learnsets, species, entry.clone())?;
    }
    Ok(learnsets)
}

fn merge_keyed_learnset_entry(
    learnsets: &mut SpeciesLearnsets,
    key: &str,
    payload: Value,
) -> Result<()> {
    #[derive(Deserialize)]
    struct Entry {
        species: String,
        learnset: Vec<LearnsetEntry>,
    }

    let entry: Entry = serde_json::from_value(payload).context("parse learnset entry")?;
    if entry.species != key {
        anyhow::bail!(
            "learnset key '{key}' does not match record species '{}'",
            entry.species
        );
    }
    validate_modpack_payload_token(&entry.species, "learnset species")?;
    insert_learnset(learnsets, entry.species, entry.learnset)?;
    Ok(())
}

fn merge_learnsets(target: &mut SpeciesLearnsets, source: SpeciesLearnsets) -> Result<()> {
    for (species, learnset) in source {
        insert_learnset(target, species, learnset)?;
    }
    Ok(())
}

fn insert_learnset(
    target: &mut SpeciesLearnsets,
    species: String,
    learnset: Vec<LearnsetEntry>,
) -> Result<()> {
    validate_modpack_payload_token(&species, "learnset species")?;
    for LearnsetEntry(_, move_id) in &learnset {
        validate_modpack_payload_token(move_id, &format!("learnset move for species '{species}'"))?;
    }
    if target.contains_key(&species) {
        anyhow::bail!("duplicate learnset for species '{species}'");
    }
    target.insert(species, learnset);
    Ok(())
}

fn merge_evolution_payload(target: &mut EvolutionTable, payload: Value) -> Result<()> {
    let Some(object) = payload.as_object() else {
        anyhow::bail!("evolution payload must be a species-keyed object");
    };
    if object.is_empty() {
        anyhow::bail!("evolution payload must contain at least one entry");
    }
    for (species, entry) in object {
        validate_modpack_payload_token(species, "evolution species")?;
        merge_keyed_evolution_entry(target, species, entry.clone())?;
    }
    Ok(())
}

fn merge_keyed_evolution_entry(
    target: &mut EvolutionTable,
    key: &str,
    payload: Value,
) -> Result<()> {
    #[derive(Deserialize)]
    struct Entry {
        species: String,
        evolutions: Vec<EvolutionEntry>,
    }

    let entry: Entry = serde_json::from_value(payload).context("parse evolution entry")?;
    if entry.species != key {
        anyhow::bail!(
            "evolution key '{key}' does not match record species '{}'",
            entry.species
        );
    }
    validate_modpack_payload_token(&entry.species, "evolution species")?;
    insert_evolutions(target, entry.species, entry.evolutions)?;
    Ok(())
}

fn merge_evolution_table(target: &mut EvolutionTable, source: &EvolutionTable) -> Result<()> {
    for (species, entries) in &source.0 {
        insert_evolutions(target, species.clone(), entries.clone())?;
    }
    Ok(())
}

fn insert_evolutions(
    target: &mut EvolutionTable,
    species: String,
    entries: Vec<EvolutionEntry>,
) -> Result<()> {
    validate_modpack_payload_token(&species, "evolution species")?;
    for (index, entry) in entries.iter().enumerate() {
        validate_evolution_entry(&species, index, entry)?;
    }
    if target.0.contains_key(&species) {
        anyhow::bail!("duplicate evolutions for species '{species}'");
    }
    target.0.insert(species, entries);
    Ok(())
}

fn validate_evolution_entry(species: &str, index: usize, entry: &EvolutionEntry) -> Result<()> {
    validate_modpack_payload_token(
        &entry.species,
        &format!("evolution entry {index} target species for '{species}'"),
    )?;
    validate_battle_table_token(
        &entry.method,
        &format!("evolution entry {index} method for '{species}'"),
    )?;
    if !is_known_evolution_method(&entry.method) {
        anyhow::bail!(
            "evolution entry {index} for '{species}' has unknown method '{}'",
            entry.method
        );
    }
    if let Some(item_id) = entry.item.as_deref() {
        validate_modpack_payload_token(
            item_id,
            &format!("evolution entry {index} item for '{species}'"),
        )?;
    }
    if let Some(held_item) = entry.held_item.as_deref()
        && held_item != TRADE_ANY_ITEM
    {
        validate_modpack_payload_token(
            held_item,
            &format!("evolution entry {index} held item for '{species}'"),
        )?;
    }
    if let Some(window) = entry.happiness.as_deref() {
        validate_battle_table_token(
            window,
            &format!("evolution entry {index} happiness window for '{species}'"),
        )?;
        if !is_known_happiness_window(window) {
            anyhow::bail!(
                "evolution entry {index} for '{species}' has unknown happiness window '{window}'"
            );
        }
    }
    if let Some(ratio) = entry.stat_ratio.as_deref() {
        validate_battle_table_token(
            ratio,
            &format!("evolution entry {index} stat ratio for '{species}'"),
        )?;
        if !is_known_stat_evolution_ratio(ratio) {
            anyhow::bail!(
                "evolution entry {index} for '{species}' has unknown stat ratio '{ratio}'"
            );
        }
    }
    Ok(())
}

fn merge_mart_payload(target: &mut MartCatalog, payload: Value) -> Result<()> {
    let marts: MartCatalog = serde_json::from_value(payload)?;
    for (mart_id, item_ids) in marts.0 {
        insert_mart_entry(target, mart_id, item_ids)?;
    }
    Ok(())
}

fn merge_mart_catalog(target: &mut MartCatalog, source: &MartCatalog) -> Result<()> {
    for (mart_id, item_ids) in &source.0 {
        insert_mart_entry(target, mart_id.clone(), item_ids.clone())?;
    }
    Ok(())
}

fn insert_mart_entry(
    target: &mut MartCatalog,
    mart_id: String,
    item_ids: Vec<String>,
) -> Result<()> {
    validate_modpack_payload_token(&mart_id, "mart catalog entry id")?;
    for item_id in &item_ids {
        validate_modpack_payload_token(item_id, "mart item id")?;
    }
    if target.0.contains_key(&mart_id) {
        anyhow::bail!("duplicate mart catalog entry for mart '{mart_id}'");
    }
    target.0.insert(mart_id, item_ids);
    Ok(())
}

fn merge_fruit_tree_payload(target: &mut FruitTreeCatalog, payload: Value) -> Result<()> {
    let fruit_trees: FruitTreeCatalog = serde_json::from_value(payload)?;
    for (tree_id, item_id) in fruit_trees.0 {
        insert_fruit_tree_entry(target, tree_id, item_id)?;
    }
    Ok(())
}

fn merge_fruit_tree_catalog(
    target: &mut FruitTreeCatalog,
    source: &FruitTreeCatalog,
) -> Result<()> {
    for (tree_id, item_id) in &source.0 {
        insert_fruit_tree_entry(target, tree_id.clone(), item_id.clone())?;
    }
    Ok(())
}

fn insert_fruit_tree_entry(
    target: &mut FruitTreeCatalog,
    tree_id: String,
    item_id: String,
) -> Result<()> {
    validate_modpack_payload_token(&tree_id, "fruit tree catalog entry id")?;
    validate_modpack_payload_token(&item_id, "fruit tree item id")?;
    if target.0.contains_key(&tree_id) {
        anyhow::bail!("duplicate fruit tree catalog entry for tree '{tree_id}'");
    }
    target.0.insert(tree_id, item_id);
    Ok(())
}

fn insert_fly_destination(
    target: &mut BTreeMap<String, FlyDestination>,
    flypoint_flag: String,
    destination: FlyDestination,
) -> Result<()> {
    anyhow::ensure!(
        destination.destination_spawn_identifier
            < crystal_core::systems::special_routines::CRYSTAL_NUM_SPAWN_POINTS,
        "fly destination '{flypoint_flag}' spawn identifier {} is outside Crystal's SpawnPoints table",
        destination.destination_spawn_identifier
    );
    if flypoint_flag != destination.flypoint_flag {
        anyhow::bail!(
            "fly destination key '{flypoint_flag}' does not match record flypoint_flag '{}'",
            destination.flypoint_flag
        );
    }
    validate_modpack_payload_token(&flypoint_flag, "fly destination key")?;
    validate_modpack_payload_token(&destination.flypoint_flag, "fly destination flypoint_flag")?;
    validate_modpack_payload_token(&destination.label, "fly destination label")?;
    if target.contains_key(&flypoint_flag) {
        anyhow::bail!("duplicate fly destination '{flypoint_flag}'");
    }
    target.insert(flypoint_flag, destination);
    Ok(())
}

fn merge_phone_contact_payload(target: &mut PhoneContactCatalog, payload: Value) -> Result<()> {
    let contacts: PhoneContactCatalog = serde_json::from_value(payload)?;
    merge_phone_contact_catalog(target, &contacts)
}

fn merge_phone_contact_catalog(
    target: &mut PhoneContactCatalog,
    source: &PhoneContactCatalog,
) -> Result<()> {
    for (contact_id, record) in &source.0 {
        insert_phone_contact(target, contact_id.clone(), record.clone())?;
    }
    Ok(())
}

fn insert_phone_contact(
    target: &mut PhoneContactCatalog,
    contact_id: String,
    record: PhoneContactRecord,
) -> Result<()> {
    validate_modpack_payload_token(&contact_id, "phone contact catalog entry id")?;
    if contact_id != record.contact_id {
        anyhow::bail!(
            "phone contact key '{contact_id}' does not match record contactId '{}'",
            record.contact_id
        );
    }
    validate_modpack_payload_token(&record.contact_id, "phone contact record contactId")?;
    if let Some(trainer_class) = record.trainer_class.as_deref() {
        validate_battle_table_token(trainer_class, "phone contact trainerClass")?;
    }
    if let Some(trainer_label) = record.trainer_label.as_deref() {
        validate_battle_table_token(trainer_label, "phone contact trainerLabel")?;
    }
    validate_exact_modpack_value(&record.primary_label, "phone contact primaryLabel")?;
    if record.lines.is_empty() {
        anyhow::bail!("phone contact '{contact_id}' must declare at least one display line");
    }
    for line in &record.lines {
        validate_exact_modpack_text(line, "phone contact display line")?;
    }
    if let Some(first_line) = record.lines.first() {
        let expected_label_line = format!("{}:", record.primary_label);
        if first_line != &record.primary_label && first_line != &expected_label_line {
            anyhow::bail!(
                "phone contact primaryLabel '{}' does not match first display line '{}'",
                record.primary_label,
                first_line
            );
        }
    }
    if let Some(map_constant) = record.map_constant.as_deref() {
        validate_battle_table_token(map_constant, "phone contact mapConstant")?;
    }
    if let Some(callee_script) = record.callee_script.as_deref() {
        validate_battle_table_token(callee_script, "phone contact calleeScript")?;
    }
    if let Some(caller_script) = record.caller_script.as_deref() {
        validate_battle_table_token(caller_script, "phone contact callerScript")?;
    }
    if target.0.contains_key(&contact_id) {
        anyhow::bail!("duplicate phone contact catalog entry for contact '{contact_id}'");
    }
    target.0.insert(contact_id, record);
    Ok(())
}

fn insert_pokedex_entry(
    target: &mut BTreeMap<String, RuntimePokedexEntry>,
    entry: RuntimePokedexEntry,
) -> Result<()> {
    let species = entry.species.clone();
    validate_modpack_payload_token(&species, "pokedex entry species id")?;
    validate_exact_modpack_value(&entry.classification, "pokedex entry classification")?;
    if entry.pages.is_empty() {
        anyhow::bail!("pokedex entry for species '{species}' must declare at least one page");
    }
    for page in &entry.pages {
        validate_exact_modpack_value(page, "pokedex entry page")?;
    }
    if target.insert(species.clone(), entry).is_some() {
        anyhow::bail!("duplicate pokedex entry for species '{species}'");
    }
    Ok(())
}

fn insert_keyed_pokedex_entry(
    target: &mut BTreeMap<String, RuntimePokedexEntry>,
    species: String,
    entry: RuntimePokedexEntry,
) -> Result<()> {
    validate_modpack_payload_token(&species, "pokedex entry species key")?;
    if species != entry.species {
        anyhow::bail!(
            "pokedex entry key '{species}' does not match record species '{}'",
            entry.species
        );
    }
    insert_pokedex_entry(target, entry)
}

fn insert_pokemon_species(
    target: &mut BTreeMap<String, PokemonSpecies>,
    species: PokemonSpecies,
) -> Result<()> {
    let species_id = species.id.clone();
    validate_modpack_payload_token(&species_id, "Pokemon species id")?;
    validate_battle_table_token(&species.type1, "Pokemon species primary type")?;
    validate_battle_table_token(&species.type2, "Pokemon species secondary type")?;
    validate_battle_table_token(&species.growth_rate, "Pokemon species growth rate")?;
    validate_battle_table_token(&species.egg_group1, "Pokemon species primary egg group")?;
    validate_battle_table_token(&species.egg_group2, "Pokemon species secondary egg group")?;
    validate_battle_table_token(&species.ability, "Pokemon species ability")?;
    for item_id in [&species.item1, &species.item2].into_iter().flatten() {
        validate_modpack_payload_token(item_id, "Pokemon species held item id")?;
    }
    for move_id in &species.tmhm_learnset {
        validate_modpack_payload_token(move_id, "Pokemon species TM/HM move id")?;
    }
    if target.contains_key(&species_id) {
        anyhow::bail!("duplicate Pokemon species '{species_id}'");
    }
    target.insert(species_id, species);
    Ok(())
}

fn insert_keyed_pokemon_species(
    target: &mut BTreeMap<String, PokemonSpecies>,
    species_id: String,
    species: PokemonSpecies,
) -> Result<()> {
    validate_modpack_payload_token(&species_id, "Pokemon species key")?;
    if species_id != species.id {
        anyhow::bail!(
            "Pokemon species key '{species_id}' does not match record id '{}'",
            species.id
        );
    }
    insert_pokemon_species(target, species)
}

fn insert_move_data(target: &mut BTreeMap<String, Move>, move_data: Move) -> Result<()> {
    validate_manifest_move(&move_data)?;
    let move_name = move_data.name.clone();
    validate_modpack_payload_token(&move_name, "move id")?;
    if target.contains_key(&move_name) {
        anyhow::bail!("duplicate move '{move_name}'");
    }
    target.insert(move_name, move_data);
    Ok(())
}

fn insert_keyed_move_data(
    target: &mut BTreeMap<String, Move>,
    move_id: String,
    move_data: Move,
) -> Result<()> {
    validate_modpack_payload_token(&move_id, "move key")?;
    if move_id != move_data.name {
        anyhow::bail!(
            "move key '{move_id}' does not match record name '{}'",
            move_data.name
        );
    }
    insert_move_data(target, move_data)
}

fn insert_growth_rate_curve(
    target: &mut BTreeMap<String, crystal_core::systems::experience::GrowthRateCurve>,
    curve: crystal_core::systems::experience::GrowthRateCurve,
) -> Result<()> {
    let curve_id = curve.id.clone();
    validate_modpack_payload_token(&curve_id, "growth rate curve id")?;
    if curve.denominator == 0 {
        anyhow::bail!("growth rate curve '{curve_id}' must declare a nonzero denominator");
    }
    if target.contains_key(&curve_id) {
        anyhow::bail!("duplicate growth rate curve '{curve_id}'");
    }
    target.insert(curve_id, curve);
    Ok(())
}

fn insert_keyed_growth_rate_curve(
    target: &mut BTreeMap<String, crystal_core::systems::experience::GrowthRateCurve>,
    curve_id: String,
    curve: crystal_core::systems::experience::GrowthRateCurve,
) -> Result<()> {
    validate_modpack_payload_token(&curve_id, "growth rate key")?;
    if curve_id != curve.id {
        anyhow::bail!(
            "growth rate key '{curve_id}' does not match record id '{}'",
            curve.id
        );
    }
    insert_growth_rate_curve(target, curve)
}

fn insert_item(target: &mut BTreeMap<String, Item>, item: Item) -> Result<()> {
    validate_manifest_item(&item)?;
    let key = item_key(&item)?;
    validate_modpack_payload_token(&key, "item id")?;
    if target.contains_key(&key) {
        anyhow::bail!("duplicate item '{key}'");
    }
    target.insert(key, item);
    Ok(())
}

fn insert_keyed_item(
    target: &mut BTreeMap<String, Item>,
    item_id: String,
    item: Item,
) -> Result<()> {
    validate_modpack_payload_token(&item_id, "item key")?;
    let key = item_key(&item)?;
    if item_id != key {
        anyhow::bail!("item key '{item_id}' does not match record script_name '{key}'");
    }
    insert_item(target, item)
}

fn merge_map_attributes(
    target: &mut BTreeMap<String, MapAttributes>,
    source: BTreeMap<String, MapAttributes>,
) -> Result<()> {
    for (map_id, attributes) in source {
        validate_modpack_payload_token(&map_id, "map attributes map id")?;
        validate_map_reference_token(&map_id, "map attributes map id")?;
        validate_map_attributes(&map_id, &attributes)?;
        if target.contains_key(&map_id) {
            anyhow::bail!("duplicate map attributes for map '{map_id}'");
        }
        target.insert(map_id, attributes);
    }
    Ok(())
}

fn validate_map_attributes(map_id: &str, attributes: &MapAttributes) -> Result<()> {
    if attributes.width == 0 || attributes.height == 0 {
        anyhow::bail!("map '{map_id}' width and height must both be greater than zero");
    }
    if !is_exact_tileset_id(&attributes.tileset_name) {
        anyhow::bail!(
            "map '{map_id}' tileset_name '{}' must be an exact tileset id",
            attributes.tileset_name
        );
    }
    for connection in &attributes.connections {
        if !is_exact_map_connection_direction(&connection.direction) {
            anyhow::bail!(
                "map '{map_id}' connection direction '{}' must be one of north, south, west, east",
                connection.direction
            );
        }
        if !is_exact_map_reference_token(&connection.target_map) {
            anyhow::bail!(
                "map '{map_id}' connection target map '{}' must be an exact map id",
                connection.target_map
            );
        }
    }
    for (field_name, value) in [
        ("time_of_day", &attributes.time_of_day),
        ("environment", &attributes.environment),
        ("location", &attributes.location),
        ("music", &attributes.music),
        ("palette", &attributes.palette),
        ("fishing_group", &attributes.fishing_group),
        ("map_constant", &attributes.map_constant),
        ("map_group_constant", &attributes.map_group_constant),
        ("blocks_label", &attributes.blocks_label),
        ("map_scripts_label", &attributes.map_scripts_label),
        ("map_events_label", &attributes.map_events_label),
    ] {
        if let Some(value) = value {
            validate_map_reference_token(value, &format!("map attributes {field_name}"))?;
        }
    }
    if let Some(connection_flags) = &attributes.connection_flags {
        validate_exact_modpack_value(connection_flags, "map attributes connection_flags")?;
    }
    Ok(())
}

fn is_exact_map_section_arg(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn validate_map_section_args(args: &[String], description: &str) -> Result<()> {
    for arg in args {
        if !is_exact_map_section_arg(arg) {
            anyhow::bail!("{description} arg '{arg}' must be exact and non-empty");
        }
    }
    Ok(())
}

fn validate_map_script_section_command_shape(
    map_id: &str,
    command: &MapScriptSectionCommand,
) -> Result<()> {
    let expected_counts = map_script_section_command_arg_counts();
    let Some(expected) = expected_counts.get(command.command.as_str()) else {
        anyhow::bail!(
            "map '{map_id}' script section command {} has unknown command '{}'",
            command.command_index,
            command.command
        );
    };
    if !expected.contains(&command.args.len()) {
        anyhow::bail!(
            "map '{map_id}' script section command {} '{}' expected one of {:?} args, found {}",
            command.command_index,
            command.command,
            expected,
            command.args.len()
        );
    }
    validate_map_section_args(
        &command.args,
        &format!(
            "map '{map_id}' script section command {} '{}'",
            command.command_index, command.command
        ),
    )
}

fn validate_map_event_section_command_shape(
    map_id: &str,
    command: &MapEventSectionCommand,
) -> Result<()> {
    let expected_counts = map_event_section_command_arg_counts();
    let Some(expected) = expected_counts.get(command.command.as_str()) else {
        anyhow::bail!(
            "map '{map_id}' event section command {} has unknown command '{}'",
            command.command_index,
            command.command
        );
    };
    if !expected.contains(&command.args.len()) {
        anyhow::bail!(
            "map '{map_id}' event section command {} '{}' expected one of {:?} args, found {}",
            command.command_index,
            command.command,
            expected,
            command.args.len()
        );
    }
    validate_map_section_args(
        &command.args,
        &format!(
            "map '{map_id}' event section command {} '{}'",
            command.command_index, command.command
        ),
    )
}

fn insert_map_module(target: &mut BTreeMap<String, MapModule>, map: MapModule) -> Result<()> {
    let map_id = map.id.clone();
    validate_modpack_payload_token(&map_id, "map module id")?;
    validate_map_reference_token(&map_id, "map module id")?;
    validate_map_attributes(&map_id, &map.attributes)?;
    validate_map_module_blocks(&map_id, &map)?;
    validate_map_module_scripts(&map_id, &map.scripts)?;
    validate_map_module_scenes_and_events(&map_id, &map)?;
    validate_npc_object_events(&map_id, &map.objects)?;
    validate_map_module_extracted_script_commands(&map_id, &map)?;
    for command in &map.map_script_section_commands {
        validate_map_script_section_command_shape(&map_id, command)?;
    }
    for command in &map.map_event_section_commands {
        validate_map_event_section_command_shape(&map_id, command)?;
    }
    if target.contains_key(&map_id) {
        anyhow::bail!("duplicate map module '{map_id}'");
    }
    target.insert(map_id, map);
    Ok(())
}

fn insert_keyed_map_module(
    target: &mut BTreeMap<String, MapModule>,
    map_id: String,
    map: MapModule,
) -> Result<()> {
    validate_modpack_payload_token(&map_id, "map module key")?;
    if map_id != map.id {
        anyhow::bail!(
            "map module key '{map_id}' does not match record id '{}'",
            map.id
        );
    }
    insert_map_module(target, map)
}

fn validate_map_module_scenes_and_events(map_id: &str, map: &MapModule) -> Result<()> {
    let script_labels: BTreeSet<String> = map.scripts.keys().cloned().collect();
    for scene in &map.scenes.scenes {
        validate_battle_table_token(&scene.scene_id, &format!("map '{map_id}' scene id"))?;
        if let Some(script_name) = scene.script_name.as_deref() {
            validate_map_local_script_reference(
                map_id,
                &script_labels,
                "scene script",
                script_name,
            )?;
        }
    }
    for warp in &map.events.warps {
        validate_battle_table_token(
            &warp.target_map_constant,
            &format!("map '{map_id}' warp {} target map constant", warp.index),
        )?;
        validate_map_reference_token(
            &warp.target_map,
            &format!("map '{map_id}' warp {} target map", warp.index),
        )?;
    }
    for event in &map.events.coord_events {
        validate_battle_table_token(
            &event.scene_id,
            &format!("map '{map_id}' coord event scene id"),
        )?;
        validate_map_local_script_reference(
            map_id,
            &script_labels,
            "coord event script",
            &event.script_name,
        )?;
    }
    for event in &map.events.bg_events {
        validate_battle_table_token(
            &event.event_type,
            &format!("map '{map_id}' background event type"),
        )?;
        validate_map_local_script_reference(
            map_id,
            &script_labels,
            "background event script",
            &event.script,
        )?;
    }
    Ok(())
}

fn validate_map_local_script_reference(
    map_id: &str,
    script_labels: &BTreeSet<String>,
    description: &str,
    script_name: &str,
) -> Result<()> {
    if !is_exact_script_label_reference_token(script_name) {
        anyhow::bail!("map '{map_id}' {description} '{script_name}' must be an exact script label");
    }
    if !script_labels.contains(script_name) {
        anyhow::bail!("map '{map_id}' {description} '{script_name}' is not a loaded script");
    }
    Ok(())
}

fn validate_map_module_extracted_script_commands(map_id: &str, map: &MapModule) -> Result<()> {
    for command in &map.script_shop_commands {
        let description = format!(
            "map '{map_id}' script shop command {} name",
            command.command_index
        );
        validate_exact_modpack_value(&command.command, &description)?;
        if !SCRIPT_SHOP_COMMANDS.contains(&command.command.as_str()) {
            anyhow::bail!(
                "map '{map_id}' script shop command {} '{}' is not a known shop command",
                command.command_index,
                command.command
            );
        }
        validate_battle_table_token(
            &command.mart_type,
            &format!(
                "map '{map_id}' script shop command {} mart type",
                command.command_index
            ),
        )?;
        validate_modpack_payload_token(
            &command.mart_id,
            &format!(
                "map '{map_id}' script shop command {} mart id",
                command.command_index
            ),
        )?;
        validate_exact_modpack_value(
            &command.source_script,
            &format!(
                "map '{map_id}' script shop command {} source script",
                command.command_index
            ),
        )?;
    }
    for command in &map.script_phone_commands {
        let description = format!(
            "map '{map_id}' script phone command {} name",
            command.command_index
        );
        validate_exact_modpack_value(&command.command, &description)?;
        if !SCRIPT_PHONE_CHECK_COMMANDS.contains(&command.command.as_str())
            && !SCRIPT_PHONE_REGISTRATION_COMMANDS.contains(&command.command.as_str())
        {
            anyhow::bail!(
                "map '{map_id}' script phone command {} '{}' is not a known phone command",
                command.command_index,
                command.command
            );
        }
        validate_modpack_payload_token(
            &command.contact_id,
            &format!(
                "map '{map_id}' script phone command {} contact id",
                command.command_index
            ),
        )?;
        validate_exact_modpack_value(
            &command.source_script,
            &format!(
                "map '{map_id}' script phone command {} source script",
                command.command_index
            ),
        )?;
    }
    for issue in script_variable_command_issues(&map.script_variable_commands) {
        anyhow::bail!(
            "map '{map_id}' script variable command {} in '{}' is malformed: {:?}",
            issue.command_index,
            issue.source_script,
            issue.error
        );
    }
    for command in &map.script_variable_commands {
        validate_exact_modpack_value(
            &command.source_script,
            &format!(
                "map '{map_id}' script variable command {} source script",
                command.command_index
            ),
        )?;
    }
    for command in &map.script_audio_commands {
        validate_script_audio_command_shape(map_id, command)?;
    }
    for grant in &map.script_item_grants {
        validate_script_item_grant_shape(map_id, grant)?;
    }
    for access in &map.script_item_checks {
        validate_script_item_access_shape(map_id, "check", access)?;
    }
    for access in &map.script_item_takes {
        validate_script_item_access_shape(map_id, "take", access)?;
    }
    for command in &map.script_flag_commands {
        validate_script_flag_command_shape(map_id, command)?;
    }
    for command in &map.script_scene_commands {
        validate_script_scene_command_shape(map_id, command)?;
    }
    for command in &map.script_economy_commands {
        validate_script_economy_command_shape(map_id, command)?;
    }
    for pickup in &map.script_field_pickups {
        validate_script_field_pickup_shape(map_id, pickup)?;
    }
    for change in &map.script_block_changes {
        validate_script_block_change_shape(map_id, map, change)?;
    }
    for movement in &map.script_movements {
        validate_script_movement_shape(map_id, movement)?;
    }
    validate_script_object_command_shapes(map_id, map)?;
    validate_script_text_command_shapes(map_id, map)?;
    validate_script_text_body_shapes(map_id, map)?;
    validate_script_menu_definition_shapes(map_id, map)?;
    validate_script_control_command_shapes(map_id, map)?;
    for command in &map.script_map_commands {
        validate_script_map_command_shape(map_id, command)?;
    }
    for command in &map.script_runtime_commands {
        validate_script_runtime_command_shape(map_id, command)?;
    }
    validate_gift_pokemon_script_shapes(map_id, map)?;
    validate_trainer_battle_record_shapes(map_id, map)?;
    Ok(())
}

fn validate_gift_pokemon_script_shapes(map_id: &str, map: &MapModule) -> Result<()> {
    let script_labels: BTreeSet<String> = map.scripts.keys().cloned().collect();
    for gift in &map.gift_pokemon_scripts {
        let context = format!("map '{map_id}' gift Pokemon command {}", gift.command_index);
        validate_modpack_payload_token(&gift.species_id, &format!("{context} species id"))?;
        validate_exact_modpack_value(&gift.level_token, &format!("{context} level token"))?;
        if gift.level == 0 {
            anyhow::bail!("{context} level must be greater than zero");
        }
        if let Some(item_id) = gift.held_item_id.as_deref() {
            validate_modpack_payload_token(item_id, &format!("{context} held item id"))?;
        }
        validate_optional_script_label(
            map_id,
            &script_labels,
            &context,
            "nickname label",
            gift.nickname_label.as_deref(),
        )?;
        validate_optional_script_label(
            map_id,
            &script_labels,
            &context,
            "original trainer label",
            gift.ot_label.as_deref(),
        )?;
        validate_exact_modpack_value(&gift.source_script, &format!("{context} source script"))?;
    }
    Ok(())
}

fn validate_optional_script_label(
    map_id: &str,
    script_labels: &BTreeSet<String>,
    context: &str,
    field: &str,
    label: Option<&str>,
) -> Result<()> {
    let Some(label) = label else {
        return Ok(());
    };
    validate_exact_modpack_value(label, &format!("{context} {field}"))?;
    if !script_labels.contains(label) {
        anyhow::bail!("{context} {field} '{label}' is not a loaded script in map '{map_id}'");
    }
    Ok(())
}

fn validate_trainer_battle_record_shapes(map_id: &str, map: &MapModule) -> Result<()> {
    for (source_script, request) in &map.trainer_scripts {
        validate_exact_modpack_value(source_script, &format!("map '{map_id}' trainer script key"))?;
        validate_trainer_battle_request_shape(
            map_id,
            &format!("map '{map_id}' trainer script '{source_script}'"),
            request,
        )?;
    }
    for battle in &map.scripted_trainer_battles {
        let context = format!(
            "map '{map_id}' scripted trainer battle command {}",
            battle.loadtrainer_command_index
        );
        validate_exact_modpack_value(&battle.source_script, &format!("{context} source script"))?;
        validate_trainer_battle_request_shape(map_id, &context, &battle.request)?;
    }
    for battle in &map.scripted_wild_battles {
        let context = format!(
            "map '{map_id}' scripted wild battle command {}",
            battle.loadwildmon_command_index
        );
        validate_exact_modpack_value(&battle.source_script, &format!("{context} source script"))?;
        validate_static_wild_battle_request_shape(&context, &battle.request)?;
    }
    Ok(())
}

fn validate_trainer_battle_request_shape(
    map_id: &str,
    context: &str,
    request: &TrainerBattleRequest,
) -> Result<()> {
    validate_battle_table_token(&request.battle_type, &format!("{context} battle type"))?;
    validate_battle_table_token(&request.trainer_class, &format!("{context} trainer class"))?;
    validate_modpack_payload_token(&request.trainer_id, &format!("{context} trainer id"))?;
    validate_optional_battle_request_token(&request.event_flag, &format!("{context} event flag"))?;
    validate_optional_exact_value(&request.seen_text, &format!("{context} seen text"))?;
    validate_optional_exact_value(&request.win_text, &format!("{context} win text"))?;
    validate_optional_exact_value(&request.loss_text, &format!("{context} loss text"))?;
    validate_optional_exact_value(&request.callback, &format!("{context} callback"))?;
    if !request.source_script.is_empty() {
        validate_exact_modpack_value(
            &request.source_script,
            &format!("{context} request source script"),
        )?;
    }
    if !is_trainer_battle_type(&request.battle_type) {
        anyhow::bail!(
            "{context} battle type '{}' is not a trainer battle type in map '{map_id}'",
            request.battle_type
        );
    }
    Ok(())
}

fn is_trainer_battle_type(battle_type: &str) -> bool {
    matches!(
        battle_type,
        "BATTLETYPE_TRAINER"
            | "BATTLETYPE_CANLOSE"
            | "BATTLETYPE_TUTORIAL"
            | "BATTLETYPE_BATTLE_TOWER"
            | "BATTLETYPE_TRAINER_HOUSE"
    )
}

fn validate_static_wild_battle_request_shape(
    context: &str,
    request: &StaticWildBattleRequest,
) -> Result<()> {
    validate_battle_table_token(&request.battle_type, &format!("{context} battle type"))?;
    if !is_static_wild_battle_type(&request.battle_type) {
        anyhow::bail!(
            "{context} battle type '{}' is not a static wild battle type",
            request.battle_type
        );
    }
    validate_modpack_payload_token(&request.species, &format!("{context} species id"))?;
    if request.level == 0 {
        anyhow::bail!("{context} level must be greater than zero");
    }
    if !request.source_script.is_empty() {
        validate_exact_modpack_value(
            &request.source_script,
            &format!("{context} request source script"),
        )?;
    }
    Ok(())
}

fn is_static_wild_battle_type(battle_type: &str) -> bool {
    matches!(
        battle_type,
        "BATTLETYPE_NORMAL"
            | "BATTLETYPE_FORCEITEM"
            | "BATTLETYPE_FORCESHINY"
            | "BATTLETYPE_SUICUNE"
            | "BATTLETYPE_TRAP"
            | "BATTLETYPE_CELEBI"
            | "BATTLETYPE_TUTORIAL"
    )
}

fn active_battle_type(state: &GameState) -> Option<&str> {
    match &state.battle {
        BattleMemory::Wild { battle_type, .. }
        | BattleMemory::StaticWild { battle_type, .. }
        | BattleMemory::Trainer { battle_type, .. } => Some(battle_type),
        BattleMemory::Inactive => None,
    }
}

fn battle_type_guarantees_escape(battle_type: &str) -> bool {
    matches!(
        battle_type,
        "BATTLETYPE_DEBUG" | "BATTLETYPE_CONTEST"
    )
}

fn battle_type_blocks_escape(battle_type: &str) -> bool {
    matches!(
        battle_type,
        "BATTLETYPE_TRAP"
            | "BATTLETYPE_CELEBI"
            | "BATTLETYPE_FORCESHINY"
            | "BATTLETYPE_SUICUNE"
    )
}

fn validate_optional_battle_request_token(value: &str, description: &str) -> Result<()> {
    if value.is_empty() {
        return Ok(());
    }
    validate_modpack_payload_token(value, description)
}

fn validate_optional_exact_value(value: &str, description: &str) -> Result<()> {
    if value.is_empty() {
        return Ok(());
    }
    validate_exact_modpack_value(value, description)
}

fn validate_script_text_command_shapes(map_id: &str, map: &MapModule) -> Result<()> {
    let text_labels: BTreeSet<String> = map.script_text_bodies.keys().cloned().collect();
    for command in &map.script_text_commands {
        validate_exact_modpack_value(
            &command.source_script,
            &format!(
                "map '{map_id}' script text command {} source script",
                command.command_index
            ),
        )?;
        for issue in script_text_command_issues(command, &text_labels) {
            anyhow::bail!(
                "map '{map_id}' script text command {} in '{}' is malformed: {:?}",
                command.command_index,
                command.source_script,
                issue
            );
        }
    }
    Ok(())
}

fn validate_script_text_body_shapes(map_id: &str, map: &MapModule) -> Result<()> {
    for (label, body) in &map.script_text_bodies {
        for issue in script_text_body_issues(label, body) {
            anyhow::bail!(
                "map '{map_id}' script text body '{label}' is malformed: {:?}",
                issue
            );
        }
        for command in &body.commands {
            for (index, arg) in command.args.iter().enumerate() {
                validate_exact_modpack_value(
                    arg,
                    &format!(
                        "map '{map_id}' script text body '{label}' command {} arg {index}",
                        command.command_index
                    ),
                )?;
            }
        }
    }
    Ok(())
}

fn validate_script_menu_definition_shapes(map_id: &str, map: &MapModule) -> Result<()> {
    for (label, menu) in &map.script_menu_definitions {
        for issue in script_menu_definition_issues(label, menu) {
            anyhow::bail!(
                "map '{map_id}' script menu definition '{label}' is malformed: {:?}",
                issue
            );
        }
        for command in &menu.commands {
            for (index, arg) in command.args.iter().enumerate() {
                validate_exact_modpack_value(
                    arg,
                    &format!(
                        "map '{map_id}' script menu definition '{label}' command {} arg {index}",
                        command.command_index
                    ),
                )?;
            }
            if command.command == "menu_coords" {
                validate_menu_coord_args(&command.args).with_context(|| {
                    format!(
                        "map '{map_id}' script menu definition '{label}' command {}",
                        command.command_index
                    )
                })?;
            }
        }
    }
    for (label, menu) in &map.script_vertical_menus {
        validate_exact_modpack_value(label, &format!("map '{map_id}' script vertical menu key"))?;
        validate_exact_modpack_value(
            &menu.source_script,
            &format!("map '{map_id}' script vertical menu '{label}' source script"),
        )?;
        validate_exact_modpack_value(
            &menu.header_label,
            &format!("map '{map_id}' script vertical menu '{label}' header label"),
        )?;
        if !map.script_menu_definitions.contains_key(&menu.header_label) {
            anyhow::bail!(
                "map '{map_id}' script vertical menu '{label}' references missing header '{}'",
                menu.header_label
            );
        }
        if let Some(data_label) = &menu.data_label {
            validate_exact_modpack_value(
                data_label,
                &format!("map '{map_id}' script vertical menu '{label}' data label"),
            )?;
            if !map.script_menu_definitions.contains_key(data_label) {
                anyhow::bail!(
                    "map '{map_id}' script vertical menu '{label}' references missing data label '{data_label}'"
                );
            }
        }
        if menu.options.is_empty() {
            anyhow::bail!("map '{map_id}' script vertical menu '{label}' has no options");
        }
        for (index, option) in menu.options.iter().enumerate() {
            validate_exact_modpack_value(
                option,
                &format!("map '{map_id}' script vertical menu '{label}' option {index}"),
            )?;
        }
    }
    for (label, elevator) in &map.script_elevators {
        validate_exact_modpack_value(label, &format!("map '{map_id}' script elevator key"))?;
        validate_exact_modpack_value(
            &elevator.source_script,
            &format!("map '{map_id}' script elevator '{label}' source script"),
        )?;
        validate_exact_modpack_value(
            &elevator.data_label,
            &format!("map '{map_id}' script elevator '{label}' data label"),
        )?;
        if !map.scripts.contains_key(&elevator.source_script) {
            anyhow::bail!(
                "map '{map_id}' script elevator '{label}' references missing source script '{}'",
                elevator.source_script
            );
        }
        if !map.scripts.contains_key(&elevator.data_label) {
            anyhow::bail!(
                "map '{map_id}' script elevator '{label}' references missing data label '{}'",
                elevator.data_label
            );
        }
        if elevator.floors.is_empty() {
            anyhow::bail!("map '{map_id}' script elevator '{label}' has no floors");
        }
        for (index, floor) in elevator.floors.iter().enumerate() {
            validate_exact_modpack_value(
                &floor.floor,
                &format!("map '{map_id}' script elevator '{label}' floor {index} label"),
            )?;
            validate_exact_modpack_value(
                &floor.target_map,
                &format!("map '{map_id}' script elevator '{label}' floor {index} target map"),
            )?;
            validate_exact_modpack_value(
                &floor.source_script,
                &format!("map '{map_id}' script elevator '{label}' floor {index} source script"),
            )?;
            if floor.source_script != elevator.data_label {
                anyhow::bail!(
                    "map '{map_id}' script elevator '{label}' floor {index} source script '{}' does not match data label '{}'",
                    floor.source_script,
                    elevator.data_label
                );
            }
            if floor.warp == 0 {
                anyhow::bail!(
                    "map '{map_id}' script elevator '{label}' floor {index} warp must be greater than zero"
                );
            }
        }
    }
    Ok(())
}

fn validate_script_control_command_shapes(map_id: &str, map: &MapModule) -> Result<()> {
    let script_labels: BTreeSet<String> = map.scripts.keys().cloned().collect();
    for command in &map.script_control_commands {
        validate_exact_modpack_value(
            &command.source_script,
            &format!(
                "map '{map_id}' script control command {} source script",
                command.command_index
            ),
        )?;
        for issue in script_control_command_issues(command, &script_labels) {
            anyhow::bail!(
                "map '{map_id}' script control command {} in '{}' is malformed: {:?}",
                command.command_index,
                command.source_script,
                issue
            );
        }
    }
    Ok(())
}

fn validate_script_map_command_shape(map_id: &str, command: &ScriptMapCommand) -> Result<()> {
    let mut local_map_ids = BTreeSet::new();
    local_map_ids.insert(map_id.to_string());
    validate_exact_modpack_value(
        &command.source_script,
        &format!(
            "map '{map_id}' script map command {} source script",
            command.command_index
        ),
    )?;
    for issue in script_map_command_issues(command, &local_map_ids) {
        if matches!(issue, ScriptMapCommandError::UnknownTargetMap { .. }) {
            continue;
        }
        anyhow::bail!(
            "map '{map_id}' script map command {} in '{}' is malformed: {:?}",
            command.command_index,
            command.source_script,
            issue
        );
    }
    Ok(())
}

fn validate_menu_coord_args<T: AsRef<str>>(args: &[T]) -> Result<()> {
    if args.len() != 4 {
        anyhow::bail!("menu_coords requires 4 operands, got {}", args.len());
    }
    for (index, value) in args.iter().enumerate() {
        parse_menu_coord_expression(value.as_ref()).with_context(|| {
            format!(
                "menu coordinate {index} must be an exact supported expression, got {:?}",
                value.as_ref()
            )
        })?;
    }
    Ok(())
}

fn parse_menu_coord_expression(value: &str) -> Result<i16> {
    parse_menu_coord_token("menu_coords", value).map_err(anyhow::Error::new)
}

fn validate_script_runtime_command_shape(
    map_id: &str,
    command: &ScriptRuntimeCommand,
) -> Result<()> {
    validate_exact_modpack_value(
        &command.source_script,
        &format!(
            "map '{map_id}' script runtime command {} source script",
            command.command_index
        ),
    )?;
    if let Err(error) = validate_script_runtime_command(command) {
        anyhow::bail!(
            "map '{map_id}' script runtime command {} in '{}' is malformed: {:?}",
            command.command_index,
            command.source_script,
            error
        );
    }
    Ok(())
}

fn validate_script_movement_shape(map_id: &str, movement: &ScriptMovement) -> Result<()> {
    validate_exact_modpack_value(
        &movement.label,
        &format!("map '{map_id}' script movement label"),
    )?;
    if let Some(source_script) = movement.source_script.as_deref() {
        validate_exact_modpack_value(
            source_script,
            &format!(
                "map '{map_id}' script movement '{}' source script",
                movement.label
            ),
        )?;
    }
    if movement.steps.is_empty() {
        anyhow::bail!(
            "map '{map_id}' script movement '{}' must include at least one step",
            movement.label
        );
    }
    if !script_movement_has_terminator(movement) {
        anyhow::bail!(
            "map '{map_id}' script movement '{}' must end with a terminating opcode",
            movement.label
        );
    }
    for step in &movement.steps {
        validate_exact_modpack_value(
            &step.command,
            &format!(
                "map '{map_id}' script movement '{}' step {} command",
                movement.label, step.index
            ),
        )?;
        for issue in script_movement_step_issues(step) {
            anyhow::bail!(
                "map '{map_id}' script movement '{}' step {} is malformed: {}",
                movement.label,
                step.index,
                script_movement_step_issue_name(&issue)
            );
        }
    }
    Ok(())
}

fn script_movement_step_issue_name(issue: &ScriptMovementStepIssue) -> &'static str {
    match issue {
        ScriptMovementStepIssue::UnexpectedDirection => "unexpected_direction",
        ScriptMovementStepIssue::UnexpectedDuration => "unexpected_duration",
        ScriptMovementStepIssue::MissingDirection => "missing_direction",
        ScriptMovementStepIssue::UnknownDirection { .. } => "unknown_direction",
        ScriptMovementStepIssue::MissingDuration => "missing_duration",
        ScriptMovementStepIssue::DurationOutOfByteRange { .. } => "duration_out_of_byte_range",
        ScriptMovementStepIssue::ZeroSleepDuration => "zero_sleep_duration",
        ScriptMovementStepIssue::UnsupportedCommand => "unsupported_command",
    }
}

fn validate_script_block_change_shape(
    map_id: &str,
    map: &MapModule,
    change: &ScriptBlockChange,
) -> Result<()> {
    validate_exact_modpack_value(
        &change.source_script,
        &format!(
            "map '{map_id}' script block change command {} source script",
            change.command_index
        ),
    )?;
    for issue in script_block_change_issues(
        std::slice::from_ref(change),
        map.attributes.width,
        map.attributes.height,
        map.blocks.len(),
    ) {
        anyhow::bail!(
            "map '{map_id}' script block change command {} in '{}' is malformed: {:?}",
            change.command_index,
            change.source_script,
            issue
        );
    }
    Ok(())
}

fn validate_script_object_command_shapes(map_id: &str, map: &MapModule) -> Result<()> {
    let object_event_flags: BTreeMap<String, String> = map
        .objects
        .iter()
        .filter_map(|object| {
            object
                .object_identifier
                .as_ref()
                .map(|object_id| (object_id.clone(), object.event_flag.clone()))
        })
        .collect();
    let hideable_event_flags: BTreeSet<String> = map
        .objects
        .iter()
        .filter(|object| is_hideable_object_event_flag(&object.event_flag))
        .map(|object| object.event_flag.clone())
        .collect();
    let movements: BTreeSet<(String, Option<String>)> = map
        .script_movements
        .iter()
        .map(|movement| (movement.label.clone(), movement.source_script.clone()))
        .collect();
    for command in &map.script_object_commands {
        validate_exact_modpack_value(
            &command.source_script,
            &format!(
                "map '{map_id}' script object command {} source script",
                command.command_index
            ),
        )?;
        for issue in script_object_command_issues(
            command,
            &object_event_flags,
            &hideable_event_flags,
            &movements,
        ) {
            anyhow::bail!(
                "map '{map_id}' script object command {} in '{}' is malformed: {}",
                command.command_index,
                command.source_script,
                script_object_command_issue_name(&issue)
            );
        }
    }
    Ok(())
}

fn script_object_command_issue_name(issue: &ScriptObjectCommandIssue) -> &'static str {
    match issue {
        ScriptObjectCommandIssue::InvalidSourceScript { .. } => "invalid_source_script",
        ScriptObjectCommandIssue::InvalidCommand { .. } => "invalid_command",
        ScriptObjectCommandIssue::MissingObjectId { .. } => "missing_object_id",
        ScriptObjectCommandIssue::UnknownObjectId { .. } => "unknown_object_id",
        ScriptObjectCommandIssue::InvalidObjectId { .. } => "invalid_object_id",
        ScriptObjectCommandIssue::UnhideableObject { .. } => "unhideable_object",
        ScriptObjectCommandIssue::MissingCoordinates { .. } => "missing_coordinates",
        ScriptObjectCommandIssue::MoveCoordinatesOutOfRange { .. } => {
            "move_coordinates_out_of_range"
        }
        ScriptObjectCommandIssue::MissingDirection { .. } => "missing_direction",
        ScriptObjectCommandIssue::UnknownDirection { .. } => "unknown_direction",
        ScriptObjectCommandIssue::MissingTargetObjectId { .. } => "missing_target_object_id",
        ScriptObjectCommandIssue::UnknownTargetObjectId { .. } => "unknown_target_object_id",
        ScriptObjectCommandIssue::InvalidTargetObjectId { .. } => "invalid_target_object_id",
        ScriptObjectCommandIssue::MissingMovement { .. } => "missing_movement",
        ScriptObjectCommandIssue::UnknownMovement { .. } => "unknown_movement",
        ScriptObjectCommandIssue::InvalidMovement { .. } => "invalid_movement",
        ScriptObjectCommandIssue::MissingEmote { .. } => "missing_emote",
        ScriptObjectCommandIssue::EmoteDurationOutOfByteRange { .. } => {
            "emote_duration_out_of_byte_range"
        }
        ScriptObjectCommandIssue::UnknownCommand { .. } => "unknown_command",
    }
}

fn validate_script_field_pickup_shape(map_id: &str, pickup: &ScriptFieldPickup) -> Result<()> {
    let context = format!(
        "map '{map_id}' script field pickup command {}",
        pickup.command_index
    );
    validate_exact_modpack_value(&pickup.command, &format!("{context} name"))?;
    validate_exact_modpack_value(&pickup.source_script, &format!("{context} source script"))?;
    if SCRIPT_FIELD_ITEM_PICKUP_COMMANDS.contains(&pickup.command.as_str()) {
        let item_id = pickup
            .item_id
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("{context} '{}' is missing item id", pickup.command))?;
        validate_modpack_payload_token(item_id, &format!("{context} item id"))?;
        if pickup.quantity == 0 {
            anyhow::bail!(
                "{context} '{}' quantity must be greater than zero",
                pickup.command
            );
        }
        let event_flag = pickup.event_flag.as_deref().ok_or_else(|| {
            anyhow::anyhow!("{context} '{}' is missing event flag", pickup.command)
        })?;
        validate_modpack_payload_token(event_flag, &format!("{context} event flag"))?;
        if pickup.fruit_tree_id.is_some() {
            anyhow::bail!(
                "{context} '{}' must not include a fruit tree id",
                pickup.command
            );
        }
        return Ok(());
    }
    if SCRIPT_FIELD_FRUIT_TREE_PICKUP_COMMANDS.contains(&pickup.command.as_str()) {
        let fruit_tree_id = pickup.fruit_tree_id.as_deref().ok_or_else(|| {
            anyhow::anyhow!("{context} '{}' is missing fruit tree id", pickup.command)
        })?;
        validate_modpack_payload_token(fruit_tree_id, &format!("{context} fruit tree id"))?;
        if pickup.quantity != 1 {
            anyhow::bail!("{context} '{}' quantity must be exactly 1", pickup.command);
        }
        if pickup.item_id.is_some() || pickup.event_flag.is_some() {
            anyhow::bail!(
                "{context} '{}' must not inline item id or event flag",
                pickup.command
            );
        }
        return Ok(());
    }
    anyhow::bail!(
        "{context} '{}' is not a known field pickup command",
        pickup.command
    );
}

fn validate_script_flag_command_shape(map_id: &str, command: &ScriptFlagCommand) -> Result<()> {
    for issue in script_flag_command_issues(command) {
        anyhow::bail!(
            "map '{map_id}' script flag command {} in '{}' is malformed: {:?}",
            command.command_index,
            command.source_script,
            issue
        );
    }
    validate_exact_modpack_value(
        &command.source_script,
        &format!(
            "map '{map_id}' script flag command {} source script",
            command.command_index
        ),
    )?;
    Ok(())
}

fn validate_script_scene_command_shape(map_id: &str, command: &ScriptSceneCommand) -> Result<()> {
    for issue in script_scene_command_issues(command) {
        anyhow::bail!(
            "map '{map_id}' script scene command {} in '{}' is malformed: {:?}",
            command.command_index,
            command.source_script,
            issue
        );
    }
    validate_exact_modpack_value(
        &command.source_script,
        &format!(
            "map '{map_id}' script scene command {} source script",
            command.command_index
        ),
    )?;
    Ok(())
}

fn validate_script_economy_command_shape(
    map_id: &str,
    command: &ScriptEconomyCommand,
) -> Result<()> {
    let context = format!(
        "map '{map_id}' script economy command {}",
        command.command_index
    );
    validate_exact_modpack_value(&command.command, &format!("{context} name"))?;
    if SCRIPT_MONEY_CHECK_COMMANDS.contains(&command.command.as_str())
        || SCRIPT_MONEY_MUTATION_COMMANDS.contains(&command.command.as_str())
    {
        let account = command.account.as_deref().ok_or_else(|| {
            anyhow::anyhow!("{context} '{}' is missing money account", command.command)
        })?;
        validate_battle_table_token(account, &format!("{context} account"))?;
    } else if SCRIPT_COIN_CHECK_COMMANDS.contains(&command.command.as_str())
        || SCRIPT_COIN_MUTATION_COMMANDS.contains(&command.command.as_str())
    {
        if command.account.is_some() {
            anyhow::bail!(
                "{context} '{}' must not include a money account",
                command.command
            );
        }
    } else {
        anyhow::bail!(
            "{context} '{}' is not a known economy command",
            command.command
        );
    }
    if command.amount_tokens.is_empty() {
        anyhow::bail!("{context} '{}' is missing amount tokens", command.command);
    }
    for (index, token) in command.amount_tokens.iter().enumerate() {
        if token == "+"
            || token == "-"
            || (!token.is_empty() && token.bytes().all(|byte| byte.is_ascii_digit()))
        {
            continue;
        }
        validate_exact_modpack_value(token, &format!("{context} amount token {index}"))?;
    }
    validate_exact_modpack_value(&command.source_script, &format!("{context} source script"))?;
    Ok(())
}

fn validate_script_item_grant_shape(map_id: &str, grant: &ScriptItemGrant) -> Result<()> {
    let context = format!(
        "map '{map_id}' script item grant command {}",
        grant.command_index
    );
    validate_modpack_payload_token(&grant.item_id, &format!("{context} item id"))?;
    if grant.quantity == 0 {
        anyhow::bail!("{context} quantity must be greater than zero");
    }
    validate_exact_modpack_value(&grant.source_script, &format!("{context} source script"))?;
    Ok(())
}

fn validate_script_item_access_shape(
    map_id: &str,
    command_kind: &str,
    access: &ScriptItemAccess,
) -> Result<()> {
    let context = format!(
        "map '{map_id}' script item {command_kind} command {}",
        access.command_index
    );
    validate_modpack_payload_token(&access.item_id, &format!("{context} item id"))?;
    validate_exact_modpack_value(&access.source_script, &format!("{context} source script"))?;
    Ok(())
}

fn validate_script_audio_command_shape(map_id: &str, command: &ScriptAudioCommand) -> Result<()> {
    let context = format!(
        "map '{map_id}' script audio command {}",
        command.command_index
    );
    validate_exact_modpack_value(&command.command, &format!("{context} name"))?;
    validate_exact_modpack_value(&command.source_script, &format!("{context} source script"))?;
    if SCRIPT_AUDIO_MUSIC_COMMANDS.contains(&command.command.as_str())
        || SCRIPT_AUDIO_SOUND_EFFECT_COMMANDS.contains(&command.command.as_str())
        || SCRIPT_AUDIO_CRY_COMMANDS.contains(&command.command.as_str())
    {
        let audio_id = command.audio_id.as_deref().ok_or_else(|| {
            anyhow::anyhow!("{context} '{}' is missing audio id", command.command)
        })?;
        validate_modpack_payload_token(audio_id, &format!("{context} audio id"))?;
        if command.fade_frames.is_some() {
            anyhow::bail!(
                "{context} '{}' must not include fade frames",
                command.command
            );
        }
        return Ok(());
    }
    if SCRIPT_AUDIO_MUSIC_FADE_COMMANDS.contains(&command.command.as_str()) {
        let audio_id = command.audio_id.as_deref().ok_or_else(|| {
            anyhow::anyhow!("{context} '{}' is missing audio id", command.command)
        })?;
        validate_modpack_payload_token(audio_id, &format!("{context} audio id"))?;
        if command.fade_frames.is_none() {
            anyhow::bail!("{context} '{}' is missing fade frames", command.command);
        }
        return Ok(());
    }
    if SCRIPT_AUDIO_NO_PAYLOAD_COMMANDS.contains(&command.command.as_str()) {
        if command.audio_id.is_some() {
            anyhow::bail!(
                "{context} '{}' must not include an audio id",
                command.command
            );
        }
        if command.fade_frames.is_some() {
            anyhow::bail!(
                "{context} '{}' must not include fade frames",
                command.command
            );
        }
        return Ok(());
    }
    anyhow::bail!(
        "{context} '{}' is not a known audio command",
        command.command
    );
}

fn validate_map_module_blocks(map_id: &str, map: &MapModule) -> Result<()> {
    let expected_blocks = map.attributes.width as usize * map.attributes.height as usize;
    if expected_blocks == 0 {
        anyhow::bail!("map module '{map_id}' width and height must both be greater than zero");
    }
    if map.blocks.is_empty() {
        let Some(blocks_label) = map
            .attributes
            .blocks_label
            .as_deref()
            .filter(|label| !label.trim().is_empty())
        else {
            anyhow::bail!(
                "map module '{map_id}' must declare inline blocks or an exact blocks_label"
            );
        };
        validate_exact_modpack_value(blocks_label, "map module blocks_label")?;
    } else if map.blocks.len() != expected_blocks {
        anyhow::bail!(
            "map module '{map_id}' has {} inline blocks but dimensions require {expected_blocks}",
            map.blocks.len()
        );
    }
    Ok(())
}

fn validate_runtime_overworld_map_blocks(map_id: &str, map: &MapModule) -> Result<()> {
    let expected_blocks = (map.attributes.width as usize)
        .checked_mul(map.attributes.height as usize)
        .with_context(|| format!("runtime map '{map_id}' dimensions overflow block count"))?;
    let metatile_width = u16::try_from(METATILE_WIDTH)
        .with_context(|| format!("runtime map '{map_id}' metatile width is outside u16 bounds"))?;
    let subtile_width = map
        .attributes
        .width
        .checked_mul(metatile_width)
        .with_context(|| format!("runtime map '{map_id}' width overflows subtile bounds"))?;
    let subtile_height = map
        .attributes
        .height
        .checked_mul(metatile_width)
        .with_context(|| format!("runtime map '{map_id}' height overflows subtile bounds"))?;
    if expected_blocks == 0 {
        anyhow::bail!("runtime map '{map_id}' width and height must both be greater than zero");
    }
    if subtile_width > i16::MAX as u16 || subtile_height > i16::MAX as u16 {
        anyhow::bail!(
            "runtime map '{map_id}' subtile dimensions {subtile_width}x{subtile_height} exceed TilePosition bounds"
        );
    }
    if map.blocks.len() != expected_blocks {
        anyhow::bail!(
            "runtime map '{map_id}' has {} blocks but dimensions require {expected_blocks}",
            map.blocks.len()
        );
    }
    Ok(())
}

fn validate_map_module_scripts(map_id: &str, scripts: &BTreeMap<String, Value>) -> Result<()> {
    for (script_key, payload) in scripts {
        validate_script_label_payload_token(script_key, "map module script")?;
        validate_raw_script_command_list("map module script payload", script_key, payload)
            .with_context(|| format!("validate map module '{map_id}' script '{script_key}'"))?;
    }
    Ok(())
}

fn insert_wild_encounter_data(
    target: &mut BTreeMap<String, WildEncounterData>,
    data: WildEncounterData,
) -> Result<()> {
    let map_name = data.map_name.clone();
    validate_exact_encounter_token(&map_name, "wild encounter map name")?;
    validate_wild_encounter_species_tokens(&map_name, &data)?;
    if target.contains_key(&map_name) {
        anyhow::bail!("duplicate wild encounter data for map '{map_name}'");
    }
    target.insert(map_name, data);
    Ok(())
}

fn insert_keyed_wild_encounter_data(
    target: &mut BTreeMap<String, WildEncounterData>,
    map_name: String,
    data: WildEncounterData,
) -> Result<()> {
    if map_name != data.map_name {
        anyhow::bail!(
            "wild encounter key '{map_name}' does not match record map_name '{}'",
            data.map_name
        );
    }
    insert_wild_encounter_data(target, data)
}

fn insert_field_encounter_data(
    target: &mut BTreeMap<String, FieldEncounterData>,
    data: FieldEncounterData,
) -> Result<()> {
    let map_name = data.map_name.clone();
    validate_exact_encounter_token(&map_name, "field encounter map name")?;
    validate_field_encounter_species_tokens(&map_name, &data)?;
    if target.contains_key(&map_name) {
        anyhow::bail!("duplicate field encounter data for map '{map_name}'");
    }
    target.insert(map_name, data);
    Ok(())
}

fn insert_keyed_field_encounter_data(
    target: &mut BTreeMap<String, FieldEncounterData>,
    map_name: String,
    data: FieldEncounterData,
) -> Result<()> {
    if map_name != data.map_name {
        anyhow::bail!(
            "field encounter key '{map_name}' does not match record map_name '{}'",
            data.map_name
        );
    }
    insert_field_encounter_data(target, data)
}

fn validate_wild_encounter_species_tokens(map_name: &str, data: &WildEncounterData) -> Result<()> {
    if let Some(rates) = data.grass_rates.as_ref() {
        for time_key in rates.keys() {
            validate_exact_encounter_token(
                time_key,
                &format!("wild encounter grass rate time for map {map_name}"),
            )?;
            if !ENCOUNTER_TIME_KEYS.contains(&time_key.as_str()) {
                anyhow::bail!(
                    "wild encounter grass rate time for map {map_name} '{time_key}' must be morning, day, or night"
                );
            }
        }
    }
    for table in [data.grass.as_ref(), data.water.as_ref()]
        .into_iter()
        .flatten()
    {
        for encounter in table
            .morning
            .iter()
            .chain(table.day.iter())
            .chain(table.night.iter())
        {
            validate_exact_encounter_token(
                &encounter.species,
                &format!("wild encounter species for map {map_name}"),
            )?;
        }
    }
    if let Some(grass) = data.grass.as_ref() {
        for time_key in ENCOUNTER_TIME_KEYS {
            let rate = data
                .grass_rates
                .as_ref()
                .and_then(|rates| rates.get(*time_key))
                .copied();
            if rate.is_none() {
                anyhow::bail!(
                    "wild encounter map {map_name} grass table requires {time_key} grass rate"
                );
            }
            if rate.is_some_and(|rate| rate > 0) {
                let slots = match *time_key {
                    "morning" => &grass.morning,
                    "day" => &grass.day,
                    "night" => &grass.night,
                    _ => unreachable!("core encounter time key must be handled"),
                };
                if slots.is_empty() {
                    anyhow::bail!(
                        "wild encounter map {map_name} has positive {time_key} grass rate but no {time_key} grass slots"
                    );
                }
            }
        }
    } else if data
        .grass_rates
        .as_ref()
        .is_some_and(|rates| rates.values().any(|rate| *rate > 0))
    {
        anyhow::bail!("wild encounter map {map_name} has positive grass rates but no grass table");
    }
    if let Some(water) = data.water.as_ref() {
        if data.water_rate.is_none() {
            anyhow::bail!("wild encounter map {map_name} water table requires water_rate");
        }
        if data.water_rate.is_some_and(|rate| rate > 0) {
            for (time_key, slots) in [
                ("morning", &water.morning),
                ("day", &water.day),
                ("night", &water.night),
            ] {
                if slots.is_empty() {
                    anyhow::bail!(
                        "wild encounter map {map_name} has positive water rate but no {time_key} water slots"
                    );
                }
            }
        }
    } else if data.water_rate.is_some_and(|rate| rate > 0) {
        anyhow::bail!("wild encounter map {map_name} has positive water rate but no water table");
    }
    Ok(())
}

fn validate_field_encounter_species_tokens(
    map_name: &str,
    data: &FieldEncounterData,
) -> Result<()> {
    for (kind, table) in &data.tables {
        validate_exact_encounter_token(
            kind,
            &format!("field encounter table kind for map {map_name}"),
        )?;
        if kind != FieldEncounterKind::Headbutt.as_key()
            && kind != FieldEncounterKind::RockSmash.as_key()
        {
            anyhow::bail!(
                "field encounter table kind for map {map_name} '{kind}' must be headbutt or rock_smash"
            );
        }
        for encounter in table.common.iter().chain(table.rare.iter()) {
            validate_exact_encounter_token(
                &encounter.species,
                &format!("field encounter species for map {map_name}"),
            )?;
        }
        validate_field_encounter_bucket(map_name, kind, "common", &table.common)?;
        if kind == FieldEncounterKind::Headbutt.as_key() {
            validate_field_encounter_bucket(map_name, kind, "rare", &table.rare)?;
        }
    }
    Ok(())
}

fn validate_field_encounter_bucket(
    map_name: &str,
    kind: &str,
    bucket: &str,
    entries: &[crystal_core::world::encounters::FieldEncounterEntry],
) -> Result<()> {
    if entries.is_empty() {
        anyhow::bail!("field encounter map {map_name} {kind} {bucket} bucket must not be empty");
    }
    for (index, entry) in entries.iter().enumerate() {
        if entry.weight == 0 {
            anyhow::bail!(
                "field encounter map {map_name} {kind} {bucket} entry {index} for species '{}' must have nonzero weight",
                entry.species
            );
        }
    }
    let total_weight: u16 = entries.iter().map(|entry| u16::from(entry.weight)).sum();
    if total_weight != 100 {
        anyhow::bail!(
            "field encounter map {map_name} {kind} {bucket} bucket weights must total 100, found {total_weight}"
        );
    }
    Ok(())
}

fn validate_exact_encounter_token(value: &str, description: &str) -> Result<()> {
    if value.is_empty()
        || value.trim() != value
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        anyhow::bail!("{description} '{value}' must be an exact non-empty encounter token");
    }
    Ok(())
}

fn merge_buena_password_categories(
    target: &mut BuenaPasswordCategories,
    source: BuenaPasswordCategories,
) -> Result<()> {
    let mut ordered_ids = BTreeSet::new();
    for id in source.order {
        validate_battle_table_token(&id, "Buena password category id")?;
        if !ordered_ids.insert(id.clone()) {
            anyhow::bail!("duplicate Buena password category id '{id}'");
        }
        let Some(category) = source.categories.get(&id) else {
            anyhow::bail!("Buena password category order references missing id '{id}'");
        };
        insert_buena_password_category(target, id, category.clone())?;
    }
    for id in source.categories.keys() {
        validate_battle_table_token(id, "Buena password category id")?;
        if !ordered_ids.contains(id) {
            anyhow::bail!("Buena password category '{id}' is missing from order");
        }
    }
    Ok(())
}

fn insert_buena_password_category(
    target: &mut BuenaPasswordCategories,
    category_id: String,
    category: BuenaPasswordCategoryDefinition,
) -> Result<()> {
    if target.categories.contains_key(&category_id) {
        anyhow::bail!("duplicate Buena password category id '{category_id}'");
    }
    validate_battle_table_token(&category_id, "Buena password category id")?;
    validate_no_reserved_payload_token(&category_id, "Buena password category id")?;
    validate_battle_table_token(&category.category_type, "Buena password category type id")?;
    if !crystal_core::systems::special_routines::is_known_buena_password_category_type(
        &category.category_type,
    ) {
        anyhow::bail!(
            "Buena password category '{category_id}' has unknown type '{}'",
            category.category_type
        );
    }
    if category.points == 0 {
        anyhow::bail!("Buena password category '{category_id}' points must be nonzero");
    }
    if category.options.is_empty() {
        anyhow::bail!("Buena password category '{category_id}' must declare options");
    }
    for (index, option) in category.options.iter().enumerate() {
        if category.category_type == "BUENA_STRING" {
            validate_exact_modpack_value(
                option,
                &format!("Buena password category '{category_id}' option {index}"),
            )?;
        } else {
            validate_battle_table_token(
                option,
                &format!("Buena password category '{category_id}' option {index}"),
            )?;
        }
    }
    target.order.push(category_id.clone());
    target.categories.insert(category_id, category);
    Ok(())
}

fn merge_buena_prizes(
    target: &mut BuenaPrizeDefinitions,
    source: BuenaPrizeDefinitions,
) -> Result<()> {
    for (item_id, cost) in source {
        insert_buena_prize(target, item_id, cost)?;
    }
    Ok(())
}

fn insert_buena_prize(target: &mut BuenaPrizeDefinitions, item_id: String, cost: u8) -> Result<()> {
    if target.contains_key(&item_id) {
        anyhow::bail!("duplicate Buena prize item id '{item_id}'");
    }
    validate_modpack_payload_token(&item_id, "Buena prize item id")?;
    if cost == 0 {
        anyhow::bail!("Buena prize item '{item_id}' cost must be nonzero");
    }
    target.insert(item_id, cost);
    Ok(())
}
