
import { z } from "zod";
import {
  PokemonType,
  GenderRatio,
  GrowthRate,
  EggGroup,
  Ability,
  MoveName,
  Stat,
  StatusCondition,
  PlayerGender,
  ItemPocket,
  BattleTurn,
  ItemEffect,
  BattleScene,
  BattleStyle,
  AILayer,
} from "./enums";
import {
  FrameType,
  MenuAccount,
  PrintOption,
  Sound,
  TextSpeed,
} from "./enums/ui-enums";
import { DAY_CARE_STEPS_PER_LEVEL, HALL_OF_FAME_TEAM_SIZE } from "./constants";
import {
  ItemSchema as ItemSchemaFromModels,
  type Item as ItemModel,
} from "./models/item";
import { DayCareControlRegisterSchema } from "./models/day-care";
import { BoxSchema, formatDefaultBoxName, validateBoxState } from "./models/box";

import {
  PokemonSpeciesSchema,
  PokemonSchema,
  LearnedMoveSchema,
  DVSchema,
  BaseStatsSchema,
  toPokemon,
  pokemonSpeciesDisplayName,
  type PokemonData,
  type Pokemon,
  type PokemonSpecies,
} from "./models/pokemon";

export {
  PokemonSpeciesSchema,
  PokemonSchema,
  LearnedMoveSchema,
  DVSchema,
  BaseStatsSchema,
  toPokemon,
  pokemonSpeciesDisplayName,
  PokemonData,
  Pokemon,
  PokemonSpecies,
};
import { PartySchema as ImportedPartySchema } from "./models/party";

export const MoveSchema = z.object({
  source_index: z.number().int().min(1).max(251),
  name: z.nativeEnum(MoveName),
  type: z.nativeEnum(PokemonType),
  power: z.number(),
  accuracy: z.number(),
  pp: z.number(),
  effect: z.string().default("NORMAL_HIT"),
  effect_chance: z.number().default(0),
  stat: z.preprocess(
    (value) => (value === null ? undefined : value),
    z.nativeEnum(Stat).optional(),
  ),
  amount: z.preprocess(
    (value) => (value === null ? undefined : value),
    z.number().optional(),
  ),
});


export const PartySchema = ImportedPartySchema;

export { BoxSchema, formatDefaultBoxName, validateBoxState };

export const TrainerSchema = z.object({
  name: z.string(),
  trainer_id: z.string().default(""),
  trainer_class: z.string().default(""),
  party: z.array(PokemonSchema),
  win_quote: z.string().default(""),
  lose_quote: z.string().default(""),
  items: z.array(z.string().nullable()).default([]),
  base_reward: z.number().default(0),
  ai_move_flags: z.number().default(0),
  ai_item_switch_flags: z.number().default(0),
  encounter_music: z.string().default(""),
  ai_layers: z.array(z.nativeEnum(AILayer)).default([]),
});


export const HallOfFamePokemonSchema = z.object({
  species: z.string().optional(),
  id: z.number().optional(),
  trainer_id: z.number().optional(),
  dvs: z.number().optional(),
  level: z.number().optional(),
  nickname: z.string().optional(),
});

const defaultHallOfFameTeam = (): z.infer<typeof HallOfFamePokemonSchema>[] =>
  Array.from({ length: HALL_OF_FAME_TEAM_SIZE }, () => ({}));

export const HallOfFameEntrySchema = z
  .object({
    win_count: z.number().default(0),
    team: z.array(HallOfFamePokemonSchema).default(defaultHallOfFameTeam),
    pokemon: z.array(HallOfFamePokemonSchema).optional(),
  })
  .transform((entry) => {
    const team = entry.team?.length ? entry.team : entry.pokemon ?? defaultHallOfFameTeam();
    const pokemon = entry.pokemon?.length ? entry.pokemon : team;
    return { ...entry, team, pokemon };
  });

export const LinkBattleStatsSchema = z.object({
  wins: z.number().default(0),
  losses: z.number().default(0),
  draws: z.number().default(0),
});

export const LinkBattleRecordSchema = z.object({
  name: z.string().default(''),
  id: z.number().default(0),
  stats: LinkBattleStatsSchema.default({ wins: 0, losses: 0, draws: 0 }),
});

export const MapAttributesSchema = z.object({
  environment: z.string(),
  connections: z.array(z.string()),
  tileset: z.string(),
  location: z.string(),
  music: z.string(),
  time_of_day: z.string(),
  fishing_group: z.string(),
});

export const MapConnectionSchema = z.object({
  direction: z.string(),
  destination: z.string(),
  offset: z.number(),
});

export const WarpEventSchema = z.object({
  x: z.number(),
  y: z.number(),
  target_map: z.string(),
  target_warp_id: z.number(),
});

export const CoordEventSchema = z.object({
  x: z.number(),
  y: z.number(),
  scene_id: z.string(),
  script_name: z.string(),
});

export const BackgroundEventSchema = z.object({
  x: z.number(),
  y: z.number(),
  event_type: z.string(),
  script: z.string(),
});

export const MapEventsSchema = z.object({
  warps: z.array(WarpEventSchema),
  coord_events: z.array(CoordEventSchema),
  bg_events: z.array(BackgroundEventSchema),
});

export const ObjectEventSchema = z.object({
  sprite: z.string(),
  x: z.number(),
  y: z.number(),
  movement: z.string(),
  radius_x: z.number(),
  radius_y: z.number(),
  hour: z.number(),
  time_of_day: z.string(),
  color: z.string(),
  sight_range: z.number(),
  script: z.string(),
  event_flag: z.string(),
});

export const RoamingPokemonSchema = z.object({
  species: z.string().default(""),
  level: z.number().default(0),
  map_group: z.number().default(-1),
  map_number: z.number().default(-1),
  hp: z.number().default(0),
  dvs: z.number().default(0),
});

export const ItemSchema = ItemSchemaFromModels;
export { DayCareControlRegisterSchema };

export const OptionsSchema = z.object({
  text_speed: z.nativeEnum(TextSpeed).default(TextSpeed.FAST),
  battle_scene: z.nativeEnum(BattleScene).default(BattleScene.ON),
  battle_style: z.nativeEnum(BattleStyle).default(BattleStyle.SHIFT),
  sound: z.nativeEnum(Sound).default(Sound.STEREO),
  menu_account: z.nativeEnum(MenuAccount).default(MenuAccount.ON),
  print_option: z.nativeEnum(PrintOption).default(PrintOption.NORMAL),
  frame: z.nativeEnum(FrameType).default(FrameType.FRAME_1),
  no_text_scroll: z.boolean().default(false),
});

export const TimeSchema = z.object({
  day: z.number().default(0),
  hour: z.number().default(0),
  minute: z.number().default(0),
  second: z.number().default(0),
});

export const DateSchema = z.object({
  day: z.number().default(0),
  month: z.number().default(0),
  year: z.number().default(0),
});

export const BadgesSchema = z.object({
  johto: z.array(z.boolean()).default(Array(8).fill(false)),
  kanto: z.array(z.boolean()).default(Array(8).fill(false)),
});

export const BugContestStateSchema = z.object({
  timer_active: z.boolean().default(false),
  park_balls_remaining: z.number().default(20),
  caught_species: z.string().optional(),
  caught_level: z.number().optional(),
  pending_caught_mon: PokemonSchema.optional(),
  party_backup: z.array(PokemonSchema.nullable()).default([]),
});

export const BugContestWinnerSchema = z.object({
  winner_id: z.number().default(0),
  mon_species: z.string().default(""),
  score: z.number().default(0),
});

const defaultBugContestWinner = () => ({ winner_id: 0, mon_species: "", score: 0 });

export const BugContestResultsSchema = z.object({
  first_place: BugContestWinnerSchema.default(defaultBugContestWinner),
  second_place: BugContestWinnerSchema.default(defaultBugContestWinner),
  third_place: BugContestWinnerSchema.default(defaultBugContestWinner),
  temp_winner: BugContestWinnerSchema.default(defaultBugContestWinner),
  winner_name: z.string().default(""),
});

export const BugContestTimerSchema = z.object({
  mins_remaining: z.number().default(0),
  secs_remaining: z.number().default(0),
  start_time: z.array(z.number()).default([0, 0, 0, 0]),
  started_at: z.number().optional(),
});

export const DayCareResidentSchema = z.object({
  pokemon: PokemonSchema.optional(),
  initial_experience: z.number().int().default(0),
  initial_level: z.number().int().default(0),
  steps: z.number().int().default(0),
});

export const DayCareSchema = z.object({
  man: DayCareResidentSchema.default(() =>
    DayCareResidentSchema.parse({})
  ),
  lady: DayCareResidentSchema.default(() =>
    DayCareResidentSchema.parse({})
  ),
  man_register: DayCareControlRegisterSchema.default(() =>
    DayCareControlRegisterSchema.parse({})
  ),
  lady_register: DayCareControlRegisterSchema.default(() =>
    DayCareControlRegisterSchema.parse({})
  ),
  compatibility_score: z.number().int().default(0),
  egg_present: z.boolean().default(false),
  egg: PokemonSchema.optional(),
  steps_since_last_egg: z.number().int().default(0),
  steps_until_next_egg: z.number().int().default(0),
});

export const TileBlockSchema = z.object({
  data: z.array(z.number()),
});

export const BackgroundMapSchema = z.object({
  width: z.number().default(32),
  height: z.number().default(32),
  tiles: z.array(z.number()),
  attributes: z.array(z.number()),
});

export const VRAMBankSchema = z.object({
  tile_blocks: z.record(z.string(), TileBlockSchema).default({}),
  sprite_pages: z.record(z.string(), TileBlockSchema).default({}),
  bg_maps: z.record(z.string(), BackgroundMapSchema).default({}),
});

export const VRAMSchema = z.object({
  bank0: VRAMBankSchema,
  bank1: VRAMBankSchema,
});

export type Party = z.infer<typeof PartySchema>;
export type Box = z.infer<typeof BoxSchema>;
export type Trainer = z.infer<typeof TrainerSchema>;
export type HallOfFamePokemon = z.infer<typeof HallOfFamePokemonSchema>;
export type HallOfFameEntry = z.infer<typeof HallOfFameEntrySchema>;
export type LinkBattleStats = z.infer<typeof LinkBattleStatsSchema>;
export type LinkBattleRecord = z.infer<typeof LinkBattleRecordSchema>;
export type RoamingPokemon = z.infer<typeof RoamingPokemonSchema>;
export type Options = z.infer<typeof OptionsSchema>;
export type Time = z.infer<typeof TimeSchema>;
export type Badges = z.infer<typeof BadgesSchema>;
export type BugContestState = z.infer<typeof BugContestStateSchema>;
export type BugContestResults = z.infer<typeof BugContestResultsSchema>;
export type BugContestTimer = z.infer<typeof BugContestTimerSchema>;
export type DayCareResident = z.infer<typeof DayCareResidentSchema>;
export type DayCare = z.infer<typeof DayCareSchema>;
export type VRAM = z.infer<typeof VRAMSchema>;
export type Move = z.infer<typeof MoveSchema>;
export type DV = z.infer<typeof DVSchema>;
export type Date = z.infer<typeof DateSchema>;
export type WarpEvent = z.infer<typeof WarpEventSchema>;
export type MapEvents = z.infer<typeof MapEventsSchema>;
export type ObjectEvent = z.infer<typeof ObjectEventSchema>;
export type LearnedMove = z.infer<typeof LearnedMoveSchema>;
export type Item = ItemModel;
export type ItemData = ItemModel;
