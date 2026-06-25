
import { GameState } from '../../../core/state';
import { Pokemon, LearnedMove, toPokemon } from '../../../core/models';
import { MoveName } from '../../../core/enums';
import { showText, waitForInput, type EventManager } from '@pokecrystal/core/engine/events/events';
import { resolveText } from '@pokecrystal/core/engine/world/story-events/text-helpers';
import { pokemonCanLearnTmhm, pokemonKnowsMove } from '../../systems/tmhm';
import { moves } from '@pokecrystal/assets/content/moves';
import type { ScriptRunner } from './utils';

const parseRunnerIndex = (
    value: unknown,
    label: string,
    { fallback }: { fallback?: number } = {},
): number => {
    if (value === undefined || value === null) {
        if (fallback !== undefined) {
            return fallback;
        }
        throw new Error(`Missing required runner index '${label}'.`);
    }

    if (typeof value === 'number' && Number.isInteger(value)) {
        return value;
    }
    if (typeof value === 'string') {
        const parsed = Number(value);
        if (Number.isInteger(parsed)) {
            return parsed;
        }
    }
    throw new Error(`Invalid runner index '${label}': ${String(value)}`);
};

const asRunnerString = (value: unknown): string | undefined => {
    return typeof value === 'string' ? value : undefined;
};


const _resolve_party_mon = (game_state: GameState, runner?: ScriptRunner | null): Pokemon | undefined => {
    const index = parseRunnerIndex(runner?.variables?.['_selected_party_index'], '_selected_party_index', { fallback: 0 });
    const mon = game_state.sram.party.pokemon[index];
    return mon ? toPokemon(mon) : undefined;
};

const _is_egg = (pokemon: Pokemon): boolean => {
    return pokemon.species.id === 'EGG' || pokemon.nickname === 'EGG';
};

const _resolve_move = (move_token: unknown): MoveName | undefined => {
    if (typeof move_token !== 'string') {
        return undefined;
    }
    if (move_token in MoveName) {
        return MoveName[move_token as keyof typeof MoveName];
    }
    return undefined;
};

const _default_move_pp = (move: MoveName): number => {
    const metadata = moves[move];
    return metadata?.pp ?? 15;
};

const _run_result = (runner: ScriptRunner | undefined, value: string, { truthy }: { truthy: boolean }): string => {
    if (runner) {
        runner.variables = runner.variables ?? {};
        runner.variables['_value'] = value;
        runner.last_value = value;
        runner.last_condition_result = truthy;
    }
    return value;
};

const _set_string_buffers = (runner: ScriptRunner | undefined, values: string[]): void => {
    if (!runner) {
        return;
    }
    runner.string_buffers = {};
    values.forEach((value, index) => {
        runner.string_buffers[`STRING_BUFFER_${index + 1}`] = value;
    });
};

const _show_special_text = (
    label: string,
    {
        runner,
        overworld,
        event_manager,
        wait = false,
        buffers = [],
    }: {
        runner?: ScriptRunner;
        overworld?: unknown;
        event_manager?: EventManager | null;
        wait?: boolean;
        buffers?: string[];
    }
): string => {
    _set_string_buffers(runner, buffers);
    const text = resolveText(runner ?? null, overworld ?? null, label);
    if (event_manager) {
        showText(event_manager, text);
        if (wait) {
            waitForInput(event_manager, { pauseRunner: Boolean(runner) });
        }
    }
    return text;
};

const _optional_string = (
    source: Record<string, unknown>,
    keys: string[],
): string | undefined => {
    for (const key of keys) {
        const value = source[key];
        if (typeof value === 'string' && value.trim()) {
            return value.trim();
        }
    }
    return undefined;
};

const _optional_number = (
    source: Record<string, unknown>,
    keys: string[],
): number | undefined => {
    for (const key of keys) {
        const value = source[key];
        if (typeof value === 'number' && Number.isFinite(value)) {
            return value;
        }
        if (typeof value === 'string' && value.trim()) {
            const parsed = Number(value);
            if (Number.isFinite(parsed)) {
                return parsed;
            }
        }
    }
    return undefined;
};

const _seer_advice_label = (currentLevel: number, caughtLevel: number): string => {
    const gained = Math.max(0, currentLevel - caughtLevel);
    if (gained <= 9) {
        return '_SeerMoreCareText';
    }
    if (gained <= 29) {
        return '_SeerMoreConfidentText';
    }
    if (gained <= 59) {
        return '_SeerMuchStrengthText';
    }
    if (gained <= 89) {
        return '_SeerMightyText';
    }
    if (gained <= 100) {
        return '_SeerImpressedText';
    }
    return '_SeerMoreCareText';
};


export const move_tutor = (
    game_state: GameState,
    { runner, overworld, event_manager }: { runner?: ScriptRunner; overworld?: unknown; event_manager?: unknown }
): string => {
    // ASM: engine/events/move_tutor.asm::MoveTutor
    void overworld;
    void event_manager;
    const mon = _resolve_party_mon(game_state, runner);
    if (!mon) {
        return _run_result(runner, "TRUE", { truthy: false });
    }

    const move_token = runner?.variables?.['_selected_move'] ?? runner?.variables?.['_value'];
    const move = _resolve_move(move_token);

    if (!move) {
        return _run_result(runner, "TRUE", { truthy: false });
    }

    if (_is_egg(mon) || pokemonKnowsMove(mon, move)) {
        return _run_result(runner, "TRUE", { truthy: false });
    }

    if (!pokemonCanLearnTmhm(mon, move)) {
        return _run_result(runner, "TRUE", { truthy: false });
    }

    const target_slot_raw = runner?.variables?.['_selected_move_index'];
    const target_slot_from_runner =
        target_slot_raw === undefined || target_slot_raw === null
            ? undefined
            : parseRunnerIndex(target_slot_raw, '_selected_move_index');
    if (mon.moves.length >= 4 && target_slot_from_runner === undefined) {
        return _run_result(runner, "TRUE", { truthy: false });
    }

    let target_slot = target_slot_from_runner ?? mon.moves.length;
    target_slot = Math.max(0, Math.min(target_slot, 3));

    const learned_move: LearnedMove = { name: move, current_pp: _default_move_pp(move), pp_ups: 0 };

    if (target_slot < mon.moves.length) {
        mon.moves[target_slot] = learned_move;
    } else if (mon.moves.length < 4) {
        mon.moves.push(learned_move);
    } else {
        return _run_result(runner, "TRUE", { truthy: false });
    }

    return _run_result(runner, "FALSE", { truthy: true });
};

export const name_rater = (
    game_state: GameState,
    { runner, overworld, event_manager }: { runner?: ScriptRunner; overworld?: unknown; event_manager?: unknown }
): string => {
    // ASM: engine/events/specials.asm::NameRater
    void overworld;
    void event_manager;
    const mon = _resolve_party_mon(game_state, runner);
    if (!mon) {
        if (runner) {
            runner.last_condition_result = false;
            runner.last_value = "";
        }
        return "";
    }

    const nickname = asRunnerString(runner?.variables?.['_selected_nickname'])?.trim();

    if (nickname) {
        mon.nickname = nickname;

        if (runner) {
            runner.last_condition_result = true;
            runner.last_value = nickname;
            runner.variables = runner.variables ?? {};
            runner.variables['_value'] = nickname;
        }

        return nickname;
    }

    return "";
};

export const poke_seer = (
    game_state: GameState,
    { runner, overworld, event_manager }: { runner?: ScriptRunner; overworld?: unknown; event_manager?: EventManager | null }
): string => {
    // ASM: engine/events/poke_seer.asm::PokeSeer
    const activeEventManager = event_manager ?? runner?.event_manager ?? runner?.eventManager ?? null;
    const intro = _show_special_text('_SeerSeeAllText', {
        runner,
        overworld,
        event_manager: activeEventManager,
        wait: true,
    });
    if (runner?.variables?._selection_cancelled) {
        const cancelText = _show_special_text('_SeerDoNothingText', {
            runner,
            overworld,
            event_manager: activeEventManager,
        });
        return _run_result(runner, cancelText, { truthy: false });
    }
    const mon = _resolve_party_mon(game_state, runner);
    if (!mon) {
        if (runner) {
            runner.last_condition_result = false;
            runner.last_value = intro;
        }
        return "";
    }

    const monRecord = mon as Pokemon & Record<string, unknown>;
    const nickname = mon.nickname || mon.species.id;
    if (_is_egg(mon)) {
        const eggText = _show_special_text('_SeerEggText', {
            runner,
            overworld,
            event_manager: activeEventManager,
        });
        return _run_result(runner, eggText, { truthy: false });
    }

    const caughtLevel = _optional_number(monRecord, [
        'caught_level',
        'caughtLevel',
        'met_level',
        'metLevel',
    ]);
    const caughtLocation = _optional_string(monRecord, [
        'caught_location',
        'caughtLocation',
        'met_location',
        'metLocation',
        'location',
    ]);
    const caughtTime = _optional_string(monRecord, [
        'caught_time',
        'caughtTime',
        'met_time',
        'metTime',
        'time_of_day',
        'timeOfDay',
    ]);

    if (caughtLevel === undefined && caughtLocation === undefined && caughtTime === undefined) {
        const unknownText = _show_special_text('_SeerCantTellAThingText', {
            runner,
            overworld,
            event_manager: activeEventManager,
        });
        return _run_result(runner, unknownText, { truthy: true });
    }

    const levelText = String(Math.max(1, Math.floor(caughtLevel ?? mon.level)));
    const timeText = caughtTime ?? 'Unknown';
    let finalText = '';
    if (!caughtLocation) {
        finalText = _show_special_text('_SeerNoLocationText', {
            runner,
            overworld,
            event_manager: activeEventManager,
            wait: true,
            buffers: [levelText],
        });
    } else if (mon.original_trainer_id !== game_state.sram.player_id) {
        const originalTrainer = mon.original_trainer_name || 'Unknown';
        _show_special_text('_SeerTradeText', {
            runner,
            overworld,
            event_manager: activeEventManager,
            wait: true,
            buffers: [nickname, originalTrainer, caughtLocation, originalTrainer, nickname],
        });
        finalText = _show_special_text('_SeerTimeLevelText', {
            runner,
            overworld,
            event_manager: activeEventManager,
            wait: true,
            buffers: [timeText, levelText],
        });
    } else {
        _show_special_text('_SeerNameLocationText', {
            runner,
            overworld,
            event_manager: activeEventManager,
            wait: true,
            buffers: [nickname, caughtLocation],
        });
        finalText = _show_special_text('_SeerTimeLevelText', {
            runner,
            overworld,
            event_manager: activeEventManager,
            wait: true,
            buffers: [timeText, levelText],
        });
    }

    const adviceText = _show_special_text(_seer_advice_label(mon.level, Number(levelText)), {
        runner,
        overworld,
        event_manager: activeEventManager,
        buffers: [nickname],
    });
    return _run_result(runner, adviceText || finalText, { truthy: true });
};
