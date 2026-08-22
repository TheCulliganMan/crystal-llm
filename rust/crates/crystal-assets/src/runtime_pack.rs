#[derive(Debug, Clone, PartialEq)]
pub struct LoadedCompiledGamePack {
    path: PathBuf,
    bytes: Vec<u8>,
    pack: CompiledGamePack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldRepelItemUseOutcome {
    pub item_use: ItemUseOutcome,
    pub repel_steps_before: u16,
    pub repel_steps_after: u16,
    pub active_repel_item_before: Option<String>,
    pub active_repel_item_after: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldBicycleItemUseOutcome {
    pub item_use: ItemUseOutcome,
    pub map_name: String,
    pub permission: u8,
    pub mode_before: MovementMode,
    pub mode_after: MovementMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldItemfinderUseOutcome {
    pub item_use: ItemUseOutcome,
    pub player_tile: TilePosition,
    pub found: Option<CoreItemfinderHiddenItem>,
    pub itemfinder_sound_cues: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSquirtBottleUseOutcome {
    pub item_use: ItemUseOutcome,
    pub player_tile: TilePosition,
    pub target_tile: TilePosition,
    pub target_object_identifier: Option<String>,
    pub target_movement: String,
    pub target_script: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldStoryKeyUseOutcome {
    pub item_use: ItemUseOutcome,
    pub map_name: String,
    pub player_tile: TilePosition,
    pub target_tile: TilePosition,
    pub target_script: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldKeyItemBalanceUseOutcome {
    pub item_use: ItemUseOutcome,
    pub balance_label: String,
    pub balance: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldTownMapUseOutcome {
    pub item_use: ItemUseOutcome,
    pub map_name: String,
    pub map_constant: String,
    pub environment: String,
    pub landmark: PokegearLandmark,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldPokegearUseOutcome {
    pub item_use: ItemUseOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingMoveLearnRuntimeResolution {
    pub resolution: PendingMoveLearnResolution,
    pub deferred_evolution: Option<EvolutionReport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldBoxItemUseOutcome {
    pub item_use: ItemUseOutcome,
    pub decoration_flag: String,
    pub already_owned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SweetScentEncounterOutcome {
    pub wild_encounter: Option<WildEncounterRoll>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FishingCastOutcome {
    pub session: FishingSession,
    pub bite: Option<bool>,
    pub wild_battle: Option<WildBattleStart>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FishingRodItemUseOutcome {
    pub item_use: ItemUseOutcome,
    pub rod: String,
    pub cast: FishingCastOutcome,
    pub cast_state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleEscapeItemUseOutcome {
    pub item_use: ItemUseOutcome,
    pub battle_escape_mode: String,
    pub escaped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleStateItemUseOutcome {
    pub item_use: ItemUseOutcome,
    pub stat_drop_guard_turns_before: u8,
    pub stat_drop_guard_turns_after: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveBattleCommandOutcome {
    Turn(BattleTurnOutcome),
    Escape(BattleEscapeAttempt),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldEscapeRopeUseOutcome {
    pub item_use: ItemUseOutcome,
    pub source_map: String,
    pub destination_map: String,
    pub destination_warp_index: u16,
    pub destination_tile: TilePosition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlyFieldMoveOutcome {
    pub actor_party_index: usize,
    pub actor_species: String,
    pub flypoint_flag: String,
    pub source_map: String,
    pub destination_spawn_identifier: u16,
    pub destination_map: String,
    pub destination_tile: TilePosition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigFieldMoveOutcome {
    pub actor_party_index: usize,
    pub actor_species: String,
    pub source_map: String,
    pub destination_map: String,
    pub destination_warp_index: u16,
    pub destination_tile: TilePosition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeleportFieldMoveOutcome {
    pub actor_party_index: usize,
    pub actor_species: String,
    pub source_map: String,
    pub destination_spawn_identifier: u16,
    pub destination_map: String,
    pub destination_tile: TilePosition,
}

impl LoadedCompiledGamePack {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn pack(&self) -> &CompiledGamePack {
        &self.pack
    }

    pub fn save_modpack_identity(&self) -> Result<SaveModpackIdentity> {
        SaveModpackIdentity::from_compiled_pack_bytes(
            self.pack.runtime_modpack_id()?,
            self.bytes.as_slice(),
        )
        .map_err(anyhow::Error::from)
    }

    pub fn pack_identity(&self) -> Result<CompiledGamePackIdentity> {
        self.pack.identity()
    }

    pub fn pack_content_hash(&self) -> Result<String> {
        Ok(self.pack_identity()?.content_hash)
    }

    pub fn into_parts(self) -> (PathBuf, Vec<u8>, CompiledGamePack) {
        (self.path, self.bytes, self.pack)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackCompileReport {
    pub manifests: Vec<String>,
    pub maps: usize,
    pub pokemon: usize,
    pub moves: usize,
    pub items: usize,
    pub graph_edges: Vec<PlayabilityGraphEdge>,
    pub reachable_maps: Vec<String>,
    pub solvable_maps: Vec<String>,
    pub solvable_events: Vec<String>,
    pub solvable_items: Vec<String>,
    pub diagnostics: Vec<VerificationError>,
}

impl ModpackCompileReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == VerificationSeverity::Error)
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
#[cfg_attr(not(test), allow(dead_code))]
fn canonical_test_compile_report(data: &GameDataSet, manifest_id: &str) -> ModpackCompileReport {
    let reachable_maps = data.maps.keys().cloned().collect::<Vec<_>>();
    let solvable_maps = data.maps.keys().cloned().collect::<Vec<_>>();
    let solvable_events = declared_progression_events(&data.playability)
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let solvable_items = declared_progression_items(&data.playability)
        .into_iter()
        .filter(|item| data.items.contains_key(*item))
        .map(str::to_string)
        .collect::<Vec<_>>();
    ModpackCompileReport {
        manifests: vec![manifest_id.to_string()],
        maps: data.maps.len(),
        pokemon: data.pokemon.len(),
        moves: data.moves.len(),
        items: data.items.len(),
        reachable_maps,
        solvable_maps,
        solvable_events,
        solvable_items,
        ..ModpackCompileReport::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayabilityGraphEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum VerificationSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationError {
    pub severity: VerificationSeverity,
    pub code: String,
    pub subject: String,
    pub message: String,
}

impl VerificationError {
    fn error(
        code: impl Into<String>,
        subject: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: VerificationSeverity::Error,
            code: code.into(),
            subject: subject.into(),
            message: message.into(),
        }
    }

    fn warning(
        code: impl Into<String>,
        subject: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: VerificationSeverity::Warning,
            code: code.into(),
            subject: subject.into(),
            message: message.into(),
        }
    }
}

pub struct ModpackCompiler<'a> {
    asset_root: &'a AssetRoot,
}

impl<'a> ModpackCompiler<'a> {
    pub fn new(asset_root: &'a AssetRoot) -> Self {
        Self { asset_root }
    }

    pub fn compile(
        &self,
        manifests: &[ModpackManifest],
        options: ModpackCompileOptions,
    ) -> Result<CompiledModpack> {
        let mut seen_manifest_ids = BTreeSet::new();
        for manifest in manifests {
            validate_manifest_shape(manifest)?;
            if !seen_manifest_ids.insert(manifest.id().to_string()) {
                anyhow::bail!("duplicate modpack manifest id '{}'", manifest.id());
            }
        }
        for manifest in manifests {
            for dependency in &manifest.dependencies {
                if !seen_manifest_ids.contains(dependency) {
                    anyhow::bail!(
                        "modpack '{}' depends on missing modpack '{}'",
                        manifest.id(),
                        dependency
                    );
                }
            }
        }
        validate_manifest_dependency_graph(manifests)?;
        let manifests_ordered = ordered_manifests_for_application(manifests)?;
        let mut data = GameDataSet::load_base_json_for_compile(self.asset_root)?;
        for manifest in &manifests_ordered {
            if manifest.payload != ModpackPayload::default() {
                data.apply_modpack(manifest)?;
            }
        }
        materialize_runtime_map_modules(&mut data)?;

        let playability = merged_playability_rules(&data.playability, &options.playability)?;
        let mut report = verify_game_data(self.asset_root, &data, &playability);
        report.manifests = if manifests_ordered.is_empty() {
            vec!["core-modular".to_string()]
        } else {
            manifests_ordered
                .into_iter()
                .map(|manifest| manifest.id().to_string())
                .collect()
        };
        report.maps = data.maps.len();
        report.pokemon = data.pokemon.len();
        report.moves = data.moves.len();
        report.items = data.items.len();

        if report.has_errors() {
            let summary = report
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.severity == VerificationSeverity::Error)
                .take(8)
                .map(|diagnostic| {
                    format!(
                        "{} [{}]: {}",
                        diagnostic.subject, diagnostic.code, diagnostic.message
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            anyhow::bail!("modpack verification failed: {summary}");
        }

        let compiled_audio = compile_audio_payloads(self.asset_root, &mut data)?;
        let audio_manifest = ModpackAudioManifest::from_assets(&data.audio, &compiled_audio)?;
        let runtime_files = compile_runtime_files(self.asset_root)?;
        let identity = derive_compiled_game_pack_identity(
            COMPILED_GAME_PACK_FORMAT_VERSION,
            &data,
            &compiled_audio,
            &runtime_files,
            &report,
        )?;
        Ok(CompiledModpack {
            data,
            compiled_audio,
            audio_manifest,
            runtime_files,
            report,
            identity,
        })
    }
}

fn compile_audio_payloads(
    asset_root: &AssetRoot,
    data: &mut GameDataSet,
) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut compiled_audio = BTreeMap::new();
    for audio_asset in &mut data.audio {
        audio_asset
            .validate()
            .with_context(|| format!("validate audio asset {}", audio_asset.id))?;
        let path = asset_root
            .resolve_data_path(&audio_asset.path)
            .with_context(|| format!("resolve audio asset {}", audio_asset.id))?;
        // The development content index intentionally enables a tiny test
        // pack.  Its one-byte audio fixtures must never replace the real
        // core soundtrack when producing the shipped core pack.
        let path = if audio_asset.path.starts_with("content-packs/test/") {
            let core_path = asset_root.resolve_data_path(audio_asset.path.replacen(
                "content-packs/test/",
                "content-packs/core-modular/",
                1,
            ))?;
            if core_path.is_file() { core_path } else { path }
        } else {
            path
        };
        let source_bytes = std::fs::read(&path).with_context(|| {
            format!(
                "read audio asset {} from {}",
                audio_asset.id, audio_asset.path
            )
        })?;
        let bytes = prepare_pcm_for_runtime(audio_asset, source_bytes)?;
        validate_compiled_audio_payload(audio_asset, &bytes).with_context(|| {
            format!(
                "validate compiled audio payload {} ({} bytes at {})",
                audio_asset.id,
                bytes.len(),
                path.display()
            )
        })?;
        if compiled_audio
            .insert(audio_asset.id.clone(), bytes)
            .is_some()
        {
            anyhow::bail!("duplicate compiled audio payload '{}'", audio_asset.id);
        }
    }
    Ok(compiled_audio)
}

fn prepare_pcm_for_runtime(audio_asset: &mut ModpackAudioAsset, bytes: Vec<u8>) -> Result<Vec<u8>> {
    let Some(format) = audio_asset.pcm_format.as_ref() else {
        return Ok(bytes);
    };
    if !matches!(audio_asset.source, ModpackAudioSource::Pcm)
        || format.sample_rate_hz != 44_100
        || format.channels != 2
        || format.bits_per_sample != 16
    {
        return Ok(bytes);
    }
    let frame_size = 4usize;
    if bytes.len() % frame_size != 0 {
        return Ok(bytes);
    }
    let input_frames = bytes.len() / frame_size;
    let output_frames = input_frames.div_ceil(2);
    let mut output = Vec::with_capacity(output_frames * 2);
    for frame in (0..input_frames).step_by(2) {
        let offset = frame * frame_size;
        let left = i16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as i32;
        let right = i16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]) as i32;
        let mono = ((left + right) / 2) as i16;
        output.extend_from_slice(&mono.to_le_bytes());
    }
    audio_asset.pcm_format = Some(ModpackPcmAudioFormat {
        sample_rate_hz: 22_050,
        channels: 1,
        bits_per_sample: 16,
    });
    audio_asset.pcm_frame_count = Some(output_frames);
    audio_asset.payload_hash = Some(format!("{:08x}", fnv1a32_bytes(&output)));
    audio_asset.loop_start_sample = audio_asset.loop_start_sample.map(|sample| sample / 2);
    audio_asset.loop_end_sample = audio_asset.loop_end_sample.map(|sample| sample / 2);
    Ok(output)
}

pub const REQUIRED_VENDOR_RUNTIME_FILE_KEYS: &[&str] = &[
    "vendor/pokecrystal/constants/credits_constants.asm",
    "vendor/pokecrystal/constants/move_constants.asm",
    "vendor/pokecrystal/data/credits_script.asm",
    "vendor/pokecrystal/data/credits_strings.asm",
    "vendor/pokecrystal/data/moves/descriptions.asm",
    "vendor/pokecrystal/gfx/card_flip/card_flip.pal",
    "vendor/pokecrystal/gfx/card_flip/card_flip.tilemap",
    "vendor/pokecrystal/gfx/card_flip/card_flip_1.png",
    "vendor/pokecrystal/gfx/card_flip/card_flip_2.png",
    "vendor/pokecrystal/gfx/card_flip/card_flip_3.png",
    "vendor/pokecrystal/gfx/card_flip/off.png",
    "vendor/pokecrystal/gfx/card_flip/on.png",
    "vendor/pokecrystal/gfx/diploma/diploma.pal",
    "vendor/pokecrystal/gfx/diploma/diploma.png",
    "vendor/pokecrystal/gfx/diploma/page1.tilemap",
    "vendor/pokecrystal/gfx/overworld/heal_machine.pal",
    "vendor/pokecrystal/gfx/overworld/heal_machine.png",
    "vendor/pokecrystal/gfx/overworld/magnet_train_bg.tilemap",
    "vendor/pokecrystal/gfx/overworld/magnet_train_fg.tilemap",
    "vendor/pokecrystal/gfx/printer/bold_a.png",
    "vendor/pokecrystal/gfx/printer/bold_b.png",
    "vendor/pokecrystal/gfx/slots/slots.pal",
    "vendor/pokecrystal/gfx/slots/slots.tilemap",
    "vendor/pokecrystal/gfx/slots/slots_1.png",
    "vendor/pokecrystal/gfx/slots/slots_2.png",
    "vendor/pokecrystal/gfx/slots/slots_3.png",
    "vendor/pokecrystal/gfx/tilesets/train_station.png",
    "vendor/pokecrystal/gfx/unown_puzzle/aerodactyl.png",
    "vendor/pokecrystal/gfx/unown_puzzle/cursor.png",
    "vendor/pokecrystal/gfx/unown_puzzle/hooh.png",
    "vendor/pokecrystal/gfx/unown_puzzle/kabuto.png",
    "vendor/pokecrystal/gfx/unown_puzzle/omanyte.png",
    "vendor/pokecrystal/gfx/unown_puzzle/start_cancel.png",
    "vendor/pokecrystal/gfx/unown_puzzle/tile_borders.png",
];

fn compile_runtime_files(asset_root: &AssetRoot) -> Result<BTreeMap<String, Vec<u8>>> {
    let root = asset_root.runtime_assets();
    let mut files = BTreeMap::new();
    collect_runtime_files(&root, &root, &mut files)?;
    for &relative in REQUIRED_VENDOR_RUNTIME_FILE_KEYS {
        let source = asset_root.repository_root.join(relative);
        let bytes = std::fs::read(&source)
            .with_context(|| format!("read embedded vendor runtime asset {}", source.display()))?;
        if files.insert(relative.to_string(), bytes).is_some() {
            anyhow::bail!("duplicate embedded runtime asset '{relative}'");
        }
    }
    validate_compiled_runtime_files(&files)?;
    Ok(files)
}

fn collect_runtime_files(
    root: &Path,
    directory: &Path,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    for entry in std::fs::read_dir(directory)
        .with_context(|| format!("read runtime asset directory {}", directory.display()))?
    {
        let entry = entry
            .with_context(|| format!("read runtime asset entry in {}", directory.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_runtime_files(root, &path, files)?;
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .with_context(|| format!("relativize runtime asset {}", path.display()))?;
        let extension = path.extension().and_then(|extension| extension.to_str());
        // Only retain files consumed by the presentation loaders.  Source,
        // metadata, compressed build intermediates, and audio are either
        // compiled into typed pack sections or intentionally streamed lazily.
        // Keeping them out of the mount materially reduces startup I/O.
        if !matches!(
            extension,
            Some(
                "png"
                    | "2bpp"
                    | "1bpp"
                    | "gbcpal"
                    | "pal"
                    | "tilemap"
                    | "bin"
                    | "attrmap"
                    | "dimensions"
            )
        ) {
            continue;
        }
        let key = relative
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        let bytes = std::fs::read(&path)
            .with_context(|| format!("read runtime asset {}", path.display()))?;
        if files.insert(key.clone(), bytes).is_some() {
            anyhow::bail!("duplicate embedded runtime asset '{key}'");
        }
    }
    Ok(())
}

#[cfg(any(test, feature = "test-fixtures"))]
fn synthetic_compiled_audio_for_tests(data: &GameDataSet) -> BTreeMap<String, Vec<u8>> {
    data.audio
        .iter()
        .map(|asset| {
            asset
                .validate()
                .unwrap_or_else(|error| panic!("invalid test audio asset {}: {error:#}", asset.id));
            let bytes = match asset.source {
                ModpackAudioSource::Midi => {
                    b"MThd\x00\x00\x00\x06\x00\x00\x00\x01\x00\x60MTrk\x00\x00\x00\x0c\x00\x90\x3c\x40\x60\x80\x3c\x40\x00\xff\x2f\x00".to_vec()
                }
                ModpackAudioSource::Pcm => {
                    let format = asset.pcm_format.as_ref().unwrap_or_else(|| {
                        panic!("test PCM audio asset {} missing format", asset.id)
                    });
                    let frame_size = format.frame_size_bytes(&asset.id).unwrap_or_else(|error| {
                        panic!("invalid test PCM audio asset {}: {error:#}", asset.id)
                    });
                    vec![0; frame_size]
                }
            };
            (asset.id.clone(), bytes)
        })
        .collect()
}

#[cfg(any(test, feature = "test-fixtures"))]
fn normalize_test_pcm_audio_metadata(data: &mut GameDataSet) {
    for asset in &mut data.audio {
        if matches!(asset.source, ModpackAudioSource::Pcm) {
            let frame_size = asset
                .pcm_format
                .as_ref()
                .and_then(|format| format.frame_size_bytes(&asset.id).ok())
                .unwrap_or(1);
            asset.pcm_frame_count = Some(1);
            asset.payload_hash = Some(format!("{:08x}", fnv1a32_bytes(&vec![0; frame_size])));
            asset.loop_start_sample = None;
            asset.loop_end_sample = None;
        }
    }
}

pub fn validate_compiled_audio_payload(asset: &ModpackAudioAsset, bytes: &[u8]) -> Result<()> {
    match asset.source {
        ModpackAudioSource::Midi => {
            validate_standard_midi_payload(&asset.id, bytes)?;
        }
        ModpackAudioSource::Pcm => {
            if bytes.is_empty() {
                anyhow::bail!("compiled PCM audio asset '{}' is empty", asset.id);
            }
            let Some(format) = &asset.pcm_format else {
                anyhow::bail!(
                    "compiled PCM audio asset '{}' is missing pcm_format",
                    asset.id
                );
            };
            let frame_size = format.frame_size_bytes(&asset.id)?;
            if bytes.len() % frame_size != 0 {
                anyhow::bail!(
                    "compiled PCM audio asset '{}' has {} bytes, not a whole number of {}-byte frames",
                    asset.id,
                    bytes.len(),
                    frame_size
                );
            }
        }
    }
    Ok(())
}

fn validate_standard_midi_payload(id: &str, bytes: &[u8]) -> Result<()> {
    let mut offset = 0usize;
    read_audio_chunk_tag(id, bytes, &mut offset, b"MThd")?;
    let header_len = read_audio_be_u32(id, bytes, &mut offset)? as usize;
    if header_len != 6 {
        anyhow::bail!("compiled MIDI audio asset '{id}' header length {header_len} is not 6");
    }
    let format = read_audio_be_u16(id, bytes, &mut offset)?;
    let track_count = read_audio_be_u16(id, bytes, &mut offset)?;
    let division = read_audio_be_u16(id, bytes, &mut offset)?;
    if format > 1 {
        anyhow::bail!("compiled MIDI audio asset '{id}' format {format} is not supported");
    }
    if format == 0 && track_count != 1 {
        anyhow::bail!("compiled MIDI audio asset '{id}' format 0 must contain exactly one track");
    }
    if track_count == 0 {
        anyhow::bail!("compiled MIDI audio asset '{id}' has no tracks");
    }
    if division & 0x8000 != 0 {
        anyhow::bail!("compiled MIDI audio asset '{id}' uses unsupported SMPTE timing");
    }
    if division == 0 {
        anyhow::bail!("compiled MIDI audio asset '{id}' ticks_per_quarter must be positive");
    }
    let mut has_note_event = false;
    for track_index in 0..track_count {
        read_audio_chunk_tag(id, bytes, &mut offset, b"MTrk")?;
        let track_len = read_audio_be_u32(id, bytes, &mut offset)? as usize;
        let track_end = offset.checked_add(track_len).ok_or_else(|| {
            anyhow::anyhow!("compiled MIDI audio asset '{id}' track {track_index} length overflow")
        })?;
        if track_end > bytes.len() {
            anyhow::bail!(
                "compiled MIDI audio asset '{id}' track {track_index} exceeds payload length"
            );
        }
        if validate_standard_midi_track_has_note(id, bytes, offset, track_end)? {
            has_note_event = true;
        }
        offset = track_end;
    }
    if offset != bytes.len() {
        anyhow::bail!("compiled MIDI audio asset '{id}' has trailing bytes after declared tracks");
    }
    if !has_note_event {
        anyhow::bail!("compiled MIDI audio asset '{id}' contains no note events");
    }
    Ok(())
}

fn validate_standard_midi_track_has_note(
    id: &str,
    bytes: &[u8],
    start: usize,
    end: usize,
) -> Result<bool> {
    let mut offset = start;
    let mut absolute_tick = 0u64;
    let mut running_status: Option<u8> = None;
    let mut active_notes = BTreeMap::<(u8, u8), Vec<u64>>::new();
    while offset < end {
        absolute_tick = absolute_tick
            .checked_add(read_audio_var_len(id, bytes, &mut offset, end)? as u64)
            .ok_or_else(|| {
                anyhow::anyhow!("compiled MIDI audio asset '{id}' absolute tick overflow")
            })?;
        let first = read_audio_u8(id, bytes, &mut offset, end)?;
        let status = if first & 0x80 != 0 {
            first
        } else {
            running_status.with_context(|| {
                format!("compiled MIDI audio asset '{id}' uses running status before status byte")
            })?
        };
        let first_data = if first & 0x80 == 0 { Some(first) } else { None };
        match status {
            0xff => {
                running_status = None;
                let meta_type = read_audio_u8(id, bytes, &mut offset, end)?;
                let len = read_audio_var_len(id, bytes, &mut offset, end)? as usize;
                skip_audio_bytes(id, bytes, &mut offset, end, len)?;
                if meta_type == 0x2f {
                    break;
                }
            }
            0xf0 | 0xf7 => {
                running_status = None;
                let len = read_audio_var_len(id, bytes, &mut offset, end)? as usize;
                skip_audio_bytes(id, bytes, &mut offset, end, len)?;
            }
            0x80..=0xef => {
                running_status = Some(status);
                let channel = status & 0x0f;
                let command = status & 0xf0;
                let data_len = midi_audio_channel_data_len(id, command)?;
                let data1 = match first_data {
                    Some(value) => value,
                    None => read_audio_u8(id, bytes, &mut offset, end)?,
                };
                let data2 = if data_len == 2 {
                    Some(read_audio_u8(id, bytes, &mut offset, end)?)
                } else {
                    None
                };
                match (command, data2) {
                    (0x80, Some(_)) | (0x90, Some(0)) => {
                        let key = (channel, data1);
                        if let Some(stack) = active_notes.get_mut(&key) {
                            if let Some(start_tick) = stack.pop() {
                                if absolute_tick > start_tick {
                                    return Ok(true);
                                }
                            }
                            if stack.is_empty() {
                                active_notes.remove(&key);
                            }
                        }
                    }
                    (0x90, Some(_velocity)) => {
                        active_notes
                            .entry((channel, data1))
                            .or_default()
                            .push(absolute_tick);
                    }
                    _ => {}
                }
            }
            _ => anyhow::bail!(
                "compiled MIDI audio asset '{id}' has invalid status byte {status:#04x}"
            ),
        }
    }
    Ok(false)
}

fn midi_audio_channel_data_len(id: &str, command: u8) -> Result<usize> {
    match command {
        0xc0 | 0xd0 => Ok(1),
        0x80 | 0x90 | 0xa0 | 0xb0 | 0xe0 => Ok(2),
        _ => anyhow::bail!(
            "compiled MIDI audio asset '{id}' has invalid channel command {command:#04x}"
        ),
    }
}

fn read_audio_chunk_tag(
    id: &str,
    bytes: &[u8],
    offset: &mut usize,
    expected: &[u8; 4],
) -> Result<()> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| anyhow::anyhow!("compiled MIDI audio asset '{id}' chunk offset overflow"))?;
    if end > bytes.len() {
        anyhow::bail!("compiled MIDI audio asset '{id}' ended before chunk tag");
    }
    if &bytes[*offset..end] != expected {
        anyhow::bail!(
            "compiled MIDI audio asset '{id}' expected {} chunk",
            std::str::from_utf8(expected).unwrap_or("MIDI")
        );
    }
    *offset = end;
    Ok(())
}

fn read_audio_u8(id: &str, bytes: &[u8], offset: &mut usize, limit: usize) -> Result<u8> {
    if *offset >= limit || *offset >= bytes.len() {
        anyhow::bail!("compiled MIDI audio asset '{id}' ended inside track event");
    }
    let value = bytes[*offset];
    *offset += 1;
    Ok(value)
}

fn read_audio_var_len(id: &str, bytes: &[u8], offset: &mut usize, limit: usize) -> Result<u32> {
    let mut value = 0u32;
    for _ in 0..4 {
        let byte = read_audio_u8(id, bytes, offset, limit)?;
        value = value
            .checked_shl(7)
            .ok_or_else(|| anyhow::anyhow!("compiled MIDI audio asset '{id}' varlen overflow"))?
            | u32::from(byte & 0x7f);
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    anyhow::bail!("compiled MIDI audio asset '{id}' has overlong variable-length value")
}

fn skip_audio_bytes(
    id: &str,
    bytes: &[u8],
    offset: &mut usize,
    limit: usize,
    len: usize,
) -> Result<()> {
    let end = offset
        .checked_add(len)
        .ok_or_else(|| anyhow::anyhow!("compiled MIDI audio asset '{id}' event length overflow"))?;
    if end > limit || end > bytes.len() {
        anyhow::bail!("compiled MIDI audio asset '{id}' event exceeds track length");
    }
    *offset = end;
    Ok(())
}

fn read_audio_be_u16(id: &str, bytes: &[u8], offset: &mut usize) -> Result<u16> {
    let end = offset
        .checked_add(2)
        .ok_or_else(|| anyhow::anyhow!("compiled MIDI audio asset '{id}' read offset overflow"))?;
    if end > bytes.len() {
        anyhow::bail!("compiled MIDI audio asset '{id}' ended inside u16 field");
    }
    let value = u16::from_be_bytes(bytes[*offset..end].try_into()?);
    *offset = end;
    Ok(value)
}

fn read_audio_be_u32(id: &str, bytes: &[u8], offset: &mut usize) -> Result<u32> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| anyhow::anyhow!("compiled MIDI audio asset '{id}' read offset overflow"))?;
    if end > bytes.len() {
        anyhow::bail!("compiled MIDI audio asset '{id}' ended inside u32 field");
    }
    let value = u32::from_be_bytes(bytes[*offset..end].try_into()?);
    *offset = end;
    Ok(value)
}

fn validate_manifest_shape(manifest: &ModpackManifest) -> Result<()> {
    if manifest.schema_version != 1 {
        anyhow::bail!(
            "unsupported modpack schema_version {} for '{}'",
            manifest.schema_version,
            manifest.id()
        );
    }
    if !is_exact_manifest_id_token(manifest.id()) {
        anyhow::bail!(
            "modpack metadata.id must be exact ASCII letters, numbers, underscores, hyphens, or dots"
        );
    }
    if !is_exact_nonempty_manifest_token(&manifest.metadata.name) {
        anyhow::bail!(
            "modpack '{}' metadata.name must be an exact non-empty value",
            manifest.id()
        );
    }
    if !is_exact_nonempty_manifest_token(&manifest.metadata.version) {
        anyhow::bail!(
            "modpack '{}' metadata.version must be an exact non-empty value",
            manifest.id()
        );
    }
    if let Some(dependency) = manifest
        .dependencies
        .iter()
        .find(|dependency| !is_exact_manifest_id_token(dependency))
    {
        anyhow::bail!(
            "modpack '{}' dependency '{}' must be exact ASCII letters, numbers, underscores, hyphens, or dots",
            manifest.id(),
            dependency
        );
    }
    let mut seen_dependencies = BTreeSet::new();
    for dependency in &manifest.dependencies {
        if dependency == manifest.id() {
            anyhow::bail!("modpack '{}' must not depend on itself", manifest.id());
        }
        if !seen_dependencies.insert(dependency.as_str()) {
            anyhow::bail!(
                "modpack '{}' declares duplicate dependency '{}'",
                manifest.id(),
                dependency
            );
        }
    }
    Ok(())
}

fn validate_manifest_dependency_graph(manifests: &[ModpackManifest]) -> Result<()> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum VisitState {
        Visiting,
        Visited,
    }

    fn visit(
        manifest_id: &str,
        manifests_by_id: &BTreeMap<&str, &ModpackManifest>,
        states: &mut BTreeMap<String, VisitState>,
        stack: &mut Vec<String>,
    ) -> Result<()> {
        match states.get(manifest_id).copied() {
            Some(VisitState::Visited) => return Ok(()),
            Some(VisitState::Visiting) => {
                let start = stack.iter().position(|id| id == manifest_id).unwrap_or(0);
                let mut cycle = stack[start..].to_vec();
                cycle.push(manifest_id.to_string());
                anyhow::bail!("modpack dependency cycle detected: {}", cycle.join(" -> "));
            }
            None => {}
        }

        states.insert(manifest_id.to_string(), VisitState::Visiting);
        stack.push(manifest_id.to_string());
        if let Some(manifest) = manifests_by_id.get(manifest_id) {
            for dependency in &manifest.dependencies {
                visit(dependency, manifests_by_id, states, stack)?;
            }
        }
        stack.pop();
        states.insert(manifest_id.to_string(), VisitState::Visited);
        Ok(())
    }

    let manifests_by_id: BTreeMap<&str, &ModpackManifest> = manifests
        .iter()
        .map(|manifest| (manifest.id(), manifest))
        .collect();
    let mut states = BTreeMap::new();
    let mut stack = Vec::new();
    for manifest_id in manifests_by_id.keys() {
        visit(manifest_id, &manifests_by_id, &mut states, &mut stack)?;
    }
    Ok(())
}

fn ordered_manifests_for_application(
    manifests: &[ModpackManifest],
) -> Result<Vec<&ModpackManifest>> {
    let manifests_by_id: BTreeMap<&str, &ModpackManifest> = manifests
        .iter()
        .map(|manifest| (manifest.id(), manifest))
        .collect();
    let mut applied = BTreeSet::new();
    let mut ordered = Vec::with_capacity(manifests.len());

    while ordered.len() < manifests.len() {
        let mut ready: Vec<&ModpackManifest> = manifests_by_id
            .values()
            .copied()
            .filter(|manifest| !applied.contains(manifest.id()))
            .filter(|manifest| {
                manifest
                    .dependencies
                    .iter()
                    .all(|dependency| applied.contains(dependency.as_str()))
            })
            .collect();
        ready.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| left.id().cmp(right.id()))
        });
        let Some(next) = ready.first().copied() else {
            anyhow::bail!("modpack dependencies could not be ordered for application");
        };
        applied.insert(next.id());
        ordered.push(next);
    }

    Ok(ordered)
}

fn is_exact_manifest_id_token(value: &str) -> bool {
    is_exact_content_pack_id_token(value)
}

fn is_exact_nonempty_manifest_token(value: &str) -> bool {
    !value.is_empty() && value.trim() == value
}

fn optional_id_for_diagnostic(value: Option<&str>) -> String {
    match value {
        Some(value) => format!("{value:?}"),
        None => "<missing>".to_string(),
    }
}
