import { GameState } from "@pokecrystal/core/core/state";
import type { ScriptRunner } from "@pokecrystal/core/engine/world/story-events/runner";
import type { SpecialContext } from "./special-types";

const FADE_FRAMES = 8;

type QueueDelaySnake = (
    frames: number,
    options: { on_complete: () => void; blocking?: boolean }
) => boolean;

type QueueDelayCamel = (
    frames: number,
    options: { onComplete: () => void; blocking?: boolean }
) => boolean;

type FadeOverworld = {
    fade_to_white?: (frames: number) => void;
    fade_from_white?: (frames: number) => void;
    fade_to_black?: (frames: number) => void;
    fade_from_black?: (frames: number) => void;
    queue_delay?: QueueDelaySnake;
    queueDelay?: QueueDelayCamel;
};

type FadeRunner = {
    _queue_overworld_task?: (scheduler: (callback: () => void) => boolean | void) => void;
    _queueOverworldTask?: (scheduler: (callback: () => void) => boolean | void) => void;
};

const queueFadeFrames = (
    overworld: FadeOverworld | null,
    runner: FadeRunner | null,
    frames: number = FADE_FRAMES
): void => {
    if (!overworld || !runner) {
        return;
    }
    const queueDelaySnake = overworld.queue_delay;
    const queueDelayCamel = overworld.queueDelay;
    if (typeof queueDelaySnake !== "function" && typeof queueDelayCamel !== "function") {
        return;
    }
    const scheduleTask = runner._queue_overworld_task ?? runner._queueOverworldTask;
    if (typeof scheduleTask !== "function") {
        return;
    }
    const schedule = (callback: () => void): boolean => {
        if (typeof queueDelaySnake === "function") {
            return Boolean(queueDelaySnake.call(overworld, frames, { on_complete: callback, blocking: true }));
        }
        if (typeof queueDelayCamel === "function") {
            return Boolean(queueDelayCamel.call(overworld, frames, { onComplete: callback, blocking: true }));
        }
        return false;
    };
    scheduleTask.call(runner, schedule);
};

export function fade_out_to_white(game_state: GameState, context?: SpecialContext): void {
    // ASM: engine/tilesets/timeofday_pals.asm::FadeOutToWhite
    void game_state;
    const overworld = (context?.overworld as FadeOverworld | null) ?? null;
    const runner = (context?.runner as ScriptRunner | null) ?? null;
    const fade_method = overworld?.fade_to_white;
    if (typeof fade_method === "function") {
        fade_method.call(overworld, FADE_FRAMES);
        queueFadeFrames(overworld, runner, FADE_FRAMES);
    }
}

export function fade_in_from_white(game_state: GameState, context?: SpecialContext): void {
    // ASM: engine/tilesets/timeofday_pals.asm::FadeInFromWhite
    void game_state;
    const overworld = (context?.overworld as FadeOverworld | null) ?? null;
    const runner = (context?.runner as ScriptRunner | null) ?? null;
    const fade_method = overworld?.fade_from_white;
    if (typeof fade_method === "function") {
        fade_method.call(overworld, FADE_FRAMES);
        queueFadeFrames(overworld, runner, FADE_FRAMES);
    }
}

export function fade_out_to_black(game_state: GameState, context?: SpecialContext): void {
    // ASM: engine/tilesets/timeofday_pals.asm::FadeOutToBlack
    void game_state;
    const overworld = (context?.overworld as FadeOverworld | null) ?? null;
    const runner = (context?.runner as ScriptRunner | null) ?? null;
    const fade_method = overworld?.fade_to_black;
    if (typeof fade_method === "function") {
        fade_method.call(overworld, FADE_FRAMES);
        queueFadeFrames(overworld, runner, FADE_FRAMES);
    }
}

export function fade_in_from_black(game_state: GameState, context?: SpecialContext): void {
    // ASM: engine/tilesets/timeofday_pals.asm::FadeInFromBlack
    void game_state;
    const overworld = (context?.overworld as FadeOverworld | null) ?? null;
    const runner = (context?.runner as ScriptRunner | null) ?? null;
    const fade_method = overworld?.fade_from_black;
    if (typeof fade_method === "function") {
        fade_method.call(overworld, FADE_FRAMES);
        queueFadeFrames(overworld, runner, FADE_FRAMES);
    }
}
