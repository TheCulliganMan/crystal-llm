import { GameState } from "@pokecrystal/core/core/state";
import type { EventManager } from "@pokecrystal/core/engine/events/events";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import type { ScriptRunner } from "@pokecrystal/core/engine/world/story-events/runner";
import { resolvePokemonCenterSystem } from "@pokecrystal/core/engine/events/misc";
import type {
  PokemonCenterOverworld,
  PokemonCenterRunner,
} from "@pokecrystal/core/engine/events/misc";

export function gs_healings(
    game_state: GameState,
    {
        runner,
        overworld,
        event_manager,
    }: { runner?: ScriptRunner; overworld?: OverworldEngine; event_manager?: EventManager },
): number {
    const healings = game_state.sram.gs_healings ?? 0;
    if (runner) {
        runner.last_value = healings;
        runner.variables['_value'] = healings;
        runner.last_condition_result = true;
    }
    return healings;
}

export function ho_oh_chamber(
    game_state: GameState,
    {
        runner,
        overworld,
        event_manager,
    }: { runner?: ScriptRunner; overworld?: OverworldEngine; event_manager?: EventManager },
): boolean {
    const wram = game_state.wram;
    const sram = game_state.sram;

    const hooh_party = sram.party.pokemon.some(p => p?.species.id === 'HO_OH');
    const suicune_unleashed = wram.event_flags['EVENT_UNLEASHED_SUICUNE'];
    const raikou_unleashed = wram.event_flags['EVENT_UNLEASHED_RAIKOU'];
    const entei_unleashed = wram.event_flags['EVENT_UNLEASHED_ENTEI'];

    const result = hooh_party && suicune_unleashed && raikou_unleashed && entei_unleashed;
    if (runner) {
        runner.last_condition_result = result;
    }
    return result;
}

// De-structure the arguments to match the Python signature, which is kwargs-only
type PokemonCenterOverworldEngine = OverworldEngine & PokemonCenterOverworld;

export function heal_party({
    game_state,
    runner,
    overworld,
    event_manager,
}: {
    game_state: GameState;
    runner?: PokemonCenterRunner;
    overworld?: PokemonCenterOverworldEngine;
    event_manager?: EventManager;
}): void {
    const system = resolvePokemonCenterSystem(runner, overworld ?? undefined);
    if (!system) {
        return;
    }
    const summary = system.heal_party();
    if (runner) {
        runner.last_condition_result = !!summary.healed_slots;
        runner.last_value = summary;
    }
}

export function heal_machine_anim(
    {
        game_state,
        runner,
        overworld,
        event_manager,
    }: {
        game_state: GameState;
        runner?: PokemonCenterRunner;
        overworld?: PokemonCenterOverworldEngine;
        event_manager?: EventManager;
    },
): boolean | Promise<boolean> {
    // ASM: engine/events/specials.asm::HealMachineAnim
    void game_state;
    void event_manager;
    const system = resolvePokemonCenterSystem(runner, overworld ?? undefined);
    if (
        !system ||
        (typeof system.playHealMachineAnimation !== "function" &&
            typeof system.playHealMachineAnimationAsync !== "function")
    ) {
        return false;
    }
    const animationToken = runner?.variables?.["_value"] ?? null;
    const animationId =
        typeof animationToken === "string" || typeof animationToken === "number"
            ? String(animationToken)
            : null;
    const maybePromise =
        typeof system.playHealMachineAnimationAsync === "function"
            ? system.playHealMachineAnimationAsync(animationId, overworld ?? null)
            : (system.playHealMachineAnimation?.(animationId, overworld ?? null), null);
    if (maybePromise && typeof maybePromise.then === "function") {
        return maybePromise.then(() => {
            if (runner) {
                runner.last_condition_result = true;
            }
            return true;
        });
    }
    if (runner) {
        runner.last_condition_result = true;
    }
    return true;
}
