import { GameState } from "../../core/state";
import { DataLoader } from "../../core/data-loader";
import { DayCare, DayCareResident } from "../../core/models";
import { setActive, setEggReady, setMonstersCompatible, setMonsterPresent } from "../../core/models/day-care";
import { Pokemon, toPokemon } from "../../core/models/pokemon";
import { hasSpace, addPokemon } from "../../core/models/party";
import { calculateExperience } from "../experience";
import { GrowthRate } from "../../core/enums";
import { ScriptRunner } from "../../engine/world/story-events/runner";
import type { EventManager } from "../events/events";
import { checkBreedingCompatibility, createEgg, DayCareBreedingState } from "./breeding";

const STEPS_PER_LEVEL = 256;

const COMPATIBILITY_MESSAGE_BRIMMING = "It's brimming with energy.";
const COMPATIBILITY_MESSAGE_NO_INTEREST = "It has no interest in {partner}.";
const COMPATIBILITY_MESSAGE_APPEARS_TO_CARE = "It appears to care for {partner}.";
const COMPATIBILITY_MESSAGE_FRIENDLY = "It's friendly with {partner}.";
const COMPATIBILITY_MESSAGE_SHOWS_INTEREST = "It shows interest in {partner}.";

const SPECIAL_EVENT_FLAG_TOKENS = new Set(["", "0", "-1"]);

type DepositResult = {
    type: "deposit";
    success: boolean;
    caretaker: string;
    reason?: "occupied" | "party_empty";
    pokemon?: string;
    level?: number;
    compatibility?: number;
    message?: string;
};

type WithdrawResult = {
    type: "withdraw";
    success: boolean;
    caretaker: string;
    reason?: "empty" | "party_full" | "insufficient_funds";
    pokemon?: string;
    level?: number;
    levels_gained?: number;
    fee?: number;
    required_fee?: number;
};

type AdvanceStepsResult = {
    type: "advance_steps";
    steps: number;
    level_changes: Record<string, number>;
    egg: boolean;
    egg_result?: string;
};

type CollectEggResult = {
    type: "collect_egg";
    success: boolean;
    reason?: "no_egg" | "party_full";
    pokemon?: string;
};

type InspectResult = {
    caretaker: string;
    occupied: boolean;
    pokemon?: string;
    nickname?: string;
    level?: number;
    message?: string;
};

const DAY_CARE_ACTION_NAMES = ["deposit", "withdraw", "advance_steps", "collect_egg", "inspect"] as const;
type DayCareActionName = (typeof DAY_CARE_ACTION_NAMES)[number];

type DayCareAction =
    | { action: "deposit"; party_index?: number | string | null }
    | { action: "withdraw" }
    | { action: "advance_steps"; steps?: number | string | null }
    | { action: "collect_egg" }
    | { action: "inspect" };

const DAY_CARE_ACTION_SET: ReadonlySet<DayCareActionName> = new Set(DAY_CARE_ACTION_NAMES);

const isDayCareAction = (value: unknown): value is DayCareAction => {
    if (!value || typeof value !== "object") {
        return false;
    }
    const candidate = value as { action?: unknown };
    if (typeof candidate.action !== "string") {
        return false;
    }
    return DAY_CARE_ACTION_SET.has(candidate.action as DayCareActionName);
};

type DayCareActionResult = DepositResult | WithdrawResult | AdvanceStepsResult | CollectEggResult | InspectResult;

type DayCareSerializedResident =
    | { occupied: false }
    | {
          occupied: true;
          pokemon: string | undefined;
          nickname: string | undefined;
          level: number | undefined;
          steps: number;
          initial_level: number;
      };

type DayCareSerializedState = {
    compatibility: number;
    egg_present: boolean;
    steps_until_next_egg: number;
    man: DayCareSerializedResident;
    lady: DayCareSerializedResident;
};

type DayCareInteractionSummary = {
    caretaker: string;
    actions: DayCareActionResult[];
    state: DayCareSerializedState;
};

type DayCareScriptResult = {
    day_care: DayCareInteractionSummary;
};

type DayCareOverworld = {
    refresh_event_flag?: (flagName: string, options?: { value?: boolean }) => void;
};

export class DayCareSystem {
    private gameState: GameState;
    private dataLoader?: DataLoader;
    private overworld?: DayCareOverworld;

    constructor(gameState: GameState, dataLoader?: DataLoader, overworld?: DayCareOverworld) {
        this.gameState = gameState;
        this.dataLoader = dataLoader;
        this.overworld = overworld;
        this._update_daycare_registers();
        this._sync_runtime_state();
        this._update_engine_flags();
    }

    public deposit(caretaker: string, party_index: number): DepositResult {
        const resident = this._resident(caretaker);
        if (resident.pokemon !== undefined) {
            return {
                type: "deposit",
                success: false,
                caretaker,
                reason: "occupied",
            };
        }

        const pokemon = this._remove_party_member(party_index);
        if (pokemon === null) {
            return {
                type: "deposit",
                success: false,
                caretaker,
                reason: "party_empty"
            }
        }
        resident.pokemon = toPokemon(pokemon);
        resident.initial_experience = pokemon.experience;
        resident.initial_level = pokemon.level;
        resident.steps = 0;
        const state = this._state();
        this._update_compatibility();
        this._refresh_breeding_timer();
        const partner = this._other_resident(caretaker);
        const partner_name = partner.pokemon?.nickname ?? partner.pokemon?.species.id;

        const summary: DepositResult = {
            type: "deposit",
            success: true,
            caretaker,
            pokemon: pokemon.species.id,
            level: pokemon.level,
            compatibility: state.compatibility_score,
        };
        this._update_daycare_registers();
        const message = this._compatibility_message(partner_name);
        if (message) {
            summary.message = message;
        }

        this._sync_runtime_state();
        this._update_engine_flags();
        return summary;
    }

    public withdraw(caretaker: string): WithdrawResult {
        const resident = this._resident(caretaker);
        const pokemon = resident.pokemon;
        if (pokemon === null) {
            return {
                type: "withdraw",
                success: false,
                caretaker,
                reason: "empty",
            };
        }

        const levels_gained = Math.max(0, (pokemon?.level ?? resident.initial_level) - resident.initial_level);
        const fee = 100 + levels_gained * 100;
        if (!hasSpace(this.gameState.sram.party)) {
            return {
                type: "withdraw",
                success: false,
                caretaker,
                reason: "party_full",
                required_fee: fee,
            };
        }

        if (this.gameState.sram.money < fee) {
            return {
                type: "withdraw",
                success: false,
                caretaker,
                reason: "insufficient_funds",
                required_fee: fee,
            };
        }

        this.gameState.sram.money -= fee;
        if (pokemon) {
            addPokemon(this.gameState.sram.party, toPokemon(pokemon));
        }

        resident.pokemon = undefined;
        resident.initial_experience = 0;
        resident.initial_level = 0;
        resident.steps = 0;

        this._update_compatibility();
        this._refresh_breeding_timer();
        this._update_daycare_registers();
        this._sync_runtime_state();
        this._update_engine_flags();

        return {
            type: "withdraw",
            success: true,
            caretaker,
            pokemon: pokemon?.species.id,
            level: pokemon?.level,
            levels_gained,
            fee,
        };
    }

    public advance_steps(steps: number): AdvanceStepsResult {
        if (steps <= 0) {
            throw new Error("steps must be a positive integer");
        }

        const state = this._state();
        const level_changes: Record<string, number> = {};

        for (const caretaker of ["man", "lady"]) {
            const resident = this._resident(caretaker);
            const pokemon = resident.pokemon;
            if (!pokemon) {
                continue;
            }
            resident.steps += steps;
            let gained = 0;
            while (resident.steps >= STEPS_PER_LEVEL && pokemon.level < 100) {
                resident.steps -= STEPS_PER_LEVEL;
                pokemon.level += 1;
                gained += 1;
                pokemon.experience = Math.max(
                    pokemon.experience,
                    this._experience_for_level(toPokemon(pokemon), pokemon.level)
                );
                pokemon.max_hp = this._calculate_max_hp(toPokemon(pokemon));
                pokemon.hp = pokemon.max_hp;
            }
            if (gained > 0) {
                level_changes[caretaker] = (level_changes[caretaker] || 0) + gained;
            }
        }

        let egg_result: string | undefined = undefined;
        if (this._should_attempt_breeding()) {
            egg_result = this._try_generate_egg(steps);
        }

        this._update_daycare_registers();

        this._sync_runtime_state();
        this._update_engine_flags();

        const summary: AdvanceStepsResult = {
            type: "advance_steps",
            steps,
            level_changes,
            egg: state.egg_present,
        };
        if (egg_result) {
            summary.egg_result = egg_result;
        }
        return summary;
    }

    public collect_egg(): CollectEggResult {
        const state = this._state();
        if (!state.egg_present || !state.egg) {
            return {
                type: "collect_egg",
                success: false,
                reason: "no_egg",
            };
        }

        if (!hasSpace(this.gameState.sram.party)) {
            return {
                type: "collect_egg",
                success: false,
                reason: "party_full",
            };
        }

        const egg = state.egg;
        addPokemon(this.gameState.sram.party, toPokemon(egg));
        state.egg_present = false;
        state.egg = undefined;
        this._refresh_breeding_timer();

        this._update_daycare_registers();

        this._sync_runtime_state();
        this._update_engine_flags();

        return {
            type: "collect_egg",
            success: true,
            pokemon: egg.species.id,
        };
    }

    public inspect(caretaker: string): InspectResult {
        const resident = this._resident(caretaker);
        const pokemon = resident.pokemon;
        const partner = this._other_resident(caretaker);
        const partner_name = partner.pokemon?.nickname ?? partner.pokemon?.species.id;
        if (!pokemon) {
            return {
                caretaker,
                occupied: false,
            };
        }
        const result: InspectResult = {
            caretaker,
            occupied: true,
            pokemon: pokemon.species.id,
            nickname: pokemon.nickname,
            level: pokemon.level,
        };
        const message = this._compatibility_message(partner_name);
        if (message) {
            result.message = message;
        }
        return result;
    }

    public run_man({ runner, event_manager }: { runner?: ScriptRunner; event_manager?: EventManager }): DayCareScriptResult {
        const state = this._state();
        setActive(state.man_register, true);
        return this._run_script_interaction("man", {
            runner,
            event_manager,
            actions_key: "_day_care_man_actions",
        });
    }

    public run_lady({ runner, event_manager }: { runner?: ScriptRunner; event_manager?: EventManager }): DayCareScriptResult {
        const state = this._state();
        setActive(state.lady_register, true);
        return this._run_script_interaction("lady", {
            runner,
            event_manager,
            actions_key: "_day_care_lady_actions",
        });
    }

    public run_man_outside({ runner }: { runner?: ScriptRunner; event_manager?: EventManager }): string {
        const key = "_day_care_outside_actions";
        let actions = this._pop_actions(runner, key);
        if (actions.length === 0) {
            actions = [{ action: "collect_egg" }];
        }

        const results: DayCareActionResult[] = [];
        let success = false;
        for (const action of actions) {
            const result = this._execute_action("man", action);
            if (result) {
                results.push(result);
                if ("success" in result && result.success) {
                    success = true;
                }
            }
        }

        const summary: DayCareInteractionSummary = {
            caretaker: "man",
            actions: results,
            state: this._serialize_state(),
        };

        if (runner) {
            runner.variables["_day_care_outside_summary"] = summary;
            runner.last_condition_result = success;
        }

        return success ? "FALSE" : "TRUE";
    }

    public run_yard_mon(index: number): InspectResult & { type: "inspect" } {
        const caretaker = index === 0 ? "man" : "lady";
        const summary = this.inspect(caretaker);
        return {
            ...summary,
            type: "inspect",
        };
    }

    private _run_script_interaction(
        caretaker: string,
        { runner, actions_key }: { runner?: ScriptRunner; event_manager?: EventManager; actions_key: string }
    ): DayCareScriptResult {
        const actions = this._pop_actions(runner, actions_key);
        const results: DayCareActionResult[] = [];
        let last_success: boolean | undefined = undefined;

        for (const action of actions) {
            const result = this._execute_action(caretaker, action);
            if (result) {
                results.push(result);
                if ("success" in result) {
                    last_success = result.success;
                }
            }
        }

        const summary: DayCareInteractionSummary = {
            caretaker,
            actions: results,
            state: this._serialize_state(),
        };

        if (runner && last_success !== undefined) {
            runner.last_condition_result = last_success;
        }

        return { day_care: summary };
    }

    private _execute_action(
        caretaker: string,
        action: DayCareAction
    ): DayCareActionResult | undefined {
        const action_type = action.action;
        if (action_type === "deposit") {
            return this.deposit(caretaker, Number(action.party_index ?? 0));
        }
        if (action_type === "withdraw") {
            return this.withdraw(caretaker);
        }
        if (action_type === "advance_steps") {
            return this.advance_steps(Number(action.steps ?? 0));
        }
        if (action_type === "collect_egg") {
            return this.collect_egg();
        }
        if (action_type === "inspect") {
            return this.inspect(caretaker);
        }
        return undefined;
    }

    private _pop_actions(runner: ScriptRunner | undefined, key: string): DayCareAction[] {
        if (!runner) {
            return [];
        }

        const raw_actions: unknown = runner.variables[key] ?? [];
        delete runner.variables[key];

        const entries = Array.isArray(raw_actions) ? raw_actions : [raw_actions];
        return entries.filter(isDayCareAction);
    }

    private _serialize_state(): DayCareSerializedState {
        const state = this._state();
        return {
            compatibility: state.compatibility_score,
            egg_present: state.egg_present,
            steps_until_next_egg: state.steps_until_next_egg,
            man: this._serialize_resident(state.man),
            lady: this._serialize_resident(state.lady),
        };
    }

    private _serialize_resident(resident: DayCareResident): DayCareSerializedResident {
        const pokemon = resident.pokemon;
        if (pokemon === undefined) {
            return { occupied: false };
        }
        return {
            occupied: true,
            pokemon: pokemon.species.id,
            nickname: pokemon.nickname,
            level: pokemon.level,
            steps: resident.steps,
            initial_level: resident.initial_level,
        };
    }

    private _resident(caretaker: string): DayCareResident {
        const state = this._state();
        if (caretaker === "man") {
            return state.man;
        }
        if (caretaker === "lady") {
            return state.lady;
        }
        throw new Error(`Unknown caretaker '${caretaker}'`);
    }

    private _state(): DayCare {
        if (!this.gameState.sram.day_care) {
            throw new Error("DayCareSystem requires access to game_state.sram.day_care.");
        }
        return this.gameState.sram.day_care;
    }

    private _remove_party_member(index: number): Pokemon | null {
        const party: (Pokemon | null)[] = this.gameState.sram.party.pokemon as any;
        if (index < 0 || index >= party.length) {
            throw new Error("party_index out of range");
        }
        const pokemon = party[index];
        if (pokemon === null) {
            return null;
        }
        for (let slot = index; slot < party.length - 1; slot++) {
            party[slot] = party[slot + 1];
        }
        party[party.length - 1] = null as any;
        return pokemon;
    }

    private _update_compatibility(): void {
        const state = this._state();
        if (state.man.pokemon && state.lady.pokemon) {
            state.compatibility_score = checkBreedingCompatibility(
                toPokemon(state.man.pokemon),
                toPokemon(state.lady.pokemon)
            );
        } else {
            state.compatibility_score = 0;
        }
    }

    private _refresh_breeding_timer(): void {
        const state = this._state();
        if (
            !state.man.pokemon ||
            !state.lady.pokemon ||
            state.compatibility_score <= 0
        ) {
            state.steps_until_next_egg = 0;
            return;
        }
        const breeding_state = DayCareBreedingState.initialize(
            this.gameState,
            toPokemon(state.man.pokemon),
            toPokemon(state.lady.pokemon),
        );
        state.compatibility_score = breeding_state.compatibility;
        state.steps_until_next_egg = breeding_state.steps_to_next_check;
    }

    private _should_attempt_breeding(): boolean {
        const state = this._state();
        return (
            !state.egg_present &&
            !!state.man.pokemon &&
            !!state.lady.pokemon &&
            state.compatibility_score > 0
        );
    }

    private _try_generate_egg(steps: number): string | undefined {
        const state = this._state();
        if (state.steps_until_next_egg === 0) {
            this._refresh_breeding_timer();
        }
        if (!state.man.pokemon || !state.lady.pokemon) {
            return undefined;
        }
        const breeding_state = new DayCareBreedingState(
            toPokemon(state.man.pokemon),
            toPokemon(state.lady.pokemon),
            state.compatibility_score,
            state.steps_until_next_egg,
        );
        let result: string | undefined = undefined;
        for (let i = 0; i < steps; i++) {
            if (!breeding_state.advance_step(this.gameState)) {
                continue;
            }
            const egg = createEgg(
                this.gameState,
                toPokemon(state.man.pokemon),
                toPokemon(state.lady.pokemon),
                this.gameState.sram.player_name || "PLAYER",
                this.gameState.sram.player_id,
            );
            state.egg = toPokemon(egg);
            state.egg_present = true;
            result = "created";
            break;
        }
        state.steps_until_next_egg = breeding_state.steps_to_next_check;
        return result;
    }

    private _experience_for_level(pokemon: Pokemon, level: number): number {
        return calculateExperience(pokemon.species.growth_rate as GrowthRate, level);
    }

    private _calculate_max_hp(pokemon: Pokemon): number {
        const base = pokemon.species.base_stats;
        return (
            Math.floor(((base.hp + pokemon.dvs.hp) * 2 * pokemon.level) / 100) +
            pokemon.level +
            10
        );
    }

    private _update_daycare_registers(): void {
        const state = this._state();
        const man_has_mon = !!state.man.pokemon;
        const lady_has_mon = !!state.lady.pokemon;
        setMonsterPresent(state.man_register, man_has_mon);
        setMonsterPresent(state.lady_register, lady_has_mon);
        setEggReady(state.man_register, state.egg_present);
        setMonstersCompatible(state.man_register, state.compatibility_score > 0 && !state.egg_present);
    }

    private _sync_runtime_state(): void {
        this.gameState.wram.day_care = JSON.parse(JSON.stringify(this.gameState.sram.day_care));
    }

    private _update_engine_flags(): void {
        const flags = this.gameState.wram.engine_flags;
        flags["ENGINE_DAY_CARE_MAN_HAS_MON"] = !!this.gameState.sram.day_care.man.pokemon;
        flags["ENGINE_DAY_CARE_LADY_HAS_MON"] = !!this.gameState.sram.day_care.lady.pokemon;
        flags["ENGINE_DAY_CARE_MAN_HAS_EGG"] = !!this.gameState.sram.day_care.egg_present;
        this._update_daycare_event_flags();
    }

    private _update_daycare_event_flags(): void {
        const state = this._state();
        const egg_present = !!state.egg_present;
        this._apply_event_flag("EVENT_DAY_CARE_MAN_IN_DAY_CARE", egg_present);
        this._apply_event_flag("EVENT_DAY_CARE_MAN_ON_ROUTE_34", !egg_present);
        this._apply_event_flag("EVENT_DAY_CARE_MON_1", !state.man.pokemon);
        this._apply_event_flag("EVENT_DAY_CARE_MON_2", !state.lady.pokemon);
    }

    private _apply_event_flag(flag_name: string | undefined, value: boolean): void {
        const normalized = this._normalize_event_flag_name(flag_name);
        if (normalized === undefined) {
            return;
        }
        const bool_value = !!value;
        const wram_flags = this.gameState.wram.event_flags;
        wram_flags[normalized] = bool_value;
        const sram_flags = this.gameState.sram.event_flags;
        if (sram_flags && sram_flags !== wram_flags) {
            sram_flags[normalized] = bool_value;
        }
        if (this.overworld && typeof this.overworld.refresh_event_flag === 'function') {
            this.overworld.refresh_event_flag(normalized, { value: bool_value });
        }
    }

    private _normalize_event_flag_name(flag_name: string | undefined): string | undefined {
        if (flag_name === undefined) {
            return undefined;
        }
        const normalized = String(flag_name).trim();
        if (SPECIAL_EVENT_FLAG_TOKENS.has(normalized)) {
            return undefined;
        }
        return normalized;
    }

    private _compatibility_message(partner_name: string | undefined): string | undefined {
        const state = this._state();
        const score = state.compatibility_score;
        if (!partner_name) {
            return undefined;
        }
        if (score === 0) {
            return COMPATIBILITY_MESSAGE_NO_INTEREST.replace("{partner}", partner_name);
        }
        if (score === 255) {
            return COMPATIBILITY_MESSAGE_BRIMMING;
        }
        if (score >= 230) {
            return COMPATIBILITY_MESSAGE_APPEARS_TO_CARE.replace("{partner}", partner_name);
        }
        if (score >= 70) {
            return COMPATIBILITY_MESSAGE_FRIENDLY.replace("{partner}", partner_name);
        }
        return COMPATIBILITY_MESSAGE_SHOWS_INTEREST.replace("{partner}", partner_name);
    }

    private _other_resident(caretaker: string): DayCareResident {
        if (caretaker === "man") {
            return this._resident("lady");
        }
        return this._resident("man");
    }
}
