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
        validate_compiled_audio_payload(audio_asset, &source_bytes).with_context(|| {
            format!(
                "validate compiled audio payload {} ({} bytes at {})",
                audio_asset.id,
                source_bytes.len(),
                path.display()
            )
        })?;
        if compiled_audio
            .insert(audio_asset.id.clone(), source_bytes)
            .is_some()
        {
            anyhow::bail!("duplicate compiled audio payload '{}'", audio_asset.id);
        }
    }
    Ok(compiled_audio)
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
            let format = asset
                .pcm_format
                .as_ref()
                .unwrap_or_else(|| panic!("test PCM audio asset {} missing format", asset.id));
            let frame_size = format.frame_size_bytes(&asset.id).unwrap_or_else(|error| {
                panic!("invalid test PCM audio asset {}: {error:#}", asset.id)
            });
            let bytes = vec![0; frame_size];
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
        ModpackAudioSource::Midi => {
            if bytes.len() < 22 || !bytes.starts_with(b"MThd") || &bytes[14..18] != b"MTrk" {
                anyhow::bail!("compiled MIDI audio asset '{}' is invalid", asset.id);
            }
        }
    }
    Ok(())
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
