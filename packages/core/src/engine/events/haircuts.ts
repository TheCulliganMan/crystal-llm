
import { Pokemon, toPokemon } from "@pokecrystal/core/core/models/pokemon";
import { GameState } from "@pokecrystal/core/core/state";
import { HardwareRNG } from "../games/rng";
import type { ScriptRunner } from "@pokecrystal/core/engine/world/special-events/utils";

const PROBABILITY_SCALE: number = 100;

type RunnerVariables = Record<string, number | string | undefined>;
type RunnerStringBuffers = Record<string, string>;

type HaircutRunner = ScriptRunner & {
    variables: RunnerVariables;
    last_value: number;
    last_condition_result: boolean;
    string_buffers?: RunnerStringBuffers;
};

type HappinessChange = {
    probability: number;
    script_value: number;
    change_code: number;
};

const OLDER_PROBABILITIES: HappinessChange[] = [
    { probability: 30, script_value: 2, change_code: 9 },
    { probability: 51, script_value: 3, change_code: 10 },
    { probability: 100, script_value: 4, change_code: 11 },
];

const YOUNGER_PROBABILITIES: HappinessChange[] = [
    { probability: 61, script_value: 2, change_code: 12 },
    { probability: 91, script_value: 3, change_code: 13 },
    { probability: 100, script_value: 4, change_code: 14 },
];

const DAISY_PROBABILITIES: HappinessChange[] = [
    { probability: 100, script_value: 2, change_code: 18 },
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

function pickPartyMon(gameState: GameState, runner?: HaircutRunner | null): Pokemon | undefined {
    let index = 0;
    if (runner?.variables && runner.variables['_selected_party_index']) {
        index = Number(runner.variables['_selected_party_index']);
    }
    const candidate = gameState.sram.party?.pokemon[index];
    return candidate ? toPokemon(candidate) : undefined;
}

function pickRoll(gameState: GameState, runner?: HaircutRunner | null): number {
    if (runner?.variables && runner.variables['_rng_roll'] !== undefined) {
        return Number(runner.variables['_rng_roll']);
    }
    return new HardwareRNG(gameState).randint(0, PROBABILITY_SCALE - 1);
}

function chooseOutcome(options: HappinessChange[], roll: number): HappinessChange {
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
    probabilities: HappinessChange[],
    runner?: HaircutRunner,
): number {
    const mon = pickPartyMon(gameState, runner);
    if (!mon) {
        if (runner) {
            runner.last_condition_result = false;
            runner.last_value = 0;
        }
        return 0;
    }

    const roll = pickRoll(gameState, runner);
    const outcome = chooseOutcome(probabilities, roll);
    applyHappinessChange(mon, outcome.change_code);

    if (runner) {
        runner.last_condition_result = true;
        runner.last_value = outcome.script_value;
        runner.variables['_value'] = outcome.script_value;
        if (runner.string_buffers) {
            const name = (mon.nickname || mon.species.id).trim() || mon.species.id;
            runner.string_buffers['STRING_BUFFER_3'] = name;
        }
    }
    return outcome.script_value;
}

export function olderHaircutBrother(
    gameState: GameState,
    runner?: HaircutRunner
): number {
    return handleHaircut(gameState, OLDER_PROBABILITIES, runner);
}

export function youngerHaircutBrother(
    gameState: GameState,
    runner?: HaircutRunner
): number {
    return handleHaircut(gameState, YOUNGER_PROBABILITIES, runner);
}

export function daisysGrooming(
    gameState: GameState,
    runner?: HaircutRunner
): number {
    return handleHaircut(gameState, DAISY_PROBABILITIES, runner);
}
