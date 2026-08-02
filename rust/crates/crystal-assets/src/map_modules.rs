#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapModule {
    pub id: String,
    pub attributes: MapAttributes,
    pub scripts: BTreeMap<String, Value>,
    pub trainer_scripts: BTreeMap<String, TrainerBattleRequest>,
    pub scripted_trainer_battles: Vec<ScriptedTrainerBattle>,
    pub scripted_wild_battles: Vec<ScriptedWildBattle>,
    pub script_item_grants: Vec<ScriptItemGrant>,
    pub script_item_checks: Vec<ScriptItemAccess>,
    pub script_item_takes: Vec<ScriptItemAccess>,
    pub script_economy_commands: Vec<ScriptEconomyCommand>,
    pub gift_pokemon_scripts: Vec<GiftPokemonScript>,
    pub script_flag_commands: Vec<ScriptFlagCommand>,
    pub script_scene_commands: Vec<ScriptSceneCommand>,
    pub script_audio_commands: Vec<ScriptAudioCommand>,
    pub script_block_changes: Vec<ScriptBlockChange>,
    pub script_object_commands: Vec<ScriptObjectCommand>,
    pub script_movements: Vec<ScriptMovement>,
    pub script_map_commands: Vec<ScriptMapCommand>,
    pub script_text_commands: Vec<ScriptTextCommand>,
    pub script_text_bodies: BTreeMap<String, ScriptTextBody>,
    pub script_menu_definitions: BTreeMap<String, ScriptMenuDefinition>,
    pub script_vertical_menus: BTreeMap<String, ScriptVerticalMenuDefinition>,
    pub script_elevators: BTreeMap<String, ScriptElevatorDefinition>,
    pub script_variable_commands: Vec<ScriptVariableCommand>,
    pub script_control_commands: Vec<ScriptControlCommand>,
    pub script_field_pickups: Vec<ScriptFieldPickup>,
    pub script_shop_commands: Vec<ScriptShopCommand>,
    pub script_phone_commands: Vec<ScriptPhoneCommand>,
    pub script_runtime_commands: Vec<ScriptRuntimeCommand>,
    pub script_swarm_commands: Vec<ScriptSwarmCommand>,
    pub map_script_section_commands: Vec<MapScriptSectionCommand>,
    pub map_event_section_commands: Vec<MapEventSectionCommand>,
    pub scenes: MapSceneTable,
    pub events: MapEvents,
    pub objects: Vec<ObjectEvent>,
    pub blocks: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptVerticalMenuDefinition {
    pub source_script: String,
    pub loadmenu_command_index: usize,
    pub verticalmenu_command_index: usize,
    pub header_label: String,
    pub data_label: Option<String>,
    pub options: Vec<String>,
    #[serde(default)]
    pub two_dimensional: bool,
    #[serde(default)]
    pub rows: Option<usize>,
    #[serde(default)]
    pub columns: Option<usize>,
    #[serde(default)]
    pub spacing: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptElevatorDefinition {
    pub source_script: String,
    pub elevator_command_index: usize,
    pub data_label: String,
    pub floors: Vec<ScriptRuntimeElevatorFloor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptedWildBattle {
    pub source_script: String,
    pub loadwildmon_command_index: usize,
    pub startbattle_command_index: usize,
    pub request: StaticWildBattleRequest,
    pub reload_map_after_battle: bool,
    pub pre_battle_event_flags: Vec<String>,
    pub post_battle_event_flags: Vec<String>,
    pub post_battle_script_flags: Vec<String>,
    pub disappear_object_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptedTrainerBattle {
    pub source_script: String,
    pub loadtrainer_command_index: usize,
    pub startbattle_command_index: usize,
    pub request: TrainerBattleRequest,
    pub reload_map_after_battle: bool,
    pub post_battle_event_flags: Vec<String>,
    pub post_battle_script_flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackCompileOptions {
    pub playability: PlayabilityRules,
}

impl Default for ModpackCompileOptions {
    fn default() -> Self {
        Self {
            playability: PlayabilityRules::default(),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayabilityRules {
    #[serde(deserialize_with = "required_pack_token_vec")]
    pub start_maps: Vec<String>,
    pub start_tiles: Vec<PlayabilityStart>,
    #[serde(deserialize_with = "required_pack_token_vec")]
    pub initial_events: Vec<String>,
    #[serde(deserialize_with = "required_pack_token_vec")]
    pub initial_items: Vec<String>,
    #[serde(deserialize_with = "required_pack_token_vec")]
    pub goal_maps: Vec<String>,
    #[serde(deserialize_with = "required_pack_token_vec")]
    pub goal_events: Vec<String>,
    #[serde(deserialize_with = "required_pack_token_vec")]
    pub goal_items: Vec<String>,
    pub progression_rules: Vec<ProgressionRule>,
    pub map_access: Vec<MapAccessRule>,
    pub require_all_maps_reachable: bool,
    pub require_walkable_maps: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlayabilityStart {
    #[serde(deserialize_with = "required_pack_token")]
    pub map: String,
    pub tile: TilePosition,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressionRequirements {
    #[serde(deserialize_with = "required_pack_token_vec")]
    pub events: Vec<String>,
    #[serde(deserialize_with = "required_pack_token_vec")]
    pub items: Vec<String>,
    #[serde(deserialize_with = "required_pack_token_vec")]
    pub maps: Vec<String>,
}

impl ProgressionRequirements {
    fn is_empty(&self) -> bool {
        self.events.is_empty() && self.items.is_empty() && self.maps.is_empty()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressionGrants {
    #[serde(deserialize_with = "required_pack_token_vec")]
    pub events: Vec<String>,
    #[serde(deserialize_with = "required_pack_token_vec")]
    pub items: Vec<String>,
    #[serde(deserialize_with = "required_pack_token_vec")]
    pub maps: Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressionRule {
    #[serde(deserialize_with = "required_progression_rule_id")]
    pub id: String,
    pub requires: ProgressionRequirements,
    pub grants: ProgressionGrants,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapAccessRule {
    #[serde(deserialize_with = "required_pack_token")]
    pub map: String,
    pub requires: ProgressionRequirements,
}

fn required_pack_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if !is_exact_pack_token(&value) {
        return Err(serde::de::Error::custom(format!(
            "pack token must be exact ASCII alphanumeric/underscore, found {value:?}"
        )));
    }
    validate_no_reserved_payload_token(&value, "pack token").map_err(serde::de::Error::custom)?;
    Ok(value)
}

fn required_crystal_word_i16<'de, D>(deserializer: D) -> Result<i16, D::Error>
where
    D: serde::Deserializer<'de>,
{
    i16::deserialize(deserializer)
}

fn required_pack_token_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = Vec::<String>::deserialize(deserializer)?;
    for value in &values {
        if !is_exact_pack_token(value) {
            return Err(serde::de::Error::custom(format!(
                "pack token must be exact ASCII alphanumeric/underscore, found {value:?}"
            )));
        }
        validate_no_reserved_payload_token(value, "pack token")
            .map_err(serde::de::Error::custom)?;
    }
    Ok(values)
}

fn required_progression_rule_id<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if !is_exact_progression_rule_id(&value) {
        return Err(serde::de::Error::custom(format!(
            "progression rule id must be exact ASCII label token, found {value:?}"
        )));
    }
    validate_no_reserved_payload_token(&value, "progression rule id")
        .map_err(serde::de::Error::custom)?;
    Ok(value)
}

fn is_exact_pack_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn is_exact_progression_rule_id(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':' | b'.' | b'@' | b'-')
        })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledModpack {
    data: GameDataSet,
    compiled_audio: BTreeMap<String, Vec<u8>>,
    audio_manifest: ModpackAudioManifest,
    /// Non-audio presentation files needed by the native renderer.  PCM audio
    /// lives in `compiled_audio` and is compressed when the binary pack is
    /// serialized.
    runtime_files: BTreeMap<String, Vec<u8>>,
    report: ModpackCompileReport,
    identity: CompiledGamePackIdentity,
}

impl CompiledModpack {
    pub fn data(&self) -> &GameDataSet {
        &self.data
    }

    pub fn report(&self) -> &ModpackCompileReport {
        &self.report
    }

    pub fn identity(&self) -> &CompiledGamePackIdentity {
        &self.identity
    }

    pub fn into_game_pack(self) -> CompiledGamePack {
        CompiledGamePack {
            format_version: COMPILED_GAME_PACK_FORMAT_VERSION,
            data: self.data,
            compiled_audio: self.compiled_audio,
            audio_manifest: self.audio_manifest,
            audio_compression: None,
            runtime_files: self.runtime_files,
            report: self.report,
            identity: self.identity,
        }
    }

    pub fn write_game_pack(&self, path: impl AsRef<Path>) -> Result<()> {
        write_compiled_game_pack(
            path,
            &CompiledGamePack {
                format_version: COMPILED_GAME_PACK_FORMAT_VERSION,
                data: self.data.clone(),
                compiled_audio: self.compiled_audio.clone(),
                audio_manifest: self.audio_manifest.clone(),
                audio_compression: None,
                runtime_files: self.runtime_files.clone(),
                report: self.report.clone(),
                identity: self.identity.clone(),
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledGamePack {
    format_version: u16,
    data: GameDataSet,
    compiled_audio: BTreeMap<String, Vec<u8>>,
    #[serde(default)]
    audio_manifest: ModpackAudioManifest,
    #[serde(default)]
    audio_compression: Option<String>,
    #[serde(default)]
    runtime_files: BTreeMap<String, Vec<u8>>,
    report: ModpackCompileReport,
    identity: CompiledGamePackIdentity,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompiledGamePackIdentity {
    pub format_version: u16,
    pub runtime_modpack_id: String,
    pub content_hash: String,
    pub pokemon_species: usize,
    pub maps: usize,
    pub items: usize,
    pub moves: usize,
    pub music: usize,
    pub sound_effects: usize,
    pub cries: usize,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn derive_compiled_game_pack_identity(
    format_version: u16,
    data: &GameDataSet,
    compiled_audio: &BTreeMap<String, Vec<u8>>,
    report: &ModpackCompileReport,
) -> Result<CompiledGamePackIdentity> {
    let manifest = ModpackAudioManifest::from_assets(&data.audio, compiled_audio)?;
    derive_compiled_game_pack_identity_from_manifest(format_version, data, &manifest, report)
}

fn derive_compiled_game_pack_identity_from_manifest(
    format_version: u16,
    data: &GameDataSet,
    manifest: &ModpackAudioManifest,
    report: &ModpackCompileReport,
) -> Result<CompiledGamePackIdentity> {
    let mut encoded = Vec::new();
    ciborium::into_writer(&(format_version, data, manifest, report), &mut encoded)
        .context("encode compiled game pack identity")?;
    Ok(CompiledGamePackIdentity {
        format_version,
        runtime_modpack_id: compiled_game_pack_runtime_modpack_id(report)?,
        content_hash: sha256_hex(&encoded),
        pokemon_species: data.pokemon.len(),
        maps: data.maps.len(),
        items: data.items.len(),
        moves: data.moves.len(),
        music: manifest.music.len(),
        sound_effects: manifest.sound_effects.len(),
        cries: manifest.cries.len(),
    })
}

#[cfg(any(test, feature = "test-fixtures"))]
fn unchecked_compiled_game_pack_identity_for_tests(
    format_version: u16,
    data: &GameDataSet,
    compiled_audio: &BTreeMap<String, Vec<u8>>,
    report: &ModpackCompileReport,
) -> CompiledGamePackIdentity {
    derive_compiled_game_pack_identity(format_version, data, compiled_audio, report).unwrap_or_else(
        |_| CompiledGamePackIdentity {
            format_version,
            runtime_modpack_id: "invalid-test-pack".to_string(),
            content_hash: "0".repeat(64),
            pokemon_species: data.pokemon.len(),
            maps: data.maps.len(),
            items: data.items.len(),
            moves: data.moves.len(),
            music: data
                .audio
                .iter()
                .filter(|asset| asset.kind == ModpackAudioKind::Music)
                .count(),
            sound_effects: data
                .audio
                .iter()
                .filter(|asset| asset.kind == ModpackAudioKind::SoundEffect)
                .count(),
            cries: data
                .audio
                .iter()
                .filter(|asset| asset.kind == ModpackAudioKind::Cry)
                .count(),
        },
    )
}

#[cfg(any(test, feature = "test-fixtures"))]
fn canonicalize_core_modular_test_report(
    data: &GameDataSet,
    report: ModpackCompileReport,
) -> ModpackCompileReport {
    if report.manifests == ["core-modular".to_string()] {
        ModpackCompileReport {
            maps: data.maps.len(),
            pokemon: data.pokemon.len(),
            moves: data.moves.len(),
            items: data.items.len(),
            ..report
        }
    } else {
        report
    }
}

impl CompiledGamePack {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_for_tests(mut data: GameDataSet, report: ModpackCompileReport) -> Self {
        normalize_test_pcm_audio_metadata(&mut data);
        let compiled_audio = synthetic_compiled_audio_for_tests(&data);
        let report = canonicalize_core_modular_test_report(&data, report);
        let identity = unchecked_compiled_game_pack_identity_for_tests(
            COMPILED_GAME_PACK_FORMAT_VERSION,
            &data,
            &compiled_audio,
            &report,
        );
        let audio_manifest =
            ModpackAudioManifest::from_assets(&data.audio, &compiled_audio).unwrap_or_default();
        Self {
            format_version: COMPILED_GAME_PACK_FORMAT_VERSION,
            data,
            compiled_audio,
            audio_compression: None,
            audio_manifest,
            runtime_files: BTreeMap::new(),
            report,
            identity,
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn new_unchecked_with_audio_for_tests(
        mut data: GameDataSet,
        compiled_audio: BTreeMap<String, Vec<u8>>,
        report: ModpackCompileReport,
    ) -> Self {
        normalize_test_pcm_audio_metadata(&mut data);
        let report = canonicalize_core_modular_test_report(&data, report);
        let identity = unchecked_compiled_game_pack_identity_for_tests(
            COMPILED_GAME_PACK_FORMAT_VERSION,
            &data,
            &compiled_audio,
            &report,
        );
        let audio_manifest =
            ModpackAudioManifest::from_assets(&data.audio, &compiled_audio).unwrap_or_default();
        Self {
            format_version: COMPILED_GAME_PACK_FORMAT_VERSION,
            data,
            compiled_audio,
            audio_compression: None,
            audio_manifest,
            runtime_files: BTreeMap::new(),
            report,
            identity,
        }
    }

    pub fn format_version(&self) -> u16 {
        self.format_version
    }

    pub fn data(&self) -> &GameDataSet {
        &self.data
    }

    pub fn compiled_audio(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.compiled_audio
    }

    pub fn runtime_files(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.runtime_files
    }

    pub fn audio_manifest(&self) -> Result<ModpackAudioManifest> {
        ModpackAudioManifest::from_assets(&self.data.audio, &self.compiled_audio)
    }

    pub fn report(&self) -> &ModpackCompileReport {
        &self.report
    }

    pub fn runtime_modpack_id(&self) -> Result<String> {
        compiled_game_pack_runtime_modpack_id(&self.report)
    }

    pub fn identity(&self) -> Result<CompiledGamePackIdentity> {
        validate_compiled_game_pack_identity(self)?;
        Ok(self.identity.clone())
    }

    pub fn into_parts(
        self,
    ) -> (
        u16,
        GameDataSet,
        BTreeMap<String, Vec<u8>>,
        ModpackAudioManifest,
        Option<String>,
        BTreeMap<String, Vec<u8>>,
        ModpackCompileReport,
        CompiledGamePackIdentity,
    ) {
        (
            self.format_version,
            self.data,
            self.compiled_audio,
            self.audio_manifest,
            self.audio_compression,
            self.runtime_files,
            self.report,
            self.identity,
        )
    }
}
