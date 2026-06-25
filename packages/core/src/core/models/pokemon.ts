import { z } from "zod";
import { PlayerGender } from "@pokecrystal/core/core/models/settings";
import { MoveName, MoveNameSchema } from "@pokecrystal/core/core/enums/move";
import {
  GrowthRateSchema,
  GenderRatioSchema,
  EggGroupSchema,
  AbilitySchema,
  Stat,
  StatSchema,
  GrowthRate,
} from "@pokecrystal/core/core/enums/pokemon";
import {
  StatusCondition,
  BattleTurn,
} from "@pokecrystal/core/core/enums/battle";
import { PokemonTypeSchema } from "@pokecrystal/core/core/enums/pokemon";
import { MailMessageSchema } from "@pokecrystal/core/core/mail";
import { GameState } from "@pokecrystal/core/core/state";
import { HardwareRNG } from "@pokecrystal/core/engine/games/rng";
import { calculateExperience } from "@pokecrystal/core/engine/experience";
import { defaultMovesForLevel } from "@pokecrystal/core/engine/systems/learnsets";
import { Move, loadAllMoves } from "./move";

export const DVSchema = z.object({
  attack: z.number().default(0),
  defense: z.number().default(0),
  speed: z.number().default(0),
  special: z.number().default(0),
  hp: z.number().default(0),
});
export type DV = z.infer<typeof DVSchema>;

export const BaseStatsSchema = z.object({
  hp: z.number(),
  attack: z.number(),
  defense: z.number(),
  speed: z.number(),
  special_attack: z.number(),
  special_defense: z.number(),
});
export type BaseStats = z.infer<typeof BaseStatsSchema>;

export const PokemonSpeciesSchema = z.object({
  evolutions: z.array(z.any()).nullish().default(null),
  id: z.string(),
  int_id: z.number(),
  base_stats: BaseStatsSchema,
  type1: PokemonTypeSchema,
  type2: PokemonTypeSchema,
  catch_rate: z.number(),
  base_exp: z.number(),
  item1: z.string().nullable().optional(),
  item2: z.string().nullable().optional(),
  gender_ratio: GenderRatioSchema,
  unknown1: z.number(),
  step_cycles_to_hatch: z.number(),
  unknown2: z.number(),
  growth_rate: GrowthRateSchema,
  egg_group1: EggGroupSchema,
  egg_group2: EggGroupSchema,
  tmhm_learnset: z.array(MoveNameSchema).default([]),
  ability: AbilitySchema.default("NONE"),
  pic_size: z.number().default(0),
  front_pic: z.number().default(0),
  back_pic: z.number().default(0),
  weight: z.number().default(0),
});
export type PokemonSpecies = z.infer<typeof PokemonSpeciesSchema>;

export const LearnedMoveSchema = z.object({
  name: MoveNameSchema,
  current_pp: z.number(),
  pp_ups: z.number(),
});
export type LearnedMove = z.infer<typeof LearnedMoveSchema>;

const nullToUndefined = (value: unknown): unknown => (value === null ? undefined : value);

const nullableOptional = <T extends z.ZodTypeAny>(schema: T) =>
  z.preprocess(nullToUndefined, schema.optional());

const DEFAULT_STAT_BOOSTS: Record<Stat, number> = {
  HP: 0,
  ATTACK: 0,
  DEFENSE: 0,
  SPEED: 0,
  SPECIAL_ATTACK: 0,
  SPECIAL_DEFENSE: 0,
  ACCURACY: 0,
  EVASION: 0,
};

const StatBoostsSchema = z
  .preprocess(
    (value) => {
      if (value === null || value === undefined) {
        return undefined;
      }
      if (typeof value === "object" && value !== null && Object.keys(value).length === 0) {
        return undefined;
      }
      return value;
    },
    z.record(StatSchema, z.number()).optional()
  )
  .transform((value) => ({
    ...DEFAULT_STAT_BOOSTS,
    ...(value ?? {}),
  }));

export const PokemonSchema = z.object({
  species: PokemonSpeciesSchema,
  nickname: z.string(),
  gender: z.nativeEnum(PlayerGender).nullable().optional(),
  item: z.string().nullable().optional(),
  mail: MailMessageSchema.optional(),
  moves: z.array(LearnedMoveSchema).default([]),
  level: z.number(),
  hp: z.number(),
  max_hp: z.number(),
  dvs: DVSchema.default({ attack: 0, defense: 0, speed: 0, special: 0, hp: 0 }),
  status: nullableOptional(z.nativeEnum(StatusCondition)),
  sleep_turns: z.number().default(0),
  flinching: z.boolean().default(false),
  rampage_turns: z.number().default(0),
  confusion_turns: z.number().default(0),
  perish_song_turns: z.number().default(0),
  focus_energy: z.boolean().default(false),
  original_trainer_name: z.string(),
  original_trainer_id: z.number(),
  experience: z.number(),
  hp_exp: z.number().default(0),
  attack_exp: z.number().default(0),
  defense_exp: z.number().default(0),
  speed_exp: z.number().default(0),
  special_exp: z.number().default(0),
  happiness: z.number(),
  turns_in_battle: z.number().default(0),
  stat_boosts: StatBoostsSchema,
  locked_move: nullableOptional(z.nativeEnum(MoveName)),
  locked_turns_remaining: z.number().default(0),
  trapped_turns: z.number().default(0),
  trapped_by_side: nullableOptional(z.nativeEnum(BattleTurn)),
  trapped_source_index: nullableOptional(z.number()),
  trapped_move: nullableOptional(MoveNameSchema),
  leech_seeded: z.boolean().default(false),
  leech_seed_source_side: nullableOptional(z.nativeEnum(BattleTurn)),
  nightmare: z.boolean().default(false),
  cursed: z.boolean().default(false),
  curse_source_side: nullableOptional(z.nativeEnum(BattleTurn)),
  attack: z.number().default(0),
  defense: z.number().default(0),
  speed: z.number().default(0),
  special_attack: z.number().default(0),
  special_defense: z.number().default(0),
  last_move_used: nullableOptional(MoveNameSchema),
  disabled_move: nullableOptional(MoveNameSchema),
  disable_turns: z.number().default(0),
  encore_turns_remaining: z.number().default(0),
  encored_move: nullableOptional(MoveNameSchema),
  destiny_bond_active: z.boolean().default(false),
  destiny_bond_action_id: nullableOptional(z.number()),
  pokerus: z.boolean().default(false),
  rage_active: z.boolean().default(false),
  rage_counter: z.number().default(0),
  fury_cutter_count: z.number().default(0),
  rollout_step: z.number().default(0),
  rollout_active: z.boolean().default(false),
  defense_curled: z.boolean().default(false),
  cant_run: z.boolean().default(false),
  bide_active: z.boolean().default(false),
  bide_turns_remaining: z.number().default(0),
  bide_damage: z.number().default(0),
  protect_active: z.boolean().default(false),
  protect_counter: z.number().default(0),
  endure_active: z.boolean().default(false),
  endure_counter: z.number().default(0),
  attract_source_side: nullableOptional(z.nativeEnum(BattleTurn)),
  foresight_active: z.boolean().default(false),
  lock_on_active: z.boolean().default(false),
  lock_on_target_index: nullableOptional(z.number()),
  substitute_hp: z.number().default(0),
  transformed: z.boolean().default(false),
  original_species: PokemonSpeciesSchema.optional(),
  transform_backup_stats: z.record(z.string(), z.number()).optional(),
  transform_backup_dvs: DVSchema.optional(),
  transform_backup_moves: z.array(LearnedMoveSchema).optional(),
  transform_backup_stat_boosts: z.record(StatSchema, z.number()).optional(),
  last_damage_taken: z.number().default(0),
  last_damage_type: nullableOptional(PokemonTypeSchema),
});
export type PokemonData = z.infer<typeof PokemonSchema>;

export type Pokemon = PokemonData & {
  _statExpForStat: (stat: Stat) => number;
  _calculateStat: (stat: Stat) => number;
};

const ASM_SPECIES_DEFAULT_NICKNAMES: Readonly<Record<string, string>> = {
  FARFETCH_D: "FARFETCH'D",
  HO_OH: "HO-OH",
  MR__MIME: "MR.MIME",
  NIDORAN_F: "NIDORAN\u2640",
  NIDORAN_M: "NIDORAN\u2642",
};

export function pokemonSpeciesDisplayName(species: PokemonSpecies | { id?: string } | string): string {
  const id = typeof species === "string" ? species : species.id;
  const normalized = String(id ?? "").toUpperCase();
  return ASM_SPECIES_DEFAULT_NICKNAMES[normalized] ?? normalized;
}

function isqrt(n: number): number {
  if (n < 0) {
    throw new Error("isqrt() argument must be non-negative");
  }
  if (n === 0) {
    return 0;
  }
  let x = Math.floor(Math.sqrt(n));
  if (Math.pow(x + 1, 2) <= n) {
    x += 1;
  }
  return x;
}

export function toPokemon(pokemon: PokemonData): Pokemon {
  const _statExpForStat = (stat: Stat): number => {
    if (stat === "HP") return pokemon.hp_exp;
    if (stat === "ATTACK") return pokemon.attack_exp;
    if (stat === "DEFENSE") return pokemon.defense_exp;
    if (stat === "SPEED") return pokemon.speed_exp;
    if (stat === "SPECIAL_ATTACK" || stat === "SPECIAL_DEFENSE")
      return pokemon.special_exp;
    throw new Error(`Unsupported stat for Stat Exp lookup: ${stat}`);
  };

  const _calculateStat = (stat: Stat): number => {
    const _dv_for_stat = (stat: Stat): number => {
      if (stat === "HP") return pokemon.dvs.hp;
      if (stat === "ATTACK") return pokemon.dvs.attack;
      if (stat === "DEFENSE") return pokemon.dvs.defense;
      if (stat === "SPEED") return pokemon.dvs.speed;
      if (stat === "SPECIAL_ATTACK" || stat === "SPECIAL_DEFENSE")
        return pokemon.dvs.special;
      throw new Error(`Unsupported stat for DV lookup: ${stat}`);
    };

    const base_stats = pokemon.species.base_stats;
    let base: number;
    if (stat === "HP") base = base_stats.hp;
    else if (stat === "ATTACK") base = base_stats.attack;
    else if (stat === "DEFENSE") base = base_stats.defense;
    else if (stat === "SPEED") base = base_stats.speed;
    else if (stat === "SPECIAL_ATTACK") base = base_stats.special_attack;
    else if (stat === "SPECIAL_DEFENSE")
      base = base_stats.special_defense;
    else throw new Error(`Unsupported stat for calculation: ${stat}`);

    const dv = _dv_for_stat(stat);
    const stat_exp = _statExpForStat(stat);
    const level = pokemon.level;
    let exp_modifier = Math.floor(
      Math.min(255, isqrt(stat_exp)) / 4
    );
    if (stat === Stat.HP && stat_exp === 65535) {
      exp_modifier += 1;
    }
    const interim_value = (base + dv) * 2 + exp_modifier;
    const main_stat_component = Math.floor(
      (interim_value * level) / 100
    );
    if (stat === Stat.HP) {
      return main_stat_component + level + 10;
    }
    return main_stat_component + 5;
  };

  return {
    _calculateStat,
    _statExpForStat,
    ...pokemon,
  } as Pokemon;
}


function _calculate_stats(species: PokemonSpecies, level: number, dvs: DV): Record<string, number> {
    const stats: Record<string, number> = {};
    const temp_pokemon_schema: PokemonData = PokemonSchema.parse({
        species,
        nickname: "",
        level,
        hp: 0,
        max_hp: 0,
        original_trainer_name: "",
        original_trainer_id: 0,
        experience: 0,
        happiness: 0,
        dvs,
    });
    const temp_pokemon = toPokemon(temp_pokemon_schema);
    stats["max_hp"] = temp_pokemon._calculateStat(Stat.HP);
    stats["attack"] = temp_pokemon._calculateStat(Stat.ATTACK);
    stats["defense"] = temp_pokemon._calculateStat(Stat.DEFENSE);
    stats["speed"] = temp_pokemon._calculateStat(Stat.SPEED);
    stats["special_attack"] = temp_pokemon._calculateStat(Stat.SPECIAL_ATTACK);
    stats["special_defense"] = temp_pokemon._calculateStat(Stat.SPECIAL_DEFENSE);
    return stats;
}

export function createPokemon(gameState: GameState, species: PokemonSpecies, level: number): Pokemon {
    const rng = new HardwareRNG(gameState);
    const attack_dv = rng.randrange(16);
    const defense_dv = rng.randrange(16);
    const speed_dv = rng.randrange(16);
    const special_dv = rng.randrange(16);

    let hp_dv = 0;
    if (attack_dv % 2) hp_dv += 8;
    if (defense_dv % 2) hp_dv += 4;
    if (speed_dv % 2) hp_dv += 2;
    if (special_dv % 2) hp_dv += 1;

    const dvs: DV = {
        attack: attack_dv,
        defense: defense_dv,
        speed: speed_dv,
        special: special_dv,
        hp: hp_dv,
    };

    const stats = _calculate_stats(species, level, dvs);
    let experience = calculateExperience(species.growth_rate, level);

    const _build_default_moves = (species_id: string, level: number): LearnedMove[] => {
        const learned: LearnedMove[] = [];
        const moves = defaultMovesForLevel(species_id, level);
        const allMoves = loadAllMoves();
        for (const move_name of moves) {
            const move_data: Move | undefined = allMoves[move_name];
            const pp = move_data ? move_data.pp : 0;
            learned.push({ name: move_name, current_pp: pp, pp_ups: 0 });
        }
        return learned;
    }

    const pokemon_schema: PokemonData = {
        species: species,
        nickname: pokemonSpeciesDisplayName(species),
        moves: _build_default_moves(species.id, level),
        level: level,
        hp: stats["max_hp"],
        max_hp: stats["max_hp"],
        dvs: dvs,
        attack: stats["attack"],
        defense: stats["defense"],
        speed: stats["speed"],
        special_attack: stats["special_attack"],
        special_defense: stats["special_defense"],
        original_trainer_name: "PLAYER",
        original_trainer_id: 0,
        experience: experience,
        happiness: 70,
    } as PokemonData;
    return toPokemon(PokemonSchema.parse(pokemon_schema));
}
