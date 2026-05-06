import { Pokemon, toPokemon } from "@pokecrystal/core/core/models/pokemon";
import { GameState } from "@pokecrystal/core/core/state";
import { HardwareRNG } from "@pokecrystal/core/engine/games/rng";

type ScriptVariableValue = string | number | boolean | null | undefined;

interface ScriptRunner {
  variables?: Map<string, ScriptVariableValue>;
  stringBuffers?: Map<string, string>;
  lastConditionResult?: boolean;
  lastValue?: number;
}

interface SpecialEventContext {
  runner: ScriptRunner;
}

const PROBABILITY_SCALE = 100;

interface HappinessChange {
  probability: number;
  scriptValue: number;
  changeCode: number;
}

const OLDER_PROBABILITIES: HappinessChange[] = [
  { probability: 30, scriptValue: 2, changeCode: 9 },
  { probability: 51, scriptValue: 3, changeCode: 10 },
  { probability: 100, scriptValue: 4, changeCode: 11 },
];

const YOUNGER_PROBABILITIES: HappinessChange[] = [
  { probability: 61, scriptValue: 2, changeCode: 12 },
  { probability: 91, scriptValue: 3, changeCode: 13 },
  { probability: 100, scriptValue: 4, changeCode: 14 },
];

const DAISY_PROBABILITIES: HappinessChange[] = [
  { probability: 100, scriptValue: 2, changeCode: 18 },
];

const HAPPINESS_CHANGE_TABLE: [number, number, number][] = [
  [5, 3, 2],
  [5, 3, 2],
  [1, 1, 0],
  [3, 2, 1],
  [1, 1, 0],
  [-1, -1, -1],
  [-5, -5, -10],
  [-5, -5, -10],
  [1, 1, 1],
  [3, 3, 1],
  [5, 5, 2],
  [1, 1, 1],
  [3, 3, 1],
  [10, 10, 4],
  [-5, -5, -10],
  [-10, -10, -15],
  [-15, -15, -20],
  [3, 3, 1],
  [10, 6, 4],
];

function pickPartyMon(
  gameState: GameState,
  runner: ScriptRunner | null
): Pokemon | null {
  let index = 0;
  if (runner && runner.variables) {
    index = Number(runner.variables.get("_selected_party_index") || 0);
  }
  const candidate = gameState.sram.party.pokemon[index];
  return candidate ? toPokemon(candidate) : null;
}

function pickRoll(gameState: GameState, runner: ScriptRunner | null): number {
  if (runner && runner.variables) {
    const forced = runner.variables.get("_rng_roll");
    if (forced !== undefined) {
      return Number(forced);
    }
  }
  return new HardwareRNG(gameState).randint(0, PROBABILITY_SCALE - 1);
}

function chooseOutcome(
  options: HappinessChange[],
  roll: number
): HappinessChange {
  for (const option of options) {
    if (roll < option.probability) {
      return option;
    }
  }
  return options[options.length - 1];
}

function applyHappinessChange(pokemon: Pokemon, code: number): number {
  const index = code - 1;
  if (index < 0 || index >= HAPPINESS_CHANGE_TABLE.length) {
    return 0;
  }
  const [low, mid, high] = HAPPINESS_CHANGE_TABLE[index];
  let delta: number;
  if (pokemon.happiness < 100) {
    delta = low;
  } else if (pokemon.happiness < 200) {
    delta = mid;
  } else {
    delta = high;
  }
  pokemon.happiness = Math.max(0, Math.min(255, pokemon.happiness + delta));
  return delta;
}

function handleHaircut(
  gameState: GameState,
  runner: ScriptRunner | null,
  probabilities: HappinessChange[]
): number {
  const mon = pickPartyMon(gameState, runner);
  if (!mon) {
    if (runner) {
      runner.lastConditionResult = false;
      runner.lastValue = 0;
    }
    return 0;
  }

  const roll = pickRoll(gameState, runner);
  const outcome = chooseOutcome(probabilities, roll);
  applyHappinessChange(mon, outcome.changeCode);

  if (runner) {
    runner.lastConditionResult = true;
    runner.lastValue = outcome.scriptValue;
    runner.variables?.set("_value", outcome.scriptValue);
    if (runner.stringBuffers) {
      const name = (mon.nickname || mon.species.id).trim() || mon.species.id;
      runner.stringBuffers.set("STRING_BUFFER_3", name);
    }
  }
  return outcome.scriptValue;
}

export function olderHaircutBrother(
  gameState: GameState,
  { runner }: SpecialEventContext
): number {
  return handleHaircut(gameState, runner, OLDER_PROBABILITIES);
}

export function youngerHaircutBrother(
  gameState: GameState,
  { runner }: SpecialEventContext
): number {
  return handleHaircut(gameState, runner, YOUNGER_PROBABILITIES);
}

export function daisysGrooming(
  gameState: GameState,
  { runner }: SpecialEventContext
): number {
  return handleHaircut(gameState, runner, DAISY_PROBABILITIES);
}
