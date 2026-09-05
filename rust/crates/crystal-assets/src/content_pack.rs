const COMPILED_GAME_PACK_MAGIC: &[u8; 12] = b"CRYSTALPACK\0";
pub const COMPILED_GAME_PACK_EXTENSION: &str = "crystalpack";
pub const COMPILED_GAME_PACK_FORMAT_VERSION: u16 = 13;
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
    Decorations,
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
    pub decorations: Vec<String>,
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
            ContentPackCategory::Decorations => &self.decorations,
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
    ContentPackCategory::Decorations,
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
            ContentPackCategory::Decorations => "decorations",
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
    pub value: u8,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DecorationCategory {
    Bed,
    Carpet,
    Plant,
    Poster,
    GameConsole,
    Ornament,
    BigDoll,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecorationDefinition {
    pub index: u8,
    pub id: String,
    pub category: DecorationCategory,
    pub display_name: String,
    pub action: String,
    pub event_flag: String,
    pub sprite: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecorationCatalog {
    pub category_order: Vec<DecorationCategory>,
    pub decorations: Vec<DecorationDefinition>,
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
    pub decorations: DecorationCatalog,
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
    #[serde(deserialize_with = "required_nullable_audio_reference_token")]
    pub title_music: Option<String>,
    pub program: RuntimePresentationProgram,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePresentationProgram {
    pub schema_version: u16,
    pub entrypoints: BTreeMap<String, String>,
    pub blocks: BTreeMap<String, RuntimePresentationBlock>,
    pub resources: Vec<RuntimePresentationResource>,
    pub audio: Vec<RuntimePresentationAudio>,
    pub text: Vec<RuntimePresentationText>,
    pub host_effects: Vec<Value>,
    pub subprograms: Vec<RuntimePresentationSubprogram>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePresentationSubprogram {
    pub id: String,
    pub source_entry: String,
    pub accepted_call_forms: Vec<String>,
    pub result: Value,
    pub phases: Vec<RuntimePresentationPhase>,
    #[serde(rename = "loop")]
    pub loop_: Value,
    pub resource_transfers: Vec<Value>,
    pub tilemap_writes: Vec<Value>,
    pub resources: Vec<RuntimePresentationSubprogramResource>,
    pub audio: Vec<RuntimePresentationAudio>,
    pub sprite_operations: Vec<Value>,
    pub sprite_programs: Vec<Value>,
    pub required_consumer: Value,
    pub source_span: RuntimePresentationSourceSpan,
    pub implementation_source_spans: Vec<RuntimePresentationSourceSpan>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePresentationPhase {
    pub id: String,
    pub source_span: RuntimePresentationSourceSpan,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, usize>,
    pub operations: Vec<RuntimePresentationOperation>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePresentationSubprogramResource {
    pub path: String,
    pub kind: String,
    pub include_source_span: RuntimePresentationSourceSpan,
    pub data_source_span: RuntimePresentationSourceSpan,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePresentationBlock {
    pub source_span: RuntimePresentationSourceSpan,
    pub operations: Vec<RuntimePresentationOperation>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePresentationOperation {
    pub op: String,
    pub source_span: RuntimePresentationSourceSpan,
    #[serde(flatten)]
    pub fields: BTreeMap<String, Value>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePresentationSourceSpan {
    pub file: String,
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePresentationResource {
    pub path: String,
    pub kind: String,
    pub source_span: RuntimePresentationSourceSpan,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePresentationAudio {
    pub id: String,
    pub kind: String,
    pub source_span: RuntimePresentationSourceSpan,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePresentationText {
    pub id: String,
    pub source_span: RuntimePresentationSourceSpan,
    pub commands: Vec<RuntimePresentationTextCommand>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimePresentationTextCommand {
    pub command: String,
    pub args: Vec<String>,
    pub source_span: RuntimePresentationSourceSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePresentationInterpreter {
    pub entrypoint: String,
    pub block: String,
    pub operation_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimePresentationStep {
    Operation(RuntimePresentationOperation),
    Jump { from: String, to: String },
    BlockComplete { block: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePresentationSubprogramInterpreter {
    pub subprogram: String,
    pub phase: String,
    pub operation_index: usize,
    pub current_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePresentationPhaseMachine {
    pub interpreter: RuntimePresentationSubprogramInterpreter,
    pub memory: BTreeMap<String, u16>,
    pub values: BTreeMap<String, u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePresentationPhaseRun {
    pub effects: Vec<RuntimePresentationOperation>,
    pub returned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuntimePresentationTimedPhaseCursor {
    pub subprogram: String,
    pub phase: String,
    pub operation_index: usize,
    pub end_operation_index: usize,
    pub wait_frames_remaining: u16,
    pub transfer_mode: Option<RuntimePresentation2bppTransferMode>,
    pub frame_t_cycles: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimePresentation2bppTransferMode {
    Default,
    Mobile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePresentationTimedPhaseTick {
    pub effects: Vec<RuntimePresentationOperation>,
    pub cpu_work_machine_cycles: u64,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTitleMainMenuItem {
    pub selection_index: usize,
    pub label: String,
    pub dispatch_target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTitleMainMenuDefinition {
    pub variants: Vec<Vec<RuntimeTitleMainMenuItem>>,
    pub new_game_variant: usize,
    pub continue_variant: usize,
    pub mystery_variant: usize,
    pub left: usize,
    pub top: usize,
    pub right: usize,
    pub bottom: usize,
    pub default_option: usize,
}

impl RuntimeTitleMainMenuDefinition {
    pub fn from_program(program: &RuntimePresentationProgram) -> Result<Self> {
        let phase = program
            .subprograms
            .iter()
            .find(|subprogram| subprogram.id == "main_menu")
            .and_then(|subprogram| {
                subprogram
                    .phases
                    .iter()
                    .find(|phase| phase.id == "main_menu")
            })
            .context("runtime presentation main_menu phase is missing")?;
        let load_menu = phase
            .operations
            .iter()
            .find(|operation| operation.op == "load_menu")
            .context("runtime presentation main_menu load_menu operation is missing")?;
        let select_variant = phase
            .operations
            .iter()
            .find(|operation| operation.op == "select_main_menu_variant")
            .context("runtime presentation main_menu variant selection is missing")?;
        let dispatch = phase
            .operations
            .iter()
            .find(|operation| {
                operation.op == "dispatch_table"
                    && operation.fields.get("dispatcher").and_then(Value::as_str)
                        == Some("MainMenu selection")
            })
            .context("runtime presentation MainMenu selection dispatch is missing")?;
        let strings = load_menu
            .fields
            .get("strings")
            .and_then(Value::as_array)
            .context("runtime presentation main_menu strings are missing")?;
        let entries = dispatch
            .fields
            .get("entries")
            .and_then(Value::as_array)
            .context("runtime presentation MainMenu selection entries are missing")?;
        anyhow::ensure!(
            !strings.is_empty() && strings.len() == entries.len(),
            "runtime presentation main_menu strings do not match dispatch entries"
        );
        let items = strings
            .iter()
            .zip(entries)
            .enumerate()
            .map(|(selection_index, (label, dispatch_target))| {
                Ok(RuntimeTitleMainMenuItem {
                    selection_index,
                    label: label
                        .as_str()
                        .filter(|label| !label.is_empty())
                        .context("runtime presentation main_menu has an invalid string")?
                        .to_string(),
                    dispatch_target: dispatch_target
                        .as_str()
                        .filter(|target| !target.is_empty())
                        .context("runtime presentation main_menu has an invalid dispatch target")?
                        .to_string(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let variants = load_menu
            .fields
            .get("item_sets")
            .and_then(Value::as_array)
            .context("runtime presentation main_menu item_sets are missing")?
            .iter()
            .map(|variant| {
                let indexes = variant
                    .as_array()
                    .filter(|indexes| !indexes.is_empty())
                    .context("runtime presentation main_menu has an empty item set")?;
                indexes
                    .iter()
                    .map(|index| {
                        let index = index
                            .as_u64()
                            .and_then(|index| usize::try_from(index).ok())
                            .context("runtime presentation main_menu has an invalid item index")?;
                        items.get(index).cloned().with_context(|| {
                            format!(
                                "runtime presentation main_menu item index {index} is out of range"
                            )
                        })
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .collect::<Result<Vec<_>>>()?;
        let coordinates = load_menu
            .fields
            .get("coordinates")
            .and_then(Value::as_object)
            .context("runtime presentation main_menu coordinates are missing")?;
        let coordinate = |field: &str| -> Result<usize> {
            coordinates
                .get(field)
                .and_then(Value::as_u64)
                .and_then(|value| usize::try_from(value).ok())
                .with_context(|| {
                    format!("runtime presentation main_menu coordinate {field} is invalid")
                })
        };
        let left = coordinate("left")?;
        let top = coordinate("top")?;
        let right = coordinate("right")?;
        let bottom = coordinate("bottom")?;
        anyhow::ensure!(
            left <= right && top <= bottom,
            "runtime presentation main_menu coordinates are inverted"
        );
        let default_option = load_menu
            .fields
            .get("default_option")
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| *value > 0)
            .context("runtime presentation main_menu default option is invalid")?;
        let variant_index = |id: &str| -> Result<usize> {
            let value = select_variant
                .fields
                .get("variants")
                .and_then(Value::as_array)
                .and_then(|variants| {
                    variants
                        .iter()
                        .find(|variant| variant.get("id").and_then(Value::as_str) == Some(id))
                })
                .and_then(|variant| variant.get("value"))
                .and_then(Value::as_u64)
                .and_then(|value| usize::try_from(value).ok())
                .with_context(|| {
                    format!("runtime presentation main_menu variant {id} is missing")
                })?;
            anyhow::ensure!(
                variants.get(value).is_some(),
                "runtime presentation main_menu variant {id} index {value} is out of range"
            );
            Ok(value)
        };
        Ok(Self {
            new_game_variant: variant_index("new_game")?,
            continue_variant: variant_index("continue")?,
            mystery_variant: variant_index("mystery")?,
            left,
            top,
            right,
            bottom,
            default_option,
            variants,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuntimeGenderMenuDefinition {
    pub items: Vec<String>,
    pub values: Vec<u8>,
    pub left: usize,
    pub top: usize,
    pub right: usize,
    pub bottom: usize,
    pub default_option: usize,
    pub confirm_delay_frames: u8,
}

impl RuntimeGenderMenuDefinition {
    pub fn from_program(program: &RuntimePresentationProgram) -> Result<Self> {
        let phase = program
            .subprograms
            .iter()
            .find(|subprogram| subprogram.id == "player_profile_setup")
            .and_then(|subprogram| {
                subprogram
                    .phases
                    .iter()
                    .find(|phase| phase.id == "gender_selection")
            })
            .context("runtime presentation gender_selection phase is missing")?;
        let operation = |name: &str| -> Result<&RuntimePresentationOperation> {
            let mut matches = phase
                .operations
                .iter()
                .filter(|operation| operation.op == name);
            let operation = matches.next().with_context(|| {
                format!("runtime presentation gender_selection {name} operation is missing")
            })?;
            anyhow::ensure!(
                matches.next().is_none(),
                "runtime presentation gender_selection has duplicate {name} operations"
            );
            Ok(operation)
        };
        let load_menu = operation("load_menu")?;
        let selection = operation("select_player_gender")?;
        let wait = operation("wait_frames")?;
        let items = load_menu
            .fields
            .get("items")
            .and_then(Value::as_array)
            .filter(|items| !items.is_empty())
            .context("runtime presentation gender_selection items are missing")?
            .iter()
            .map(|item| {
                item.as_str()
                    .filter(|item| !item.is_empty())
                    .map(str::to_string)
                    .context("runtime presentation gender_selection has an invalid item")
            })
            .collect::<Result<Vec<_>>>()?;
        anyhow::ensure!(
            items.len() == 2,
            "runtime presentation gender_selection must contain exactly two items"
        );
        let domain = selection
            .fields
            .get("domain")
            .and_then(Value::as_array)
            .context("runtime presentation gender_selection domain is missing")?;
        anyhow::ensure!(
            domain.len() == items.len(),
            "runtime presentation gender_selection domain does not match its items"
        );
        let values = domain
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let cursor = entry
                    .get("cursor")
                    .and_then(Value::as_u64)
                    .and_then(|value| usize::try_from(value).ok())
                    .context("runtime presentation gender_selection cursor is invalid")?;
                anyhow::ensure!(
                    cursor == index + 1,
                    "runtime presentation gender_selection cursor {cursor} is out of order"
                );
                let label = entry
                    .get("label")
                    .and_then(Value::as_str)
                    .context("runtime presentation gender_selection label is invalid")?;
                anyhow::ensure!(
                    label == items[index],
                    "runtime presentation gender_selection label does not match its menu item"
                );
                entry
                    .get("value")
                    .and_then(Value::as_u64)
                    .and_then(|value| u8::try_from(value).ok())
                    .context("runtime presentation gender_selection value is invalid")
            })
            .collect::<Result<Vec<_>>>()?;
        anyhow::ensure!(
            values.as_slice() == [0, 1],
            "runtime presentation gender_selection has unsupported player-gender values"
        );
        let coordinates = load_menu
            .fields
            .get("coordinates")
            .and_then(Value::as_object)
            .context("runtime presentation gender_selection coordinates are missing")?;
        let coordinate = |field: &str| -> Result<usize> {
            coordinates
                .get(field)
                .and_then(Value::as_u64)
                .and_then(|value| usize::try_from(value).ok())
                .with_context(|| {
                    format!("runtime presentation gender_selection coordinate {field} is invalid")
                })
        };
        let left = coordinate("left")?;
        let top = coordinate("top")?;
        let right = coordinate("right")?;
        let bottom = coordinate("bottom")?;
        anyhow::ensure!(
            left <= right && top <= bottom && right < 20 && bottom < 18,
            "runtime presentation gender_selection coordinates are outside the LCD"
        );
        let default_option = load_menu
            .fields
            .get("default_option")
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| (1..=items.len()).contains(value))
            .context("runtime presentation gender_selection default option is invalid")?;
        let confirm_delay_frames = wait
            .fields
            .get("frames")
            .and_then(Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
            .filter(|value| *value > 0)
            .context("runtime presentation gender_selection wait is invalid")?;
        Ok(Self {
            items,
            values,
            left,
            top,
            right,
            bottom,
            default_option,
            confirm_delay_frames,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeIntroPresentationParameters {
    pub scene_labels: Vec<String>,
    pub scene_operation_offsets: Vec<usize>,
    pub completion_wait_frames: Vec<u8>,
    pub sprite_scheduler_frame_crossings: Vec<RuntimeIntroSpriteSchedulerFrameCrossing>,
    pub interrupt_timing: RuntimeIntroInterruptTiming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuntimeIntroSpriteSchedulerFrameCrossing {
    pub dispatcher_entry: usize,
    pub dispatch_tick: u16,
    pub elapsed_t_cycles_between_hooks: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuntimeIntroFrameClock {
    pub frame_t_cycles: u32,
    pub phase_t_cycles: u32,
    pub elapsed_t_cycles: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuntimeIntroInstructionClockAdvance {
    pub frame_boundaries_crossed: u64,
    pub vblank_interrupts_serviced: u16,
    pub lcd_interrupts_serviced: u16,
    pub timer_interrupts_serviced: u16,
}

impl RuntimeIntroFrameClock {
    pub fn new(frame_t_cycles: u32, phase_t_cycles: u32) -> Result<Self> {
        anyhow::ensure!(frame_t_cycles > 0, "intro frame clock has no frame duration");
        anyhow::ensure!(
            phase_t_cycles < frame_t_cycles,
            "intro frame clock phase is outside its frame"
        );
        Ok(Self {
            frame_t_cycles,
            phase_t_cycles,
            elapsed_t_cycles: 0,
        })
    }

    pub fn advance_t_cycles(&mut self, elapsed_t_cycles: u64) -> Result<u64> {
        let total = u64::from(self.phase_t_cycles)
            .checked_add(elapsed_t_cycles)
            .context("intro frame clock T-cycle total overflowed")?;
        let frame_t_cycles = u64::from(self.frame_t_cycles);
        let crossed = total / frame_t_cycles;
        self.phase_t_cycles = u32::try_from(total % frame_t_cycles)
            .context("intro frame clock phase exceeds u32")?;
        self.elapsed_t_cycles = self
            .elapsed_t_cycles
            .checked_add(elapsed_t_cycles)
            .context("intro frame clock elapsed T-cycle count overflowed")?;
        Ok(crossed)
    }

    pub fn advance_machine_cycles(&mut self, elapsed_machine_cycles: u64) -> Result<u64> {
        let elapsed_t_cycles = elapsed_machine_cycles
            .checked_mul(4)
            .context("intro frame clock machine-cycle conversion overflowed")?;
        self.advance_t_cycles(elapsed_t_cycles)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuntimeIntroInterruptTiming {
    pub frame_t_cycles: u32,
    pub intro_entry_phase_t_cycles: u32,
    pub entry_to_first_input_machine_cycles: u16,
    pub joy_text_delay_pressed_repeat_reset_machine_cycles: u16,
    pub joy_text_delay_repeat_suppressed_machine_cycles: u16,
    pub joy_text_delay_repeat_restart_machine_cycles: u16,
    pub joy_text_delay_common_instruction_machine_cycles: Vec<u8>,
    pub joy_text_delay_pressed_repeat_reset_tail_machine_cycles: Vec<u8>,
    pub joy_text_delay_repeat_suppressed_tail_machine_cycles: Vec<u8>,
    pub joy_text_delay_repeat_restart_tail_machine_cycles: Vec<u8>,
    pub after_input_before_scene_dispatch_machine_cycles: u16,
    pub scene_dispatch_to_sprite_scheduler_machine_cycles: u16,
    pub sprite_scheduler_to_frame_wait_machine_cycles: u16,
    pub hardware_entry_machine_cycles: u16,
    pub vector_jump_machine_cycles: u16,
    pub lcd_interrupts_per_visible_frame: u16,
    pub lcd_scanline_t_cycles: u16,
    pub lcd_hblank_request_t_cycles: u16,
    pub vblank_request_t_cycles: u32,
    pub lcd_callback_zero_machine_cycles: u16,
    pub lcd_callback_nonzero_machine_cycles: u16,
    pub timer_request_period_t_cycles: u32,
    pub first_timer_request_after_intro_entry_t_cycles: u32,
    pub inactive_timer_machine_cycles: u16,
    pub inactive_game_timer_machine_cycles: u16,
    pub vblank_wrapper_epilogue_machine_cycles: u16,
    pub sound_update_is_state_dependent: bool,
    pub inactive_channels_sound_update_machine_cycles: u16,
    pub inactive_pitch_slide_machine_cycles: u16,
    pub inactive_track_vibrato_machine_cycles: u16,
    pub inactive_noise_machine_cycles: u16,
    pub inactive_danger_machine_cycles: u16,
    pub inactive_music_fade_machine_cycles: u16,
    pub active_music_channel_extra_machine_cycles: u16,
    pub active_sfx_channel_extra_machine_cycles: u16,
    pub shadowed_music_channel_extra_machine_cycles: u16,
    pub note_over_extra_before_parse_machine_cycles: u16,
    pub track_vibrato: RuntimeIntroTrackVibratoTiming,
    pub update_channels: RuntimeIntroUpdateChannelsTiming,
    pub parse_music: RuntimeIntroParseMusicTiming,
    pub noise: RuntimeIntroNoiseTiming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeIntroActiveSoundChannelClass {
    MusicWithoutActiveSfx,
    Sfx,
    MusicShadowedByActiveSfx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuntimeIntroActiveSoundChannelTiming {
    pub class: RuntimeIntroActiveSoundChannelClass,
    pub pitch_slide_machine_cycles: u16,
    pub track_vibrato_machine_cycles: u16,
    pub noise_machine_cycles: u16,
    pub update_channels_machine_cycles: Option<u16>,
    pub note_over: bool,
    pub parse_music_machine_cycles: Option<u16>,
}

impl RuntimeIntroInterruptTiming {
    fn advance_interrupt_scheduled_segment(
        &self,
        clock: &mut RuntimeIntroFrameClock,
        elapsed_t_cycles: u64,
        next_vblank_request: &mut u64,
        next_lcd_request: &mut u64,
        next_timer_request: &mut u64,
        pending_vblank: &mut bool,
        pending_lcd: &mut bool,
        pending_timer: &mut bool,
    ) -> Result<u64> {
        let start_elapsed = clock.elapsed_t_cycles;
        let start_phase = clock.phase_t_cycles;
        let end_elapsed = clock
            .elapsed_t_cycles
            .checked_add(elapsed_t_cycles)
            .context("intro interrupt segment elapsed T-cycle count overflowed")?;
        while *next_vblank_request <= end_elapsed {
            *pending_vblank = true;
            *next_vblank_request = next_vblank_request
                .checked_add(u64::from(self.frame_t_cycles))
                .context("next intro VBlank request overflowed")?;
        }
        while *next_lcd_request <= end_elapsed {
            *pending_lcd = true;
            let request_offset = next_lcd_request
                .checked_sub(start_elapsed)
                .context("next intro LCD request precedes the current segment")?;
            let request_phase = u32::try_from(
                (u64::from(start_phase) + request_offset) % u64::from(self.frame_t_cycles),
            )
            .context("intro LCD request phase exceeds u32")?;
            let scanline = u32::from(self.lcd_scanline_t_cycles);
            let line = request_phase / scanline;
            anyhow::ensure!(
                line < u32::from(self.lcd_interrupts_per_visible_frame)
                    && request_phase % scanline == u32::from(self.lcd_hblank_request_t_cycles),
                "intro LCD request is outside the HBlank lattice"
            );
            let distance = if line + 1 < u32::from(self.lcd_interrupts_per_visible_frame) {
                scanline
            } else {
                self.frame_t_cycles
                    .checked_sub(request_phase)
                    .and_then(|remaining| {
                        remaining.checked_add(u32::from(self.lcd_hblank_request_t_cycles))
                    })
                    .context("next-frame LCD request distance overflowed")?
            };
            *next_lcd_request = clock
                .elapsed_t_cycles
                .checked_add(request_offset)
                .and_then(|request| request.checked_add(u64::from(distance)))
                .context("next intro LCD request overflowed")?;
        }
        while *next_timer_request <= end_elapsed {
            *pending_timer = true;
            *next_timer_request = next_timer_request
                .checked_add(u64::from(self.timer_request_period_t_cycles))
                .context("next intro timer request overflowed")?;
        }
        clock.advance_t_cycles(elapsed_t_cycles)
    }

    #[allow(clippy::too_many_arguments)]
    fn service_pending_instruction_interrupts(
        &self,
        clock: &mut RuntimeIntroFrameClock,
        callback_nonzero: bool,
        vblank_body_machine_cycles: &mut dyn FnMut() -> Result<u64>,
        next_vblank_request: &mut u64,
        next_lcd_request: &mut u64,
        next_timer_request: &mut u64,
        pending_vblank: &mut bool,
        pending_lcd: &mut bool,
        pending_timer: &mut bool,
        vblank_interrupts_serviced: &mut u16,
        lcd_interrupts_serviced: &mut u16,
        timer_interrupts_serviced: &mut u16,
    ) -> Result<u64> {
        let mut crossed_frames = 0_u64;
        while *pending_vblank || *pending_lcd || *pending_timer {
            let handler_machine_cycles = if *pending_vblank {
                *pending_vblank = false;
                *vblank_interrupts_serviced = vblank_interrupts_serviced
                    .checked_add(1)
                    .context("intro VBlank interrupt count overflowed")?;
                vblank_body_machine_cycles()?
                    .checked_add(u64::from(self.hardware_entry_machine_cycles))
                    .and_then(|value| value.checked_add(u64::from(self.vector_jump_machine_cycles)))
                    .context("intro VBlank interrupt timing overflowed")?
            } else if *pending_lcd {
                *pending_lcd = false;
                *lcd_interrupts_serviced = lcd_interrupts_serviced
                    .checked_add(1)
                    .context("intro LCD interrupt count overflowed")?;
                u64::from(if callback_nonzero {
                    self.lcd_callback_nonzero_machine_cycles
                } else {
                    self.lcd_callback_zero_machine_cycles
                })
            } else {
                *pending_timer = false;
                *timer_interrupts_serviced = timer_interrupts_serviced
                    .checked_add(1)
                    .context("intro timer interrupt count overflowed")?;
                u64::from(self.inactive_timer_machine_cycles)
            };
            anyhow::ensure!(
                handler_machine_cycles > 0,
                "intro interrupt handler has zero duration"
            );
            let handler_t_cycles = handler_machine_cycles
                .checked_mul(4)
                .context("intro interrupt handler T-cycle count overflowed")?;
            crossed_frames = crossed_frames
                .checked_add(self.advance_interrupt_scheduled_segment(
                    clock,
                    handler_t_cycles,
                    next_vblank_request,
                    next_lcd_request,
                    next_timer_request,
                    pending_vblank,
                    pending_lcd,
                    pending_timer,
                )?)
                .context("intro instruction frame-boundary count overflowed")?;
        }
        Ok(crossed_frames)
    }

    pub fn advance_instruction_sequence_with_interrupts(
        &self,
        clock: &mut RuntimeIntroFrameClock,
        instruction_machine_cycles: impl IntoIterator<Item = u64>,
        callback_nonzero: bool,
        vblank_body_machine_cycles: &mut dyn FnMut() -> Result<u64>,
    ) -> Result<RuntimeIntroInstructionClockAdvance> {
        anyhow::ensure!(
            self.timer_request_period_t_cycles > 0,
            "intro timer request period is zero"
        );
        let mut frame_boundaries_crossed = 0_u64;
        let mut vblank_interrupts_serviced = 0_u16;
        let mut lcd_interrupts_serviced = 0_u16;
        let mut timer_interrupts_serviced = 0_u16;
        let vblank_distance = self.t_cycles_until_next_vblank(clock)?;
        let mut pending_vblank = vblank_distance == 0;
        let mut next_vblank_request = clock
            .elapsed_t_cycles
            .checked_add(u64::from(if pending_vblank {
                self.frame_t_cycles
            } else {
                vblank_distance
            }))
            .context("next intro VBlank request overflowed")?;
        let lcd_distance = self.t_cycles_until_next_lcd_hblank(clock)?;
        let mut pending_lcd = lcd_distance == 0;
        let mut next_lcd_request = clock
            .elapsed_t_cycles
            .checked_add(u64::from(if pending_lcd {
                self.t_cycles_until_lcd_hblank_after_current(clock)?
            } else {
                lcd_distance
            }))
            .context("next intro LCD request overflowed")?;
        let timer_period = u64::from(self.timer_request_period_t_cycles);
        let first_timer_request = u64::from(self.first_timer_request_after_intro_entry_t_cycles);
        let mut next_timer_request = if clock.elapsed_t_cycles <= first_timer_request {
            first_timer_request
        } else {
            let elapsed_after_first = clock.elapsed_t_cycles - first_timer_request;
            let periods = elapsed_after_first
                .checked_add(timer_period - 1)
                .context("intro timer request rounding overflowed")?
                / timer_period;
            first_timer_request
                .checked_add(
                    periods
                        .checked_mul(timer_period)
                        .context("intro timer request period overflowed")?,
                )
                .context("next intro timer request overflowed")?
        };
        let mut pending_timer = next_timer_request == clock.elapsed_t_cycles;
        if pending_timer {
            next_timer_request = next_timer_request
                .checked_add(timer_period)
                .context("next intro timer request overflowed")?;
        }
        frame_boundaries_crossed = frame_boundaries_crossed
            .checked_add(self.service_pending_instruction_interrupts(
                clock,
                callback_nonzero,
                vblank_body_machine_cycles,
                &mut next_vblank_request,
                &mut next_lcd_request,
                &mut next_timer_request,
                &mut pending_vblank,
                &mut pending_lcd,
                &mut pending_timer,
                &mut vblank_interrupts_serviced,
                &mut lcd_interrupts_serviced,
                &mut timer_interrupts_serviced,
            )?)
            .context("intro instruction frame-boundary count overflowed")?;
        for machine_cycles in instruction_machine_cycles {
            anyhow::ensure!(
                machine_cycles > 0,
                "intro instruction clock received a zero-cycle instruction"
            );
            let instruction_t_cycles = machine_cycles
                .checked_mul(4)
                .context("intro instruction T-cycle conversion overflowed")?;
            frame_boundaries_crossed = frame_boundaries_crossed
                .checked_add(self.advance_interrupt_scheduled_segment(
                    clock,
                    instruction_t_cycles,
                    &mut next_vblank_request,
                    &mut next_lcd_request,
                    &mut next_timer_request,
                    &mut pending_vblank,
                    &mut pending_lcd,
                    &mut pending_timer,
                )?)
                .context("intro instruction frame-boundary count overflowed")?;
            frame_boundaries_crossed = frame_boundaries_crossed
                .checked_add(self.service_pending_instruction_interrupts(
                    clock,
                    callback_nonzero,
                    vblank_body_machine_cycles,
                    &mut next_vblank_request,
                    &mut next_lcd_request,
                    &mut next_timer_request,
                    &mut pending_vblank,
                    &mut pending_lcd,
                    &mut pending_timer,
                    &mut vblank_interrupts_serviced,
                    &mut lcd_interrupts_serviced,
                    &mut timer_interrupts_serviced,
                )?)
                .context("intro instruction frame-boundary count overflowed")?;
        }
        Ok(RuntimeIntroInstructionClockAdvance {
            frame_boundaries_crossed,
            vblank_interrupts_serviced,
            lcd_interrupts_serviced,
            timer_interrupts_serviced,
        })
    }

    pub fn joy_text_delay_instruction_machine_cycles(
        &self,
        joy_pressed: bool,
        text_delay_frames: u8,
    ) -> impl Iterator<Item = u64> + '_ {
        let tail = if joy_pressed {
            self.joy_text_delay_pressed_repeat_reset_tail_machine_cycles
                .as_slice()
        } else if text_delay_frames == 0 {
            self.joy_text_delay_repeat_restart_tail_machine_cycles
                .as_slice()
        } else {
            self.joy_text_delay_repeat_suppressed_tail_machine_cycles
                .as_slice()
        };
        self.joy_text_delay_common_instruction_machine_cycles
            .iter()
            .chain(tail)
            .copied()
            .map(u64::from)
    }

    pub fn t_cycles_until_next_lcd_hblank(
        &self,
        clock: &RuntimeIntroFrameClock,
    ) -> Result<u32> {
        anyhow::ensure!(
            clock.frame_t_cycles == self.frame_t_cycles,
            "intro LCD timing and frame clock disagree"
        );
        let scanline = u32::from(self.lcd_scanline_t_cycles);
        let hblank = u32::from(self.lcd_hblank_request_t_cycles);
        let visible_lines = u32::from(self.lcd_interrupts_per_visible_frame);
        let current_line = clock.phase_t_cycles / scanline;
        if current_line < visible_lines {
            let request_phase = current_line * scanline + hblank;
            if request_phase >= clock.phase_t_cycles {
                return Ok(request_phase - clock.phase_t_cycles);
            }
            if current_line + 1 < visible_lines {
                return Ok((current_line + 1) * scanline + hblank - clock.phase_t_cycles);
            }
        }
        self.frame_t_cycles
            .checked_sub(clock.phase_t_cycles)
            .and_then(|remaining| remaining.checked_add(hblank))
            .context("intro next-frame LCD request distance overflowed")
    }

    fn t_cycles_until_lcd_hblank_after_current(
        &self,
        clock: &RuntimeIntroFrameClock,
    ) -> Result<u32> {
        let distance = self.t_cycles_until_next_lcd_hblank(clock)?;
        if distance != 0 {
            return Ok(distance);
        }
        let scanline = u32::from(self.lcd_scanline_t_cycles);
        let visible_lines = u32::from(self.lcd_interrupts_per_visible_frame);
        let current_line = clock.phase_t_cycles / scanline;
        if current_line + 1 < visible_lines {
            return Ok(scanline);
        }
        self.frame_t_cycles
            .checked_sub(clock.phase_t_cycles)
            .and_then(|remaining| {
                remaining.checked_add(u32::from(self.lcd_hblank_request_t_cycles))
            })
            .context("intro next strict LCD request distance overflowed")
    }

    pub fn t_cycles_until_next_vblank(&self, clock: &RuntimeIntroFrameClock) -> Result<u32> {
        anyhow::ensure!(
            clock.frame_t_cycles == self.frame_t_cycles,
            "intro VBlank timing and frame clock disagree"
        );
        if self.vblank_request_t_cycles >= clock.phase_t_cycles {
            return Ok(self.vblank_request_t_cycles - clock.phase_t_cycles);
        }
        self.frame_t_cycles
            .checked_sub(clock.phase_t_cycles)
            .and_then(|remaining| remaining.checked_add(self.vblank_request_t_cycles))
            .context("intro next-frame VBlank request distance overflowed")
    }

    pub fn joy_text_delay_machine_cycles(
        &self,
        joy_pressed: bool,
        text_delay_frames: u8,
    ) -> u16 {
        if joy_pressed {
            self.joy_text_delay_pressed_repeat_reset_machine_cycles
        } else if text_delay_frames == 0 {
            self.joy_text_delay_repeat_restart_machine_cycles
        } else {
            self.joy_text_delay_repeat_suppressed_machine_cycles
        }
    }

    pub fn outer_loop_body_machine_cycles(&self) -> Result<u16> {
        self.after_input_before_scene_dispatch_machine_cycles
            .checked_add(self.scene_dispatch_to_sprite_scheduler_machine_cycles)
            .and_then(|value| {
                value.checked_add(self.sprite_scheduler_to_frame_wait_machine_cycles)
            })
            .context("intro outer-loop body machine-cycle total overflowed")
    }

    pub fn sound_update_machine_cycles(
        &self,
        active_channels: &[RuntimeIntroActiveSoundChannelTiming],
    ) -> Result<u16> {
        let mut total = u32::from(self.inactive_channels_sound_update_machine_cycles);
        for channel in active_channels {
            let channel_overhead = match channel.class {
                RuntimeIntroActiveSoundChannelClass::MusicWithoutActiveSfx => {
                    anyhow::ensure!(
                        channel.update_channels_machine_cycles.is_some(),
                        "unshadowed music channel has no UpdateChannels timing"
                    );
                    self.active_music_channel_extra_machine_cycles
                }
                RuntimeIntroActiveSoundChannelClass::Sfx => {
                    anyhow::ensure!(
                        channel.update_channels_machine_cycles.is_some(),
                        "SFX channel has no UpdateChannels timing"
                    );
                    self.active_sfx_channel_extra_machine_cycles
                }
                RuntimeIntroActiveSoundChannelClass::MusicShadowedByActiveSfx => {
                    anyhow::ensure!(
                        channel.update_channels_machine_cycles.is_none()
                            && channel.parse_music_machine_cycles.is_none(),
                        "shadowed music channel executed an inaudible write or parser"
                    );
                    self.shadowed_music_channel_extra_machine_cycles
                }
            };
            anyhow::ensure!(
                channel.note_over || channel.parse_music_machine_cycles.is_none(),
                "sound channel parsed music without reaching note-over"
            );
            total += u32::from(channel_overhead)
                + u32::from(channel.pitch_slide_machine_cycles)
                + u32::from(channel.track_vibrato_machine_cycles)
                + u32::from(channel.noise_machine_cycles)
                + u32::from(channel.update_channels_machine_cycles.unwrap_or(0));
            if channel.note_over {
                total += u32::from(self.note_over_extra_before_parse_machine_cycles);
            }
            total += u32::from(channel.parse_music_machine_cycles.unwrap_or(0));
        }
        u16::try_from(total).context("_UpdateSound timing exceeds the SM83 u16 cycle domain")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuntimeIntroParseMusicTiming {
    pub normal_note_base_machine_cycles: u16,
    pub music_noise_note_base_machine_cycles: u16,
    pub octave_command_machine_cycles: u16,
    pub set_note_duration: RuntimeIntroSetNoteDurationTiming,
    pub get_frequency: RuntimeIntroGetFrequencyTiming,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuntimeIntroSetNoteDurationTiming {
    pub fixed_machine_cycles: u16,
    pub multiply_per_bit_machine_cycles: u16,
    pub multiply_fixed_machine_cycles: u16,
    pub multiply_set_bit_extra_machine_cycles: u16,
    pub minimum_multiply_iterations: u8,
}

impl RuntimeIntroSetNoteDurationTiming {
    fn multiply_machine_cycles(&self, value: u8) -> u16 {
        let bit_length = if value == 0 {
            0
        } else {
            u8::BITS - value.leading_zeros()
        };
        let iterations = bit_length.max(u32::from(self.minimum_multiply_iterations));
        self.multiply_fixed_machine_cycles
            + self.multiply_per_bit_machine_cycles * iterations as u16
            + self.multiply_set_bit_extra_machine_cycles * value.count_ones() as u16
    }

    pub fn machine_cycles(&self, note_length: u8, duration_nibble: u8) -> u16 {
        let duration_units = (duration_nibble & 0x0f).wrapping_add(1);
        let scaled_length = note_length.wrapping_mul(duration_units);
        self.fixed_machine_cycles
            + self.multiply_machine_cycles(note_length)
            + self.multiply_machine_cycles(scaled_length)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuntimeIntroGetFrequencyTiming {
    pub fixed_machine_cycles: u16,
    pub per_right_shift_machine_cycles: u16,
    pub target_octave: u8,
}

impl RuntimeIntroGetFrequencyTiming {
    pub fn machine_cycles(&self, effective_octave: u8) -> u16 {
        self.fixed_machine_cycles
            + u16::from(self.target_octave.saturating_sub(effective_octave))
                * self.per_right_shift_machine_cycles
    }
}

impl RuntimeIntroParseMusicTiming {
    pub fn normal_note_machine_cycles(
        &self,
        note_length: u8,
        duration_nibble: u8,
        effective_octave: u8,
        octave_command_count: u8,
    ) -> u16 {
        self.normal_note_base_machine_cycles
            + self.set_note_duration.machine_cycles(note_length, duration_nibble)
            + self.get_frequency.machine_cycles(effective_octave)
            + u16::from(octave_command_count) * self.octave_command_machine_cycles
    }

    pub fn music_noise_note_machine_cycles(
        &self,
        note_length: u8,
        duration_nibble: u8,
        octave_command_count: u8,
    ) -> u16 {
        self.music_noise_note_base_machine_cycles
            + self.set_note_duration.machine_cycles(note_length, duration_nibble)
            + u16::from(octave_command_count) * self.octave_command_machine_cycles
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuntimeIntroUpdateChannelsTiming {
    pub pulse1_unchanged_machine_cycles: u16,
    pub pulse1_noise_sampling_machine_cycles: u16,
    pub pulse2_unchanged_machine_cycles: u16,
    pub pulse2_vibrato_override_machine_cycles: u16,
    pub wave_unchanged_machine_cycles: u16,
    pub wave_noise_sampling_machine_cycles: u16,
    pub noise_unchanged_machine_cycles: u16,
    pub noise_noise_sampling_machine_cycles: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeIntroUpdateChannelsPath {
    Pulse1Unchanged,
    Pulse1NoiseSampling,
    Pulse2Unchanged,
    Pulse2VibratoOverride,
    WaveUnchanged,
    WaveNoiseSampling,
    NoiseUnchanged,
    NoiseNoiseSampling,
}

impl RuntimeIntroUpdateChannelsTiming {
    pub fn machine_cycles(&self, path: RuntimeIntroUpdateChannelsPath) -> u16 {
        match path {
            RuntimeIntroUpdateChannelsPath::Pulse1Unchanged => {
                self.pulse1_unchanged_machine_cycles
            }
            RuntimeIntroUpdateChannelsPath::Pulse1NoiseSampling => {
                self.pulse1_noise_sampling_machine_cycles
            }
            RuntimeIntroUpdateChannelsPath::Pulse2Unchanged => {
                self.pulse2_unchanged_machine_cycles
            }
            RuntimeIntroUpdateChannelsPath::Pulse2VibratoOverride => {
                self.pulse2_vibrato_override_machine_cycles
            }
            RuntimeIntroUpdateChannelsPath::WaveUnchanged => self.wave_unchanged_machine_cycles,
            RuntimeIntroUpdateChannelsPath::WaveNoiseSampling => {
                self.wave_noise_sampling_machine_cycles
            }
            RuntimeIntroUpdateChannelsPath::NoiseUnchanged => self.noise_unchanged_machine_cycles,
            RuntimeIntroUpdateChannelsPath::NoiseNoiseSampling => {
                self.noise_noise_sampling_machine_cycles
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuntimeIntroTrackVibratoTiming {
    pub base_machine_cycles: u16,
    pub duty_loop_extra_machine_cycles: u16,
    pub pitch_offset_extra_machine_cycles: u16,
    pub delay_count_nonzero_extra_machine_cycles: u16,
    pub zero_extent_extra_machine_cycles: u16,
    pub rate_count_nonzero_extra_machine_cycles: u16,
    pub toggle_up_no_borrow_extra_machine_cycles: u16,
    pub toggle_up_borrow_extra_machine_cycles: u16,
    pub toggle_down_no_carry_extra_machine_cycles: u16,
    pub toggle_down_carry_extra_machine_cycles: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeIntroVibratoBranch {
    Disabled,
    DelayCountNonzero,
    ZeroExtent,
    RateCountNonzero,
    ToggleUpNoBorrow,
    ToggleUpBorrow,
    ToggleDownNoCarry,
    ToggleDownCarry,
}

impl RuntimeIntroTrackVibratoTiming {
    pub fn machine_cycles(
        &self,
        duty_loop: bool,
        pitch_offset: bool,
        vibrato: RuntimeIntroVibratoBranch,
    ) -> u16 {
        let vibrato_extra = match vibrato {
            RuntimeIntroVibratoBranch::Disabled => 0,
            RuntimeIntroVibratoBranch::DelayCountNonzero => {
                self.delay_count_nonzero_extra_machine_cycles
            }
            RuntimeIntroVibratoBranch::ZeroExtent => self.zero_extent_extra_machine_cycles,
            RuntimeIntroVibratoBranch::RateCountNonzero => {
                self.rate_count_nonzero_extra_machine_cycles
            }
            RuntimeIntroVibratoBranch::ToggleUpNoBorrow => {
                self.toggle_up_no_borrow_extra_machine_cycles
            }
            RuntimeIntroVibratoBranch::ToggleUpBorrow => {
                self.toggle_up_borrow_extra_machine_cycles
            }
            RuntimeIntroVibratoBranch::ToggleDownNoCarry => {
                self.toggle_down_no_carry_extra_machine_cycles
            }
            RuntimeIntroVibratoBranch::ToggleDownCarry => {
                self.toggle_down_carry_extra_machine_cycles
            }
        };
        self.base_machine_cycles
            + u16::from(duty_loop) * self.duty_loop_extra_machine_cycles
            + u16::from(pitch_offset) * self.pitch_offset_extra_machine_cycles
            + vibrato_extra
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuntimeIntroNoiseTiming {
    pub inactive_machine_cycles: u16,
    pub sfx_prefix_machine_cycles: u16,
    pub music_ch8_off_prefix_machine_cycles: u16,
    pub music_ch8_non_noise_prefix_machine_cycles: u16,
    pub music_blocked_by_noise_ch8_machine_cycles: u16,
    pub nonzero_delay_machine_cycles: u16,
    pub zero_delay_machine_cycles: u16,
    pub empty_address_machine_cycles: u16,
    pub sound_ret_machine_cycles: u16,
    pub sample_machine_cycles: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeIntroNoiseSamplePath {
    EmptyAddress,
    SoundRet,
    Sample,
}

impl RuntimeIntroNoiseTiming {
    pub fn machine_cycles(
        &self,
        noise_enabled: bool,
        sfx_channel: bool,
        channel8_on: bool,
        channel8_noise: bool,
        delay_nonzero: bool,
        sample_path: RuntimeIntroNoiseSamplePath,
    ) -> u16 {
        if !noise_enabled {
            return self.inactive_machine_cycles;
        }
        if !sfx_channel && channel8_on && channel8_noise {
            return self.music_blocked_by_noise_ch8_machine_cycles;
        }
        let prefix = if sfx_channel {
            self.sfx_prefix_machine_cycles
        } else if channel8_on {
            self.music_ch8_non_noise_prefix_machine_cycles
        } else {
            self.music_ch8_off_prefix_machine_cycles
        };
        if delay_nonzero {
            return prefix + self.nonzero_delay_machine_cycles;
        }
        let sample = match sample_path {
            RuntimeIntroNoiseSamplePath::EmptyAddress => self.empty_address_machine_cycles,
            RuntimeIntroNoiseSamplePath::SoundRet => self.sound_ret_machine_cycles,
            RuntimeIntroNoiseSamplePath::Sample => self.sample_machine_cycles,
        };
        prefix + self.zero_delay_machine_cycles + sample
    }
}

impl RuntimeIntroPresentationParameters {
    pub fn from_program(program: &RuntimePresentationProgram) -> Result<Self> {
        let subprogram = program
            .subprograms
            .iter()
            .find(|subprogram| subprogram.id == "crystal_intro")
            .context("runtime presentation crystal_intro subprogram is missing")?;
        let phase = subprogram
            .phases
            .iter()
            .find(|phase| phase.id == "scene_dispatch")
            .context("runtime presentation crystal_intro scene_dispatch phase is missing")?;
        let dispatch = phase
            .operations
            .iter()
            .find(|operation| {
                operation.op == "dispatch_table"
                    && operation.fields.get("table").and_then(Value::as_str)
                        == Some("IntroScenes")
            })
            .context("runtime presentation crystal_intro dispatch_table operation is missing")?;
        let dispatch_contract = subprogram
            .loop_
            .get("scene_dispatch")
            .context("runtime presentation crystal_intro loop scene_dispatch is missing")?;
        let scene_labels = dispatch_contract
            .get("entries")
            .and_then(Value::as_array)
            .context("runtime presentation crystal_intro dispatch entries are missing")?
            .iter()
            .map(|entry| {
                entry
                    .as_str()
                    .map(str::to_string)
                    .context("runtime presentation crystal_intro dispatch entry is not a label")
            })
            .collect::<Result<Vec<_>>>()?;
        let operation_entries = dispatch
            .fields
            .get("entries")
            .and_then(Value::as_array)
            .context("runtime presentation crystal_intro operation dispatch entries are missing")?;
        let operation_labels = operation_entries
            .iter()
            .map(|entry| {
                entry.as_str().context(
                    "runtime presentation crystal_intro operation dispatch entry is not a label",
                )
            })
            .collect::<Result<Vec<_>>>()?;
        anyhow::ensure!(
            operation_labels == scene_labels.iter().map(String::as_str).collect::<Vec<_>>(),
            "runtime presentation crystal_intro loop and operation dispatch entries disagree"
        );
        anyhow::ensure!(
            !scene_labels.is_empty(),
            "runtime presentation crystal_intro dispatch is empty"
        );
        let entry_offsets = dispatch_contract
            .get("entry_offsets")
            .and_then(Value::as_object)
            .context("runtime presentation crystal_intro dispatch entry offsets are missing")?;
        let mut scene_offsets = Vec::with_capacity(scene_labels.len());
        let mut previous_offset = None;
        for label in &scene_labels {
            let offset = entry_offsets
                .get(label)
                .and_then(Value::as_u64)
                .and_then(|value| usize::try_from(value).ok())
                .with_context(|| {
                    format!("runtime presentation crystal_intro source label {label} is missing")
                })?;
            anyhow::ensure!(
                offset < phase.operations.len(),
                "runtime presentation crystal_intro source label {label} offset {offset} is out of range"
            );
            if let Some(previous) = previous_offset {
                anyhow::ensure!(
                    offset > previous,
                    "runtime presentation crystal_intro source labels are not in dispatch order"
                );
            } else {
                anyhow::ensure!(
                    offset == 0,
                    "runtime presentation crystal_intro first scene does not begin at operation zero"
                );
            }
            previous_offset = Some(offset);
            scene_offsets.push(offset);
        }
        for (label, offset) in scene_labels.iter().zip(&scene_offsets) {
            anyhow::ensure!(
                phase.labels.get(label) == Some(offset),
                "runtime presentation crystal_intro source label {label} does not resolve to its dispatch offset {offset}"
            );
        }
        anyhow::ensure!(
            entry_offsets.len() == scene_labels.len(),
            "runtime presentation crystal_intro has labels outside the source dispatch table"
        );
        for (dispatcher_entry, start) in scene_offsets.iter().enumerate() {
            let end = scene_offsets
                .get(dispatcher_entry + 1)
                .copied()
                .unwrap_or(phase.operations.len());
            for operation in &phase.operations[*start..end] {
                if let Some(target) = operation
                    .fields
                    .get("target")
                    .and_then(Value::as_str)
                    .filter(|target| {
                        target.starts_with('.') && !target.ends_with("@CrystalIntro")
                    })
                {
                    let target_offset = phase.labels.get(target).copied().with_context(|| {
                        format!(
                            "runtime presentation crystal_intro scene branch target {target} is missing"
                        )
                    })?;
                    anyhow::ensure!(
                        target_offset >= *start && target_offset < end,
                        "runtime presentation crystal_intro scene branch target {target} leaves dispatcher entry {dispatcher_entry}"
                    );
                }
                if operation.op == "play_audio" {
                    let operation_entry = operation
                        .fields
                        .get("dispatcher_entry")
                        .and_then(Value::as_u64)
                        .and_then(|value| usize::try_from(value).ok())
                        .context("runtime presentation crystal_intro audio operation has no dispatcher entry")?;
                    let operation_tick = operation
                        .fields
                        .get("dispatch_tick")
                        .and_then(Value::as_u64)
                        .context("runtime presentation crystal_intro audio operation has no dispatch tick")?;
                    let audio = operation
                        .fields
                        .get("audio")
                        .and_then(Value::as_str)
                        .filter(|audio| !audio.is_empty())
                        .context("runtime presentation crystal_intro audio operation has no audio id")?;
                    anyhow::ensure!(
                        operation_entry == dispatcher_entry && operation_tick > 0,
                        "runtime presentation crystal_intro audio operation disagrees with its scene operation range"
                    );
                    anyhow::ensure!(
                        subprogram.audio.iter().any(|candidate| candidate.id == audio)
                            || program.audio.iter().any(|candidate| candidate.id == audio),
                        "runtime presentation crystal_intro audio operation references missing audio {audio}"
                    );
                    continue;
                }
                if operation.op == "scheduled_audio" {
                    anyhow::ensure!(
                        operation.fields.get("clock").and_then(Value::as_str)
                            == Some("wIntroSceneFrameCounter")
                            && operation.fields.get("sentinel").and_then(Value::as_u64)
                                == Some(u64::from(u8::MAX)),
                        "runtime presentation crystal_intro scheduled audio has an invalid clock or sentinel"
                    );
                    let on_match = operation
                        .fields
                        .get("on_match")
                        .context("runtime presentation crystal_intro scheduled audio has no match behavior")?;
                    anyhow::ensure!(
                        on_match.get("play_entry").and_then(Value::as_bool) == Some(true)
                            && on_match
                                .get("stop_sfx_channels")
                                .and_then(Value::as_array)
                                .is_some_and(|channels| {
                                    channels
                                        .iter()
                                        .map(Value::as_u64)
                                        .eq([Some(5), Some(6), Some(7), Some(8)])
                                }),
                        "runtime presentation crystal_intro scheduled audio has invalid playback semantics"
                    );
                    let entries = operation
                        .fields
                        .get("entries")
                        .and_then(Value::as_array)
                        .context("runtime presentation crystal_intro scheduled audio has no entries")?;
                    anyhow::ensure!(
                        !entries.is_empty(),
                        "runtime presentation crystal_intro scheduled audio is empty"
                    );
                    let mut previous_frame = None;
                    for entry in entries {
                        let frame = entry
                            .get("frame")
                            .and_then(Value::as_u64)
                            .and_then(|value| u8::try_from(value).ok())
                            .context("runtime presentation crystal_intro scheduled audio frame is invalid")?;
                        let audio = entry
                            .get("audio")
                            .and_then(Value::as_str)
                            .filter(|audio| !audio.is_empty())
                            .context("runtime presentation crystal_intro scheduled audio id is invalid")?;
                        anyhow::ensure!(
                            frame < u8::MAX
                                && previous_frame.is_none_or(|previous| frame > previous),
                            "runtime presentation crystal_intro scheduled audio frames are not strictly ordered before the sentinel"
                        );
                        previous_frame = Some(frame);
                        anyhow::ensure!(
                            subprogram.audio.iter().any(|candidate| candidate.id == audio)
                                || program.audio.iter().any(|candidate| candidate.id == audio),
                            "runtime presentation crystal_intro scheduled audio references missing audio {audio}"
                        );
                    }
                    continue;
                }
                if !matches!(operation.op.as_str(), "sprite_init_group" | "sprite_activate") {
                    continue;
                }
                let operation_entry = operation
                    .fields
                    .get("dispatcher_entry")
                    .and_then(Value::as_u64)
                    .and_then(|value| usize::try_from(value).ok())
                    .context("runtime presentation crystal_intro sprite activation has no dispatcher entry")?;
                let operation_tick = operation
                    .fields
                    .get("dispatch_tick")
                    .and_then(Value::as_u64)
                    .context("runtime presentation crystal_intro sprite activation has no dispatch tick")?;
                anyhow::ensure!(
                    operation_entry == dispatcher_entry && operation_tick > 0,
                    "runtime presentation crystal_intro sprite activation disagrees with its scene operation range"
                );
                if operation.op == "sprite_activate" {
                    let lifetime = operation
                        .fields
                        .get("lifetime")
                        .context("runtime presentation crystal_intro sprite activation has no lifetime")?;
                    anyhow::ensure!(
                        lifetime
                            .get("allocation_dispatcher_entry")
                            .and_then(Value::as_u64)
                            == Some(operation_entry as u64)
                            && lifetime
                                .get("allocation_dispatch_tick")
                                .and_then(Value::as_u64)
                                == Some(operation_tick),
                        "runtime presentation crystal_intro sprite activation disagrees with its source lifetime"
                    );
                } else {
                    let instances = operation
                        .fields
                        .get("instances")
                        .and_then(Value::as_array)
                        .context("runtime presentation crystal_intro sprite group has no instances")?;
                    anyhow::ensure!(
                        !instances.is_empty(),
                        "runtime presentation crystal_intro sprite group is empty"
                    );
                    for instance in instances {
                        let suffix = instance
                            .as_str()
                            .and_then(|instance| {
                                instance.strip_prefix("sprite:engine/movie/intro.asm:")
                            })
                            .context("runtime presentation crystal_intro sprite group has a malformed instance")?;
                        let mut fields = suffix.split(':');
                        let source_line = fields.next();
                        let instance_tick = fields.next().and_then(|tick| tick.parse::<u64>().ok());
                        let callback_line = fields.next();
                        anyhow::ensure!(
                            source_line.is_some_and(|line| !line.is_empty())
                                && instance_tick == Some(operation_tick)
                                && callback_line.is_some_and(|line| !line.is_empty())
                                && fields.next().is_none(),
                            "runtime presentation crystal_intro sprite group instance disagrees with its dispatch tick"
                        );
                    }
                }
            }
        }
        let mut decompression_count = 0_usize;
        let mut decompression_request_count = 0_usize;
        for (operation_index, operation) in phase
            .operations
            .iter()
            .enumerate()
            .filter(|(_, operation)| operation.op == "decompress_lz3_resource")
        {
            decompression_count += 1;
            let frame_boundaries = operation
                .fields
                .get("decompress_frame_boundaries_crossed")
                .and_then(Value::as_u64)
                .and_then(|value| u16::try_from(value).ok())
                .context(
                    "runtime presentation crystal_intro decompression has no exact frame-boundary count",
                )?;
            let timing_oracle = operation
                .fields
                .get("timing_oracle")
                .and_then(Value::as_object)
                .context(
                    "runtime presentation crystal_intro decompression has no ROM timing oracle",
                )?;
            anyhow::ensure!(
                timing_oracle.get("runner").and_then(Value::as_str)
                    == Some("tools/asm-oracle/intro_trace.py --timing")
                    && timing_oracle.get("rom_sha1").and_then(Value::as_str)
                        == Some("f4cd194bdee0d04ca4eac29e09b8e4e9d818c133"),
                "runtime presentation crystal_intro decompression timing oracle is invalid"
            );
            let start_frame = timing_oracle
                .get("start_intro_frame")
                .and_then(Value::as_u64)
                .context(
                    "runtime presentation crystal_intro decompression oracle has no start frame",
                )?;
            let end_frame = timing_oracle
                .get("end_intro_frame")
                .and_then(Value::as_u64)
                .context(
                    "runtime presentation crystal_intro decompression oracle has no end frame",
                )?;
            anyhow::ensure!(
                end_frame.checked_sub(start_frame) == Some(u64::from(frame_boundaries)),
                "runtime presentation crystal_intro decompression frame count disagrees with its ROM oracle"
            );
            let elapsed_t_cycles = timing_oracle
                .get("elapsed_t_cycles_between_hooks")
                .and_then(Value::as_u64)
                .context(
                    "runtime presentation crystal_intro decompression oracle has no elapsed T-cycle count",
                )?;
            let start_phase_t_cycles = timing_oracle
                .get("start_frame_phase_t_cycles")
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
                .context(
                    "runtime presentation crystal_intro decompression oracle has no start phase",
                )?;
            let mut decompression_clock =
                RuntimeIntroFrameClock::new(70_224, start_phase_t_cycles)?;
            anyhow::ensure!(
                decompression_clock.advance_t_cycles(elapsed_t_cycles)?
                    == u64::from(frame_boundaries),
                "runtime presentation crystal_intro decompression frame count disagrees with its phase oracle"
            );
            let body_machine_cycles = operation
                .fields
                .get("decompress_machine_cycles")
                .and_then(Value::as_u64)
                .filter(|value| *value > 0)
                .context(
                    "runtime presentation crystal_intro decompression has no body cycle count",
                )?;
            anyhow::ensure!(
                elapsed_t_cycles >= body_machine_cycles * 4,
                "runtime presentation crystal_intro decompression oracle is shorter than its source body"
            );
            let request = phase
                .operations
                .get(operation_index + 1)
                .filter(|operation| operation.op == "request_2bpp_transfer")
                .context(
                    "runtime presentation crystal_intro decompression is not followed by its 2bpp request",
                )?;
            decompression_request_count += 1;
            let wrapper_frame_boundaries = request
                .fields
                .get("decompress_request_frame_boundaries_crossed")
                .and_then(Value::as_u64)
                .and_then(|value| u16::try_from(value).ok())
                .context(
                    "runtime presentation crystal_intro decompression request has no exact wrapper frame count",
                )?;
            let request_frame_boundaries = request
                .fields
                .get("request_2bpp_frame_boundaries_crossed")
                .and_then(Value::as_u64)
                .and_then(|value| u16::try_from(value).ok())
                .filter(|value| *value > 0)
                .context(
                    "runtime presentation crystal_intro decompression request has no exact frame count",
                )?;
            let tile_count = request
                .fields
                .get("tile_count")
                .and_then(Value::as_u64)
                .and_then(|value| u16::try_from(value).ok())
                .filter(|value| *value > 0)
                .context(
                    "runtime presentation crystal_intro decompression request has no positive tile count",
                )?;
            let tiles_per_vblank = request
                .fields
                .get("chunking")
                .and_then(Value::as_object)
                .and_then(|chunking| chunking.get("default_tiles_per_vblank"))
                .and_then(Value::as_u64)
                .and_then(|value| u16::try_from(value).ok())
                .filter(|value| *value > 0)
                .context(
                    "runtime presentation crystal_intro decompression request has no normal transfer rate",
                )?;
            anyhow::ensure!(
                request_frame_boundaries >= tile_count.div_ceil(tiles_per_vblank)
                    && frame_boundaries.checked_add(request_frame_boundaries)
                        == Some(wrapper_frame_boundaries),
                "runtime presentation crystal_intro decompression request wrapper frame count disagrees with its child calls"
            );
            let request_oracle = request
                .fields
                .get("request_2bpp_timing_oracle")
                .and_then(Value::as_object)
                .context(
                    "runtime presentation crystal_intro 2bpp request has no ROM timing oracle",
                )?;
            anyhow::ensure!(
                request_oracle.get("runner").and_then(Value::as_str)
                    == Some("tools/asm-oracle/intro_trace.py --timing")
                    && request_oracle.get("rom_sha1").and_then(Value::as_str)
                        == Some("f4cd194bdee0d04ca4eac29e09b8e4e9d818c133"),
                "runtime presentation crystal_intro 2bpp request timing oracle is invalid"
            );
            let request_start_frame = request_oracle
                .get("start_intro_frame")
                .and_then(Value::as_u64)
                .context(
                    "runtime presentation crystal_intro 2bpp request oracle has no start frame",
                )?;
            let request_end_frame = request_oracle
                .get("end_intro_frame")
                .and_then(Value::as_u64)
                .context(
                    "runtime presentation crystal_intro 2bpp request oracle has no end frame",
                )?;
            let request_elapsed_t_cycles = request_oracle
                .get("elapsed_t_cycles_between_hooks")
                .and_then(Value::as_u64)
                .context(
                    "runtime presentation crystal_intro 2bpp request oracle has no elapsed T-cycle count",
                )?;
            let request_start_phase = request_oracle
                .get("start_frame_phase_t_cycles")
                .and_then(Value::as_u64)
                .filter(|value| *value < 70_224)
                .context(
                    "runtime presentation crystal_intro 2bpp request oracle has no valid start phase",
                )?;
            anyhow::ensure!(
                request_end_frame.checked_sub(request_start_frame)
                    == Some(u64::from(request_frame_boundaries))
                    && request_start_phase
                        .checked_add(request_elapsed_t_cycles)
                        .map(|cycles| cycles / 70_224)
                        == Some(u64::from(request_frame_boundaries)),
                "runtime presentation crystal_intro 2bpp request frame count disagrees with its phase oracle"
            );
            let wrapper_oracle = request
                .fields
                .get("decompress_request_timing_oracle")
                .and_then(Value::as_object)
                .context(
                    "runtime presentation crystal_intro decompression request has no ROM timing oracle",
                )?;
            anyhow::ensure!(
                wrapper_oracle.get("runner").and_then(Value::as_str)
                    == Some("tools/asm-oracle/intro_trace.py --timing")
                    && wrapper_oracle.get("rom_sha1").and_then(Value::as_str)
                        == Some("f4cd194bdee0d04ca4eac29e09b8e4e9d818c133"),
                "runtime presentation crystal_intro decompression request timing oracle is invalid"
            );
            let wrapper_start_frame = wrapper_oracle
                .get("start_intro_frame")
                .and_then(Value::as_u64)
                .context(
                    "runtime presentation crystal_intro decompression request oracle has no start frame",
                )?;
            let wrapper_end_frame = wrapper_oracle
                .get("end_intro_frame")
                .and_then(Value::as_u64)
                .context(
                    "runtime presentation crystal_intro decompression request oracle has no end frame",
                )?;
            let wrapper_elapsed_t_cycles = wrapper_oracle
                .get("elapsed_t_cycles_between_hooks")
                .and_then(Value::as_u64)
                .context(
                    "runtime presentation crystal_intro decompression request oracle has no elapsed T-cycle count",
                )?;
            anyhow::ensure!(
                wrapper_end_frame.checked_sub(wrapper_start_frame)
                    == Some(u64::from(wrapper_frame_boundaries))
                    && wrapper_elapsed_t_cycles >= elapsed_t_cycles,
                "runtime presentation crystal_intro decompression request disagrees with its ROM oracle"
            );
        }
        anyhow::ensure!(
            decompression_count == decompression_request_count,
            "runtime presentation crystal_intro decompression request timing coverage is incomplete"
        );
        let completion_wait_frames = dispatch_contract
            .get("completion_wait_frames")
            .and_then(Value::as_array)
            .context("runtime presentation crystal_intro completion waits are missing")?
            .iter()
            .map(|frames| {
                frames
                    .as_u64()
                    .and_then(|value| u8::try_from(value).ok())
                    .context("runtime presentation crystal_intro completion wait exceeds one byte")
            })
            .collect::<Result<Vec<_>>>()?;
        anyhow::ensure!(
            completion_wait_frames.len() == scene_labels.len(),
            "runtime presentation crystal_intro completion wait count disagrees with its dispatch entries"
        );
        let central_wait_span: RuntimePresentationSourceSpan = serde_json::from_value(
            subprogram
                .loop_
                .get("frame_wait")
                .and_then(|wait| wait.get("source_span"))
                .cloned()
                .context("runtime presentation crystal_intro central frame wait is missing")?,
        )
        .context("runtime presentation crystal_intro central frame wait span is invalid")?;
        let derived_wait_frames = scene_offsets
            .iter()
            .enumerate()
            .map(|(index, start)| {
                let end = scene_offsets
                    .get(index + 1)
                    .copied()
                    .unwrap_or(phase.operations.len());
                phase.operations[*start..end]
                    .iter()
                    .filter(|operation| {
                        operation.op == "wait_frames"
                            && operation.source_span.file == "engine/movie/intro.asm"
                            && operation.source_span != central_wait_span
                    })
                    .try_fold(0_u8, |total, operation| {
                        let frames = operation
                            .fields
                            .get("frames")
                            .and_then(Value::as_u64)
                            .and_then(|value| u8::try_from(value).ok())
                            .context("runtime presentation crystal_intro source wait is invalid")?;
                        total.checked_add(frames).context(
                            "runtime presentation crystal_intro completion waits overflow one byte",
                        )
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        anyhow::ensure!(
            completion_wait_frames == derived_wait_frames,
            "runtime presentation crystal_intro completion waits disagree with source operations"
        );
        let scheduler = subprogram
            .loop_
            .get("scheduler")
            .context("runtime presentation crystal_intro sprite scheduler is missing")?;
        anyhow::ensure!(
            scheduler.get("op").and_then(Value::as_str) == Some("sprite_scheduler_step")
                && scheduler
                    .get("timing_oracle")
                    .and_then(|oracle| oracle.get("runner"))
                    .and_then(Value::as_str)
                    == Some("tools/asm-oracle/intro_trace.py --timing")
                && scheduler
                    .get("timing_oracle")
                    .and_then(|oracle| oracle.get("rom_sha1"))
                    .and_then(Value::as_str)
                    == Some("f4cd194bdee0d04ca4eac29e09b8e4e9d818c133"),
            "runtime presentation crystal_intro sprite scheduler timing oracle is invalid"
        );
        let raw_crossings = scheduler
            .get("rom_frame_crossings")
            .and_then(Value::as_array)
            .filter(|crossings| !crossings.is_empty())
            .context(
                "runtime presentation crystal_intro sprite scheduler frame crossings are missing",
            )?;
        let mut sprite_scheduler_frame_crossings = Vec::with_capacity(raw_crossings.len());
        let mut previous_crossing = None;
        for crossing in raw_crossings {
            let dispatcher_entry = crossing
                .get("dispatcher_entry")
                .and_then(Value::as_u64)
                .and_then(|value| usize::try_from(value).ok())
                .filter(|value| *value < scene_labels.len())
                .context(
                    "runtime presentation crystal_intro sprite crossing has an invalid dispatcher entry",
                )?;
            let dispatch_tick = crossing
                .get("dispatch_tick")
                .and_then(Value::as_u64)
                .and_then(|value| u16::try_from(value).ok())
                .filter(|value| *value > 0)
                .context(
                    "runtime presentation crystal_intro sprite crossing has an invalid dispatch tick",
                )?;
            let elapsed_t_cycles_between_hooks = crossing
                .get("elapsed_t_cycles_between_hooks")
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
                .filter(|value| *value > 0)
                .context(
                    "runtime presentation crystal_intro sprite crossing has no elapsed T-cycle count",
                )?;
            let key = (dispatcher_entry, dispatch_tick);
            anyhow::ensure!(
                previous_crossing.is_none_or(|previous| previous < key),
                "runtime presentation crystal_intro sprite crossings are not strictly ordered"
            );
            previous_crossing = Some(key);
            sprite_scheduler_frame_crossings.push(RuntimeIntroSpriteSchedulerFrameCrossing {
                dispatcher_entry,
                dispatch_tick,
                elapsed_t_cycles_between_hooks,
            });
        }
        let interrupt_timing = subprogram
            .loop_
            .get("interrupt_timing")
            .context("runtime presentation crystal_intro interrupt timing is missing")?;
        anyhow::ensure!(
            interrupt_timing.get("unit").and_then(Value::as_str)
                == Some("sm83_machine_cycles"),
            "runtime presentation crystal_intro interrupt timing unit is invalid"
        );
        let exact_u16 = |value: Option<&Value>, field: &str| -> Result<u16> {
            value
                .and_then(Value::as_u64)
                .and_then(|value| u16::try_from(value).ok())
                .filter(|value| *value > 0)
                .with_context(|| {
                    format!(
                        "runtime presentation crystal_intro interrupt timing {field} is invalid"
                    )
                })
        };
        let exact_u32 = |value: Option<&Value>, field: &str| -> Result<u32> {
            value
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
                .filter(|value| *value > 0)
                .with_context(|| {
                    format!(
                        "runtime presentation crystal_intro interrupt timing {field} is invalid"
                    )
                })
        };
        let exact_cycle_list = |value: Option<&Value>, field: &str| -> Result<Vec<u8>> {
            value
                .and_then(Value::as_array)
                .filter(|values| !values.is_empty())
                .with_context(|| {
                    format!(
                        "runtime presentation crystal_intro interrupt timing {field} is invalid"
                    )
                })?
                .iter()
                .map(|value| {
                    value
                        .as_u64()
                        .and_then(|value| u8::try_from(value).ok())
                        .filter(|value| *value > 0)
                        .with_context(|| {
                            format!(
                                "runtime presentation crystal_intro interrupt timing {field} contains an invalid instruction cost"
                            )
                        })
                })
                .collect()
        };
        let frame_clock = interrupt_timing
            .get("frame_clock")
            .context("runtime presentation crystal_intro frame clock is missing")?;
        let frame_t_cycles = exact_u32(
            frame_clock.get("frame_t_cycles"),
            "frame_clock.frame_t_cycles",
        )?;
        let intro_entry_phase_t_cycles = frame_clock
            .get("intro_entry_phase_t_cycles")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .context(
                "runtime presentation crystal_intro interrupt timing frame_clock.intro_entry_phase_t_cycles is invalid",
            )?;
        anyhow::ensure!(
            intro_entry_phase_t_cycles < frame_t_cycles
                && frame_clock
                    .pointer("/timing_oracle/runner")
                    .and_then(Value::as_str)
                    == Some("tools/asm-oracle/intro_trace.py --timing")
                && frame_clock
                    .pointer("/timing_oracle/rom_sha1")
                    .and_then(Value::as_str)
                    == Some("f4cd194bdee0d04ca4eac29e09b8e4e9d818c133"),
            "runtime presentation crystal_intro frame clock oracle is invalid"
        );
        let outer_loop_body = interrupt_timing
            .get("outer_loop_body")
            .context("runtime presentation crystal_intro outer-loop timing is missing")?;
        let joy_text_delay = interrupt_timing
            .get("joy_text_delay")
            .context("runtime presentation crystal_intro JoyTextDelay timing is missing")?;
        let vectors = interrupt_timing
            .get("vectors")
            .context("runtime presentation crystal_intro interrupt vectors are missing")?;
        anyhow::ensure!(
            vectors.get("instruction").and_then(Value::as_str) == Some("jp"),
            "runtime presentation crystal_intro interrupt vector instruction is invalid"
        );
        let lcd = interrupt_timing
            .get("lcd_stat")
            .context("runtime presentation crystal_intro LCD interrupt timing is missing")?;
        anyhow::ensure!(
            lcd.get("handler").and_then(Value::as_str) == Some("LCD")
                && lcd.get("trigger_register").and_then(Value::as_str) == Some("rSTAT")
                && lcd.get("trigger_mask").and_then(Value::as_str) == Some("STAT_MODE_0")
                && lcd.get("trigger").and_then(Value::as_str) == Some("hblank")
                && lcd.get("callback_pointer").and_then(Value::as_str)
                    == Some("hLCDCPointer"),
            "runtime presentation crystal_intro LCD interrupt semantics are invalid"
        );
        let timer = interrupt_timing
            .get("timer")
            .context("runtime presentation crystal_intro timer interrupt timing is missing")?;
        anyhow::ensure!(
            timer.get("handler").and_then(Value::as_str) == Some("MobileTimer")
                && timer.get("enable").and_then(Value::as_str) == Some("IE_TIMER")
                && timer.pointer("/inactive_guard/source").and_then(Value::as_str)
                    == Some("hMobile")
                && timer.pointer("/inactive_guard/predicate").and_then(Value::as_str)
                    == Some("zero"),
            "runtime presentation crystal_intro inactive timer semantics are invalid"
        );
        let vblank = interrupt_timing
            .get("vblank_normal")
            .context("runtime presentation crystal_intro VBlank timing is missing")?;
        anyhow::ensure!(
            vblank.get("handler").and_then(Value::as_str) == Some("VBlank_Normal")
                && vblank.pointer("/selector/source").and_then(Value::as_str)
                    == Some("hVBlank")
                && vblank.pointer("/selector/value").and_then(Value::as_u64) == Some(0)
                && vblank.pointer("/audio_update/routine").and_then(Value::as_str)
                    == Some("_UpdateSound")
                && vblank.pointer("/audio_update/cadence").and_then(Value::as_str)
                    == Some("every_vblank")
                && vblank.pointer("/audio_update/playing_guard/source").and_then(Value::as_str)
                    == Some("wMusicPlaying")
                && vblank.pointer("/audio_update/playing_guard/predicate").and_then(Value::as_str)
                    == Some("nonzero")
                && vblank.pointer("/audio_update/timing").and_then(Value::as_str)
                    == Some("state_dependent")
                && vblank
                    .pointer("/audio_update/all_channels_inactive/predicate")
                    .and_then(Value::as_str)
                    == Some("all_SOUND_CHANNEL_ON_flags_clear")
                && vblank
                    .pointer("/audio_update/helper_inactive_paths/apply_pitch_slide/guard")
                    .and_then(Value::as_str)
                    == Some("SOUND_PITCH_SLIDE_clear")
                && vblank
                    .pointer("/audio_update/helper_inactive_paths/handle_track_vibrato/guard")
                    .and_then(Value::as_str)
                    == Some("SOUND_DUTY_LOOP_SOUND_PITCH_OFFSET_SOUND_VIBRATO_clear")
                && vblank
                    .pointer("/audio_update/helper_inactive_paths/handle_noise/guard")
                    .and_then(Value::as_str)
                    == Some("SOUND_NOISE_clear")
                && vblank
                    .pointer("/audio_update/helper_inactive_paths/play_danger/guard")
                    .and_then(Value::as_str)
                    == Some("DANGER_ON_clear")
                && vblank
                    .pointer("/audio_update/helper_inactive_paths/fade_music/guard")
                    .and_then(Value::as_str)
                    == Some("wMusicFade_zero")
                && vblank
                    .pointer("/audio_update/active_channel_paths/guard/sfx_priority")
                    .and_then(Value::as_str)
                    == Some("zero")
                && vblank
                    .pointer(
                        "/audio_update/active_channel_paths/guard/sustained_note_duration",
                    )
                    .and_then(Value::as_str)
                    == Some("at_least_2"),
            "runtime presentation crystal_intro VBlank sound timing semantics are invalid"
        );
        let interrupt_timing = RuntimeIntroInterruptTiming {
            frame_t_cycles,
            intro_entry_phase_t_cycles,
            entry_to_first_input_machine_cycles: exact_u16(
                interrupt_timing.get("entry_to_first_input_machine_cycles"),
                "entry_to_first_input_machine_cycles",
            )?,
            joy_text_delay_pressed_repeat_reset_machine_cycles: exact_u16(
                joy_text_delay.get("pressed_repeat_reset_machine_cycles"),
                "joy_text_delay.pressed_repeat_reset_machine_cycles",
            )?,
            joy_text_delay_repeat_suppressed_machine_cycles: exact_u16(
                joy_text_delay.get("repeat_suppressed_machine_cycles"),
                "joy_text_delay.repeat_suppressed_machine_cycles",
            )?,
            joy_text_delay_repeat_restart_machine_cycles: exact_u16(
                joy_text_delay.get("repeat_restart_machine_cycles"),
                "joy_text_delay.repeat_restart_machine_cycles",
            )?,
            joy_text_delay_common_instruction_machine_cycles: exact_cycle_list(
                joy_text_delay.get("common_instruction_machine_cycles"),
                "joy_text_delay.common_instruction_machine_cycles",
            )?,
            joy_text_delay_pressed_repeat_reset_tail_machine_cycles: exact_cycle_list(
                joy_text_delay.get("pressed_repeat_reset_tail_machine_cycles"),
                "joy_text_delay.pressed_repeat_reset_tail_machine_cycles",
            )?,
            joy_text_delay_repeat_suppressed_tail_machine_cycles: exact_cycle_list(
                joy_text_delay.get("repeat_suppressed_tail_machine_cycles"),
                "joy_text_delay.repeat_suppressed_tail_machine_cycles",
            )?,
            joy_text_delay_repeat_restart_tail_machine_cycles: exact_cycle_list(
                joy_text_delay.get("repeat_restart_tail_machine_cycles"),
                "joy_text_delay.repeat_restart_tail_machine_cycles",
            )?,
            after_input_before_scene_dispatch_machine_cycles: exact_u16(
                outer_loop_body.get("after_input_before_scene_dispatch"),
                "outer_loop_body.after_input_before_scene_dispatch",
            )?,
            scene_dispatch_to_sprite_scheduler_machine_cycles: exact_u16(
                outer_loop_body.get("scene_dispatch_to_sprite_scheduler"),
                "outer_loop_body.scene_dispatch_to_sprite_scheduler",
            )?,
            sprite_scheduler_to_frame_wait_machine_cycles: exact_u16(
                outer_loop_body.get("sprite_scheduler_to_frame_wait"),
                "outer_loop_body.sprite_scheduler_to_frame_wait",
            )?,
            hardware_entry_machine_cycles: exact_u16(
                interrupt_timing.get("hardware_entry"),
                "hardware_entry",
            )?,
            vector_jump_machine_cycles: exact_u16(
                vectors.get("machine_cycles"),
                "vectors.machine_cycles",
            )?,
            lcd_interrupts_per_visible_frame: exact_u16(
                lcd.get("interrupts_per_visible_frame"),
                "lcd_stat.interrupts_per_visible_frame",
            )?,
            lcd_scanline_t_cycles: exact_u16(
                lcd.get("scanline_t_cycles"),
                "lcd_stat.scanline_t_cycles",
            )?,
            lcd_hblank_request_t_cycles: exact_u16(
                lcd.get("hblank_request_t_cycles"),
                "lcd_stat.hblank_request_t_cycles",
            )?,
            vblank_request_t_cycles: exact_u32(
                lcd.get("vblank_request_t_cycles"),
                "lcd_stat.vblank_request_t_cycles",
            )?,
            lcd_callback_zero_machine_cycles: exact_u16(
                lcd.get("callback_zero_machine_cycles"),
                "lcd_stat.callback_zero_machine_cycles",
            )?,
            lcd_callback_nonzero_machine_cycles: exact_u16(
                lcd.get("callback_nonzero_machine_cycles"),
                "lcd_stat.callback_nonzero_machine_cycles",
            )?,
            timer_request_period_t_cycles: exact_u32(
                timer.get("request_period_t_cycles"),
                "timer.request_period_t_cycles",
            )?,
            first_timer_request_after_intro_entry_t_cycles: exact_u32(
                timer.get("first_request_after_intro_entry_t_cycles"),
                "timer.first_request_after_intro_entry_t_cycles",
            )?,
            inactive_timer_machine_cycles: exact_u16(
                timer.get("inactive_machine_cycles"),
                "timer.inactive_machine_cycles",
            )?,
            inactive_game_timer_machine_cycles: exact_u16(
                vblank.get("inactive_game_timer_machine_cycles"),
                "vblank_normal.inactive_game_timer_machine_cycles",
            )?,
            vblank_wrapper_epilogue_machine_cycles: exact_u16(
                vblank.get("wrapper_epilogue_machine_cycles"),
                "vblank_normal.wrapper_epilogue_machine_cycles",
            )?,
            sound_update_is_state_dependent: true,
            inactive_channels_sound_update_machine_cycles: exact_u16(
                vblank.pointer("/audio_update/all_channels_inactive/machine_cycles"),
                "vblank_normal.audio_update.all_channels_inactive.machine_cycles",
            )?,
            inactive_pitch_slide_machine_cycles: exact_u16(
                vblank.pointer(
                    "/audio_update/helper_inactive_paths/apply_pitch_slide/machine_cycles",
                ),
                "vblank_normal.audio_update.helper_inactive_paths.apply_pitch_slide.machine_cycles",
            )?,
            inactive_track_vibrato_machine_cycles: exact_u16(
                vblank.pointer(
                    "/audio_update/helper_inactive_paths/handle_track_vibrato/machine_cycles",
                ),
                "vblank_normal.audio_update.helper_inactive_paths.handle_track_vibrato.machine_cycles",
            )?,
            inactive_noise_machine_cycles: exact_u16(
                vblank.pointer(
                    "/audio_update/helper_inactive_paths/handle_noise/machine_cycles",
                ),
                "vblank_normal.audio_update.helper_inactive_paths.handle_noise.machine_cycles",
            )?,
            inactive_danger_machine_cycles: exact_u16(
                vblank.pointer(
                    "/audio_update/helper_inactive_paths/play_danger/machine_cycles",
                ),
                "vblank_normal.audio_update.helper_inactive_paths.play_danger.machine_cycles",
            )?,
            inactive_music_fade_machine_cycles: exact_u16(
                vblank.pointer(
                    "/audio_update/helper_inactive_paths/fade_music/machine_cycles",
                ),
                "vblank_normal.audio_update.helper_inactive_paths.fade_music.machine_cycles",
            )?,
            active_music_channel_extra_machine_cycles: exact_u16(
                vblank.pointer(
                    "/audio_update/active_channel_paths/extra_over_inactive_channel_machine_cycles/music_without_active_sfx",
                ),
                "vblank_normal.audio_update.active_channel_paths.music_without_active_sfx",
            )?,
            active_sfx_channel_extra_machine_cycles: exact_u16(
                vblank.pointer(
                    "/audio_update/active_channel_paths/extra_over_inactive_channel_machine_cycles/sfx",
                ),
                "vblank_normal.audio_update.active_channel_paths.sfx",
            )?,
            shadowed_music_channel_extra_machine_cycles: exact_u16(
                vblank.pointer(
                    "/audio_update/active_channel_paths/extra_over_inactive_channel_machine_cycles/music_shadowed_by_active_sfx",
                ),
                "vblank_normal.audio_update.active_channel_paths.music_shadowed_by_active_sfx",
            )?,
            note_over_extra_before_parse_machine_cycles: exact_u16(
                vblank.pointer(
                    "/audio_update/active_channel_paths/note_over_extra_before_parse_machine_cycles",
                ),
                "vblank_normal.audio_update.active_channel_paths.note_over_extra_before_parse_machine_cycles",
            )?,
            track_vibrato: RuntimeIntroTrackVibratoTiming {
                base_machine_cycles: exact_u16(
                    vblank.pointer(
                        "/audio_update/helper_cycle_models/handle_track_vibrato/base_machine_cycles",
                    ),
                    "vblank_normal.audio_update.helper_cycle_models.handle_track_vibrato.base_machine_cycles",
                )?,
                duty_loop_extra_machine_cycles: exact_u16(
                    vblank.pointer(
                        "/audio_update/helper_cycle_models/handle_track_vibrato/duty_loop_extra_machine_cycles",
                    ),
                    "vblank_normal.audio_update.helper_cycle_models.handle_track_vibrato.duty_loop_extra_machine_cycles",
                )?,
                pitch_offset_extra_machine_cycles: exact_u16(
                    vblank.pointer(
                        "/audio_update/helper_cycle_models/handle_track_vibrato/pitch_offset_extra_machine_cycles",
                    ),
                    "vblank_normal.audio_update.helper_cycle_models.handle_track_vibrato.pitch_offset_extra_machine_cycles",
                )?,
                delay_count_nonzero_extra_machine_cycles: exact_u16(
                    vblank.pointer(
                        "/audio_update/helper_cycle_models/handle_track_vibrato/vibrato_extra_machine_cycles/delay_count_nonzero",
                    ),
                    "vblank_normal.audio_update.helper_cycle_models.handle_track_vibrato.delay_count_nonzero",
                )?,
                zero_extent_extra_machine_cycles: exact_u16(
                    vblank.pointer(
                        "/audio_update/helper_cycle_models/handle_track_vibrato/vibrato_extra_machine_cycles/zero_extent",
                    ),
                    "vblank_normal.audio_update.helper_cycle_models.handle_track_vibrato.zero_extent",
                )?,
                rate_count_nonzero_extra_machine_cycles: exact_u16(
                    vblank.pointer(
                        "/audio_update/helper_cycle_models/handle_track_vibrato/vibrato_extra_machine_cycles/rate_count_nonzero",
                    ),
                    "vblank_normal.audio_update.helper_cycle_models.handle_track_vibrato.rate_count_nonzero",
                )?,
                toggle_up_no_borrow_extra_machine_cycles: exact_u16(
                    vblank.pointer(
                        "/audio_update/helper_cycle_models/handle_track_vibrato/vibrato_extra_machine_cycles/toggle_up_no_borrow",
                    ),
                    "vblank_normal.audio_update.helper_cycle_models.handle_track_vibrato.toggle_up_no_borrow",
                )?,
                toggle_up_borrow_extra_machine_cycles: exact_u16(
                    vblank.pointer(
                        "/audio_update/helper_cycle_models/handle_track_vibrato/vibrato_extra_machine_cycles/toggle_up_borrow",
                    ),
                    "vblank_normal.audio_update.helper_cycle_models.handle_track_vibrato.toggle_up_borrow",
                )?,
                toggle_down_no_carry_extra_machine_cycles: exact_u16(
                    vblank.pointer(
                        "/audio_update/helper_cycle_models/handle_track_vibrato/vibrato_extra_machine_cycles/toggle_down_no_carry",
                    ),
                    "vblank_normal.audio_update.helper_cycle_models.handle_track_vibrato.toggle_down_no_carry",
                )?,
                toggle_down_carry_extra_machine_cycles: exact_u16(
                    vblank.pointer(
                        "/audio_update/helper_cycle_models/handle_track_vibrato/vibrato_extra_machine_cycles/toggle_down_carry",
                    ),
                    "vblank_normal.audio_update.helper_cycle_models.handle_track_vibrato.toggle_down_carry",
                )?,
            },
            update_channels: RuntimeIntroUpdateChannelsTiming {
                pulse1_unchanged_machine_cycles: exact_u16(
                    vblank.pointer("/audio_update/helper_cycle_models/update_channels_intro_paths/pulse1/unchanged"),
                    "vblank_normal.audio_update.helper_cycle_models.update_channels_intro_paths.pulse1.unchanged",
                )?,
                pulse1_noise_sampling_machine_cycles: exact_u16(
                    vblank.pointer("/audio_update/helper_cycle_models/update_channels_intro_paths/pulse1/noise_sampling"),
                    "vblank_normal.audio_update.helper_cycle_models.update_channels_intro_paths.pulse1.noise_sampling",
                )?,
                pulse2_unchanged_machine_cycles: exact_u16(
                    vblank.pointer("/audio_update/helper_cycle_models/update_channels_intro_paths/pulse2/unchanged"),
                    "vblank_normal.audio_update.helper_cycle_models.update_channels_intro_paths.pulse2.unchanged",
                )?,
                pulse2_vibrato_override_machine_cycles: exact_u16(
                    vblank.pointer("/audio_update/helper_cycle_models/update_channels_intro_paths/pulse2/vibrato_override"),
                    "vblank_normal.audio_update.helper_cycle_models.update_channels_intro_paths.pulse2.vibrato_override",
                )?,
                wave_unchanged_machine_cycles: exact_u16(
                    vblank.pointer("/audio_update/helper_cycle_models/update_channels_intro_paths/wave/unchanged"),
                    "vblank_normal.audio_update.helper_cycle_models.update_channels_intro_paths.wave.unchanged",
                )?,
                wave_noise_sampling_machine_cycles: exact_u16(
                    vblank.pointer("/audio_update/helper_cycle_models/update_channels_intro_paths/wave/noise_sampling"),
                    "vblank_normal.audio_update.helper_cycle_models.update_channels_intro_paths.wave.noise_sampling",
                )?,
                noise_unchanged_machine_cycles: exact_u16(
                    vblank.pointer("/audio_update/helper_cycle_models/update_channels_intro_paths/noise/unchanged"),
                    "vblank_normal.audio_update.helper_cycle_models.update_channels_intro_paths.noise.unchanged",
                )?,
                noise_noise_sampling_machine_cycles: exact_u16(
                    vblank.pointer("/audio_update/helper_cycle_models/update_channels_intro_paths/noise/noise_sampling"),
                    "vblank_normal.audio_update.helper_cycle_models.update_channels_intro_paths.noise.noise_sampling",
                )?,
            },
            parse_music: RuntimeIntroParseMusicTiming {
                normal_note_base_machine_cycles: exact_u16(
                    vblank.pointer("/audio_update/helper_cycle_models/parse_music_intro_paths/normal_note_base_machine_cycles"),
                    "vblank_normal.audio_update.helper_cycle_models.parse_music_intro_paths.normal_note_base_machine_cycles",
                )?,
                music_noise_note_base_machine_cycles: exact_u16(
                    vblank.pointer("/audio_update/helper_cycle_models/parse_music_intro_paths/music_noise_note_base_machine_cycles"),
                    "vblank_normal.audio_update.helper_cycle_models.parse_music_intro_paths.music_noise_note_base_machine_cycles",
                )?,
                octave_command_machine_cycles: exact_u16(
                    vblank.pointer("/audio_update/helper_cycle_models/parse_music_intro_paths/octave_command_machine_cycles"),
                    "vblank_normal.audio_update.helper_cycle_models.parse_music_intro_paths.octave_command_machine_cycles",
                )?,
                set_note_duration: RuntimeIntroSetNoteDurationTiming {
                    fixed_machine_cycles: exact_u16(
                        vblank.pointer("/audio_update/helper_cycle_models/parse_music_intro_paths/set_note_duration/fixed_machine_cycles"),
                        "vblank_normal.audio_update.helper_cycle_models.parse_music_intro_paths.set_note_duration.fixed_machine_cycles",
                    )?,
                    multiply_per_bit_machine_cycles: exact_u16(
                        vblank.pointer("/audio_update/helper_cycle_models/parse_music_intro_paths/set_note_duration/multiply_per_bit_machine_cycles"),
                        "vblank_normal.audio_update.helper_cycle_models.parse_music_intro_paths.set_note_duration.multiply_per_bit_machine_cycles",
                    )?,
                    multiply_fixed_machine_cycles: exact_u16(
                        vblank.pointer("/audio_update/helper_cycle_models/parse_music_intro_paths/set_note_duration/multiply_fixed_machine_cycles"),
                        "vblank_normal.audio_update.helper_cycle_models.parse_music_intro_paths.set_note_duration.multiply_fixed_machine_cycles",
                    )?,
                    multiply_set_bit_extra_machine_cycles: exact_u16(
                        vblank.pointer("/audio_update/helper_cycle_models/parse_music_intro_paths/set_note_duration/multiply_set_bit_extra_machine_cycles"),
                        "vblank_normal.audio_update.helper_cycle_models.parse_music_intro_paths.set_note_duration.multiply_set_bit_extra_machine_cycles",
                    )?,
                    minimum_multiply_iterations: u8::try_from(exact_u16(
                        vblank.pointer("/audio_update/helper_cycle_models/parse_music_intro_paths/set_note_duration/minimum_multiply_iterations"),
                        "vblank_normal.audio_update.helper_cycle_models.parse_music_intro_paths.set_note_duration.minimum_multiply_iterations",
                    )?)?,
                },
                get_frequency: RuntimeIntroGetFrequencyTiming {
                    fixed_machine_cycles: exact_u16(
                        vblank.pointer("/audio_update/helper_cycle_models/parse_music_intro_paths/get_frequency/fixed_machine_cycles"),
                        "vblank_normal.audio_update.helper_cycle_models.parse_music_intro_paths.get_frequency.fixed_machine_cycles",
                    )?,
                    per_right_shift_machine_cycles: exact_u16(
                        vblank.pointer("/audio_update/helper_cycle_models/parse_music_intro_paths/get_frequency/per_right_shift_machine_cycles"),
                        "vblank_normal.audio_update.helper_cycle_models.parse_music_intro_paths.get_frequency.per_right_shift_machine_cycles",
                    )?,
                    target_octave: u8::try_from(exact_u16(
                        vblank.pointer("/audio_update/helper_cycle_models/parse_music_intro_paths/get_frequency/target_octave"),
                        "vblank_normal.audio_update.helper_cycle_models.parse_music_intro_paths.get_frequency.target_octave",
                    )?)?,
                },
            },
            noise: RuntimeIntroNoiseTiming {
                inactive_machine_cycles: exact_u16(
                    vblank.pointer(
                        "/audio_update/helper_cycle_models/handle_noise/inactive_machine_cycles",
                    ),
                    "vblank_normal.audio_update.helper_cycle_models.handle_noise.inactive_machine_cycles",
                )?,
                sfx_prefix_machine_cycles: exact_u16(
                    vblank.pointer(
                        "/audio_update/helper_cycle_models/handle_noise/prefix_to_delay_check_machine_cycles/sfx_channel",
                    ),
                    "vblank_normal.audio_update.helper_cycle_models.handle_noise.sfx_channel",
                )?,
                music_ch8_off_prefix_machine_cycles: exact_u16(
                    vblank.pointer(
                        "/audio_update/helper_cycle_models/handle_noise/prefix_to_delay_check_machine_cycles/music_channel_with_ch8_off",
                    ),
                    "vblank_normal.audio_update.helper_cycle_models.handle_noise.music_channel_with_ch8_off",
                )?,
                music_ch8_non_noise_prefix_machine_cycles: exact_u16(
                    vblank.pointer(
                        "/audio_update/helper_cycle_models/handle_noise/prefix_to_delay_check_machine_cycles/music_channel_with_non_noise_ch8",
                    ),
                    "vblank_normal.audio_update.helper_cycle_models.handle_noise.music_channel_with_non_noise_ch8",
                )?,
                music_blocked_by_noise_ch8_machine_cycles: exact_u16(
                    vblank.pointer(
                        "/audio_update/helper_cycle_models/handle_noise/music_blocked_by_noise_ch8_machine_cycles",
                    ),
                    "vblank_normal.audio_update.helper_cycle_models.handle_noise.music_blocked_by_noise_ch8_machine_cycles",
                )?,
                nonzero_delay_machine_cycles: exact_u16(
                    vblank.pointer(
                        "/audio_update/helper_cycle_models/handle_noise/delay_machine_cycles/nonzero_return",
                    ),
                    "vblank_normal.audio_update.helper_cycle_models.handle_noise.nonzero_return",
                )?,
                zero_delay_machine_cycles: exact_u16(
                    vblank.pointer(
                        "/audio_update/helper_cycle_models/handle_noise/delay_machine_cycles/zero_to_sample_reader",
                    ),
                    "vblank_normal.audio_update.helper_cycle_models.handle_noise.zero_to_sample_reader",
                )?,
                empty_address_machine_cycles: exact_u16(
                    vblank.pointer(
                        "/audio_update/helper_cycle_models/handle_noise/sample_reader_machine_cycles/empty_address",
                    ),
                    "vblank_normal.audio_update.helper_cycle_models.handle_noise.empty_address",
                )?,
                sound_ret_machine_cycles: exact_u16(
                    vblank.pointer(
                        "/audio_update/helper_cycle_models/handle_noise/sample_reader_machine_cycles/sound_ret",
                    ),
                    "vblank_normal.audio_update.helper_cycle_models.handle_noise.sound_ret",
                )?,
                sample_machine_cycles: exact_u16(
                    vblank.pointer(
                        "/audio_update/helper_cycle_models/handle_noise/sample_reader_machine_cycles/sample",
                    ),
                    "vblank_normal.audio_update.helper_cycle_models.handle_noise.sample",
                )?,
            },
        };
        anyhow::ensure!(
            u32::from(interrupt_timing.lcd_scanline_t_cycles) * 154
                == interrupt_timing.frame_t_cycles
                && interrupt_timing.lcd_hblank_request_t_cycles
                    < interrupt_timing.lcd_scanline_t_cycles
                && u32::from(interrupt_timing.lcd_scanline_t_cycles)
                    * u32::from(interrupt_timing.lcd_interrupts_per_visible_frame)
                    == interrupt_timing.vblank_request_t_cycles
                && interrupt_timing.timer_request_period_t_cycles > 0
                && interrupt_timing.first_timer_request_after_intro_entry_t_cycles
                    < interrupt_timing.timer_request_period_t_cycles,
            "runtime presentation crystal_intro interrupt event calendar is invalid"
        );
        for (joy_pressed, delay, expected) in [
            (
                true,
                0,
                interrupt_timing.joy_text_delay_pressed_repeat_reset_machine_cycles,
            ),
            (
                false,
                1,
                interrupt_timing.joy_text_delay_repeat_suppressed_machine_cycles,
            ),
            (
                false,
                0,
                interrupt_timing.joy_text_delay_repeat_restart_machine_cycles,
            ),
        ] {
            anyhow::ensure!(
                interrupt_timing
                    .joy_text_delay_instruction_machine_cycles(joy_pressed, delay)
                    .sum::<u64>()
                    == u64::from(expected),
                "runtime presentation crystal_intro JoyTextDelay instruction timing does not compose"
            );
        }
        Ok(Self {
            scene_labels,
            scene_operation_offsets: scene_offsets,
            completion_wait_frames,
            sprite_scheduler_frame_crossings,
            interrupt_timing,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTitlePresentationParameters {
    pub entrance_start_scx: u8,
    pub entrance_scroll_step: u8,
    pub timeout_frames: u16,
    pub timeout_fade_frames: u16,
    pub timeout_fade_rate: u8,
    pub timeout_fade_register: String,
    pub timeout_fade_audio: String,
    pub crystal_oam_target: String,
    pub crystal_initial_y: u8,
    pub suicune_iterator_operation_index: usize,
    pub teardown_start_operation_index: usize,
    pub teardown_dispatch_operation_index: usize,
    pub suicune_frames: Vec<u8>,
    pub suicune_selector_mask: u8,
    pub suicune_selector_shift_left: u8,
    pub suicune_selector_swap_nibbles: bool,
    pub delete_save_mask: u8,
    pub clock_reset_arm_mask: u8,
    pub clock_reset_finish_mask: u8,
    pub start_mask: u8,
}

impl RuntimeTitlePresentationParameters {
    pub fn from_program(program: &RuntimePresentationProgram) -> Result<Self> {
        let title_phase = program
            .subprograms
            .iter()
            .find(|subprogram| subprogram.id == "start_title_screen")
            .and_then(|subprogram| {
                subprogram
                    .phases
                    .iter()
                    .find(|phase| phase.id == "title_screen")
            })
            .context("runtime presentation start_title_screen title_screen phase is missing")?;
        let numeric_field = |operation: &RuntimePresentationOperation, field: &str| {
            operation.fields.get(field).and_then(Value::as_u64)
        };
        let entrance_start_scx = title_phase
            .operations
            .iter()
            .find(|operation| {
                operation.op == "write_memory_byte"
                    && operation.fields.get("target").and_then(Value::as_str) == Some("hSCX")
            })
            .and_then(|operation| numeric_field(operation, "value"))
            .and_then(|value| u8::try_from(value).ok())
            .context("runtime presentation title phase has no exact initial hSCX write")?;
        let entrance_scroll_step = title_phase
            .operations
            .iter()
            .find(|operation| {
                operation.op == "subtract_memory_byte"
                    && operation.fields.get("target").and_then(Value::as_str) == Some("hSCX")
            })
            .and_then(|operation| numeric_field(operation, "delta"))
            .and_then(|value| u8::try_from(value).ok())
            .filter(|value| *value > 0)
            .context("runtime presentation title phase has no exact hSCX decrement")?;
        let timeout_frames = title_phase
            .operations
            .iter()
            .find(|operation| {
                operation.op == "write_memory_word"
                    && operation.fields.get("target").and_then(Value::as_str)
                        == Some("wTitleScreenTimer")
            })
            .and_then(|operation| numeric_field(operation, "value"))
            .and_then(|value| u16::try_from(value).ok())
            .filter(|value| *value > 0)
            .context("runtime presentation title phase has no exact title timer write")?;
        let input_mask_for_target = |target: &str| {
            title_phase
                .operations
                .iter()
                .find(|operation| {
                    operation.op == "input_chord_branch"
                        && operation.fields.get("target").and_then(Value::as_str) == Some(target)
                })
                .and_then(|operation| numeric_field(operation, "mask"))
                .and_then(|value| u8::try_from(value).ok())
                .filter(|value| *value > 0)
                .with_context(|| {
                    format!("runtime presentation title phase has no input mask for {target}")
                })
        };
        let timeout_fade = title_phase
            .operations
            .iter()
            .find(|operation| operation.op == "fade_audio")
            .context("runtime presentation title phase has no exact timeout audio fade")?;
        let timeout_fade_frames = numeric_field(timeout_fade, "frames")
            .and_then(|value| u16::try_from(value).ok())
            .filter(|value| *value > 0)
            .context("runtime presentation title phase has no exact timeout audio fade")?;
        let timeout_fade_register = timeout_fade
            .fields
            .get("fade_register")
            .and_then(Value::as_object)
            .and_then(|register| register.get("target"))
            .and_then(Value::as_str)
            .filter(|target| !target.is_empty())
            .context("runtime presentation title fade has no exact WRAM register")?
            .to_string();
        let timeout_fade_rate = timeout_fade
            .fields
            .get("fade_register")
            .and_then(Value::as_object)
            .and_then(|register| register.get("value"))
            .and_then(Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
            .filter(|value| *value > 0 && *value <= 0x3f)
            .context("runtime presentation title fade has no exact source rate byte")?;
        let timeout_fade_audio = timeout_fade
            .fields
            .get("audio")
            .and_then(Value::as_str)
            .filter(|audio| !audio.is_empty())
            .context("runtime presentation title fade has no exact audio target")?
            .to_string();
        anyhow::ensure!(
            timeout_fade_frames == u16::from(timeout_fade_rate) * 8,
            "runtime presentation title fade duration {timeout_fade_frames} does not match source rate {timeout_fade_rate} across eight volume boundaries"
        );
        let crystal_oam = title_phase
            .operations
            .iter()
            .find(|operation| operation.op == "initialize_title_crystal_oam")
            .context("runtime presentation title phase has no exact crystal OAM initialization")?;
        let _crystal_oam_base = crystal_oam
            .fields
            .get("target")
            .and_then(Value::as_str)
            .filter(|target| !target.is_empty())
            .context("runtime presentation title crystal OAM initialization has no target")?;
        let crystal_initial_y = crystal_oam
            .fields
            .get("initial_y")
            .and_then(Value::as_i64)
            .filter(|value| (-128..=255).contains(value))
            .map(|value| value.rem_euclid(256) as u8)
            .context("runtime presentation title crystal OAM initialization has no byte Y")?;
        let crystal_animation = title_phase
            .operations
            .iter()
            .find(|operation| operation.op == "animate_title_crystal")
            .context("runtime presentation title phase has no exact crystal OAM animation")?;
        let crystal_oam_target = crystal_animation
            .fields
            .get("target")
            .and_then(Value::as_str)
            .filter(|target| !target.is_empty())
            .context("runtime presentation title crystal OAM animation has no target")?
            .to_string();
        anyhow::ensure!(
            crystal_animation
                .fields
                .get("stop_at")
                .and_then(Value::as_u64)
                .and_then(|value| u8::try_from(value).ok())
                .is_some()
                && crystal_animation
                    .fields
                    .get("y_delta")
                    .and_then(Value::as_u64)
                    .and_then(|value| u8::try_from(value).ok())
                    .is_some_and(|value| value > 0),
            "runtime presentation title crystal OAM animation has invalid bounds"
        );
        let suicune_iterator_operation_index = title_phase
            .operations
            .iter()
            .position(|operation| {
                operation.op == "postincrement_memory_byte"
                    && operation.fields.get("target").and_then(Value::as_str)
                        == Some("wSuicuneFrame")
                    && operation.fields.get("result").and_then(Value::as_str)
                        == Some("title_suicune_frame")
            })
            .context("runtime presentation title phase has no executable Suicune iterator")?;
        let title_end_index = *title_phase
            .labels
            .get("TitleScreenEnd")
            .context("runtime presentation title phase has no TitleScreenEnd label")?;
        let teardown_start_operation_index = title_phase
            .operations
            .iter()
            .enumerate()
            .skip(title_end_index)
            .find_map(|(index, operation)| {
                (operation.op == "fill_memory"
                    && operation.fields.get("target").and_then(Value::as_str)
                        == Some("wShadowOAM"))
                .then_some(index)
            })
            .context("runtime presentation title phase has no teardown OAM clear")?;
        let teardown_dispatch_operation_index = title_phase
            .operations
            .iter()
            .enumerate()
            .skip(teardown_start_operation_index)
            .find_map(|(index, operation)| {
                (operation.op == "dispatch_table"
                    && operation.fields.get("dispatcher").and_then(Value::as_str)
                        == Some("StartTitleScreen option tail"))
                .then_some(index)
            })
            .context("runtime presentation title phase has no option-tail dispatch")?;
        let suicune_animation = title_phase
            .operations
            .iter()
            .find(|operation| operation.op == "draw_indexed_title_suicune_frame")
            .context("runtime presentation title phase has no indexed Suicune animation")?;
        let suicune_frames = suicune_animation
            .fields
            .get("frames")
            .and_then(Value::as_array)
            .filter(|frames| !frames.is_empty())
            .context("runtime presentation title Suicune animation has no frames")?
            .iter()
            .map(|value| {
                value
                    .as_u64()
                    .and_then(|value| u8::try_from(value).ok())
                    .context("runtime presentation title Suicune frame exceeds one byte")
            })
            .collect::<Result<Vec<_>>>()?;
        let suicune_selector = suicune_animation
            .fields
            .get("selector")
            .and_then(Value::as_object)
            .context("runtime presentation title Suicune animation has no selector")?;
        let suicune_selector_mask = suicune_selector
            .get("mask")
            .and_then(Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
            .context("runtime presentation title Suicune selector has no byte mask")?;
        let suicune_selector_shift_left = suicune_selector
            .get("shift_left")
            .and_then(Value::as_u64)
            .and_then(|value| u8::try_from(value).ok())
            .filter(|value| *value < 8)
            .context("runtime presentation title Suicune selector has invalid shift")?;
        let suicune_selector_swap_nibbles = suicune_selector
            .get("swap_nibbles")
            .and_then(Value::as_bool)
            .context("runtime presentation title Suicune selector has no swap flag")?;
        for counter in 0_u8..=u8::MAX {
            let mut selector = (counter & suicune_selector_mask)
                .wrapping_shl(u32::from(suicune_selector_shift_left));
            if suicune_selector_swap_nibbles {
                selector = selector.rotate_left(4);
            }
            anyhow::ensure!(
                usize::from(selector) < suicune_frames.len(),
                "runtime presentation title Suicune selector produces missing frame {selector}"
            );
        }
        anyhow::ensure!(
            entrance_start_scx % entrance_scroll_step == 0,
            "runtime presentation title entrance SCX {entrance_start_scx} is not divisible by its scroll step {entrance_scroll_step}"
        );
        Ok(Self {
            entrance_start_scx,
            entrance_scroll_step,
            timeout_frames,
            timeout_fade_frames,
            timeout_fade_rate,
            timeout_fade_register,
            timeout_fade_audio,
            crystal_oam_target,
            crystal_initial_y,
            suicune_iterator_operation_index,
            teardown_start_operation_index,
            teardown_dispatch_operation_index,
            suicune_frames,
            suicune_selector_mask,
            suicune_selector_shift_left,
            suicune_selector_swap_nibbles,
            delete_save_mask: input_mask_for_target(".delete_save_data@TitleScreenMain")?,
            clock_reset_arm_mask: input_mask_for_target(".check_start@TitleScreenMain")?,
            clock_reset_finish_mask: input_mask_for_target(".reset_clock@TitleScreenMain")?,
            start_mask: input_mask_for_target(".incave@TitleScreenMain")?,
        })
    }
}

impl RuntimePresentationSubprogramInterpreter {
    pub fn new(
        program: &RuntimePresentationProgram,
        subprogram_id: &str,
        phase_id: &str,
    ) -> Result<Self> {
        let subprogram = program
            .subprograms
            .iter()
            .find(|candidate| candidate.id == subprogram_id)
            .with_context(|| {
                format!("runtime presentation subprogram {subprogram_id} is missing")
            })?;
        anyhow::ensure!(
            subprogram.phases.iter().any(|phase| phase.id == phase_id),
            "runtime presentation subprogram {subprogram_id} phase {phase_id} is missing"
        );
        Ok(Self {
            subprogram: subprogram_id.to_string(),
            phase: phase_id.to_string(),
            operation_index: 0,
            current_label: None,
        })
    }

    pub fn jump_to_label(
        &mut self,
        program: &RuntimePresentationProgram,
        target: &str,
    ) -> Result<()> {
        let phase = program
            .subprograms
            .iter()
            .find(|candidate| candidate.id == self.subprogram)
            .and_then(|subprogram| {
                subprogram
                    .phases
                    .iter()
                    .find(|phase| phase.id == self.phase)
            })
            .with_context(|| {
                format!(
                    "runtime presentation subprogram {} phase {} is missing",
                    self.subprogram, self.phase
                )
            })?;
        let operation_index = phase.labels.get(target).copied().with_context(|| {
            format!(
                "runtime presentation subprogram {} phase {} label {target} is missing",
                self.subprogram, self.phase
            )
        })?;
        self.operation_index = operation_index;
        self.current_label = Some(target.to_string());
        Ok(())
    }

    pub fn step(
        &mut self,
        program: &RuntimePresentationProgram,
    ) -> Result<Option<RuntimePresentationOperation>> {
        let phase = program
            .subprograms
            .iter()
            .find(|candidate| candidate.id == self.subprogram)
            .and_then(|subprogram| {
                subprogram
                    .phases
                    .iter()
                    .find(|phase| phase.id == self.phase)
            })
            .with_context(|| {
                format!(
                    "runtime presentation subprogram {} phase {} is missing",
                    self.subprogram, self.phase
                )
            })?;
        let operation = phase.operations.get(self.operation_index).cloned();
        if operation.is_some() {
            self.operation_index += 1;
        }
        Ok(operation)
    }
}

impl RuntimePresentationTimedPhaseCursor {
    pub fn new(
        program: &RuntimePresentationProgram,
        subprogram_id: &str,
        phase_id: &str,
        operation_index: usize,
        end_operation_index: usize,
    ) -> Result<Self> {
        let phase = program
            .subprograms
            .iter()
            .find(|candidate| candidate.id == subprogram_id)
            .and_then(|subprogram| {
                subprogram
                    .phases
                    .iter()
                    .find(|phase| phase.id == phase_id)
            })
            .with_context(|| {
                format!(
                    "runtime presentation subprogram {subprogram_id} phase {phase_id} is missing"
                )
            })?;
        anyhow::ensure!(
            operation_index <= end_operation_index && end_operation_index < phase.operations.len(),
            "runtime presentation timed range {operation_index}..={end_operation_index} is outside subprogram {subprogram_id} phase {phase_id}"
        );
        Ok(Self {
            subprogram: subprogram_id.to_string(),
            phase: phase_id.to_string(),
            operation_index,
            end_operation_index,
            wait_frames_remaining: 0,
            transfer_mode: None,
            frame_t_cycles: None,
        })
    }

    pub fn with_2bpp_transfer_mode(
        mut self,
        transfer_mode: RuntimePresentation2bppTransferMode,
    ) -> Self {
        self.transfer_mode = Some(transfer_mode);
        self
    }

    pub fn with_frame_t_cycles(mut self, frame_t_cycles: u32) -> Result<Self> {
        anyhow::ensure!(frame_t_cycles > 0, "runtime presentation frame clock is zero");
        self.frame_t_cycles = Some(frame_t_cycles);
        Ok(self)
    }

    pub fn tick(
        &mut self,
        program: &RuntimePresentationProgram,
    ) -> Result<RuntimePresentationTimedPhaseTick> {
        let phase = program
            .subprograms
            .iter()
            .find(|candidate| candidate.id == self.subprogram)
            .and_then(|subprogram| {
                subprogram
                    .phases
                    .iter()
                    .find(|phase| phase.id == self.phase)
            })
            .with_context(|| {
                format!(
                    "runtime presentation subprogram {} phase {} is missing",
                    self.subprogram, self.phase
                )
            })?;
        if self.wait_frames_remaining > 0 {
            self.wait_frames_remaining -= 1;
            if self.wait_frames_remaining > 0 {
                return Ok(RuntimePresentationTimedPhaseTick {
                    effects: Vec::new(),
                    cpu_work_machine_cycles: 0,
                    complete: false,
                });
            }
        }
        let mut effects = Vec::new();
        let mut cpu_work_machine_cycles = 0_u64;
        for _ in 0..1_024 {
            if self.operation_index > self.end_operation_index {
                return Ok(RuntimePresentationTimedPhaseTick {
                    effects,
                    cpu_work_machine_cycles,
                    complete: true,
                });
            }
            let operation = phase
                .operations
                .get(self.operation_index)
                .cloned()
                .context("runtime presentation timed cursor reached a missing operation")?;
            self.operation_index += 1;
            if operation.op == "decompress_lz3_resource" {
                let machine_cycles = operation
                    .fields
                    .get("decompress_machine_cycles")
                    .and_then(Value::as_u64)
                    .filter(|value| *value > 0)
                    .context(
                        "runtime presentation decompression has no positive exact machine-cycle cost",
                    )?;
                cpu_work_machine_cycles = cpu_work_machine_cycles
                    .checked_add(machine_cycles)
                    .context("runtime presentation CPU work cycle count overflowed")?;
                let frame_boundaries_crossed = operation
                    .fields
                    .get("decompress_frame_boundaries_crossed")
                    .and_then(Value::as_u64)
                    .and_then(|value| u16::try_from(value).ok())
                    .context(
                        "runtime presentation decompression has no exact frame-boundary count",
                    )?;
                if let Some(oracle) = operation
                    .fields
                    .get("timing_oracle")
                    .and_then(Value::as_object)
                {
                    let start_phase = oracle
                        .get("start_frame_phase_t_cycles")
                        .and_then(Value::as_u64)
                        .and_then(|value| u32::try_from(value).ok())
                        .context(
                            "runtime presentation exact decompression has no start phase",
                        )?;
                    let elapsed_t_cycles = oracle
                        .get("elapsed_t_cycles_between_hooks")
                        .and_then(Value::as_u64)
                        .context(
                            "runtime presentation exact decompression has no elapsed T-cycle count",
                        )?;
                    let mut frame_clock = RuntimeIntroFrameClock::new(
                        self.frame_t_cycles.context(
                            "runtime presentation exact decompression requires a frame clock",
                        )?,
                        start_phase,
                    )?;
                    anyhow::ensure!(
                        frame_clock.advance_t_cycles(elapsed_t_cycles)?
                            == u64::from(frame_boundaries_crossed),
                        "runtime presentation exact decompression disagrees with its frame clock"
                    );
                }
                effects.push(operation);
                if frame_boundaries_crossed > 0 {
                    self.wait_frames_remaining = frame_boundaries_crossed;
                    return Ok(RuntimePresentationTimedPhaseTick {
                        effects,
                        cpu_work_machine_cycles,
                        complete: false,
                    });
                }
                continue;
            }
            if operation.op == "wait_frames" {
                let frames = operation
                    .fields
                    .get("frames")
                    .and_then(Value::as_u64)
                    .and_then(|frames| u16::try_from(frames).ok())
                    .filter(|frames| *frames > 0)
                    .context("runtime presentation wait_frames has no positive exact duration")?;
                self.wait_frames_remaining = frames;
                effects.push(operation);
                return Ok(RuntimePresentationTimedPhaseTick {
                    effects,
                    cpu_work_machine_cycles,
                    complete: false,
                });
            }
            if operation.op == "request_2bpp_transfer" {
                let completion = operation
                    .fields
                    .get("completion")
                    .and_then(Value::as_object)
                    .context("runtime presentation 2bpp request has no completion contract")?;
                anyhow::ensure!(
                    completion.get("blocking").and_then(Value::as_bool) == Some(true)
                        && completion.get("wait").and_then(Value::as_str)
                            == Some("DelayFrame")
                        && completion.get("until").and_then(Value::as_str)
                            == Some("wRequested2bppSize == 0"),
                    "runtime presentation 2bpp request has an unsupported completion contract"
                );
                let chunking = operation
                    .fields
                    .get("chunking")
                    .and_then(Value::as_object)
                    .context("runtime presentation 2bpp request has no chunking contract")?;
                let chunk_field = match self.transfer_mode.context(
                    "runtime presentation blocking 2bpp request requires an exact transfer mode",
                )? {
                    RuntimePresentation2bppTransferMode::Default => {
                        "default_tiles_per_vblank"
                    }
                    RuntimePresentation2bppTransferMode::Mobile => "mobile_tiles_per_vblank",
                };
                let tiles_per_vblank = chunking
                    .get(chunk_field)
                    .and_then(Value::as_u64)
                    .and_then(|value| u16::try_from(value).ok())
                    .filter(|value| *value > 0)
                    .with_context(|| {
                        format!(
                            "runtime presentation 2bpp request has no positive exact {chunk_field}"
                        )
                    })?;
                let tile_count = operation
                    .fields
                    .get("tile_count")
                    .and_then(Value::as_u64)
                    .and_then(|value| u16::try_from(value).ok())
                    .filter(|value| *value > 0)
                    .context("runtime presentation 2bpp request has no positive exact tile_count")?;
                let exact_frame_boundaries = operation
                    .fields
                    .get("request_2bpp_frame_boundaries_crossed")
                    .map(|value| {
                        value
                            .as_u64()
                            .and_then(|value| u16::try_from(value).ok())
                            .filter(|value| *value > 0)
                            .context(
                                "runtime presentation 2bpp request has an invalid exact ROM frame count",
                            )
                    })
                    .transpose()?;
                if let Some(expected_boundaries) = exact_frame_boundaries {
                    let oracle = operation
                        .fields
                        .get("request_2bpp_timing_oracle")
                        .and_then(Value::as_object)
                        .context(
                            "runtime presentation exact 2bpp request has no phase oracle",
                        )?;
                    let start_phase = oracle
                        .get("start_frame_phase_t_cycles")
                        .and_then(Value::as_u64)
                        .and_then(|value| u32::try_from(value).ok())
                        .context(
                            "runtime presentation exact 2bpp request has no start phase",
                        )?;
                    let elapsed_t_cycles = oracle
                        .get("elapsed_t_cycles_between_hooks")
                        .and_then(Value::as_u64)
                        .context(
                            "runtime presentation exact 2bpp request has no elapsed T-cycle count",
                        )?;
                    let mut frame_clock = RuntimeIntroFrameClock::new(
                        self.frame_t_cycles.context(
                            "runtime presentation exact 2bpp request requires a frame clock",
                        )?,
                        start_phase,
                    )?;
                    anyhow::ensure!(
                        frame_clock.advance_t_cycles(elapsed_t_cycles)?
                            == u64::from(expected_boundaries),
                        "runtime presentation exact 2bpp request disagrees with its frame clock"
                    );
                }
                self.wait_frames_remaining = exact_frame_boundaries
                    .unwrap_or_else(|| tile_count.div_ceil(tiles_per_vblank));
                effects.push(operation);
                return Ok(RuntimePresentationTimedPhaseTick {
                    effects,
                    cpu_work_machine_cycles,
                    complete: false,
                });
            }
            effects.push(operation);
        }
        anyhow::bail!("runtime presentation timed phase exceeded its operation limit")
    }
}

impl RuntimePresentationPhaseMachine {
    pub fn new(
        program: &RuntimePresentationProgram,
        subprogram_id: &str,
        phase_id: &str,
    ) -> Result<Self> {
        Ok(Self {
            interpreter: RuntimePresentationSubprogramInterpreter::new(
                program,
                subprogram_id,
                phase_id,
            )?,
            memory: BTreeMap::new(),
            values: BTreeMap::new(),
        })
    }

    pub fn run_from_label(
        &mut self,
        program: &RuntimePresentationProgram,
        label: &str,
        input: u8,
    ) -> Result<RuntimePresentationPhaseRun> {
        self.interpreter.jump_to_label(program, label)?;
        self.run_until_return(program, input)
    }

    pub fn run_from_operation_index(
        &mut self,
        program: &RuntimePresentationProgram,
        operation_index: usize,
        input: u8,
    ) -> Result<RuntimePresentationPhaseRun> {
        let phase = program
            .subprograms
            .iter()
            .find(|candidate| candidate.id == self.interpreter.subprogram)
            .and_then(|subprogram| {
                subprogram
                    .phases
                    .iter()
                    .find(|phase| phase.id == self.interpreter.phase)
            })
            .with_context(|| {
                format!(
                    "runtime presentation subprogram {} phase {} is missing",
                    self.interpreter.subprogram, self.interpreter.phase
                )
            })?;
        anyhow::ensure!(
            operation_index < phase.operations.len(),
            "runtime presentation operation index {operation_index} is outside subprogram {} phase {}",
            self.interpreter.subprogram,
            self.interpreter.phase
        );
        self.interpreter.operation_index = operation_index;
        self.interpreter.current_label = None;
        self.run_until_return(program, input)
    }

    fn run_until_return(
        &mut self,
        program: &RuntimePresentationProgram,
        input: u8,
    ) -> Result<RuntimePresentationPhaseRun> {
        let mut effects = Vec::new();
        for _ in 0..1_024 {
            let operation = self
                .interpreter
                .step(program)?
                .context("runtime presentation source label reached the end without returning")?;
            let numeric = |field: &str| {
                operation
                    .fields
                    .get(field)
                    .and_then(Value::as_u64)
                    .and_then(|value| u16::try_from(value).ok())
                    .with_context(|| {
                        format!(
                            "runtime presentation operation {} has no exact numeric {field}",
                            operation.op
                        )
                    })
            };
            let string = |field: &str| {
                operation
                    .fields
                    .get(field)
                    .and_then(Value::as_str)
                    .with_context(|| {
                        format!(
                            "runtime presentation operation {} has no exact string {field}",
                            operation.op
                        )
                    })
            };
            let condition_matches = || -> Result<bool> {
                let Some(condition) = operation.fields.get("condition") else {
                    return Ok(true);
                };
                let condition = condition.as_object().with_context(|| {
                    format!(
                        "runtime presentation operation {} has a malformed condition",
                        operation.op
                    )
                })?;
                let predicate = condition
                    .get("predicate")
                    .and_then(Value::as_str)
                    .with_context(|| {
                        format!(
                            "runtime presentation operation {} condition has no predicate",
                            operation.op
                        )
                    })?;
                if predicate == "always" {
                    return Ok(true);
                }
                let source = condition
                    .get("source")
                    .and_then(Value::as_str)
                    .with_context(|| {
                        format!(
                            "runtime presentation operation {} condition has no source",
                            operation.op
                        )
                    })?;
                let value = self.memory.get(source).copied().with_context(|| {
                    format!(
                        "runtime presentation condition memory {source} was read before initialization"
                    )
                })?;
                match predicate {
                    "zero" => Ok(value == 0),
                    "nonzero" => Ok(value != 0),
                    predicate => anyhow::bail!(
                        "runtime presentation operation {} has unsupported condition predicate {predicate}",
                        operation.op
                    ),
                }
            };
            match operation.op.as_str() {
                "return" | "return_with_carry" => {
                    return Ok(RuntimePresentationPhaseRun {
                        effects,
                        returned: true,
                    });
                }
                "jump" => self.interpreter.jump_to_label(program, string("target")?)?,
                "branch_memory_compare" => {
                    let source = string("source")?;
                    let operand = numeric("operand")?;
                    let value = self.memory.get(source).copied().with_context(|| {
                        format!(
                            "runtime presentation memory {source} was read before initialization"
                        )
                    })?;
                    let matches = match string("predicate")? {
                        "equal" => value == operand,
                        "not_equal" => value != operand,
                        "unsigned_less_than" => value < operand,
                        predicate => anyhow::bail!(
                            "runtime presentation branch_memory_compare has unsupported predicate {predicate}"
                        ),
                    };
                    if matches {
                        self.interpreter.jump_to_label(program, string("target")?)?;
                    }
                }
                "input_chord_branch" => {
                    let sample = string("sample")?;
                    let sampled = u8::try_from(
                        self.memory.get(sample).copied().with_context(|| {
                            format!("runtime presentation memory {sample} was read before initialization")
                        })?,
                    )
                    .context("runtime presentation input sample exceeds one byte")?;
                    let mask = u8::try_from(numeric("mask")?)
                        .context("runtime presentation input mask exceeds one byte")?;
                    let matches = match string("predicate")? {
                        "masked_equals" => {
                            let operand = u8::try_from(numeric("operand")?)
                                .context("runtime presentation input operand exceeds one byte")?;
                            sampled & mask == operand
                        }
                        "masked_not_equal" => {
                            let operand = u8::try_from(numeric("operand")?)
                                .context("runtime presentation input operand exceeds one byte")?;
                            sampled & mask != operand
                        }
                        "masked_nonzero" => sampled & mask != 0,
                        predicate => anyhow::bail!(
                            "runtime presentation input_chord_branch has unsupported predicate {predicate}"
                        ),
                    };
                    if matches {
                        self.interpreter.jump_to_label(program, string("target")?)?;
                    }
                }
                "input_bit_branch" => {
                    let sample = string("sample")?;
                    let sampled = u8::try_from(
                        self.memory.get(sample).copied().with_context(|| {
                            format!("runtime presentation memory {sample} was read before initialization")
                        })?,
                    )
                    .context("runtime presentation input sample exceeds one byte")?;
                    let bit = u8::try_from(numeric("bit")?)
                        .context("runtime presentation input bit exceeds one byte")?;
                    anyhow::ensure!(bit < 8, "runtime presentation input bit is out of range");
                    let set = sampled & (1_u8 << bit) != 0;
                    let matches = match string("predicate")? {
                        "set" => set,
                        "clear" => !set,
                        predicate => anyhow::bail!(
                            "runtime presentation input_bit_branch has unsupported predicate {predicate}"
                        ),
                    };
                    if matches {
                        self.interpreter.jump_to_label(program, string("target")?)?;
                    }
                }
                "branch_compare" => {
                    let name = string("value")?;
                    let value = self.values.get(name).copied().with_context(|| {
                        format!(
                            "runtime presentation result {name} was read before initialization"
                        )
                    })?;
                    let operand = numeric("operand")?;
                    let matches = match string("predicate")? {
                        "equal" => value == operand,
                        "not_equal" => value != operand,
                        "unsigned_greater_or_equal" => value >= operand,
                        predicate => anyhow::bail!(
                            "runtime presentation branch_compare has unsupported predicate {predicate}"
                        ),
                    };
                    if matches {
                        self.interpreter.jump_to_label(program, string("target")?)?;
                    }
                }
                "sample_input" => {
                    self.memory
                        .insert(string("result")?.to_string(), u16::from(input));
                }
                "read_memory_byte" => {
                    let target = string("target")?;
                    let value = self.memory.get(target).copied().with_context(|| {
                        format!(
                            "runtime presentation memory {target} was read before initialization"
                        )
                    })?;
                    anyhow::ensure!(
                        value <= u16::from(u8::MAX),
                        "runtime presentation memory {target} exceeds one byte"
                    );
                    self.values.insert(string("result")?.to_string(), value);
                }
                "transform_memory_byte" => {
                    anyhow::ensure!(
                        string("wrap")? == "u8",
                        "runtime presentation transform_memory_byte has unsupported wrapping semantics"
                    );
                    let input = string("input")?;
                    let value = u8::try_from(self.values.get(input).copied().with_context(|| {
                        format!(
                            "runtime presentation result {input} was read before initialization"
                        )
                    })?)
                    .context("runtime presentation transform input exceeds one byte")?;
                    let operand = u8::try_from(numeric("operand")?)
                        .context("runtime presentation transform operand exceeds one byte")?;
                    let result = match string("operator")? {
                        "subtract" => value.wrapping_sub(operand),
                        operator => anyhow::bail!(
                            "runtime presentation transform_memory_byte has unsupported operator {operator}"
                        ),
                    };
                    self.memory
                        .insert(string("target")?.to_string(), u16::from(result));
                    effects.push(operation);
                }
                "write_memory_byte" | "write_memory_word" => {
                    if condition_matches()? {
                        let value = numeric("value")?;
                        self.memory.insert(string("target")?.to_string(), value);
                        effects.push(operation);
                    }
                }
                "write_memory_byte_from_result" => {
                    let result = string("result")?;
                    let value = self.values.get(result).copied().with_context(|| {
                        format!(
                            "runtime presentation result {result} was read before initialization"
                        )
                    })?;
                    anyhow::ensure!(
                        value <= u16::from(u8::MAX),
                        "runtime presentation result {result} exceeds one byte"
                    );
                    self.memory.insert(string("target")?.to_string(), value);
                    effects.push(operation);
                }
                "write_memory_byte_from_masked_result" => {
                    let result = string("result")?;
                    let value = u8::try_from(self.values.get(result).copied().with_context(|| {
                        format!(
                            "runtime presentation result {result} was read before initialization"
                        )
                    })?)
                    .context("runtime presentation masked byte result exceeds one byte")?;
                    let mask = u8::try_from(numeric("mask")?)
                        .context("runtime presentation masked byte mask exceeds one byte")?;
                    let shift = operation
                        .fields
                        .get("shift_right")
                        .and_then(Value::as_u64)
                        .map(u8::try_from)
                        .transpose()
                        .context("runtime presentation masked byte shift exceeds one byte")?
                        .unwrap_or(0);
                    anyhow::ensure!(
                        shift < 8,
                        "runtime presentation masked byte shift is out of range"
                    );
                    self.memory.insert(
                        string("target")?.to_string(),
                        u16::from((value & mask) >> shift),
                    );
                    effects.push(operation);
                }
                "set_local" => {
                    self.values.insert(string("name")?.to_string(), numeric("value")?);
                }
                "set_local_from_result" => {
                    let source = string("source")?;
                    let value = u8::try_from(self.values.get(source).copied().with_context(|| {
                        format!(
                            "runtime presentation result {source} was read before initialization"
                        )
                    })?)
                    .context("runtime presentation local byte source exceeds one byte")?;
                    anyhow::ensure!(
                        string("wrap")? == "u8",
                        "runtime presentation set_local_from_result has unsupported wrapping semantics"
                    );
                    let subtract = u8::try_from(numeric("subtract")?)
                        .context("runtime presentation local byte subtraction exceeds one byte")?;
                    self.values.insert(
                        string("name")?.to_string(),
                        u16::from(value.wrapping_sub(subtract)),
                    );
                }
                "set_local_from_masked_result" => {
                    let source = string("source")?;
                    let value = u8::try_from(self.values.get(source).copied().with_context(|| {
                        format!(
                            "runtime presentation result {source} was read before initialization"
                        )
                    })?)
                    .context("runtime presentation masked local source exceeds one byte")?;
                    let mask = u8::try_from(numeric("mask")?)
                        .context("runtime presentation masked local mask exceeds one byte")?;
                    let masked = value & mask;
                    let result = if operation
                        .fields
                        .get("swap_nibbles")
                        .and_then(Value::as_bool)
                        == Some(true)
                    {
                        anyhow::ensure!(
                            !operation.fields.contains_key("shift_left"),
                            "runtime presentation masked local has two transforms"
                        );
                        masked.rotate_left(4)
                    } else {
                        anyhow::ensure!(
                            string("wrap")? == "u8",
                            "runtime presentation set_local_from_masked_result has unsupported wrapping semantics"
                        );
                        let shift = u8::try_from(numeric("shift_left")?)
                            .context("runtime presentation masked local shift exceeds one byte")?;
                        anyhow::ensure!(
                            shift < 8,
                            "runtime presentation masked local shift is out of range"
                        );
                        masked.wrapping_shl(u32::from(shift))
                    };
                    let valid = operation
                        .fields
                        .get("valid_values")
                        .and_then(Value::as_array)
                        .context("runtime presentation masked local has no valid-value domain")?;
                    anyhow::ensure!(
                        valid.iter().any(|candidate| candidate.as_u64() == Some(u64::from(result))),
                        "runtime presentation masked local result leaves its valid-value domain"
                    );
                    self.values
                        .insert(string("name")?.to_string(), u16::from(result));
                }
                "set_local_from_memory" => {
                    let source = string("source")?;
                    let value = u8::try_from(self.memory.get(source).copied().with_context(|| {
                        format!(
                            "runtime presentation memory {source} was read before initialization"
                        )
                    })?)
                    .context("runtime presentation memory-local source exceeds one byte")?;
                    anyhow::ensure!(
                        string("wrap")? == "u8",
                        "runtime presentation set_local_from_memory has unsupported wrapping semantics"
                    );
                    let subtract = u8::try_from(numeric("subtract")?)
                        .context("runtime presentation memory-local subtraction exceeds one byte")?;
                    self.values.insert(
                        string("name")?.to_string(),
                        u16::from(value.wrapping_sub(subtract)),
                    );
                }
                "compute_byte" => {
                    let input = string("input")?;
                    let mut value = u8::try_from(self.values.get(input).copied().with_context(|| {
                        format!(
                            "runtime presentation result {input} was read before initialization"
                        )
                    })?)
                    .context("runtime presentation compute_byte input exceeds one byte")?;
                    let steps = operation
                        .fields
                        .get("steps")
                        .and_then(Value::as_array)
                        .context("runtime presentation compute_byte has no steps")?;
                    anyhow::ensure!(
                        !steps.is_empty(),
                        "runtime presentation compute_byte has an empty transform"
                    );
                    for step in steps {
                        let step = step
                            .as_object()
                            .context("runtime presentation compute_byte step is malformed")?;
                        let op = step
                            .get("op")
                            .and_then(Value::as_str)
                            .context("runtime presentation compute_byte step has no operation")?;
                        let operand = step
                            .get("value")
                            .and_then(Value::as_u64)
                            .and_then(|value| u8::try_from(value).ok())
                            .context("runtime presentation compute_byte step operand is invalid")?;
                        value = match op {
                            "mask" => value & operand,
                            "or" => value | operand,
                            "shift_left" => {
                                anyhow::ensure!(
                                    operand < 8,
                                    "runtime presentation compute_byte left shift is out of range"
                                );
                                value.wrapping_shl(u32::from(operand))
                            }
                            "shift_right" => {
                                anyhow::ensure!(
                                    operand < 8,
                                    "runtime presentation compute_byte right shift is out of range"
                                );
                                value >> operand
                            }
                            "swap_nibbles" => {
                                anyhow::ensure!(
                                    operand == 0,
                                    "runtime presentation compute_byte nibble swap has an operand"
                                );
                                value.rotate_left(4)
                            }
                            op => anyhow::bail!(
                                "runtime presentation compute_byte has unsupported step {op}"
                            ),
                        };
                    }
                    self.values
                        .insert(string("result")?.to_string(), u16::from(value));
                }
                "scheduled_audio" => {
                    let clock = string("clock")?;
                    let value = self.memory.get(clock).copied().with_context(|| {
                        format!(
                            "runtime presentation memory {clock} was read before initialization"
                        )
                    })?;
                    anyhow::ensure!(
                        value <= u16::from(u8::MAX),
                        "runtime presentation scheduled-audio clock exceeds one byte"
                    );
                    let entries = operation
                        .fields
                        .get("entries")
                        .and_then(Value::as_array)
                        .context("runtime presentation scheduled_audio has no entries")?;
                    let matched = entries.iter().any(|entry| {
                        entry.get("frame").and_then(Value::as_u64) == Some(u64::from(value))
                    });
                    if matched {
                        effects.push(operation);
                    }
                }
                "indexed_2bpp_request" => {
                    let condition = operation
                        .fields
                        .get("condition")
                        .and_then(Value::as_object)
                        .context("runtime presentation indexed_2bpp_request has no condition")?;
                    let source = condition
                        .get("source")
                        .and_then(Value::as_str)
                        .context("runtime presentation indexed_2bpp_request has no condition source")?;
                    let value = self.memory.get(source).copied().with_context(|| {
                        format!(
                            "runtime presentation memory {source} was read before initialization"
                        )
                    })?;
                    let operand = condition
                        .get("operand")
                        .and_then(Value::as_u64)
                        .and_then(|value| u16::try_from(value).ok())
                        .context("runtime presentation indexed_2bpp_request condition operand is invalid")?;
                    let matches = match condition.get("predicate").and_then(Value::as_str) {
                        Some("unsigned_less_than") => value < operand,
                        Some(predicate) => anyhow::bail!(
                            "runtime presentation indexed_2bpp_request has unsupported predicate {predicate}"
                        ),
                        None => anyhow::bail!(
                            "runtime presentation indexed_2bpp_request has no condition predicate"
                        ),
                    };
                    if matches {
                        effects.push(operation);
                    }
                }
                "subtract_memory_byte" => {
                    let target = string("target")?.to_string();
                    let value = self.memory.get(&target).copied().with_context(|| {
                        format!(
                            "runtime presentation memory {target} was read before initialization"
                        )
                    })? as u8;
                    let delta = u8::try_from(numeric("delta")?)
                        .context("runtime presentation byte subtraction exceeds one byte")?;
                    let result = value.wrapping_sub(delta);
                    self.memory.insert(target, u16::from(result));
                    if let Some(name) = operation.fields.get("result").and_then(Value::as_str) {
                        self.values.insert(name.to_string(), u16::from(result));
                    }
                    effects.push(operation);
                }
                "add_memory_byte" => {
                    let target = string("target")?.to_string();
                    let value = u8::try_from(self.memory.get(&target).copied().with_context(|| {
                        format!(
                            "runtime presentation memory {target} was read before initialization"
                        )
                    })?)
                    .context("runtime presentation byte-add source exceeds one byte")?;
                    let delta = u8::try_from(numeric("delta")?)
                        .context("runtime presentation byte addition exceeds one byte")?;
                    self.memory
                        .insert(target, u16::from(value.wrapping_add(delta)));
                    effects.push(operation);
                }
                "conditional_tilemap_xor" => {
                    let clock = string("clock")?;
                    let value = self.memory.get(clock).copied().with_context(|| {
                        format!(
                            "runtime presentation memory {clock} was read before initialization"
                        )
                    })?;
                    let masked = value & numeric("clock_mask")?;
                    let mut applied = false;
                    for (phase_name, write_name) in [
                        ("prepare_phase", "write"),
                        ("swap_phase", "completion_write"),
                    ] {
                        let phase = operation
                            .fields
                            .get(phase_name)
                            .and_then(Value::as_object)
                            .with_context(|| {
                                format!(
                                    "runtime presentation conditional_tilemap_xor has no {phase_name}"
                                )
                            })?;
                        let equals = phase
                            .get("equals")
                            .and_then(Value::as_u64)
                            .and_then(|value| u16::try_from(value).ok())
                            .with_context(|| {
                                format!(
                                    "runtime presentation conditional_tilemap_xor {phase_name} equality is invalid"
                                )
                            })?;
                        if masked == equals {
                            let write = phase
                                .get(write_name)
                                .and_then(Value::as_object)
                                .with_context(|| {
                                    format!(
                                        "runtime presentation conditional_tilemap_xor {phase_name} has no {write_name}"
                                    )
                                })?;
                            let target = write
                                .get("target")
                                .and_then(Value::as_str)
                                .with_context(|| {
                                    format!(
                                        "runtime presentation conditional_tilemap_xor {phase_name} write has no target"
                                    )
                                })?;
                            let written = write
                                .get("value")
                                .and_then(Value::as_u64)
                                .and_then(|value| u16::try_from(value).ok())
                                .with_context(|| {
                                    format!(
                                        "runtime presentation conditional_tilemap_xor {phase_name} write is invalid"
                                    )
                                })?;
                            self.memory.insert(target.to_string(), written);
                            applied = true;
                        }
                    }
                    if applied {
                        effects.push(operation);
                    }
                }
                "increment_memory_byte" => {
                    let target = string("target")?.to_string();
                    let value = self.memory.get(&target).copied().with_context(|| {
                        format!(
                            "runtime presentation memory {target} was read before initialization"
                        )
                    })? as u8;
                    let delta = u8::try_from(numeric("delta")?)
                        .context("runtime presentation byte increment exceeds one byte")?;
                    self.memory
                        .insert(target, u16::from(value.wrapping_add(delta)));
                }
                "postincrement_memory_byte" => {
                    let target = string("target")?.to_string();
                    let value = self.memory.get(&target).copied().with_context(|| {
                        format!(
                            "runtime presentation memory {target} was read before initialization"
                        )
                    })? as u8;
                    self.values
                        .insert(string("result")?.to_string(), u16::from(value));
                    let delta = u8::try_from(numeric("delta")?)
                        .context("runtime presentation byte increment exceeds one byte")?;
                    self.memory
                        .insert(target, u16::from(value.wrapping_add(delta)));
                }
                "decrement_memory_byte" => {
                    let target = string("target")?.to_string();
                    let value = u8::try_from(self.memory.get(&target).copied().with_context(|| {
                        format!(
                            "runtime presentation memory {target} was read before initialization"
                        )
                    })?)
                    .context("runtime presentation byte decrement source exceeds one byte")?;
                    anyhow::ensure!(
                        string("wrap")? == "u8",
                        "runtime presentation decrement_memory_byte has unsupported wrapping semantics"
                    );
                    let delta = u8::try_from(numeric("delta")?)
                        .context("runtime presentation byte decrement exceeds one byte")?;
                    self.values
                        .insert(string("comparison_value")?.to_string(), u16::from(value));
                    self.memory
                        .insert(target, u16::from(value.wrapping_sub(delta)));
                }
                "decrement_memory_word_unless_zero" => {
                    let target = string("target")?.to_string();
                    let value = self.memory.get(&target).copied().with_context(|| {
                        format!(
                            "runtime presentation memory {target} was read before initialization"
                        )
                    })?;
                    if value == 0 {
                        self.interpreter
                            .jump_to_label(program, string("zero_target")?)?;
                    } else {
                        self.memory.insert(target, value - 1);
                    }
                }
                "set_memory_bit" => {
                    let target = string("target")?.to_string();
                    let bit = u8::try_from(numeric("bit")?)
                        .context("runtime presentation memory bit exceeds one byte")?;
                    anyhow::ensure!(bit < 16, "runtime presentation memory bit is out of range");
                    let value = self.memory.get(&target).copied().with_context(|| {
                        format!(
                            "runtime presentation memory {target} was read before initialization"
                        )
                    })? | (1_u16 << bit);
                    self.memory.insert(target, value);
                }
                "select_title_option" => {
                    let current_label = self.interpreter.current_label.as_deref().context(
                        "runtime presentation select_title_option has no incoming source label",
                    )?;
                    let options = operation
                        .fields
                        .get("options")
                        .and_then(Value::as_array)
                        .context("runtime presentation select_title_option has no options")?;
                    let selected = options.iter().find_map(|candidate| {
                        let candidate = candidate.as_object()?;
                        (candidate.get("source")?.as_str()? == current_label)
                            .then(|| candidate.get("value")?.as_u64())
                            .flatten()
                    });
                    let selected = selected.with_context(|| {
                        format!(
                            "runtime presentation select_title_option has no value for {current_label}"
                        )
                    })?;
                    self.memory.insert(
                        string("target")?.to_string(),
                        u16::try_from(selected)
                            .context("runtime presentation title option exceeds two bytes")?,
                    );
                }
                "return_if_memory_nonzero" => {
                    let source = string("source")?;
                    let value = self.memory.get(source).copied().with_context(|| {
                        format!(
                            "runtime presentation memory {source} was read before initialization"
                        )
                    })?;
                    if value != 0 {
                        return Ok(RuntimePresentationPhaseRun {
                            effects,
                            returned: true,
                        });
                    }
                }
                "return_if_memory_zero" => {
                    let source = string("source")?;
                    let value = self.memory.get(source).copied().with_context(|| {
                        format!(
                            "runtime presentation memory {source} was read before initialization"
                        )
                    })?;
                    if value == 0 {
                        if let Some(target) = operation.fields.get("target").and_then(Value::as_str)
                        {
                            self.interpreter.jump_to_label(program, target)?;
                        } else {
                            return Ok(RuntimePresentationPhaseRun {
                                effects,
                                returned: true,
                            });
                        }
                    }
                }
                "return_if_memory_equal" => {
                    let source = string("source")?;
                    let value = self.memory.get(source).copied().with_context(|| {
                        format!(
                            "runtime presentation memory {source} was read before initialization"
                        )
                    })?;
                    if value == numeric("operand")? {
                        return Ok(RuntimePresentationPhaseRun {
                            effects,
                            returned: true,
                        });
                    }
                }
                "return_if_compare" => {
                    let name = string("value")?;
                    let value = self.values.get(name).copied().with_context(|| {
                        format!(
                            "runtime presentation result {name} was read before initialization"
                        )
                    })?;
                    let operand = numeric("operand")?;
                    let matches = match string("predicate")? {
                        "equal" => value == operand,
                        "not_equal" => value != operand,
                        "unsigned_greater_or_equal" => value >= operand,
                        "unsigned_less_than" => value < operand,
                        predicate => anyhow::bail!(
                            "runtime presentation return_if_compare has unsupported predicate {predicate}"
                        ),
                    };
                    if matches {
                        return Ok(RuntimePresentationPhaseRun {
                            effects,
                            returned: true,
                        });
                    }
                }
                "return_unless_compare" => {
                    let name = string("value")?;
                    let value = self.values.get(name).copied().with_context(|| {
                        format!(
                            "runtime presentation result {name} was read before initialization"
                        )
                    })?;
                    let operand = numeric("operand")?;
                    let matches = match string("predicate")? {
                        "equal" => value == operand,
                        "not_equal" => value != operand,
                        "unsigned_greater_or_equal" => value >= operand,
                        "unsigned_less_than" => value < operand,
                        predicate => anyhow::bail!(
                            "runtime presentation return_unless_compare has unsupported predicate {predicate}"
                        ),
                    };
                    if !matches {
                        return Ok(RuntimePresentationPhaseRun {
                            effects,
                            returned: true,
                        });
                    }
                }
                "return_unless_mask_equal" => {
                    let source = string("source")?;
                    let value = self.values.get(source).copied().with_context(|| {
                        format!(
                            "runtime presentation result {source} was read before initialization"
                        )
                    })?;
                    if value & numeric("mask")? != numeric("operand")? {
                        return Ok(RuntimePresentationPhaseRun {
                            effects,
                            returned: true,
                        });
                    }
                }
                "return_unless_masked_zero" => {
                    let name = string("value")?;
                    let value = self.values.get(name).copied().with_context(|| {
                        format!("runtime presentation result {name} was read before initialization")
                    })?;
                    if value & numeric("mask")? != 0 {
                        return Ok(RuntimePresentationPhaseRun {
                            effects,
                            returned: true,
                        });
                    }
                }
                "fade_audio" => {
                    let frames = numeric("frames")?;
                    if let Some(register) = operation
                        .fields
                        .get("fade_register")
                        .and_then(Value::as_object)
                    {
                        let target = register
                            .get("target")
                            .and_then(Value::as_str)
                            .context("runtime presentation fade_audio has no fade-register target")?;
                        let value = register
                            .get("value")
                            .and_then(Value::as_u64)
                            .and_then(|value| u16::try_from(value).ok())
                            .context("runtime presentation fade_audio has no exact fade-register value")?;
                        anyhow::ensure!(
                            value > 0 && value <= 0x3f,
                            "runtime presentation fade_audio rate {value} is outside wMusicFade's low-six-bit domain"
                        );
                        anyhow::ensure!(
                            frames == value * 8,
                            "runtime presentation fade_audio duration {frames} does not match rate {value} across eight volume boundaries"
                        );
                        self.memory.insert(target.to_string(), value);
                    }
                    effects.push(operation);
                }
                "animate_title_crystal" => {
                    let target = string("target")?.to_string();
                    let current = u8::try_from(
                        self.memory.get(&target).copied().with_context(|| {
                            format!(
                                "runtime presentation memory {target} was read before initialization"
                            )
                        })?,
                    )
                    .context("runtime presentation title crystal Y exceeds one byte")?;
                    let stop_at = u8::try_from(numeric("stop_at")?)
                        .context("runtime presentation title crystal stop exceeds one byte")?;
                    if current != stop_at {
                        let delta = u8::try_from(numeric("y_delta")?)
                            .context("runtime presentation title crystal delta exceeds one byte")?;
                        self.memory
                            .insert(target, u16::from(current.wrapping_add(delta)));
                    }
                }
                "fill_memory" | "palette_transfer_request" | "wait_frames" => {
                    if condition_matches()? {
                        effects.push(operation);
                    }
                }
                "fill_memory_from_result"
                | "fill_strided_memory_from_transformed_result"
                | "draw_indexed_title_suicune_frame"
                | "play_audio"
                | "sprite_init_group"
                | "sprite_activate"
                | "deinitialize_all_sprites"
                | "copy_indexed_palette"
                | "broadcast_indexed_palette"
                | "fade_unown_word_palettes"
                | "palette_fade_lookup"
                | "perspective_scroll" => effects.push(operation),
                op => anyhow::bail!(
                    "runtime presentation phase machine cannot execute source operation {op}"
                ),
            }
        }
        anyhow::bail!("runtime presentation source phase exceeded its operation limit")
    }

    pub fn dispatch_label(
        &self,
        program: &RuntimePresentationProgram,
        dispatcher: &str,
        index: usize,
    ) -> Result<String> {
        let phase = program
            .subprograms
            .iter()
            .find(|candidate| candidate.id == self.interpreter.subprogram)
            .and_then(|subprogram| {
                subprogram
                    .phases
                    .iter()
                    .find(|phase| phase.id == self.interpreter.phase)
            })
            .with_context(|| {
                format!(
                    "runtime presentation subprogram {} phase {} is missing",
                    self.interpreter.subprogram, self.interpreter.phase
                )
            })?;
        let operation = phase
            .operations
            .iter()
            .find(|operation| {
                operation.op == "dispatch_table"
                    && operation.fields.get("dispatcher").and_then(Value::as_str)
                        == Some(dispatcher)
            })
            .with_context(|| {
                format!(
                    "runtime presentation subprogram {} phase {} dispatcher {dispatcher} is missing",
                    self.interpreter.subprogram, self.interpreter.phase
                )
            })?;
        let label = operation
            .fields
            .get("entries")
            .and_then(Value::as_array)
            .and_then(|entries| entries.get(index))
            .and_then(Value::as_str)
            .with_context(|| {
                format!("runtime presentation dispatcher {dispatcher} has no entry {index}")
            })?;
        anyhow::ensure!(
            phase.labels.contains_key(label),
            "runtime presentation dispatcher {dispatcher} entry {index} targets missing label {label}"
        );
        Ok(label.to_string())
    }
}

impl RuntimePresentationInterpreter {
    pub fn new(program: &RuntimePresentationProgram, entrypoint: &str) -> Result<Self> {
        let block = program
            .entrypoints
            .get(entrypoint)
            .with_context(|| format!("runtime presentation entrypoint {entrypoint} is missing"))?;
        anyhow::ensure!(
            program.blocks.contains_key(block),
            "runtime presentation entrypoint {entrypoint} targets missing block {block}"
        );
        Ok(Self {
            entrypoint: entrypoint.to_string(),
            block: block.clone(),
            operation_index: 0,
        })
    }

    pub fn jump(&mut self, program: &RuntimePresentationProgram, target: &str) -> Result<()> {
        anyhow::ensure!(
            program.blocks.contains_key(target),
            "runtime presentation jump targets missing block {target}"
        );
        self.block = target.to_string();
        self.operation_index = 0;
        Ok(())
    }

    pub fn step(
        &mut self,
        program: &RuntimePresentationProgram,
    ) -> Result<RuntimePresentationStep> {
        let block = program
            .blocks
            .get(&self.block)
            .with_context(|| format!("runtime presentation block {} is missing", self.block))?;
        let Some(operation) = block.operations.get(self.operation_index).cloned() else {
            return Ok(RuntimePresentationStep::BlockComplete {
                block: self.block.clone(),
            });
        };
        self.operation_index += 1;
        if operation.op == "jump" {
            let target = operation
                .fields
                .get("target")
                .and_then(Value::as_str)
                .context("runtime presentation jump operation is missing target")?
                .to_string();
            let from = self.block.clone();
            self.jump(program, &target)?;
            return Ok(RuntimePresentationStep::Jump { from, to: target });
        }
        Ok(RuntimePresentationStep::Operation(operation))
    }
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

fn required_nullable_value<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TilesetDefinition {
    pub collision: BTreeMap<String, Vec<String>>,
    pub palette_map: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    Pcm,
    Midi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ModpackAudioPlaybackMode {
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
                mode: ModpackAudioPlaybackMode::RawPcm,
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
        let expected_mode = ModpackAudioPlaybackMode::RawPcm;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackMidiAudioProgram {
    pub profile: String,
    pub midi_base64: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackAsmAudioCommand {
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeIntroAudioPitch {
    C,
    CSharp,
    D,
    DSharp,
    E,
    F,
    FSharp,
    G,
    GSharp,
    A,
    ASharp,
    B,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuntimeIntroAudioCommand {
    Label(String),
    Tempo(u16),
    Volume { left: u8, right: u8 },
    PitchOffset(i16),
    Vibrato { delay: u8, extent: u8, rate: u8 },
    DutyCycle(u8),
    StereoPanning { left: bool, right: bool },
    NoteType { length: u8, volume: u8, fade: i8 },
    Octave(u8),
    Note { pitch: RuntimeIntroAudioPitch, duration: u8 },
    Rest(u8),
    ToggleNoise(u8),
    DrumSpeed(u8),
    DrumNote { instrument: u8, duration: u8 },
    PitchSweep { duration: u8, pitch: i8 },
    SquareNote { duration: u8, volume: u8, fade: i8, frequency: u16 },
    NoiseNote { duration: u8, volume: u8, fade: i8, frequency: u8 },
    SoundRet,
}

impl ModpackAsmAudioCommand {
    fn require_args(&self, id: &str, count: usize) -> Result<()> {
        anyhow::ensure!(
            self.args.len() == count,
            "audio asset '{id}' ASM command '{}' requires {count} arguments, found {}",
            self.command,
            self.args.len()
        );
        Ok(())
    }

    fn integer<T>(&self, id: &str, index: usize, name: &str) -> Result<T>
    where
        T: std::str::FromStr,
        T::Err: std::fmt::Display,
    {
        self.args[index].parse::<T>().map_err(|error| {
            anyhow::anyhow!(
                "audio asset '{id}' ASM command '{}' has invalid {name} '{}': {error}",
                self.command,
                self.args[index]
            )
        })
    }

    fn nibble(&self, id: &str, index: usize, name: &str) -> Result<u8> {
        let value = self.integer::<u8>(id, index, name)?;
        anyhow::ensure!(value <= 0x0f, "audio asset '{id}' ASM command '{}' {name} exceeds one nibble", self.command);
        Ok(value)
    }

    fn signed_nibble(&self, id: &str, index: usize, name: &str) -> Result<i8> {
        let value = self.integer::<i8>(id, index, name)?;
        anyhow::ensure!((-8..=8).contains(&value), "audio asset '{id}' ASM command '{}' {name} cannot be encoded in one signed-magnitude nibble", self.command);
        Ok(value)
    }

    fn duration_nibble(&self, id: &str, index: usize) -> Result<u8> {
        let duration = self.integer::<u8>(id, index, "duration")?;
        anyhow::ensure!((1..=16).contains(&duration), "audio asset '{id}' ASM command '{}' duration is outside 1..=16", self.command);
        Ok(duration)
    }

    fn boolean(&self, id: &str, index: usize, name: &str) -> Result<bool> {
        match self.args[index].as_str() {
            "TRUE" => Ok(true),
            "FALSE" => Ok(false),
            value => anyhow::bail!("audio asset '{id}' ASM command '{}' has invalid {name} '{value}'", self.command),
        }
    }

    pub fn intro_command(&self, id: &str) -> Result<RuntimeIntroAudioCommand> {
        let parsed = match self.command.as_str() {
            "label" => {
                self.require_args(id, 1)?;
                RuntimeIntroAudioCommand::Label(self.args[0].clone())
            }
            "tempo" => {
                self.require_args(id, 1)?;
                RuntimeIntroAudioCommand::Tempo(self.integer(id, 0, "tempo")?)
            }
            "volume" => {
                self.require_args(id, 2)?;
                let left = self.nibble(id, 0, "left volume")?;
                let right = self.nibble(id, 1, "right volume")?;
                anyhow::ensure!(
                    left <= 7 && right <= 7,
                    "audio asset '{id}' master volume exceeds the hardware 0..=7 domain"
                );
                RuntimeIntroAudioCommand::Volume { left, right }
            }
            "pitch_offset" => {
                self.require_args(id, 1)?;
                RuntimeIntroAudioCommand::PitchOffset(self.integer(id, 0, "pitch offset")?)
            }
            "vibrato" => {
                self.require_args(id, 3)?;
                RuntimeIntroAudioCommand::Vibrato { delay: self.integer(id, 0, "delay")?, extent: self.nibble(id, 1, "extent")?, rate: self.nibble(id, 2, "rate")? }
            }
            "duty_cycle" => {
                self.require_args(id, 1)?;
                let duty = self.integer::<u8>(id, 0, "duty cycle")?;
                anyhow::ensure!(duty <= 3, "audio asset '{id}' duty cycle exceeds two bits");
                RuntimeIntroAudioCommand::DutyCycle(duty)
            }
            "stereo_panning" => {
                self.require_args(id, 2)?;
                RuntimeIntroAudioCommand::StereoPanning { left: self.boolean(id, 0, "left panning")?, right: self.boolean(id, 1, "right panning")? }
            }
            "note_type" => {
                self.require_args(id, 3)?;
                RuntimeIntroAudioCommand::NoteType { length: self.integer(id, 0, "note length")?, volume: self.nibble(id, 1, "volume")?, fade: self.signed_nibble(id, 2, "fade")? }
            }
            "octave" => {
                self.require_args(id, 1)?;
                let octave = self.integer::<u8>(id, 0, "octave")?;
                anyhow::ensure!((1..=8).contains(&octave), "audio asset '{id}' octave is outside 1..=8");
                RuntimeIntroAudioCommand::Octave(octave)
            }
            "note" => {
                self.require_args(id, 2)?;
                let pitch = match self.args[0].as_str() {
                    "C_" => RuntimeIntroAudioPitch::C, "C#" => RuntimeIntroAudioPitch::CSharp,
                    "D_" => RuntimeIntroAudioPitch::D, "D#" => RuntimeIntroAudioPitch::DSharp,
                    "E_" => RuntimeIntroAudioPitch::E, "F_" => RuntimeIntroAudioPitch::F,
                    "F#" => RuntimeIntroAudioPitch::FSharp, "G_" => RuntimeIntroAudioPitch::G,
                    "G#" => RuntimeIntroAudioPitch::GSharp, "A_" => RuntimeIntroAudioPitch::A,
                    "A#" => RuntimeIntroAudioPitch::ASharp, "B_" => RuntimeIntroAudioPitch::B,
                    pitch => anyhow::bail!("audio asset '{id}' ASM note has invalid pitch '{pitch}'"),
                };
                RuntimeIntroAudioCommand::Note { pitch, duration: self.duration_nibble(id, 1)? }
            }
            "rest" => {
                self.require_args(id, 1)?;
                RuntimeIntroAudioCommand::Rest(self.duration_nibble(id, 0)?)
            }
            "toggle_noise" => {
                self.require_args(id, 1)?;
                RuntimeIntroAudioCommand::ToggleNoise(self.integer(id, 0, "drumkit")?)
            }
            "drum_speed" => {
                self.require_args(id, 1)?;
                RuntimeIntroAudioCommand::DrumSpeed(self.integer(id, 0, "drum speed")?)
            }
            "drum_note" => {
                self.require_args(id, 2)?;
                let instrument = self.nibble(id, 0, "drum instrument")?;
                anyhow::ensure!((1..=12).contains(&instrument), "audio asset '{id}' drum instrument is outside 1..=12");
                RuntimeIntroAudioCommand::DrumNote { instrument, duration: self.duration_nibble(id, 1)? }
            }
            "pitch_sweep" => {
                self.require_args(id, 2)?;
                RuntimeIntroAudioCommand::PitchSweep { duration: self.nibble(id, 0, "sweep duration")?, pitch: self.signed_nibble(id, 1, "sweep pitch")? }
            }
            "square_note" => {
                self.require_args(id, 4)?;
                let frequency = self.integer::<u16>(id, 3, "frequency")?;
                anyhow::ensure!(frequency <= 0x07ff, "audio asset '{id}' square-note frequency exceeds 11 bits");
                RuntimeIntroAudioCommand::SquareNote { duration: self.integer(id, 0, "duration")?, volume: self.nibble(id, 1, "volume")?, fade: self.signed_nibble(id, 2, "fade")?, frequency }
            }
            "noise_note" => {
                self.require_args(id, 4)?;
                RuntimeIntroAudioCommand::NoiseNote { duration: self.integer(id, 0, "duration")?, volume: self.nibble(id, 1, "volume")?, fade: self.signed_nibble(id, 2, "fade")?, frequency: self.integer(id, 3, "frequency")? }
            }
            "sound_ret" => {
                self.require_args(id, 0)?;
                RuntimeIntroAudioCommand::SoundRet
            }
            command => anyhow::bail!("audio asset '{id}' uses unsupported CrystalIntro ASM command '{command}'"),
        };
        Ok(parsed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackAsmAudioSource {
    pub number: Option<u8>,
    pub commands: Vec<ModpackAsmAudioCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackAsmMusicData {
    pub channel_count: u8,
    pub channels: BTreeMap<String, ModpackAsmAudioSource>,
    pub subroutines: BTreeMap<String, ModpackAsmAudioSource>,
    #[serde(default)]
    pub shared_sources: BTreeMap<String, ModpackAsmAudioSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModpackCrystalMidiProgram {
    pub profile: String,
    pub music_data: ModpackAsmMusicData,
    pub cry_pitch: Option<i64>,
    pub cry_length: Option<i64>,
}

impl ModpackCrystalMidiProgram {
    fn validate(&self, id: &str) -> Result<()> {
        anyhow::ensure!(
            self.profile == "pokecrystal-midi-v1",
            "audio asset '{id}' embedded ASM program has unsupported profile '{}'",
            self.profile
        );
        anyhow::ensure!(
            self.music_data.channel_count > 0
                && usize::from(self.music_data.channel_count)
                    == self.music_data.channels.len(),
            "audio asset '{id}' embedded ASM program has an invalid channel count"
        );
        let mut channel_numbers = BTreeSet::new();
        for (label, source) in &self.music_data.channels {
            anyhow::ensure!(
                !label.is_empty() && !source.commands.is_empty(),
                "audio asset '{id}' embedded ASM channel '{label}' is empty"
            );
            let number = source.number.with_context(|| {
                format!("audio asset '{id}' embedded ASM channel '{label}' has no number")
            })?;
            anyhow::ensure!(
                (1..=8).contains(&number) && channel_numbers.insert(number),
                "audio asset '{id}' embedded ASM channel number {number} is invalid or duplicated"
            );
        }
        for (label, source) in self
            .music_data
            .subroutines
            .iter()
            .chain(&self.music_data.shared_sources)
        {
            anyhow::ensure!(
                !label.is_empty() && !source.commands.is_empty(),
                "audio asset '{id}' embedded ASM source '{label}' is empty"
            );
        }
        for command in self
            .music_data
            .channels
            .values()
            .chain(self.music_data.subroutines.values())
            .chain(self.music_data.shared_sources.values())
            .flat_map(|source| &source.commands)
        {
            anyhow::ensure!(
                !command.command.is_empty() && command.args.iter().all(|arg| !arg.is_empty()),
                "audio asset '{id}' embedded ASM program contains an empty command token"
            );
        }
        Ok(())
    }

    pub fn intro_channel_programs(
        &self,
        id: &str,
    ) -> Result<BTreeMap<u8, Vec<RuntimeIntroAudioCommand>>> {
        self.validate(id)?;
        anyhow::ensure!(
            self.music_data.subroutines.is_empty() && self.music_data.shared_sources.is_empty(),
            "audio asset '{id}' CrystalIntro program uses unsupported auxiliary sources"
        );
        let mut channels = BTreeMap::new();
        for source in self.music_data.channels.values() {
            let number = source
                .number
                .context("validated CrystalIntro channel has no number")?;
            let commands = source
                .commands
                .iter()
                .map(|command| command.intro_command(id))
                .collect::<Result<Vec<_>>>()?;
            anyhow::ensure!(
                matches!(commands.first(), Some(RuntimeIntroAudioCommand::Label(_)))
                    && matches!(commands.last(), Some(RuntimeIntroAudioCommand::SoundRet)),
                "audio asset '{id}' CrystalIntro channel {number} is not label/return delimited"
            );
            anyhow::ensure!(
                channels.insert(number, commands).is_none(),
                "audio asset '{id}' duplicates CrystalIntro channel {number}"
            );
        }
        Ok(channels)
    }
}

const POKECRYSTAL_MIDI_MAGIC: &[u8] = b"POKECRYSTAL-MIDI-1\0";

fn read_midi_u32(bytes: &[u8], offset: usize, id: &str) -> Result<u32> {
    let encoded = bytes
        .get(offset..offset + 4)
        .with_context(|| format!("audio asset '{id}' MIDI contains a truncated u32"))?;
    Ok(u32::from_be_bytes(encoded.try_into().expect("four-byte slice")))
}

fn read_midi_vlq(bytes: &[u8], mut offset: usize, end: usize, id: &str) -> Result<(u32, usize)> {
    let mut value = 0_u32;
    for _ in 0..4 {
        anyhow::ensure!(offset < end, "audio asset '{id}' MIDI contains a truncated VLQ");
        let byte = bytes[offset];
        offset += 1;
        value = value
            .checked_shl(7)
            .and_then(|value| value.checked_add(u32::from(byte & 0x7f)))
            .context("MIDI VLQ overflowed")?;
        if byte & 0x80 == 0 {
            return Ok((value, offset));
        }
    }
    anyhow::bail!("audio asset '{id}' MIDI contains an invalid VLQ")
}

fn crystal_midi_program_from_bytes(bytes: &[u8], id: &str) -> Result<ModpackCrystalMidiProgram> {
    anyhow::ensure!(
        bytes.starts_with(b"MThd"),
        "audio asset '{id}' MIDI is missing its header"
    );
    let header_length = usize::try_from(read_midi_u32(bytes, 4, id)?)
        .context("MIDI header length exceeds usize")?;
    let mut chunk_offset = 8_usize
        .checked_add(header_length)
        .context("MIDI header offset overflowed")?;
    while chunk_offset + 8 <= bytes.len() {
        let chunk_type = &bytes[chunk_offset..chunk_offset + 4];
        let chunk_length = usize::try_from(read_midi_u32(bytes, chunk_offset + 4, id)?)
            .context("MIDI chunk length exceeds usize")?;
        let start = chunk_offset + 8;
        let end = start
            .checked_add(chunk_length)
            .context("MIDI chunk end overflowed")?;
        anyhow::ensure!(end <= bytes.len(), "audio asset '{id}' MIDI chunk is truncated");
        if chunk_type == b"MTrk" {
            let mut offset = start;
            let mut running_status = None;
            while offset < end {
                (_, offset) = read_midi_vlq(bytes, offset, end, id)?;
                anyhow::ensure!(offset < end, "audio asset '{id}' MIDI event is truncated");
                let status = if bytes[offset] < 0x80 {
                    running_status.context("MIDI running status has no preceding channel event")?
                } else {
                    let status = bytes[offset];
                    offset += 1;
                    if status < 0xf0 {
                        running_status = Some(status);
                    }
                    status
                };
                if status == 0xff {
                    anyhow::ensure!(offset < end, "audio asset '{id}' MIDI meta event is truncated");
                    let event_type = bytes[offset];
                    offset += 1;
                    let (length, payload_start) = read_midi_vlq(bytes, offset, end, id)?;
                    let payload_end = payload_start
                        .checked_add(usize::try_from(length).context("MIDI event length exceeds usize")?)
                        .context("MIDI event end overflowed")?;
                    anyhow::ensure!(payload_end <= end, "audio asset '{id}' MIDI meta payload is truncated");
                    let payload = &bytes[payload_start..payload_end];
                    if event_type == 0x7f && payload.starts_with(POKECRYSTAL_MIDI_MAGIC) {
                        let program: ModpackCrystalMidiProgram = serde_json::from_slice(
                            &payload[POKECRYSTAL_MIDI_MAGIC.len()..],
                        )
                        .with_context(|| format!("decode audio asset '{id}' ASM program"))?;
                        program.validate(id)?;
                        return Ok(program);
                    }
                    offset = payload_end;
                    continue;
                }
                if status == 0xf0 || status == 0xf7 {
                    let (length, payload_start) = read_midi_vlq(bytes, offset, end, id)?;
                    offset = payload_start
                        .checked_add(usize::try_from(length).context("MIDI SysEx length exceeds usize")?)
                        .context("MIDI SysEx end overflowed")?;
                    anyhow::ensure!(offset <= end, "audio asset '{id}' MIDI SysEx payload is truncated");
                    continue;
                }
                anyhow::ensure!(status < 0xf0, "audio asset '{id}' MIDI has an unsupported system event");
                let command = status & 0xf0;
                let data_length = if command == 0xc0 || command == 0xd0 { 1 } else { 2 };
                offset = offset
                    .checked_add(data_length)
                    .context("MIDI channel event end overflowed")?;
                anyhow::ensure!(offset <= end, "audio asset '{id}' MIDI channel event is truncated");
            }
        }
        chunk_offset = end;
    }
    anyhow::bail!("audio asset '{id}' MIDI has no embedded PokeCrystal ASM program")
}

impl ModpackMidiAudioProgram {
    pub fn crystal_program(&self, id: &str) -> Result<ModpackCrystalMidiProgram> {
        use base64::Engine as _;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&self.midi_base64)
            .with_context(|| format!("decode audio asset '{id}' MIDI payload"))?;
        crystal_midi_program_from_bytes(&bytes, id)
    }

    pub(crate) fn validate(&self, id: &str) -> Result<()> {
        if self.profile != "pokecrystal-midi-v1" {
            anyhow::bail!(
                "audio asset '{id}' has unsupported MIDI profile '{}'",
                self.profile
            );
        }
        use base64::Engine as _;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&self.midi_base64)
            .with_context(|| format!("decode audio asset '{id}' MIDI payload"))?;
        if bytes.len() < 22 || !bytes.starts_with(b"MThd") || &bytes[14..18] != b"MTrk" {
            anyhow::bail!("audio asset '{id}' does not contain a valid MIDI file");
        }
        Ok(())
    }
}

impl ModpackPcmAudioFormat {
    fn validate(&self, id: &str) -> Result<()> {
        if self.sample_rate_hz != 22_050 || self.channels != 2 || self.bits_per_sample != 16 {
            anyhow::bail!(
                "PCM audio asset '{id}' must use canonical 22050 Hz stereo signed 16-bit format"
            );
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

fn canonical_pcm_format() -> ModpackPcmAudioFormat {
    ModpackPcmAudioFormat {
        sample_rate_hz: 22_050,
        channels: 2,
        bits_per_sample: 16,
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
    pub sfx_priority: Option<u8>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub midi_program: Option<ModpackMidiAudioProgram>,
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
            sfx_priority: Option<u8>,
            pcm_format: Option<ModpackPcmAudioFormat>,
            pcm_frame_count: Option<usize>,
            payload_hash: Option<String>,
            loop_start_sample: Option<usize>,
            loop_end_sample: Option<usize>,
            midi_program: Option<ModpackMidiAudioProgram>,
        }

        let raw = RawModpackAudioAsset::deserialize(deserializer)?;
        let asset = Self {
            id: raw.id,
            path: raw.path,
            kind: raw.kind,
            source: raw.source,
            sfx_priority: raw.sfx_priority,
            pcm_format: raw.pcm_format,
            pcm_frame_count: raw.pcm_frame_count,
            payload_hash: raw.payload_hash,
            loop_start_sample: raw.loop_start_sample,
            loop_end_sample: raw.loop_end_sample,
            midi_program: raw.midi_program,
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
            source: ModpackAudioSource::Pcm,
            sfx_priority: None,
            pcm_format: Some(canonical_pcm_format()),
            pcm_frame_count: None,
            payload_hash: None,
            loop_start_sample: None,
            loop_end_sample: None,
            midi_program: None,
        };
        asset.validate()?;
        Ok(asset)
    }

    pub fn cry(id: impl Into<String>, path: impl Into<String>) -> Result<Self> {
        let asset = Self {
            id: id.into(),
            path: path.into(),
            kind: ModpackAudioKind::Cry,
            source: ModpackAudioSource::Pcm,
            sfx_priority: None,
            pcm_format: Some(canonical_pcm_format()),
            pcm_frame_count: None,
            payload_hash: None,
            loop_start_sample: None,
            loop_end_sample: None,
            midi_program: None,
        };
        asset.validate()?;
        Ok(asset)
    }

    pub fn sound_effect(
        id: impl Into<String>,
        path: impl Into<String>,
        sfx_priority: u8,
    ) -> Result<Self> {
        let asset = Self {
            id: id.into(),
            path: path.into(),
            kind: ModpackAudioKind::SoundEffect,
            source: ModpackAudioSource::Pcm,
            sfx_priority: Some(sfx_priority),
            pcm_format: Some(canonical_pcm_format()),
            pcm_frame_count: None,
            payload_hash: None,
            loop_start_sample: None,
            loop_end_sample: None,
            midi_program: None,
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
            sfx_priority: None,
            pcm_format: Some(pcm_format),
            pcm_frame_count: None,
            payload_hash: None,
            loop_start_sample: None,
            loop_end_sample: None,
            midi_program: None,
        };
        asset.validate()?;
        Ok(asset)
    }

    pub fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            anyhow::bail!("audio asset id is required");
        }
        validate_modpack_audio_id(self.kind, &self.id)?;
        match (self.kind, self.sfx_priority) {
            (ModpackAudioKind::SoundEffect, None) => {
                anyhow::bail!("sound-effect audio asset '{}' must declare sfx_priority", self.id)
            }
            (ModpackAudioKind::Music | ModpackAudioKind::Cry, Some(_)) => anyhow::bail!(
                "non-SFX audio asset '{}' must not declare sfx_priority",
                self.id
            ),
            _ => {}
        }
        let path = Path::new(&self.path);
        if let Some(program) = &self.midi_program {
            program.validate(&self.id)?;
        }
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
                anyhow::bail!("PCM audio asset '{}' must use a .pcm file", self.id)
            }
            ModpackAudioSource::Midi if extension == "mid" => {
                let Some(format) = &self.pcm_format else {
                    anyhow::bail!("MIDI audio asset '{}' must declare output pcm_format", self.id);
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
            ModpackAudioSource::Midi => {
                anyhow::bail!("MIDI audio asset '{}' must use a .mid file", self.id)
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
    pub sfx_priority: Option<u8>,
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
            sfx_priority: Option<u8>,
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
            sfx_priority: raw.sfx_priority,
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
            sfx_priority: self.sfx_priority,
            pcm_format: self.pcm_format.clone(),
            pcm_frame_count: None,
            payload_hash: None,
            loop_start_sample: None,
            loop_end_sample: None,
            midi_program: None,
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
            ModpackAudioSource::Pcm | ModpackAudioSource::Midi => {
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
                    let format = asset.pcm_format.as_ref().with_context(|| {
                        format!(
                            "PCM audio asset '{}' missing validated pcm_format",
                            asset.id
                        )
                    })?;
                    let frame_count = Some(bytes.len() / format.frame_size_bytes(&asset.id)?);
                    (
                        bytes.len(),
                        format!("{:08x}", fnv1a32_bytes(bytes)),
                        frame_count,
                    )
                }
                (ModpackAudioSource::Pcm | ModpackAudioSource::Midi, None) => {
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
            };
            let entry = ModpackAudioManifestEntry {
                id: asset.id.clone(),
                path: asset.path.clone(),
                kind: asset.kind,
                source: asset.source,
                sfx_priority: asset.sfx_priority,
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
