const COMPILED_GAME_PACK_MAGIC: &[u8; 12] = b"CRYSTALPACK\0";
pub const COMPILED_GAME_PACK_EXTENSION: &str = "crystalpack";
pub const COMPILED_GAME_PACK_FORMAT_VERSION: u16 = 6;
const COMPILED_GAME_PACK_VERSION_OFFSET: usize = COMPILED_GAME_PACK_MAGIC.len();
const COMPILED_GAME_PACK_PAYLOAD_LENGTH_OFFSET: usize = COMPILED_GAME_PACK_VERSION_OFFSET + 2;
const COMPILED_GAME_PACK_PAYLOAD_HASH_OFFSET: usize = COMPILED_GAME_PACK_PAYLOAD_LENGTH_OFFSET + 4;
const COMPILED_GAME_PACK_HEADER_LEN: usize = COMPILED_GAME_PACK_PAYLOAD_HASH_OFFSET + 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRoot {
    pub repository_root: PathBuf,
}

impl AssetRoot {
    pub fn new(repository_root: impl Into<PathBuf>) -> Self {
        Self {
            repository_root: repository_root.into(),
        }
    }

    pub fn vendor_pokecrystal(&self) -> PathBuf {
        self.repository_root.join("vendor/pokecrystal")
    }

    pub fn typescript_assets(&self) -> PathBuf {
        self.repository_root.join("packages/assets/src")
    }

    pub fn runtime_assets(&self) -> PathBuf {
        self.repository_root.join("apps/web/assets")
    }

    pub fn resolve_vendor(&self, relative_path: impl AsRef<Path>) -> PathBuf {
        self.vendor_pokecrystal().join(relative_path)
    }

    pub fn resolve_data_path(&self, relative_path: impl AsRef<Path>) -> Result<PathBuf> {
        let relative_path = relative_path.as_ref();
        if relative_path.is_absolute() {
            anyhow::bail!(
                "runtime data path '{}' must be relative to assets/data",
                relative_path.display()
            );
        }
        let relative_text = relative_path.to_string_lossy();
        if relative_text.starts_with("assets/data/") {
            anyhow::bail!(
                "runtime data path '{relative_text}' must not include the assets/data prefix"
            );
        }
        if relative_path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            anyhow::bail!(
                "runtime data path '{}' must not traverse parent directories",
                relative_path.display()
            );
        }
        if path_contains_current_directory_alias(relative_path) {
            anyhow::bail!(
                "runtime data path '{}' must not include current-directory components",
                relative_path.display()
            );
        }
        Ok(self.runtime_assets().join("data").join(relative_path))
    }

    pub fn load_content_pack_index(&self) -> Result<ContentPackIndex> {
        let mut index: ContentPackIndex =
            read_json_file(&self.runtime_assets().join("data/content-packs/index.json"))?;
        index.validate()?;
        index.sort_packs();
        Ok(index)
    }

    fn load_raw_content_pack_index_for_compile(&self) -> Result<ContentPackIndex> {
        let mut index: ContentPackIndex =
            read_json_file(&self.runtime_assets().join("data/content-packs/index.json"))?;
        index.packs.retain(|pack| pack.id == "core-modular");
        for pack in &mut index.packs {
            if pack.compiled.is_some() {
                let generated_pack: ContentPack = read_json_file(
                    &self
                        .runtime_assets()
                        .join("data/content-packs/core-modular.generated.json"),
                )
                .context("load generated core content pack manifest")?;
                validate_generated_core_content_pack_manifest(&generated_pack)?;
                *pack = generated_pack;
            }
        }
        index.validate()?;
        index.sort_packs();
        Ok(index)
    }

    pub fn load_modpack_manifest(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<ModpackManifest> {
        read_json_file(&self.repository_root.join(relative_path))
    }

    #[cfg(test)]
    pub(crate) fn load_base_game_data(&self) -> Result<GameDataSet> {
        let mut data = GameDataSet::load_base_json_for_compile(self)?;
        materialize_runtime_map_modules(&mut data)?;
        Ok(data)
    }

    pub fn compile_modpacks(
        &self,
        manifests: &[ModpackManifest],
        options: ModpackCompileOptions,
    ) -> Result<CompiledModpack> {
        ModpackCompiler::new(self).compile(manifests, options)
    }

    #[cfg(test)]
    fn load_compiled_game_pack(&self, relative_path: impl AsRef<Path>) -> Result<CompiledGamePack> {
        read_compiled_game_pack(resolve_compiled_game_pack_data_path(
            self,
            relative_path.as_ref(),
        )?)
    }

    #[cfg(test)]
    fn load_loaded_compiled_game_pack(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<LoadedCompiledGamePack> {
        read_loaded_compiled_game_pack(resolve_compiled_game_pack_data_path(
            self,
            relative_path.as_ref(),
        )?)
    }

    pub fn load_verified_compiled_game_pack(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<CompiledGamePack> {
        read_verified_compiled_game_pack(resolve_compiled_game_pack_data_path(
            self,
            relative_path.as_ref(),
        )?)
    }

    pub fn load_loaded_verified_compiled_game_pack(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<LoadedCompiledGamePack> {
        read_loaded_verified_compiled_game_pack(resolve_compiled_game_pack_data_path(
            self,
            relative_path.as_ref(),
        )?)
    }

    pub fn load_verified_compiled_game_data(
        &self,
        relative_path: impl AsRef<Path>,
    ) -> Result<GameDataSet> {
        Ok(self.load_verified_compiled_game_pack(relative_path)?.data)
    }

    #[cfg(test)]
    fn load_tileset_collision(&self, tileset_name: &str) -> Result<TilesetCollision> {
        let path = self
            .runtime_assets()
            .join("data/tilesets")
            .join(format!("{tileset_name}.json"));
        let raw: BTreeMap<String, Vec<Value>> = read_json_file(&path)?;
        let ids = raw
            .keys()
            .map(|key| {
                parse_metatile_id(key)
                    .with_context(|| format!("parse metatile id '{key}' in {}", path.display()))
            })
            .collect::<Result<BTreeSet<_>>>()?;
        require_dense_metatile_ids(&ids, &format!("tileset collision file {}", path.display()))?;
        let max_id = ids
            .iter()
            .copied()
            .max()
            .with_context(|| format!("tileset collision file {} is empty", path.display()))?;
        let mut metatiles = vec![None; max_id + 1];
        for (id, quadrants) in raw {
            if quadrants.len() != 4 {
                anyhow::bail!(
                    "tileset {tileset_name} metatile {id} has {} collision quadrants",
                    quadrants.len()
                );
            }
            let index = parse_metatile_id(&id)?;
            let mut collision = [0_u8; 4];
            for (quadrant, value) in quadrants.into_iter().enumerate() {
                collision[quadrant] = match value {
                    Value::Number(number) => number
                        .as_u64()
                        .and_then(|value| u8::try_from(value).ok())
                        .with_context(|| {
                            format!("invalid collision value for {tileset_name}:{id}")
                        })?,
                    Value::String(token) => resolve_collision_token(&token).with_context(|| {
                        format!("unknown collision token {token} in {tileset_name}:{id}")
                    })?,
                    _ => anyhow::bail!("invalid collision entry in {tileset_name}:{id}"),
                };
            }
            metatiles[index] = Some(MetatileCollision { collision });
        }
        Ok(TilesetCollision {
            metatiles: metatiles
                .into_iter()
                .collect::<Option<Vec<_>>>()
                .with_context(|| {
                    format!(
                        "tileset collision file {} has missing metatile ids",
                        path.display()
                    )
                })?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ContentPackCategory {
    Pokemon,
    Moves,
    GrowthRates,
    Learnsets,
    LevelUpMoves,
    EggMoves,
    Evolutions,
    Maps,
    MapScripts,
    MapBlocks,
    MapAttributes,
    MapDimensions,
    WildEncounters,
    FieldEncounters,
    RuntimeSpawnPoints,
    RuntimeMapMetadata,
    FleeMons,
    RoamingPokemon,
    BuenaPasswordCategories,
    BuenaPrizes,
    KurtApricornRecipes,
    ShuckieGift,
    DratiniMoveSets,
    BugContestConfig,
    BattleTowerRules,
    OakRatings,
    OddEggDefinitions,
    MagikarpLengths,
    HappinessData,
    EncounterSlotTables,
    EncounterMusicModifiers,
    BattleStatMultipliers,
    CaptureWobbleProbabilities,
    CaptureRules,
    BattleEscapeRules,
    MovePriorities,
    TypeCategories,
    TypeEffectiveness,
    WeatherModifiers,
    BattleRewardRules,
    StepEventRules,
    Fishing,
    FruitTrees,
    FieldMoves,
    FieldBoxItems,
    RuntimeTitleScreen,
    FlyDestinations,
    Npcs,
    PokegearLandmarks,
    PcStrings,
    MenuIcons,
    Items,
    Marts,
    CurrencyConstants,
    Trainers,
    TrainerClassNames,
    Pokedex,
    PokedexEntries,
    PokemonFrontpicAnim,
    InitializeEvents,
    StoryEventScriptConstants,
    StoryEvents,
    PhoneScripts,
    PhoneContacts,
    PermanentPhoneNumbers,
    SpecialPhoneCalls,
    NpcTrades,
    SpecialRoutines,
    AsmText,
    MoveNames,
    BattleAnimations,
    BattleAnimationTable,
    BattleAnimBundle,
    SpriteAnimBundle,
    SpritePaletteDefaults,
    PokegearTownMapPaletteMap,
    PokemonCries,
    Audio,
    Tilesets,
    Playability,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentPackFiles {
    pub pokemon: Vec<String>,
    pub moves: Vec<String>,
    pub growth_rates: Vec<String>,
    pub learnsets: Vec<String>,
    pub level_up_moves: Vec<String>,
    pub egg_moves: Vec<String>,
    pub evolutions: Vec<String>,
    pub maps: Vec<String>,
    pub map_scripts: Vec<String>,
    pub map_blocks: Vec<String>,
    pub map_attributes: Vec<String>,
    pub map_dimensions: Vec<String>,
    pub wild_encounters: Vec<String>,
    pub field_encounters: Vec<String>,
    pub runtime_spawn_points: Vec<String>,
    pub runtime_map_metadata: Vec<String>,
    pub flee_mons: Vec<String>,
    pub roaming_pokemon: Vec<String>,
    pub buena_password_categories: Vec<String>,
    pub buena_prizes: Vec<String>,
    pub kurt_apricorn_recipes: Vec<String>,
    pub shuckie_gift: Vec<String>,
    pub dratini_move_sets: Vec<String>,
    pub bug_contest_config: Vec<String>,
    pub battle_tower_rules: Vec<String>,
    pub oak_ratings: Vec<String>,
    pub odd_egg_definitions: Vec<String>,
    pub magikarp_lengths: Vec<String>,
    pub happiness_data: Vec<String>,
    pub encounter_slot_tables: Vec<String>,
    pub encounter_music_modifiers: Vec<String>,
    pub battle_stat_multipliers: Vec<String>,
    pub capture_wobble_probabilities: Vec<String>,
    pub capture_rules: Vec<String>,
    pub battle_escape_rules: Vec<String>,
    pub move_priorities: Vec<String>,
    pub type_categories: Vec<String>,
    pub type_effectiveness: Vec<String>,
    pub weather_modifiers: Vec<String>,
    pub battle_reward_rules: Vec<String>,
    pub step_event_rules: Vec<String>,
    pub fishing: Vec<String>,
    pub fruit_trees: Vec<String>,
    pub field_moves: Vec<String>,
    pub field_box_items: Vec<String>,
    pub runtime_title_screen: Vec<String>,
    pub fly_destinations: Vec<String>,
    pub npcs: Vec<String>,
    pub pokegear_landmarks: Vec<String>,
    pub pc_strings: Vec<String>,
    pub menu_icons: Vec<String>,
    pub items: Vec<String>,
    pub marts: Vec<String>,
    pub currency_constants: Vec<String>,
    pub trainers: Vec<String>,
    pub trainer_class_names: Vec<String>,
    pub pokedex: Vec<String>,
    pub pokedex_entries: Vec<String>,
    pub pokemon_frontpic_anim: Vec<String>,
    pub initialize_events: Vec<String>,
    pub story_event_script_constants: Vec<String>,
    pub story_events: Vec<String>,
    pub phone_scripts: Vec<String>,
    pub phone_contacts: Vec<String>,
    pub permanent_phone_numbers: Vec<String>,
    pub special_phone_calls: Vec<String>,
    pub npc_trades: Vec<String>,
    pub special_routines: Vec<String>,
    pub asm_text: Vec<String>,
    pub move_names: Vec<String>,
    pub battle_animations: Vec<String>,
    pub battle_animation_table: Vec<String>,
    pub battle_anim_bundle: Vec<String>,
    pub sprite_anim_bundle: Vec<String>,
    pub sprite_palette_defaults: Vec<String>,
    pub pokegear_town_map_palette_map: Vec<String>,
    pub pokemon_cries: Vec<String>,
    pub audio: Vec<String>,
    pub tilesets: Vec<String>,
    pub playability: Vec<String>,
}

impl ContentPackFiles {
    pub fn entries(&self, category: ContentPackCategory) -> &[String] {
        match category {
            ContentPackCategory::Pokemon => &self.pokemon,
            ContentPackCategory::Moves => &self.moves,
            ContentPackCategory::GrowthRates => &self.growth_rates,
            ContentPackCategory::Learnsets => &self.learnsets,
            ContentPackCategory::LevelUpMoves => &self.level_up_moves,
            ContentPackCategory::EggMoves => &self.egg_moves,
            ContentPackCategory::Evolutions => &self.evolutions,
            ContentPackCategory::Maps => &self.maps,
            ContentPackCategory::MapScripts => &self.map_scripts,
            ContentPackCategory::MapBlocks => &self.map_blocks,
            ContentPackCategory::MapAttributes => &self.map_attributes,
            ContentPackCategory::MapDimensions => &self.map_dimensions,
            ContentPackCategory::WildEncounters => &self.wild_encounters,
            ContentPackCategory::FieldEncounters => &self.field_encounters,
            ContentPackCategory::RuntimeSpawnPoints => &self.runtime_spawn_points,
            ContentPackCategory::RuntimeMapMetadata => &self.runtime_map_metadata,
            ContentPackCategory::FleeMons => &self.flee_mons,
            ContentPackCategory::RoamingPokemon => &self.roaming_pokemon,
            ContentPackCategory::BuenaPasswordCategories => &self.buena_password_categories,
            ContentPackCategory::BuenaPrizes => &self.buena_prizes,
            ContentPackCategory::KurtApricornRecipes => &self.kurt_apricorn_recipes,
            ContentPackCategory::ShuckieGift => &self.shuckie_gift,
            ContentPackCategory::DratiniMoveSets => &self.dratini_move_sets,
            ContentPackCategory::BugContestConfig => &self.bug_contest_config,
            ContentPackCategory::BattleTowerRules => &self.battle_tower_rules,
            ContentPackCategory::OakRatings => &self.oak_ratings,
            ContentPackCategory::OddEggDefinitions => &self.odd_egg_definitions,
            ContentPackCategory::MagikarpLengths => &self.magikarp_lengths,
            ContentPackCategory::HappinessData => &self.happiness_data,
            ContentPackCategory::EncounterSlotTables => &self.encounter_slot_tables,
            ContentPackCategory::EncounterMusicModifiers => &self.encounter_music_modifiers,
            ContentPackCategory::BattleStatMultipliers => &self.battle_stat_multipliers,
            ContentPackCategory::CaptureWobbleProbabilities => &self.capture_wobble_probabilities,
            ContentPackCategory::CaptureRules => &self.capture_rules,
            ContentPackCategory::BattleEscapeRules => &self.battle_escape_rules,
            ContentPackCategory::MovePriorities => &self.move_priorities,
            ContentPackCategory::TypeCategories => &self.type_categories,
            ContentPackCategory::TypeEffectiveness => &self.type_effectiveness,
            ContentPackCategory::WeatherModifiers => &self.weather_modifiers,
            ContentPackCategory::BattleRewardRules => &self.battle_reward_rules,
            ContentPackCategory::StepEventRules => &self.step_event_rules,
            ContentPackCategory::Fishing => &self.fishing,
            ContentPackCategory::FruitTrees => &self.fruit_trees,
            ContentPackCategory::FieldMoves => &self.field_moves,
            ContentPackCategory::FieldBoxItems => &self.field_box_items,
            ContentPackCategory::RuntimeTitleScreen => &self.runtime_title_screen,
            ContentPackCategory::FlyDestinations => &self.fly_destinations,
            ContentPackCategory::Npcs => &self.npcs,
            ContentPackCategory::PokegearLandmarks => &self.pokegear_landmarks,
            ContentPackCategory::PcStrings => &self.pc_strings,
            ContentPackCategory::MenuIcons => &self.menu_icons,
            ContentPackCategory::Items => &self.items,
            ContentPackCategory::Marts => &self.marts,
            ContentPackCategory::CurrencyConstants => &self.currency_constants,
            ContentPackCategory::Trainers => &self.trainers,
            ContentPackCategory::TrainerClassNames => &self.trainer_class_names,
            ContentPackCategory::Pokedex => &self.pokedex,
            ContentPackCategory::PokedexEntries => &self.pokedex_entries,
            ContentPackCategory::PokemonFrontpicAnim => &self.pokemon_frontpic_anim,
            ContentPackCategory::InitializeEvents => &self.initialize_events,
            ContentPackCategory::StoryEventScriptConstants => &self.story_event_script_constants,
            ContentPackCategory::StoryEvents => &self.story_events,
            ContentPackCategory::PhoneScripts => &self.phone_scripts,
            ContentPackCategory::PhoneContacts => &self.phone_contacts,
            ContentPackCategory::PermanentPhoneNumbers => &self.permanent_phone_numbers,
            ContentPackCategory::SpecialPhoneCalls => &self.special_phone_calls,
            ContentPackCategory::NpcTrades => &self.npc_trades,
            ContentPackCategory::SpecialRoutines => &self.special_routines,
            ContentPackCategory::AsmText => &self.asm_text,
            ContentPackCategory::MoveNames => &self.move_names,
            ContentPackCategory::BattleAnimations => &self.battle_animations,
            ContentPackCategory::BattleAnimationTable => &self.battle_animation_table,
            ContentPackCategory::BattleAnimBundle => &self.battle_anim_bundle,
            ContentPackCategory::SpriteAnimBundle => &self.sprite_anim_bundle,
            ContentPackCategory::SpritePaletteDefaults => &self.sprite_palette_defaults,
            ContentPackCategory::PokegearTownMapPaletteMap => &self.pokegear_town_map_palette_map,
            ContentPackCategory::PokemonCries => &self.pokemon_cries,
            ContentPackCategory::Audio => &self.audio,
            ContentPackCategory::Tilesets => &self.tilesets,
            ContentPackCategory::Playability => &self.playability,
        }
    }
}

const CONTENT_PACK_CATEGORIES: &[ContentPackCategory] = &[
    ContentPackCategory::Pokemon,
    ContentPackCategory::Moves,
    ContentPackCategory::GrowthRates,
    ContentPackCategory::Learnsets,
    ContentPackCategory::LevelUpMoves,
    ContentPackCategory::EggMoves,
    ContentPackCategory::Evolutions,
    ContentPackCategory::Maps,
    ContentPackCategory::MapScripts,
    ContentPackCategory::MapBlocks,
    ContentPackCategory::MapAttributes,
    ContentPackCategory::MapDimensions,
    ContentPackCategory::WildEncounters,
    ContentPackCategory::FieldEncounters,
    ContentPackCategory::RuntimeSpawnPoints,
    ContentPackCategory::RuntimeMapMetadata,
    ContentPackCategory::FleeMons,
    ContentPackCategory::RoamingPokemon,
    ContentPackCategory::BuenaPasswordCategories,
    ContentPackCategory::BuenaPrizes,
    ContentPackCategory::KurtApricornRecipes,
    ContentPackCategory::ShuckieGift,
    ContentPackCategory::DratiniMoveSets,
    ContentPackCategory::BugContestConfig,
    ContentPackCategory::BattleTowerRules,
    ContentPackCategory::OakRatings,
    ContentPackCategory::OddEggDefinitions,
    ContentPackCategory::MagikarpLengths,
    ContentPackCategory::HappinessData,
    ContentPackCategory::EncounterSlotTables,
    ContentPackCategory::EncounterMusicModifiers,
    ContentPackCategory::BattleStatMultipliers,
    ContentPackCategory::CaptureWobbleProbabilities,
    ContentPackCategory::CaptureRules,
    ContentPackCategory::BattleEscapeRules,
    ContentPackCategory::MovePriorities,
    ContentPackCategory::TypeCategories,
    ContentPackCategory::TypeEffectiveness,
    ContentPackCategory::WeatherModifiers,
    ContentPackCategory::BattleRewardRules,
    ContentPackCategory::StepEventRules,
    ContentPackCategory::Fishing,
    ContentPackCategory::FruitTrees,
    ContentPackCategory::FieldMoves,
    ContentPackCategory::FieldBoxItems,
    ContentPackCategory::RuntimeTitleScreen,
    ContentPackCategory::FlyDestinations,
    ContentPackCategory::Npcs,
    ContentPackCategory::PokegearLandmarks,
    ContentPackCategory::PcStrings,
    ContentPackCategory::MenuIcons,
    ContentPackCategory::Items,
    ContentPackCategory::Marts,
    ContentPackCategory::CurrencyConstants,
    ContentPackCategory::Trainers,
    ContentPackCategory::TrainerClassNames,
    ContentPackCategory::Pokedex,
    ContentPackCategory::PokedexEntries,
    ContentPackCategory::PokemonFrontpicAnim,
    ContentPackCategory::InitializeEvents,
    ContentPackCategory::StoryEventScriptConstants,
    ContentPackCategory::StoryEvents,
    ContentPackCategory::PhoneScripts,
    ContentPackCategory::PhoneContacts,
    ContentPackCategory::PermanentPhoneNumbers,
    ContentPackCategory::SpecialPhoneCalls,
    ContentPackCategory::NpcTrades,
    ContentPackCategory::SpecialRoutines,
    ContentPackCategory::AsmText,
    ContentPackCategory::MoveNames,
    ContentPackCategory::BattleAnimations,
    ContentPackCategory::BattleAnimationTable,
    ContentPackCategory::BattleAnimBundle,
    ContentPackCategory::SpriteAnimBundle,
    ContentPackCategory::SpritePaletteDefaults,
    ContentPackCategory::PokegearTownMapPaletteMap,
    ContentPackCategory::PokemonCries,
    ContentPackCategory::Audio,
    ContentPackCategory::Tilesets,
    ContentPackCategory::Playability,
];

impl ContentPackCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            ContentPackCategory::Pokemon => "pokemon",
            ContentPackCategory::Moves => "moves",
            ContentPackCategory::GrowthRates => "growth_rates",
            ContentPackCategory::Learnsets => "learnsets",
            ContentPackCategory::LevelUpMoves => "level_up_moves",
            ContentPackCategory::EggMoves => "egg_moves",
            ContentPackCategory::Evolutions => "evolutions",
            ContentPackCategory::Maps => "maps",
            ContentPackCategory::MapScripts => "map_scripts",
            ContentPackCategory::MapBlocks => "map_blocks",
            ContentPackCategory::MapAttributes => "map_attributes",
            ContentPackCategory::MapDimensions => "map_dimensions",
            ContentPackCategory::WildEncounters => "wild_encounters",
            ContentPackCategory::FieldEncounters => "field_encounters",
            ContentPackCategory::RuntimeSpawnPoints => "runtime_spawn_points",
            ContentPackCategory::RuntimeMapMetadata => "runtime_map_metadata",
            ContentPackCategory::FleeMons => "flee_mons",
            ContentPackCategory::RoamingPokemon => "roaming_pokemon",
            ContentPackCategory::BuenaPasswordCategories => "buena_password_categories",
            ContentPackCategory::BuenaPrizes => "buena_prizes",
            ContentPackCategory::KurtApricornRecipes => "kurt_apricorn_recipes",
            ContentPackCategory::ShuckieGift => "shuckie_gift",
            ContentPackCategory::DratiniMoveSets => "dratini_move_sets",
            ContentPackCategory::BugContestConfig => "bug_contest_config",
            ContentPackCategory::BattleTowerRules => "battle_tower_rules",
            ContentPackCategory::OakRatings => "oak_ratings",
            ContentPackCategory::OddEggDefinitions => "odd_egg_definitions",
            ContentPackCategory::MagikarpLengths => "magikarp_lengths",
            ContentPackCategory::HappinessData => "happiness_data",
            ContentPackCategory::EncounterSlotTables => "encounter_slot_tables",
            ContentPackCategory::EncounterMusicModifiers => "encounter_music_modifiers",
            ContentPackCategory::BattleStatMultipliers => "battle_stat_multipliers",
            ContentPackCategory::CaptureWobbleProbabilities => "capture_wobble_probabilities",
            ContentPackCategory::CaptureRules => "capture_rules",
            ContentPackCategory::BattleEscapeRules => "battle_escape_rules",
            ContentPackCategory::MovePriorities => "move_priorities",
            ContentPackCategory::TypeCategories => "type_categories",
            ContentPackCategory::TypeEffectiveness => "type_effectiveness",
            ContentPackCategory::WeatherModifiers => "weather_modifiers",
            ContentPackCategory::BattleRewardRules => "battle_reward_rules",
            ContentPackCategory::StepEventRules => "step_event_rules",
            ContentPackCategory::Fishing => "fishing",
            ContentPackCategory::FruitTrees => "fruit_trees",
            ContentPackCategory::FieldMoves => "field_moves",
            ContentPackCategory::FieldBoxItems => "field_box_items",
            ContentPackCategory::RuntimeTitleScreen => "runtime_title_screen",
            ContentPackCategory::FlyDestinations => "fly_destinations",
            ContentPackCategory::Npcs => "npcs",
            ContentPackCategory::PokegearLandmarks => "pokegear_landmarks",
            ContentPackCategory::PcStrings => "pc_strings",
            ContentPackCategory::MenuIcons => "menu_icons",
            ContentPackCategory::Items => "items",
            ContentPackCategory::Marts => "marts",
            ContentPackCategory::CurrencyConstants => "currency_constants",
            ContentPackCategory::Trainers => "trainers",
            ContentPackCategory::TrainerClassNames => "trainer_class_names",
            ContentPackCategory::Pokedex => "pokedex",
            ContentPackCategory::PokedexEntries => "pokedex_entries",
            ContentPackCategory::PokemonFrontpicAnim => "pokemon_frontpic_anim",
            ContentPackCategory::InitializeEvents => "initialize_events",
            ContentPackCategory::StoryEventScriptConstants => "story_event_script_constants",
            ContentPackCategory::StoryEvents => "story_events",
            ContentPackCategory::PhoneScripts => "phone_scripts",
            ContentPackCategory::PhoneContacts => "phone_contacts",
            ContentPackCategory::PermanentPhoneNumbers => "permanent_phone_numbers",
            ContentPackCategory::SpecialPhoneCalls => "special_phone_calls",
            ContentPackCategory::NpcTrades => "npc_trades",
            ContentPackCategory::SpecialRoutines => "special_routines",
            ContentPackCategory::AsmText => "asm_text",
            ContentPackCategory::MoveNames => "move_names",
            ContentPackCategory::BattleAnimations => "battle_animations",
            ContentPackCategory::BattleAnimationTable => "battle_animation_table",
            ContentPackCategory::BattleAnimBundle => "battle_anim_bundle",
            ContentPackCategory::SpriteAnimBundle => "sprite_anim_bundle",
            ContentPackCategory::SpritePaletteDefaults => "sprite_palette_defaults",
            ContentPackCategory::PokegearTownMapPaletteMap => "pokegear_town_map_palette_map",
            ContentPackCategory::PokemonCries => "pokemon_cries",
            ContentPackCategory::Audio => "audio",
            ContentPackCategory::Tilesets => "tilesets",
            ContentPackCategory::Playability => "playability",
        }
    }
}

fn required_nullable_string<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentPack {
    pub id: String,
    pub enabled: bool,
    pub priority: i32,
    pub path: String,
    #[serde(deserialize_with = "required_nullable_string")]
    pub compiled: Option<String>,
    pub files: ContentPackFiles,
}

impl Default for ContentPack {
    fn default() -> Self {
        Self {
            id: String::new(),
            enabled: true,
            priority: 0,
            path: String::new(),
            compiled: None,
            files: ContentPackFiles::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentPackIndex {
    pub version: u16,
    pub packs: Vec<ContentPack>,
}

impl Default for ContentPackIndex {
    fn default() -> Self {
        Self {
            version: 1,
            packs: Vec::new(),
        }
    }
}

impl ContentPackIndex {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1 {
            anyhow::bail!(
                "content pack index version {} is unsupported; expected 1",
                self.version
            );
        }
        let mut seen = BTreeSet::new();
        for pack in &self.packs {
            if !is_exact_content_pack_id_token(&pack.id) {
                anyhow::bail!(
                    "content pack id '{}' must be exact ASCII letters, numbers, underscores, hyphens, or dots",
                    pack.id
                );
            }
            if !seen.insert(pack.id.as_str()) {
                anyhow::bail!(
                    "content pack index includes duplicate pack id '{}'",
                    pack.id
                );
            }
            let expected_path = format!("content-packs/{}", pack.id);
            if pack.path != expected_path {
                anyhow::bail!(
                    "content pack {} path '{}' must be exactly {expected_path}",
                    pack.id,
                    pack.path
                );
            }
            if pack.compiled.is_some() {
                for category in CONTENT_PACK_CATEGORIES {
                    if let Some(entry) = pack.files.entries(*category).first() {
                        anyhow::bail!(
                            "content pack {} declares compiled content and raw {} file entry {}; choose one source",
                            pack.id,
                            category.as_str(),
                            entry
                        );
                    }
                }
            }
        }
        let enabled_compiled = self
            .packs
            .iter()
            .filter(|pack| pack.enabled && pack.compiled.is_some())
            .map(|pack| pack.id.as_str())
            .collect::<Vec<_>>();
        if enabled_compiled.len() > 1 {
            anyhow::bail!(
                "content pack index enables multiple compiled game packs: {}",
                enabled_compiled.join(", ")
            );
        }
        if let Some(compiled_pack_id) = enabled_compiled.first() {
            if self
                .packs
                .iter()
                .any(|pack| pack.enabled && pack.id != *compiled_pack_id)
            {
                anyhow::bail!(
                    "content pack index compiled game pack '{}' must be the only enabled content source",
                    compiled_pack_id
                );
            }
        }
        Ok(())
    }

    pub fn sort_packs(&mut self) {
        self.packs
            .sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.id.cmp(&b.id)));
    }

    pub fn enabled_packs_sorted(&self) -> Vec<&ContentPack> {
        let mut packs: Vec<&ContentPack> = self.packs.iter().filter(|pack| pack.enabled).collect();
        packs.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.id.cmp(&b.id)));
        packs
    }
}

fn validate_generated_core_content_pack_manifest(pack: &ContentPack) -> Result<()> {
    if pack.id != "core-modular" {
        anyhow::bail!(
            "generated core content pack manifest id '{}' must be core-modular",
            pack.id
        );
    }
    if !pack.enabled {
        anyhow::bail!("generated core content pack manifest must be enabled");
    }
    if pack.priority != -100 {
        anyhow::bail!(
            "generated core content pack manifest priority {} must be -100",
            pack.priority
        );
    }
    if pack.path != "content-packs/core-modular" {
        anyhow::bail!(
            "generated core content pack manifest path '{}' must be content-packs/core-modular",
            pack.path
        );
    }
    if pack.compiled.is_some() {
        anyhow::bail!("generated core content pack manifest must not declare compiled content");
    }
    let required_categories = [
        ContentPackCategory::MapScripts,
        ContentPackCategory::MapBlocks,
        ContentPackCategory::MapAttributes,
        ContentPackCategory::MapDimensions,
        ContentPackCategory::Npcs,
        ContentPackCategory::Items,
        ContentPackCategory::FieldMoves,
        ContentPackCategory::FieldBoxItems,
        ContentPackCategory::RuntimeSpawnPoints,
        ContentPackCategory::RuntimeMapMetadata,
        ContentPackCategory::RuntimeTitleScreen,
        ContentPackCategory::AsmText,
        ContentPackCategory::Tilesets,
        ContentPackCategory::Playability,
    ];
    for category in required_categories {
        if pack.files.entries(category).is_empty() {
            anyhow::bail!(
                "generated core content pack manifest must include {} data for runtime playability",
                category.as_str()
            );
        }
    }
    Ok(())
}

fn is_exact_content_pack_id_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackManifest {
    pub schema_version: u16,
    pub metadata: ModpackMetadata,
    pub priority: i32,
    pub dependencies: Vec<String>,
    pub payload: ModpackPayload,
}

impl Default for ModpackManifest {
    fn default() -> Self {
        Self {
            schema_version: 1,
            metadata: ModpackMetadata::default(),
            priority: 0,
            dependencies: Vec::new(),
            payload: ModpackPayload::default(),
        }
    }
}

impl ModpackManifest {
    pub fn id(&self) -> &str {
        &self.metadata.id
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(deserialize_with = "required_nullable_string")]
    pub author: Option<String>,
    #[serde(deserialize_with = "required_nullable_string")]
    pub description: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpecialPhoneCallRule {
    pub condition: String,
    pub contact_id: String,
    pub caller_script: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NpcTradeRule {
    #[serde(default)]
    pub dialog_set: String,
    #[serde(default)]
    pub requested_species: String,
    #[serde(default)]
    pub offered_species: String,
    #[serde(default)]
    pub nickname: String,
    #[serde(default)]
    pub dvs: Vec<u8>,
    #[serde(default)]
    pub held_item: String,
    #[serde(default)]
    pub original_trainer_id: u16,
    #[serde(default)]
    pub original_trainer_name: String,
    #[serde(default)]
    pub gender_requirement: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpecialRoutineRule {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldBoxItemRule {
    pub item_id: String,
    pub effect: String,
    pub decoration_flag: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackPayload {
    pub pokemon: BTreeMap<String, PokemonSpecies>,
    pub maps: BTreeMap<String, MapModule>,
    pub items: BTreeMap<String, Item>,
    pub moves: BTreeMap<String, Move>,
    pub evolutions: EvolutionTable,
    pub marts: MartCatalog,
    pub currency_constants: CurrencyCatalog,
    pub battle_reward_rules: BattleRewardRules,
    pub battle_escape_rules: BattleEscapeRules,
    pub step_event_rules: StepEventRules,
    pub fishing: FishingCatalog,
    pub fruit_trees: FruitTreeCatalog,
    pub field_moves: FieldMoveCatalog,
    pub field_box_items: BTreeMap<String, FieldBoxItemRule>,
    pub runtime_title_screen: RuntimeTitleScreen,
    pub runtime_spawn_points: BTreeMap<String, RuntimeSpawnPoint>,
    pub runtime_map_metadata: BTreeMap<String, RuntimeMapMetadata>,
    pub flee_mons: FleeMonTables,
    pub buena_password_categories: BuenaPasswordCategories,
    pub roaming_pokemon: RoamingPokemonCatalog,
    pub buena_prizes: BuenaPrizeDefinitions,
    pub kurt_apricorn_recipes: KurtApricornRecipes,
    #[serde(deserialize_with = "required_nullable_value")]
    pub shuckie_gift: Option<ShuckieGiftDefinition>,
    pub dratini_move_sets: DratiniMoveSets,
    #[serde(deserialize_with = "required_nullable_value")]
    pub bug_contest_config: Option<BugContestConfig>,
    #[serde(deserialize_with = "required_nullable_value")]
    pub battle_tower_rules: Option<BattleTowerRules>,
    pub oak_ratings: Vec<OakRatingEntry>,
    pub odd_egg_definitions: Vec<OddEggDefinition>,
    pub magikarp_lengths: Vec<MagikarpLengthEntry>,
    #[serde(deserialize_with = "required_nullable_value")]
    pub happiness_data: Option<HappinessData>,
    pub encounter_slot_tables: EncounterSlotTables,
    pub encounter_music_modifiers: EncounterMusicModifiers,
    pub battle_stat_multipliers: BattleStatMultiplierTables,
    pub capture_wobble_probabilities: Vec<CaptureWobbleProbability>,
    pub move_priorities: MovePriorityTable,
    pub type_categories: TypeCategories,
    pub type_effectiveness: TypeEffectivenessTable,
    pub weather_modifiers: WeatherModifiers,
    pub pc_strings: BTreeMap<String, String>,
    pub menu_icons: BTreeMap<String, String>,
    pub pokedex_entries: BTreeMap<String, RuntimePokedexEntry>,
    pub pokemon_frontpic_anim: BTreeMap<String, FrontpicAnimProgram>,
    pub initialize_events: InitializeEventsConfig,
    pub story_event_script_constants: StoryEventScriptConstants,
    pub asm_text: BTreeMap<String, String>,
    pub move_names: Vec<String>,
    pub battle_animations: BTreeMap<String, Vec<String>>,
    pub battle_animation_table: Vec<String>,
    pub battle_anim_bundle: String,
    pub sprite_anim_bundle: String,
    pub sprite_palette_defaults: BTreeMap<String, i64>,
    pub pokegear_town_map_palette_map: BTreeMap<String, Vec<String>>,
    pub pokegear_landmarks: PokegearLandmarksPayload,
    pub pokemon_cries: BTreeMap<String, PokemonCryMetadata>,
    pub wild_encounters: BTreeMap<String, WildEncounterData>,
    pub field_encounters: BTreeMap<String, FieldEncounterData>,
    pub trainers: TrainerCatalog,
    pub trainer_class_names: BTreeMap<String, String>,
    pub phone_contacts: PhoneContactCatalog,
    pub permanent_phone_numbers: BTreeMap<String, PermanentPhoneNumberRule>,
    pub special_phone_calls: BTreeMap<String, SpecialPhoneCallRule>,
    pub npc_trades: BTreeMap<String, NpcTradeRule>,
    pub special_routines: BTreeMap<String, SpecialRoutineRule>,
    pub audio: BTreeMap<String, ModpackAudioAsset>,
    pub capture_rules: CaptureRules,
    pub tilesets: BTreeMap<String, TilesetDefinition>,
    pub playability: PlayabilityRules,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PokemonCryMetadata {
    #[serde(deserialize_with = "required_audio_reference_token")]
    pub cry: String,
    #[serde(deserialize_with = "required_crystal_word_i16")]
    pub pitch: i16,
    #[serde(deserialize_with = "required_crystal_word_i16")]
    pub length: i16,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeTitleScreen {
    #[serde(deserialize_with = "required_nullable_u16")]
    pub new_game_spawn_identifier: Option<u16>,
    #[serde(deserialize_with = "required_nullable_audio_reference_token")]
    pub title_music: Option<String>,
}

fn required_audio_reference_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if !is_exact_audio_reference_token(&value) {
        return Err(serde::de::Error::custom(format!(
            "audio reference token must be exact ASCII alphanumeric/underscore, found {value:?}"
        )));
    }
    validate_no_reserved_payload_token(&value, "audio reference token")
        .map_err(serde::de::Error::custom)?;
    Ok(value)
}

fn required_nullable_audio_reference_token<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    let Some(value) = value else {
        return Ok(None);
    };
    if !is_exact_audio_reference_token(&value) {
        return Err(serde::de::Error::custom(format!(
            "audio reference token must be exact ASCII alphanumeric/underscore, found {value:?}"
        )));
    }
    validate_no_reserved_payload_token(&value, "audio reference token")
        .map_err(serde::de::Error::custom)?;
    Ok(Some(value))
}

fn required_nullable_u16<'de, D>(deserializer: D) -> Result<Option<u16>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<u16>::deserialize(deserializer)
}

fn required_nullable_value<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[cfg(test)]
fn required_crystal_byte_i16<'de, D>(deserializer: D) -> Result<i16, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = i16::deserialize(deserializer)?;
    if (0..=255).contains(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "Crystal byte value must be in 0..=255, found {value}"
        )))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TilesetDefinition {
    pub collision: BTreeMap<String, Vec<String>>,
    pub palette_map: Vec<u8>,
}

#[cfg(test)]
fn is_default<T>(value: &T) -> bool
where
    T: Default + PartialEq,
{
    value == &T::default()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ModpackAudioKind {
    Music,
    SoundEffect,
    Cry,
}

impl ModpackAudioKind {
    pub fn runtime_name(self) -> &'static str {
        match self {
            Self::Music => "music",
            Self::SoundEffect => "sound_effect",
            Self::Cry => "cry",
        }
    }

    pub fn save_name(self) -> &'static str {
        match self {
            Self::Music => "Music",
            Self::SoundEffect => "SoundEffect",
            Self::Cry => "Cry",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ModpackAudioSource {
    Midi,
    Pcm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ModpackAudioPlaybackMode {
    SequencedMidi,
    RawPcm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ModpackAudioLoopPolicy {
    Once,
    Loop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackAudioPlaybackEntry {
    pub id: String,
    pub kind: ModpackAudioKind,
    pub mode: ModpackAudioPlaybackMode,
    pub loop_policy: ModpackAudioLoopPolicy,
}

impl<'de> Deserialize<'de> for ModpackAudioPlaybackEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawModpackAudioPlaybackEntry {
            id: String,
            kind: ModpackAudioKind,
            mode: ModpackAudioPlaybackMode,
            loop_policy: ModpackAudioLoopPolicy,
        }

        let raw = RawModpackAudioPlaybackEntry::deserialize(deserializer)?;
        let entry = Self {
            id: raw.id,
            kind: raw.kind,
            mode: raw.mode,
            loop_policy: raw.loop_policy,
        };
        entry.validate().map_err(serde::de::Error::custom)?;
        Ok(entry)
    }
}

impl ModpackAudioPlaybackEntry {
    fn validate(&self) -> Result<()> {
        validate_modpack_audio_id(self.kind, &self.id)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackAudioPlaybackPlan {
    pub music: BTreeMap<String, ModpackAudioPlaybackEntry>,
    pub sound_effects: BTreeMap<String, ModpackAudioPlaybackEntry>,
    pub cries: BTreeMap<String, ModpackAudioPlaybackEntry>,
}

impl<'de> Deserialize<'de> for ModpackAudioPlaybackPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawModpackAudioPlaybackPlan {
            music: BTreeMap<String, ModpackAudioPlaybackEntry>,
            sound_effects: BTreeMap<String, ModpackAudioPlaybackEntry>,
            cries: BTreeMap<String, ModpackAudioPlaybackEntry>,
        }

        let raw = RawModpackAudioPlaybackPlan::deserialize(deserializer)?;
        let plan = Self {
            music: raw.music,
            sound_effects: raw.sound_effects,
            cries: raw.cries,
        };
        plan.validate_structure()
            .map_err(serde::de::Error::custom)?;
        Ok(plan)
    }
}

impl ModpackAudioPlaybackPlan {
    pub fn from_manifest(manifest: &ModpackAudioManifest) -> Result<Self> {
        let mut plan = Self::default();
        for entry in manifest
            .music
            .values()
            .chain(manifest.sound_effects.values())
            .chain(manifest.cries.values())
        {
            plan.insert(ModpackAudioPlaybackEntry {
                id: entry.id.clone(),
                kind: entry.kind,
                mode: match entry.source {
                    ModpackAudioSource::Midi => ModpackAudioPlaybackMode::SequencedMidi,
                    ModpackAudioSource::Pcm => ModpackAudioPlaybackMode::RawPcm,
                },
                loop_policy: match entry.kind {
                    ModpackAudioKind::Music => ModpackAudioLoopPolicy::Loop,
                    ModpackAudioKind::SoundEffect | ModpackAudioKind::Cry => {
                        ModpackAudioLoopPolicy::Once
                    }
                },
            })?;
        }
        Ok(plan)
    }

    pub fn insert(&mut self, entry: ModpackAudioPlaybackEntry) -> Result<()> {
        entry.validate()?;
        let target = match entry.kind {
            ModpackAudioKind::Music => &mut self.music,
            ModpackAudioKind::SoundEffect => &mut self.sound_effects,
            ModpackAudioKind::Cry => &mut self.cries,
        };
        if target.insert(entry.id.clone(), entry).is_some() {
            anyhow::bail!("duplicate audio playback entry");
        }
        Ok(())
    }

    pub fn validate_for_manifest(&self, manifest: &ModpackAudioManifest) -> Result<()> {
        self.validate_structure()?;
        validate_audio_playback_entries("music", &self.music, &manifest.music)?;
        validate_audio_playback_entries(
            "sound_effects",
            &self.sound_effects,
            &manifest.sound_effects,
        )?;
        validate_audio_playback_entries("cries", &self.cries, &manifest.cries)?;
        Ok(())
    }

    fn validate_structure(&self) -> Result<()> {
        validate_audio_playback_bucket("music", ModpackAudioKind::Music, &self.music)?;
        validate_audio_playback_bucket(
            "sound_effects",
            ModpackAudioKind::SoundEffect,
            &self.sound_effects,
        )?;
        validate_audio_playback_bucket("cries", ModpackAudioKind::Cry, &self.cries)
    }
}

fn validate_audio_playback_bucket(
    label: &str,
    expected_kind: ModpackAudioKind,
    playback: &BTreeMap<String, ModpackAudioPlaybackEntry>,
) -> Result<()> {
    for (id, entry) in playback {
        entry.validate()?;
        if entry.id != *id {
            anyhow::bail!(
                "audio playback plan {label} map key {id} does not match entry id {}",
                entry.id
            );
        }
        if entry.kind != expected_kind {
            anyhow::bail!(
                "audio playback plan {label} entry {id} has kind {:?}, expected {:?}",
                entry.kind,
                expected_kind
            );
        }
    }
    Ok(())
}

fn validate_audio_playback_entries(
    label: &str,
    playback: &BTreeMap<String, ModpackAudioPlaybackEntry>,
    manifest: &BTreeMap<String, ModpackAudioManifestEntry>,
) -> Result<()> {
    if playback.len() != manifest.len() {
        anyhow::bail!(
            "audio playback plan {label} count {} does not match manifest count {}",
            playback.len(),
            manifest.len()
        );
    }
    for (id, manifest_entry) in manifest {
        let entry = playback
            .get(id)
            .with_context(|| format!("audio playback plan {label} missing manifest id {id}"))?;
        if entry.id != *id || entry.kind != manifest_entry.kind {
            anyhow::bail!(
                "audio playback plan {label} entry {id} does not match manifest identity"
            );
        }
        let expected_mode = match manifest_entry.source {
            ModpackAudioSource::Midi => ModpackAudioPlaybackMode::SequencedMidi,
            ModpackAudioSource::Pcm => ModpackAudioPlaybackMode::RawPcm,
        };
        if entry.mode != expected_mode {
            anyhow::bail!(
                "audio playback plan {label} entry {id} mode does not match manifest source"
            );
        }
        let expected_loop_policy = match manifest_entry.kind {
            ModpackAudioKind::Music => ModpackAudioLoopPolicy::Loop,
            ModpackAudioKind::SoundEffect | ModpackAudioKind::Cry => ModpackAudioLoopPolicy::Once,
        };
        if entry.loop_policy != expected_loop_policy {
            anyhow::bail!(
                "audio playback plan {label} entry {id} loop policy does not match manifest kind"
            );
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackPcmAudioFormat {
    pub sample_rate_hz: u32,
    pub channels: u8,
    pub bits_per_sample: u8,
}

impl ModpackPcmAudioFormat {
    fn validate(&self, id: &str) -> Result<()> {
        if self.sample_rate_hz == 0 {
            anyhow::bail!("PCM audio asset '{id}' must declare a positive sample_rate_hz");
        }
        if self.channels == 0 {
            anyhow::bail!("PCM audio asset '{id}' must declare at least one channel");
        }
        if self.bits_per_sample != 8 && self.bits_per_sample != 16 {
            anyhow::bail!("PCM audio asset '{id}' bits_per_sample must be 8 or 16");
        }
        Ok(())
    }

    fn frame_size_bytes(&self, id: &str) -> Result<usize> {
        self.validate(id)?;
        let bytes_per_sample = usize::from(self.bits_per_sample / 8);
        let frame_size = usize::from(self.channels)
            .checked_mul(bytes_per_sample)
            .ok_or_else(|| anyhow::anyhow!("PCM audio asset '{id}' frame size overflow"))?;
        if frame_size == 0 {
            anyhow::bail!("PCM audio asset '{id}' frame size must be positive");
        }
        Ok(frame_size)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackAudioAsset {
    pub id: String,
    pub path: String,
    pub kind: ModpackAudioKind,
    pub source: ModpackAudioSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pcm_format: Option<ModpackPcmAudioFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pcm_frame_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_start_sample: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_end_sample: Option<usize>,
}

impl<'de> Deserialize<'de> for ModpackAudioAsset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawModpackAudioAsset {
            id: String,
            path: String,
            kind: ModpackAudioKind,
            source: ModpackAudioSource,
            pcm_format: Option<ModpackPcmAudioFormat>,
            pcm_frame_count: Option<usize>,
            payload_hash: Option<String>,
            loop_start_sample: Option<usize>,
            loop_end_sample: Option<usize>,
        }

        let raw = RawModpackAudioAsset::deserialize(deserializer)?;
        let asset = Self {
            id: raw.id,
            path: raw.path,
            kind: raw.kind,
            source: raw.source,
            pcm_format: raw.pcm_format,
            pcm_frame_count: raw.pcm_frame_count,
            payload_hash: raw.payload_hash,
            loop_start_sample: raw.loop_start_sample,
            loop_end_sample: raw.loop_end_sample,
        };
        asset.validate().map_err(serde::de::Error::custom)?;
        Ok(asset)
    }
}

impl ModpackAudioAsset {
    pub fn music(id: impl Into<String>, path: impl Into<String>) -> Result<Self> {
        let asset = Self {
            id: id.into(),
            path: path.into(),
            kind: ModpackAudioKind::Music,
            source: ModpackAudioSource::Midi,
            pcm_format: None,
            pcm_frame_count: None,
            payload_hash: None,
            loop_start_sample: None,
            loop_end_sample: None,
        };
        asset.validate()?;
        Ok(asset)
    }

    pub fn cry(id: impl Into<String>, path: impl Into<String>) -> Result<Self> {
        let asset = Self {
            id: id.into(),
            path: path.into(),
            kind: ModpackAudioKind::Cry,
            source: ModpackAudioSource::Midi,
            pcm_format: None,
            pcm_frame_count: None,
            payload_hash: None,
            loop_start_sample: None,
            loop_end_sample: None,
        };
        asset.validate()?;
        Ok(asset)
    }

    pub fn sound_effect(id: impl Into<String>, path: impl Into<String>) -> Result<Self> {
        let asset = Self {
            id: id.into(),
            path: path.into(),
            kind: ModpackAudioKind::SoundEffect,
            source: ModpackAudioSource::Midi,
            pcm_format: None,
            pcm_frame_count: None,
            payload_hash: None,
            loop_start_sample: None,
            loop_end_sample: None,
        };
        asset.validate()?;
        Ok(asset)
    }

    pub fn pcm(
        id: impl Into<String>,
        path: impl Into<String>,
        kind: ModpackAudioKind,
        pcm_format: ModpackPcmAudioFormat,
    ) -> Result<Self> {
        let asset = Self {
            id: id.into(),
            path: path.into(),
            kind,
            source: ModpackAudioSource::Pcm,
            pcm_format: Some(pcm_format),
            pcm_frame_count: None,
            payload_hash: None,
            loop_start_sample: None,
            loop_end_sample: None,
        };
        asset.validate()?;
        Ok(asset)
    }

    pub fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            anyhow::bail!("audio asset id is required");
        }
        validate_modpack_audio_id(self.kind, &self.id)?;
        let path = Path::new(&self.path);
        validate_modpack_audio_asset_path(&self.id, path)?;
        validate_modpack_audio_asset_directory(&self.id, self.kind, path)?;
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| {
                anyhow::anyhow!("audio asset '{}' path must have a file stem", self.id)
            })?;
        if stem != self.id {
            anyhow::bail!(
                "audio asset '{}' path stem '{}' must match the exact audio id",
                self.id,
                stem
            );
        }
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .ok_or_else(|| {
                anyhow::anyhow!("audio asset '{}' path must have a file extension", self.id)
            })?;
        match self.source {
            ModpackAudioSource::Midi if extension == "mid" => {
                if self.pcm_format.is_some()
                    || self.pcm_frame_count.is_some()
                    || self.payload_hash.is_some()
                    || self.loop_start_sample.is_some()
                    || self.loop_end_sample.is_some()
                {
                    anyhow::bail!(
                        "MIDI audio asset '{}' must not declare PCM metadata",
                        self.id
                    );
                }
                Ok(())
            }
            ModpackAudioSource::Midi => {
                if self.pcm_format.is_some()
                    || self.pcm_frame_count.is_some()
                    || self.payload_hash.is_some()
                    || self.loop_start_sample.is_some()
                    || self.loop_end_sample.is_some()
                {
                    anyhow::bail!(
                        "MIDI audio asset '{}' must not declare PCM metadata",
                        self.id
                    );
                }
                anyhow::bail!("MIDI audio asset '{}' must use a .mid file", self.id)
            }
            ModpackAudioSource::Pcm if extension == "pcm" => {
                let Some(format) = &self.pcm_format else {
                    anyhow::bail!("PCM audio asset '{}' must declare pcm_format", self.id);
                };
                format.validate(&self.id)?;
                validate_optional_pcm_payload_metadata(
                    &self.id,
                    self.pcm_frame_count,
                    self.payload_hash.as_deref(),
                )?;
                validate_pcm_loop_metadata(
                    &self.id,
                    self.pcm_frame_count,
                    self.loop_start_sample,
                    self.loop_end_sample,
                )
            }
            ModpackAudioSource::Pcm => {
                if self.pcm_format.is_none() {
                    anyhow::bail!("PCM audio asset '{}' must declare pcm_format", self.id);
                }
                anyhow::bail!("PCM audio asset '{}' must use a .pcm file", self.id)
            }
        }
    }
}

fn validate_pcm_loop_metadata(
    id: &str,
    frame_count: Option<usize>,
    loop_start_sample: Option<usize>,
    loop_end_sample: Option<usize>,
) -> Result<()> {
    if loop_start_sample.is_some() != loop_end_sample.is_some() {
        anyhow::bail!(
            "PCM audio asset '{id}' must declare both loop_start_sample and loop_end_sample"
        );
    }
    let Some(loop_start_sample) = loop_start_sample else {
        return Ok(());
    };
    let loop_end_sample = loop_end_sample.expect("validated paired PCM loop metadata");
    let frame_count = frame_count.with_context(|| {
        format!("PCM audio asset '{id}' loop metadata requires pcm_frame_count")
    })?;
    if loop_start_sample >= loop_end_sample || loop_end_sample > frame_count {
        anyhow::bail!(
            "PCM audio asset '{id}' loop range [{loop_start_sample}, {loop_end_sample}) is outside {frame_count} frames"
        );
    }
    Ok(())
}

fn validate_optional_pcm_payload_metadata(
    id: &str,
    frame_count: Option<usize>,
    payload_hash: Option<&str>,
) -> Result<()> {
    if frame_count.is_some() != payload_hash.is_some() {
        anyhow::bail!("PCM audio asset '{id}' must declare both pcm_frame_count and payload_hash");
    }
    if let Some(frame_count) = frame_count {
        if frame_count == 0 {
            anyhow::bail!("PCM audio asset '{id}' pcm_frame_count must be positive");
        }
    }
    if let Some(payload_hash) = payload_hash {
        if payload_hash.len() != 8
            || !payload_hash
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            anyhow::bail!(
                "PCM audio asset '{id}' payload_hash must be exact lowercase 8-digit hex"
            );
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackAudioManifestEntry {
    pub id: String,
    pub path: String,
    pub kind: ModpackAudioKind,
    pub source: ModpackAudioSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pcm_format: Option<ModpackPcmAudioFormat>,
    pub byte_len: usize,
    pub payload_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pcm_frame_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_start_sample: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_end_sample: Option<usize>,
}

impl<'de> Deserialize<'de> for ModpackAudioManifestEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawModpackAudioManifestEntry {
            id: String,
            path: String,
            kind: ModpackAudioKind,
            source: ModpackAudioSource,
            pcm_format: Option<ModpackPcmAudioFormat>,
            byte_len: usize,
            payload_hash: String,
            pcm_frame_count: Option<usize>,
            loop_start_sample: Option<usize>,
            loop_end_sample: Option<usize>,
        }

        let raw = RawModpackAudioManifestEntry::deserialize(deserializer)?;
        let entry = Self {
            id: raw.id,
            path: raw.path,
            kind: raw.kind,
            source: raw.source,
            pcm_format: raw.pcm_format,
            byte_len: raw.byte_len,
            payload_hash: raw.payload_hash,
            pcm_frame_count: raw.pcm_frame_count,
            loop_start_sample: raw.loop_start_sample,
            loop_end_sample: raw.loop_end_sample,
        };
        entry.validate().map_err(serde::de::Error::custom)?;
        Ok(entry)
    }
}

impl ModpackAudioManifestEntry {
    fn validate(&self) -> Result<()> {
        let asset = ModpackAudioAsset {
            id: self.id.clone(),
            path: self.path.clone(),
            kind: self.kind,
            source: self.source,
            pcm_format: self.pcm_format.clone(),
            pcm_frame_count: None,
            payload_hash: None,
            loop_start_sample: None,
            loop_end_sample: None,
        };
        asset.validate()?;
        if self.byte_len == 0 {
            anyhow::bail!(
                "audio manifest entry '{}' byte_len must be positive",
                self.id
            );
        }
        if self.payload_hash.len() != 8
            || !self
                .payload_hash
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            anyhow::bail!(
                "audio manifest entry '{}' payload_hash must be exact lowercase 8-digit hex",
                self.id
            );
        }
        match self.source {
            ModpackAudioSource::Midi => {
                if self.pcm_frame_count.is_some()
                    || self.loop_start_sample.is_some()
                    || self.loop_end_sample.is_some()
                {
                    anyhow::bail!(
                        "MIDI audio manifest entry '{}' must not declare pcm_frame_count",
                        self.id
                    );
                }
            }
            ModpackAudioSource::Pcm => {
                let frame_count = self.pcm_frame_count.with_context(|| {
                    format!(
                        "PCM audio manifest entry '{}' must declare pcm_frame_count",
                        self.id
                    )
                })?;
                if frame_count == 0 {
                    anyhow::bail!(
                        "PCM audio manifest entry '{}' pcm_frame_count must be positive",
                        self.id
                    );
                }
                let frame_size = self
                    .pcm_format
                    .as_ref()
                    .with_context(|| {
                        format!("PCM audio manifest entry '{}' missing pcm_format", self.id)
                    })?
                    .frame_size_bytes(&self.id)?;
                let expected_byte_len = frame_count.checked_mul(frame_size).ok_or_else(|| {
                    anyhow::anyhow!(
                        "PCM audio manifest entry '{}' pcm_frame_count byte length overflow",
                        self.id
                    )
                })?;
                if expected_byte_len != self.byte_len {
                    anyhow::bail!(
                        "PCM audio manifest entry '{}' byte_len {} does not match {} frames of {} bytes",
                        self.id,
                        self.byte_len,
                        frame_count,
                        frame_size
                    );
                }
                validate_pcm_loop_metadata(
                    &self.id,
                    self.pcm_frame_count,
                    self.loop_start_sample,
                    self.loop_end_sample,
                )?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackAudioManifest {
    pub music: BTreeMap<String, ModpackAudioManifestEntry>,
    pub sound_effects: BTreeMap<String, ModpackAudioManifestEntry>,
    pub cries: BTreeMap<String, ModpackAudioManifestEntry>,
}

impl ModpackAudioManifest {
    pub fn from_assets(
        assets: &[ModpackAudioAsset],
        compiled_audio: &BTreeMap<String, Vec<u8>>,
    ) -> Result<Self> {
        let mut manifest = Self::default();
        let declared_ids = assets
            .iter()
            .map(|asset| asset.id.as_str())
            .collect::<BTreeSet<_>>();
        for audio_id in compiled_audio.keys() {
            if !declared_ids.contains(audio_id.as_str()) {
                anyhow::bail!(
                    "compiled audio payload '{}' is not declared by the definitive modpack",
                    audio_id
                );
            }
        }
        for asset in assets {
            asset.validate()?;
            let embedded = compiled_audio.get(&asset.id);
            let (byte_len, payload_hash, pcm_frame_count) = match (asset.source, embedded) {
                (_, Some(bytes)) => {
                    validate_compiled_audio_payload(asset, bytes)?;
                    let frame_count = match asset.source {
                        ModpackAudioSource::Midi => None,
                        ModpackAudioSource::Pcm => {
                            let format = asset.pcm_format.as_ref().with_context(|| {
                                format!(
                                    "PCM audio asset '{}' missing validated pcm_format",
                                    asset.id
                                )
                            })?;
                            Some(bytes.len() / format.frame_size_bytes(&asset.id)?)
                        }
                    };
                    (
                        bytes.len(),
                        format!("{:08x}", fnv1a32_bytes(bytes)),
                        frame_count,
                    )
                }
                (ModpackAudioSource::Pcm, None) => {
                    let frame_count = asset.pcm_frame_count.with_context(|| {
                        format!(
                            "external PCM audio asset '{}' missing pcm_frame_count",
                            asset.id
                        )
                    })?;
                    let payload_hash = asset.payload_hash.clone().with_context(|| {
                        format!(
                            "external PCM audio asset '{}' missing payload_hash",
                            asset.id
                        )
                    })?;
                    let format = asset.pcm_format.as_ref().with_context(|| {
                        format!("external PCM audio asset '{}' missing pcm_format", asset.id)
                    })?;
                    let byte_len = frame_count
                        .checked_mul(format.frame_size_bytes(&asset.id)?)
                        .context("external PCM byte length overflow")?;
                    (byte_len, payload_hash, Some(frame_count))
                }
                (ModpackAudioSource::Midi, None) => anyhow::bail!(
                    "compiled audio manifest missing payload for definitive MIDI asset '{}'",
                    asset.id
                ),
            };
            let entry = ModpackAudioManifestEntry {
                id: asset.id.clone(),
                path: asset.path.clone(),
                kind: asset.kind,
                source: asset.source,
                pcm_format: asset.pcm_format.clone(),
                byte_len,
                payload_hash,
                pcm_frame_count,
                loop_start_sample: asset.loop_start_sample,
                loop_end_sample: asset.loop_end_sample,
            };
            manifest.insert(entry)?;
        }
        Ok(manifest)
    }

    pub fn all_ids(&self) -> BTreeSet<&str> {
        self.music
            .keys()
            .chain(self.sound_effects.keys())
            .chain(self.cries.keys())
            .map(String::as_str)
            .collect()
    }

    fn insert(&mut self, entry: ModpackAudioManifestEntry) -> Result<()> {
        entry.validate()?;
        let id = entry.id.clone();
        let target = match entry.kind {
            ModpackAudioKind::Music => &mut self.music,
            ModpackAudioKind::SoundEffect => &mut self.sound_effects,
            ModpackAudioKind::Cry => &mut self.cries,
        };
        if target.insert(id.clone(), entry).is_some() {
            anyhow::bail!("duplicate definitive audio manifest id '{}'", id);
        }
        Ok(())
    }
}

fn validate_modpack_audio_id(kind: ModpackAudioKind, id: &str) -> Result<()> {
    let prefix = match kind {
        ModpackAudioKind::Music => "MUSIC_",
        ModpackAudioKind::SoundEffect => "SFX_",
        ModpackAudioKind::Cry => "CRY_",
    };
    let valid = id.starts_with(prefix)
        && id
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_');
    if !valid {
        anyhow::bail!("audio asset '{id}' must use an exact {kind:?} id");
    }
    let payload = &id[prefix.len()..];
    if payload.starts_with("FALLBACK") || payload.starts_with("LEGACY") {
        anyhow::bail!("audio asset '{id}' uses reserved runtime pack prefix");
    }
    Ok(())
}

fn validate_modpack_audio_asset_path(id: &str, path: &Path) -> Result<()> {
    if path.is_absolute() {
        anyhow::bail!("audio asset '{id}' path must be relative to assets/data");
    }
    let path_text = path.to_string_lossy();
    if path_text.starts_with("assets/data/") {
        anyhow::bail!("audio asset '{id}' path must not include the assets/data prefix");
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        anyhow::bail!("audio asset '{id}' path must not traverse parent directories");
    }
    if path_contains_current_directory_alias(path) {
        anyhow::bail!("audio asset '{id}' path must not include current-directory components");
    }
    Ok(())
}

fn validate_modpack_audio_asset_directory(
    id: &str,
    kind: ModpackAudioKind,
    path: &Path,
) -> Result<()> {
    let expected_directory = match kind {
        ModpackAudioKind::Music => "music",
        ModpackAudioKind::SoundEffect => "sfx",
        ModpackAudioKind::Cry => "cries",
    };
    let actual_directory = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            anyhow::anyhow!("audio asset '{id}' path must live under {expected_directory}")
        })?;
    if actual_directory != expected_directory {
        anyhow::bail!(
            "audio asset '{id}' path must live under {expected_directory}, found {actual_directory}"
        );
    }
    Ok(())
}
