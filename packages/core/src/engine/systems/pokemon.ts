import { GameState } from "../../core/state";
import { Pokemon, PokemonSchema, PokemonSpecies, LearnedMove, DV, toPokemon, pokemonSpeciesDisplayName } from "../../core/models";
import { GrowthRate, MoveName } from "../../core/enums";
import { HardwareRNG } from "../games/rng";
import { calculateExperience } from "../experience";
import { defaultMovesForLevel } from "./learnsets";
import { loadMergedMovesDataSync } from "../../core/content-packs";
import { normalizeDvs } from "../../core/pokemon-dvs";

const memoize = <T>(fn: () => T): () => T => {
  let initialized = false;
  let result: T;
  return () => {
    if (!initialized) {
      result = fn();
      initialized = true;
    }
    return result;
  };
};

const loadMovesData = memoize(() => {
  return loadMergedMovesDataSync() as Record<string, { pp: number }>;
});

const movePpMap = memoize((): Map<MoveName, number> => {
  const mapping = new Map<MoveName, number>();
  for (const [name, details] of Object.entries(loadMovesData())) {
    mapping.set(name as MoveName, (details as { pp: number }).pp);
  }
  return mapping;
});

export function createPokemon(
  gameState: GameState,
  species: PokemonSpecies,
  level: number,
  options: { dvs?: DV } = {}
): Pokemon {
  // Mirrors GeneratePartyMonStats / CalcMonStats in pokecrystal_disassembly/engine/pokemon/move_mon.asm.
  const rng = new HardwareRNG(gameState);
  const dvs: DV = options.dvs
    ? normalizeDvs(options.dvs)
    : normalizeDvs({
        attack: rng.randrange(16),
        defense: rng.randrange(16),
        speed: rng.randrange(16),
        special: rng.randrange(16),
      });

  const baseStats = species.base_stats;
  const maxHp = Math.floor(((baseStats.hp + dvs.hp) * 2 * level) / 100) + level + 10;
  const attackStat = Math.floor(((baseStats.attack + dvs.attack) * 2 * level) / 100) + 5;
  const defenseStat = Math.floor(((baseStats.defense + dvs.defense) * 2 * level) / 100) + 5;
  const speedStat = Math.floor(((baseStats.speed + dvs.speed) * 2 * level) / 100) + 5;
  const specialAttackStat =
    Math.floor(((baseStats.special_attack + dvs.special) * 2 * level) / 100) + 5;
  const specialDefenseStat =
    Math.floor(((baseStats.special_defense + dvs.special) * 2 * level) / 100) + 5;

  const experience = Math.max(
    0,
    calculateExperience(species.growth_rate as GrowthRate, level)
  );

  const ppMap = movePpMap();
  const learnedMoves: LearnedMove[] = defaultMovesForLevel(species.id, level).map(
    (move) => ({
      name: move,
      current_pp: ppMap.get(move) ?? 0,
      pp_ups: 0,
    })
  );

  return toPokemon(
    PokemonSchema.parse({
    species,
    nickname: pokemonSpeciesDisplayName(species),
    level,
    hp: maxHp,
    max_hp: maxHp,
    dvs,
    attack: attackStat,
    defense: defenseStat,
    speed: speedStat,
    special_attack: specialAttackStat,
    special_defense: specialDefenseStat,
    original_trainer_name: "PLAYER",
    original_trainer_id: 0,
    experience,
    happiness: 70,
    moves: learnedMoves,
  }));
}
