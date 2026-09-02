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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTitleMainMenuItem {
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
            .map(|(label, dispatch_target)| {
                Ok(RuntimeTitleMainMenuItem {
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
        Ok(Self {
            scene_labels,
            scene_operation_offsets: scene_offsets,
            completion_wait_frames,
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
                "sample_input" => {
                    self.memory
                        .insert(string("result")?.to_string(), u16::from(input));
                }
                "write_memory_byte" | "write_memory_word" => {
                    let value = numeric("value")?;
                    self.memory.insert(string("target")?.to_string(), value);
                    effects.push(operation);
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
                "fill_memory_from_result"
                | "fill_strided_memory_from_transformed_result"
                | "play_audio" => effects.push(operation),
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

impl ModpackMidiAudioProgram {
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
