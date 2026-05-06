import { GameState } from "../../../core/state";
import { HardwareRNG } from "../../games/rng";
import { MoveName, Stat } from "../../../core/enums";
import { DV, LearnedMove, Pokemon, PokemonSchema, PokemonSpecies, toPokemon } from "../../../core/models";
import { addPokemon, getFilledSlots, getNextOpenSlot } from "../../../core/models/party";
import { loadMergedMovesDataSync, loadMergedPokemonDataSync } from "../../../core/content-packs";
import { ScriptRunner, ensureRunnerVariables } from "./utils";

const ODD_OT_NAME = "ODD";
const ODD_EGG_LEVEL = 5;
const ODD_EGG_EXPERIENCE = 125;
const ODD_EGG_HATCH_CYCLES = 20;

const ODD_EGG_PROBABILITIES: readonly number[] = [8, 1, 16, 3, 16, 3, 14, 2, 10, 2, 12, 2, 10, 1];

type OddEggDefinition = {
  species_id: string;
  moves: readonly string[];
  ot_id: number;
  dvs: [number, number, number, number];
};

const ODD_EGG_DEFINITIONS: readonly OddEggDefinition[] = [
  { species_id: "PICHU", moves: ["THUNDERSHOCK", "CHARM", "DIZZY_PUNCH"], ot_id: 2048, dvs: [0, 0, 0, 0] },
  { species_id: "PICHU", moves: ["THUNDERSHOCK", "CHARM", "DIZZY_PUNCH"], ot_id: 256, dvs: [2, 10, 10, 10] },
  { species_id: "CLEFFA", moves: ["POUND", "CHARM", "DIZZY_PUNCH"], ot_id: 4096, dvs: [0, 0, 0, 0] },
  { species_id: "CLEFFA", moves: ["POUND", "CHARM", "DIZZY_PUNCH"], ot_id: 768, dvs: [2, 10, 10, 10] },
  { species_id: "IGGLYBUFF", moves: ["SING", "CHARM", "DIZZY_PUNCH"], ot_id: 4096, dvs: [0, 0, 0, 0] },
  { species_id: "IGGLYBUFF", moves: ["SING", "CHARM", "DIZZY_PUNCH"], ot_id: 768, dvs: [2, 10, 10, 10] },
  { species_id: "SMOOCHUM", moves: ["POUND", "LICK", "DIZZY_PUNCH"], ot_id: 3584, dvs: [0, 0, 0, 0] },
  { species_id: "SMOOCHUM", moves: ["POUND", "LICK", "DIZZY_PUNCH"], ot_id: 512, dvs: [2, 10, 10, 10] },
  { species_id: "MAGBY", moves: ["EMBER", "DIZZY_PUNCH"], ot_id: 2560, dvs: [0, 0, 0, 0] },
  { species_id: "MAGBY", moves: ["EMBER", "DIZZY_PUNCH"], ot_id: 512, dvs: [2, 10, 10, 10] },
  { species_id: "ELEKID", moves: ["QUICK_ATTACK", "LEER", "DIZZY_PUNCH"], ot_id: 3072, dvs: [0, 0, 0, 0] },
  { species_id: "ELEKID", moves: ["QUICK_ATTACK", "LEER", "DIZZY_PUNCH"], ot_id: 512, dvs: [2, 10, 10, 10] },
  { species_id: "TYROGUE", moves: ["TACKLE", "DIZZY_PUNCH"], ot_id: 2560, dvs: [0, 0, 0, 0] },
  { species_id: "TYROGUE", moves: ["TACKLE", "DIZZY_PUNCH"], ot_id: 256, dvs: [2, 10, 10, 10] },
];

const buildThresholds = (probabilities: readonly number[]): number[] => {
  let total = 0;
  const thresholds: number[] = [];
  for (const weight of probabilities) {
    total += weight;
    thresholds.push(Math.floor((total * 0xffff) / 100));
  }
  if (total !== 100) {
    throw new Error("Odd Egg probability table must sum to 100%.");
  }
  return thresholds;
};

const ODD_EGG_THRESHOLDS = buildThresholds(ODD_EGG_PROBABILITIES);

const loadPokemonData = (() => {
  let cached: PokemonSpecies[] | null = null;
  return (): PokemonSpecies[] => {
    if (cached) {
      return cached;
    }
    cached = Object.values(loadMergedPokemonDataSync()) as PokemonSpecies[];
    return cached;
  };
})();

const getSpeciesMap = (() => {
  let cached: Map<string, PokemonSpecies> | null = null;
  return (): Map<string, PokemonSpecies> => {
    if (cached) {
      return cached;
    }
    const mapping = new Map<string, PokemonSpecies>();
    for (const entry of loadPokemonData()) {
      mapping.set(entry.id, entry);
    }
    cached = mapping;
    return cached;
  };
})();

type MoveDataEntry = {
  pp?: number | string;
};

const loadMovesData = (() => {
  let cached: Record<string, MoveDataEntry> | null = null;
  return (): Record<string, MoveDataEntry> => {
    if (cached) {
      return cached;
    }
    cached = loadMergedMovesDataSync() as Record<string, MoveDataEntry>;
    return cached;
  };
})();

const getMoveMap = (() => {
  let cached: Map<string, { pp: number }> | null = null;
  return (): Map<string, { pp: number }> => {
    if (cached) {
      return cached;
    }
    const mapping = new Map<string, { pp: number }>();
    for (const [name, data] of Object.entries(loadMovesData())) {
      mapping.set(name.toUpperCase(), { pp: Number(data.pp ?? 0) });
    }
    cached = mapping;
    return cached;
  };
})();

const hpDv = (attackDv: number, defenseDv: number, speedDv: number, specialDv: number): number => {
  return ((attackDv & 1) << 3) | ((defenseDv & 1) << 2) | ((speedDv & 1) << 1) | (specialDv & 1);
};

const statValue = (baseStat: number, dv: number, level: number): number => {
  return Math.floor(((baseStat + dv) * 2 * level) / 100) + 5;
};

const moveNameLookup = MoveName as Record<string, MoveName>;
const buildMoveset = (moveNames: readonly string[]): LearnedMove[] => {
  return moveNames.map((name) => {
    const key = name.toUpperCase();
    const move = moveNameLookup[key];
    if (!move) {
      throw new Error(`Unknown move '${name}'.`);
    }
    const data = getMoveMap().get(key);
    if (!data) {
      throw new Error(`Missing move metadata for '${name}'.`);
    }
    return { name: move, current_pp: data.pp } as LearnedMove;
  });
};

const composeOddEgg = (definition: OddEggDefinition): Pokemon => {
  const species = getSpeciesMap().get(definition.species_id);
  if (!species) {
    throw new Error(`Unknown Odd Egg species '${definition.species_id}'.`);
  }
  const [attackDv, defenseDv, speedDv, specialDv] = definition.dvs;
  const hp = hpDv(attackDv, defenseDv, speedDv, specialDv);
  const dvs: DV = {
    attack: attackDv,
    defense: defenseDv,
    speed: speedDv,
    special: specialDv,
    hp,
  };

  let maxHp = Math.floor(((species.base_stats.hp + hp) * 2 * ODD_EGG_LEVEL) / 100);
  maxHp += ODD_EGG_LEVEL + 10;

  const moves = buildMoveset(definition.moves);

  return toPokemon(
    PokemonSchema.parse({
      species,
      nickname: "EGG",
      gender: null,
      item: null,
      moves,
      level: ODD_EGG_LEVEL,
      hp: 0,
      max_hp: maxHp,
      dvs,
      status: null,
      original_trainer_name: ODD_OT_NAME,
      original_trainer_id: definition.ot_id,
      experience: ODD_EGG_EXPERIENCE,
      happiness: ODD_EGG_HATCH_CYCLES,
      stat_boosts: {
        [Stat.ATTACK]: 0,
        [Stat.DEFENSE]: 0,
        [Stat.SPEED]: 0,
        [Stat.SPECIAL_ATTACK]: 0,
        [Stat.SPECIAL_DEFENSE]: 0,
        [Stat.ACCURACY]: 0,
        [Stat.EVASION]: 0,
      },
      attack: statValue(species.base_stats.attack, attackDv, ODD_EGG_LEVEL),
      defense: statValue(species.base_stats.defense, defenseDv, ODD_EGG_LEVEL),
      speed: statValue(species.base_stats.speed, speedDv, ODD_EGG_LEVEL),
      special_attack: statValue(species.base_stats.special_attack, specialDv, ODD_EGG_LEVEL),
      special_defense: statValue(species.base_stats.special_defense, specialDv, ODD_EGG_LEVEL),
    })
  );
};

const drawRandomIndex = (game_state: GameState, rng: { nextByte: () => number }): number => {
  rng.nextByte();
  const randomWord = ((game_state.hram.hRandomSub ?? 0) & 0xff) << 8 | ((game_state.hram.hRandomAdd ?? 0) & 0xff);
  for (let index = 0; index < ODD_EGG_THRESHOLDS.length; index += 1) {
    if (randomWord <= ODD_EGG_THRESHOLDS[index]) {
      return index;
    }
  }
  throw new Error("Odd Egg selection failed to find a matching entry.");
};

type Overworld = unknown;

type EventManager = unknown;

type RNGSource = { nextByte: () => number };
type LegacyRng = { next_byte: () => number };
type RNGLike = RNGSource | LegacyRng;

const isRNGSource = (value: unknown): value is RNGSource =>
  typeof value === "object" && value !== null && typeof (value as RNGSource).nextByte === "function";

const isLegacyRng = (value: unknown): value is LegacyRng =>
  typeof value === "object" && value !== null && typeof (value as LegacyRng).next_byte === "function";

const normalizeRng = (rng: RNGLike | null | undefined, game_state: GameState): RNGSource => {
  if (!rng) {
    return new HardwareRNG(game_state);
  }
  if (isRNGSource(rng)) {
    return rng;
  }
  if (isLegacyRng(rng)) {
    return { nextByte: () => rng.next_byte() };
  }
  throw new Error("Odd Egg RNG must expose nextByte/next_byte.");
};

export function give_odd_egg(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
    rng,
  }: {
    runner?: ScriptRunner | null;
    overworld?: Overworld | null;
    event_manager?: EventManager | null;
    rng?: RNGLike | null;
  } = {}
): boolean {
  // ASM: engine/events/odd_egg.asm::_GiveOddEgg
  void overworld;
  void event_manager;

  const hardwareRng = normalizeRng(rng, game_state);
  const index = drawRandomIndex(game_state, hardwareRng);
  const definition = ODD_EGG_DEFINITIONS[index];
  const egg = composeOddEgg(definition);

  const slot = getNextOpenSlot(game_state.sram.party);
  if (slot === null) {
    throw new Error("Odd Egg could not be added because the party is full.");
  }
  if (!addPokemon(game_state.sram.party, egg)) {
    throw new Error("Odd Egg addition failed unexpectedly.");
  }

  game_state.wram.wPartyCount = getFilledSlots(game_state.sram.party);
  game_state.wram.wCurPartySpecies = egg.species.id;
  game_state.wram.wCurPartyMon = slot;

  if (runner) {
    const variables = ensureRunnerVariables(runner);
    runner.last_value = 1;
    runner.last_condition_result = true;
    variables._value = 1;
  }
  return true;
}
