import { GameState } from "@pokecrystal/core/core/state";
import type { EventManager } from "@pokecrystal/core/engine/events/events";
import type { EventManagerLike } from "@pokecrystal/core/engine/events/event-manager-like";
import type { DataLoader, ScriptEntry } from "@pokecrystal/core/core/data-loader";
import { DayCareSystem } from "@pokecrystal/core/engine/systems/day-care";
import { ItemSystem } from "@pokecrystal/core/engine/systems/items";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { open_text, close_text, wait_for_input } from "@pokecrystal/core/engine/world/events";
import { TextFormatter } from "./text-formatter";
import { Command, ScriptFrame, normalizeScriptName } from "./commands/base";
import {
    CloseTextCommand,
    OpenTextCommand,
    WaitButtonCommand,
    WriteTextCommand,
} from "./commands/text";
import { FacePlayerCommand } from "./commands/overworld";
import { CommandFactory } from "./command-factory";
import { STANDARD_SCRIPT_HANDLERS } from "./specials/handlers";
import { mapConstantToName } from "@pokecrystal/core/engine/world/maps";
import { isDebugEnabled } from "@pokecrystal/core/core/debug-flags";
import { pushDebugLog } from "@pokecrystal/core/core/debug-log";
import { PokemonCenterSystem } from "@pokecrystal/core/ui/screens/pokemon-center";
import { createCircuitBreaker } from "@pokecrystal/core/utils";

const FALLTHROUGH_TERMINATORS = new Set([
    "end",
    "endcallback",
    "trainer",
    "itemball",
    "hiddenitem",
    "fruittree",
    "prompt",
    "next",
    "done",
    "step_end",
    "db",
    "dw",
    "dbw",
    "dbb",
    "dbbw",
    "dba",
    "dn",
    "menu_coords",
    "cmdqueue",
    "stonetable",
]);
const FALLTHROUGH_JUMPS = new Set([
    "sjump",
    "jump",
    "jumptext",
    "jumptextfaceplayer",
    "jumpstd",
]);

type DialogueState = {
    acknowledge_wait?: () => boolean;
    pending_waits?: number;
    pending_waits_count?: number;
    waiting_for_input?: boolean;
    active?: boolean;
    visible?: boolean;
    script_paused?: boolean;
    _script_paused?: boolean;
    scriptPaused?: boolean;
};

type OverworldWithExtras = OverworldEngine & {
    dialogue?: DialogueState;
    item_system?: ItemSystem;
    day_care?: DayCareSystem | null;
    pokemon_center?: PokemonCenterSystem | null;
    hasTemporaryMusicOverride?: () => boolean;
    restartMapMusic?: () => void;
};

type PendingEventManager = EventManagerLike & {
    process_pending_events?: () => void;
    has_pending_events?: boolean;
};

type CommandWithName = Command & { name?: string };

const getDialogueState = (overworld?: OverworldEngine | null): DialogueState | null =>
    (overworld as OverworldWithExtras | undefined)?.dialogue ?? null;

type ScriptDataEntry = ScriptEntry;

const isScriptDataEntry = (value: unknown): value is ScriptDataEntry =>
  Boolean(value && typeof value === "object");

const getScriptCommandName = (entry: ScriptDataEntry): string => {
    const candidate = entry["command"];
    return candidate !== undefined && candidate !== null ? String(candidate) : "";
};

type ScriptData = ReadonlyArray<ScriptEntry>;

const extractLastCommand = (scriptData: ScriptData): string | null => {
    if (!scriptData.length) {
        return null;
    }
    for (let i = scriptData.length - 1; i >= 0; i -= 1) {
        const entry = scriptData[i];
        if (!isScriptDataEntry(entry)) {
            continue;
        }
        const command = getScriptCommandName(entry).trim();
        if (command) {
            return command.toLowerCase();
        }
    }
    return null;
};

const shouldAllowFallthrough = (scriptData: ScriptData): boolean => {
    if (!scriptData.length) {
        return false;
    }
    const lastCommand = extractLastCommand(scriptData);
    if (!lastCommand) {
        return true;
    }
    if (FALLTHROUGH_TERMINATORS.has(lastCommand)) {
        return false;
    }
    if (FALLTHROUGH_JUMPS.has(lastCommand)) {
        return false;
    }
    return true;
};

export interface ScriptRunner {
    game_state: GameState;
    gameState?: GameState;
    event_manager: EventManager;
    eventManager?: EventManager;
    overworld: OverworldEngine;
    item_system?: ItemSystem;
    itemSystem?: ItemSystem;
    data_loader?: DataLoader | null;
    dataLoader?: DataLoader | null;
    day_care?: DayCareSystem | null;
    pokemon_center?: PokemonCenterSystem | null;
    format_text?: (text: string) => string;
    formatText?: (text: string) => string;
    queuePhoneCall?: (contact: string) => void;
    consumePhoneCall?: (contact: string) => { contact?: string } | void;
    variables: Record<string, unknown>;
    last_value?: unknown;
    last_condition_result?: boolean;
    _awaiting_resume?: number;
    _ensure_map_scene_initialized?: (map: string) => [string, number] | null;
    _normalise_map_name?: (map: string) => string;
    _set_map_scene?: (map: string, scene: string) => void;
    allow_event_flag_refresh?: boolean;
    last_sound_effect?: string | null;
    last_yes_no_result?: boolean;
    string_buffers: Record<string, string>;
    state?: string | number;
    stopExecution?: boolean;
    stop_execution?: boolean;
    is_busy?: boolean;
    run_phone_script?: (scriptName: string) => void;
    standard_scripts?: Record<string, (runner: ScriptRunner) => void>;
    run(
        scriptName: string,
        options?: { allow_fallthrough?: boolean; allowFallthrough?: boolean },
    ): void;
    jump(scriptName: string, parentScript?: string | null): void;
    call(scriptName: string, parentScript?: string | null): void;
    defer(scriptName: string): void;
    pause(): void;
    resume(): void;
    _script_stack?: ScriptFrame[];
    _queued_overworld_task_count?: number;
    _queue_overworld_task?: (scheduler: (callback: () => void) => boolean | void) => void;
    _queueOverworldTask?: (scheduler: (callback: () => void) => boolean | void) => void;
    _consume_script_choice?: (key: string, defaultValue: unknown) => unknown;
    _consumeScriptChoice?: (key: string, defaultValue: unknown) => unknown;
    _find_parent_script_name?: () => string | null;
    _terminate_current_script?: () => void;
    stop_all_scripts?: () => void;
    audio_engine?: AudioEngine | null;
    audioEngine?: AudioEngine | null;
    pending_reload_map?: string | null;
    last_interaction_object_index?: number | null;
    just_battled?: boolean;
    loaded_trainer?: unknown;
    loaded_trainer_id?: string | null;
}

export enum ScriptRunnerState {
    IDLE,
    RUNNING,
    PAUSED,
}

export class ScriptRunnerImpl implements ScriptRunner {
    public gameState: GameState;
    public eventManager: EventManager;
    public dataLoader: DataLoader; 
    public overworld: OverworldEngine;
    public commandFactory: CommandFactory;
    public stopExecution: boolean = false;
    public itemSystem: ItemSystem;
    public day_care: DayCareSystem | null = null;
    public pokemon_center: PokemonCenterSystem;
    public textFormatter: TextFormatter;
    public last_yes_no_result: boolean = false;
    public last_condition_result: boolean = false;
    public last_sound_effect: string | null = null;
    public variables: Record<string, unknown>;
    public last_value: unknown = null;
    public loaded_trainer: unknown = null;
    public loaded_trainer_id: string | null = null;
    public just_battled: boolean = false;
    public pending_reload_map: string | null = null;
    public last_interaction_object_index: number | null = null;
    public _consume_script_choice?: (key: string, defaultValue: unknown) => unknown;
    public _consumeScriptChoice?: (key: string, defaultValue: unknown) => unknown;

    private _state: ScriptRunnerState = ScriptRunnerState.IDLE;
    public _script_stack: ScriptFrame[] = [];
    private _pause_execution: boolean = false;
    public _awaiting_resume: number = 0;
    public _queued_overworld_task_count: number = 0;
    private _deferred_scripts: string[] = [];
    private _paused_frame: ScriptFrame | null = null;
    private _halt_after_frame_pop = false;
    private _advancing_execution = false;
    public allow_event_flag_refresh: boolean = true;
    private _event_flag_refresh_suppression_count = 0;
    private _event_flag_refresh_restore: boolean | null = null;
    private _standard_scripts: Record<string, (runner: ScriptRunner) => void> =
        STANDARD_SCRIPT_HANDLERS;
    private _last_trace: { script: string; index: number; command: string } | null = null;
    private _restored_audio_for_idle = false;

    constructor(
        gameState: GameState,
        eventManager: EventManager,
        dataLoader: DataLoader,
        overworld: OverworldEngine,
    ) {
        this.gameState = gameState;
        this.eventManager = eventManager;
        this.dataLoader = dataLoader;
        this.overworld = overworld;
        this.textFormatter = new TextFormatter(gameState, {
            getMapName: () => this.overworld?.current_map_name ?? null,
        });
        this.itemSystem = new ItemSystem(gameState, dataLoader);
        this.day_care = new DayCareSystem(gameState, dataLoader, overworld);
        this.pokemon_center = new PokemonCenterSystem(gameState, dataLoader);
        const overworldExtras = this.overworld as OverworldWithExtras;
        if (overworldExtras && !overworldExtras.item_system) {
            overworldExtras.item_system = this.itemSystem;
        }
        if (overworldExtras && !overworldExtras.day_care) {
            overworldExtras.day_care = this.day_care;
        }
        if (overworldExtras && !overworldExtras.pokemon_center) {
            overworldExtras.pokemon_center = this.pokemon_center;
        }
        this.variables = gameState.wram.script_memory;
        this.commandFactory = new CommandFactory(this);
        this.dataLoader.reload_story_events?.();
    }

    public get string_buffers(): Record<string, string> {
        return this.textFormatter.stringBuffers;
    }

    public set string_buffers(value: Record<string, string>) {
        this.textFormatter.stringBuffers = value ?? {};
    }

    public get standard_scripts(): Record<string, (runner: ScriptRunner) => void> {
        return this._standard_scripts;
    }

    public formatText(text: string): string {
        if (!this.textFormatter) {
            this.textFormatter = new TextFormatter(this.gameState, {
                getMapName: () => this.overworld?.current_map_name ?? null,
            });
        }
        return this.textFormatter.formatText(text);
    }

    public get data_loader(): DataLoader {
        return this.dataLoader;
    }

    public set data_loader(value: DataLoader) {
        this.dataLoader = value;
    }

    public get event_manager(): EventManager {
        return this.eventManager;
    }

    public get game_state(): GameState {
        return this.gameState;
    }

    public get item_system(): ItemSystem {
        return this.itemSystem;
    }

    public set item_system(value: ItemSystem) {
        this.itemSystem = value;
    }

    public run_phone_script(script_name: string): void {
        open_text(this.eventManager);
        this.run(script_name);
        wait_for_input(this.eventManager);
        close_text(this.eventManager);
    }

    public get stop_execution(): boolean {
        return this.stopExecution;
    }

    public set stop_execution(value: boolean) {
        this.stopExecution = value;
    }

    public get is_busy(): boolean {
        this._sync_state();
        return this._state !== ScriptRunnerState.IDLE;
    }

    public get state(): ScriptRunnerState {
        this._sync_state();
        return this._state;
    }

    public run(
        scriptName: string,
        options: { allow_fallthrough?: boolean; allowFallthrough?: boolean } = {},
    ): void {
        const traceRun = isDebugEnabled("script:run") || isDebugEnabled("script");
        const allowFallthrough =
            options.allow_fallthrough ?? options.allowFallthrough ?? undefined;
        const currentMapName = this.overworld?.current_map_name ?? null;
        if (currentMapName) {
            this._ensure_map_scene_initialized(currentMapName);
        }
        if (traceRun) {
            pushDebugLog(`[script] run request ${String(scriptName)}`, {
                map: currentMapName ?? undefined,
                allowFallthrough: allowFallthrough ?? undefined,
                stackDepth: this._script_stack.length,
                awaitingResume: this._awaiting_resume,
            });
        }
        if (!this._push_script(scriptName, undefined, allowFallthrough)) {
            return;
        }

        // ASM: object event flags are evaluated on load; suppress auto-refresh while scripts run.
        this._suppress_event_flag_refresh_until_idle();
        this.stopExecution = false;
        this._state = ScriptRunnerState.RUNNING;
        this._halt_after_frame_pop = true;
        this._advance_execution();
        this._halt_after_frame_pop = false;
        if (this._awaiting_resume > 0 || this._script_stack.length > 0) {
            this.stopExecution = true;
        }
    }

    public pause(): void {
        if (this._script_stack.length === 0) {
            return
        }
        if (isDebugEnabled("script:pause") || isDebugEnabled("script")) {
            pushDebugLog("[script] pause", {
                stackDepth: this._script_stack.length,
                awaitingResume: this._awaiting_resume,
            });
        }
        this._pause_execution = true;
        this.stopExecution = true;
        this._awaiting_resume++;
        this._state = ScriptRunnerState.PAUSED;
    }

    public resume(): void {
        if (this._script_stack.length === 0) {
            this._pause_execution = false;
            this.stopExecution = false;
            this._awaiting_resume = 0;
            return;
        }

        if (isDebugEnabled("script:pause") || isDebugEnabled("script")) {
            pushDebugLog("[script] resume", {
                stackDepth: this._script_stack.length,
                awaitingResume: this._awaiting_resume,
            });
        }
        const dialogue = getDialogueState(this.overworld);
        dialogue?.acknowledge_wait?.();
        if (this._awaiting_resume > 0) {
            this._awaiting_resume = Math.max(0, this._awaiting_resume - 1);
        }
        this._pause_execution = false;
        this.stopExecution = false;
        this._state = ScriptRunnerState.RUNNING;
        this._advance_execution();
    }

    private _release_pause(): void {
        if (this._awaiting_resume > 0) {
            this._awaiting_resume = Math.max(0, this._awaiting_resume - 1);
        }
    }

    public _queue_overworld_task(
        scheduler: (callback: () => void) => boolean | void,
    ): void {
        let resumed = false;
        let queueReleased = false;
        let frameSnapshot: ScriptFrame | null = this._script_stack.length
            ? this._script_stack[this._script_stack.length - 1]
            : null;

        const releaseQueuedTask = (): void => {
            if (queueReleased) {
                return;
            }
            queueReleased = true;
            this._queued_overworld_task_count = Math.max(0, this._queued_overworld_task_count - 1);
        };

        const resumeOnce = (): void => {
            if (resumed) {
                return;
            }
            const overworld = this.overworld ?? null;
            const dialogue = getDialogueState(overworld);
            const readPendingWaits = (): number =>
                Number(dialogue?.pending_waits ?? dialogue?.pending_waits_count ?? 0);
            const readDialogueWaiting = (): boolean =>
                Boolean(dialogue?.waiting_for_input) || readPendingWaits() > 0;
            const dialogueVisible = Boolean(dialogue?.active ?? dialogue?.visible);
            if (readDialogueWaiting() && dialogueVisible) {
                this._release_pause();
                releaseQueuedTask();
                return;
            }
            if (readDialogueWaiting() && !dialogueVisible) {
                dialogue?.acknowledge_wait?.();
            }
            if (readDialogueWaiting()) {
                this._release_pause();
                releaseQueuedTask();
                return;
            }
            if (!this._script_stack.length && frameSnapshot) {
                this._script_stack.push(frameSnapshot);
                frameSnapshot = null;
            }
            resumed = true;
            releaseQueuedTask();
            this.resume();
        };

        if (isDebugEnabled("script:tasks") || isDebugEnabled("script")) {
            pushDebugLog("[script] queue overworld task", {
                stackDepth: this._script_stack.length,
                awaitingResume: this._awaiting_resume,
            });
        }
        this.pause();
        this._queued_overworld_task_count += 1;
        try {
            const result = scheduler(resumeOnce);
            if (result === false && !resumed) {
                resumeOnce();
            }
        } catch (error) {
            releaseQueuedTask();
            this._pause_execution = false;
            this.stopExecution = false;
            throw error;
        }
    }

    public _queueOverworldTask(
        scheduler: (callback: () => void) => boolean | void,
    ): void {
        this._queue_overworld_task(scheduler);
    }

    private _push_script(
        scriptName: string,
        parentScript?: string | null,
        allowFallthrough?: boolean,
    ): boolean {
        this._restored_audio_for_idle = false;
        const normalized = normalizeScriptName(scriptName);
        const parent = parentScript ?? (normalized.startsWith(".") ? this._find_parent_script_name() : null);
        const scriptData = this.dataLoader.get_script(normalized, parent ?? undefined);
        if (scriptData) {
            if (this._script_stack.length === 0) {
                pushDebugLog(`[script] run ${normalized}`, parent ? { parent } : undefined);
            }
            if (isDebugEnabled("script:stack") || isDebugEnabled("script")) {
                pushDebugLog(`[script] push ${normalized}`, {
                    parent: parent ?? undefined,
                    depth: this._script_stack.length + 1,
                    allowFallthrough: allowFallthrough ?? undefined,
                });
            }
            const script = this.parse(scriptData);
            const frame: ScriptFrame = {
                name: normalized,
                commands: script,
                index: 0,
                parent: parent ?? undefined,
                allowFallthrough: shouldAllowFallthrough(scriptData),
            };
            if (allowFallthrough !== undefined) {
                frame.allowFallthrough = allowFallthrough;
            }
            this._script_stack.push(frame);
            return true;
        }
        const standard = this._run_standard_script(normalized);
        if (!standard && this._script_stack.length === 0) {
            pushDebugLog(`[script] missing ${normalized}`, parent ? { parent } : undefined);
        }
        return standard;
    }

    private _run_standard_script(scriptName: string): boolean {
        const handler = this._standard_scripts?.[scriptName];
        if (!handler) {
            return false;
        }
        const sentinel: ScriptFrame = { name: scriptName, commands: [], index: 0 };
        this._script_stack.push(sentinel);
        this._state = ScriptRunnerState.RUNNING;
        try {
            handler(this);
        } finally {
            const idx = this._script_stack.indexOf(sentinel);
            if (idx !== -1) {
                this._script_stack.splice(idx, 1);
            }
        }
        this._sync_state();
        return true;
    }

    public _find_parent_script_name(): string | null {
        if (this._script_stack.length) {
            const current = this._script_stack[this._script_stack.length - 1];
            if (current.parent) {
                return current.parent;
            }
        }
        for (let i = this._script_stack.length - 1; i >= 0; i -= 1) {
            const frame = this._script_stack[i];
            if (!frame.name.startsWith(".")) {
                return frame.name;
            }
        }
        return null;
    }

    public defer(scriptName: string): void {
        const normalized = normalizeScriptName(scriptName);
        this._deferred_scripts.push(normalized);
    }

    public _set_map_scene(mapName: string, sceneName: string): void {
        const mapKey = this._normalise_map_name(mapName);
        this._ensure_map_scene_initialized(mapKey);
        const order = Array.from(this.dataLoader?.map_scene_order?.get?.(mapKey) ?? []);
        if (order.length && !order.includes(sceneName)) {
            order.push(sceneName);
            if (this.dataLoader?.map_scene_order) {
                this.dataLoader.map_scene_order.set(mapKey, order);
            }
        } else if (!order.length && this.dataLoader?.map_scene_order) {
            this.dataLoader.map_scene_order.set(mapKey, [sceneName]);
            order.push(sceneName);
        }
        const index = order.includes(sceneName) ? order.indexOf(sceneName) : 0;
        const previousScene = this.gameState.wram.map_scenes[mapKey] ?? "";
        this.gameState.wram.map_scenes[mapKey] = sceneName;
        this.gameState.wram.map_scene_indices[mapKey] = index;
        if (this.overworld?.current_map_name === mapKey) {
            this.gameState.wram.scene_name = sceneName;
        }
        if (this.overworld?._logger?.info) {
            this.overworld._logger.info(
                "ScriptRunner recorded scene %s -> %s (index=%d) for map %s",
                previousScene || "<unset>",
                sceneName,
                index,
                mapKey,
            );
        }
    }

    public _ensure_map_scene_initialized(mapName: string): [string, number] | null {
        const mapKey = this._normalise_map_name(mapName);
        const wram = this.gameState.wram;
        if (mapKey in wram.map_scene_indices) {
            const index = wram.map_scene_indices[mapKey] ?? 0;
            let name = wram.map_scenes[mapKey] ?? "";
            if (!name) {
                const order = this.dataLoader?.map_scene_order?.get?.(mapKey) ?? null;
                if (order && order.length) {
                    name = order[0];
                    wram.map_scenes[mapKey] = name;
                }
            }
            return [name ?? "", typeof index === "number" ? index : 0];
        }

        const order = this.dataLoader?.map_scene_order?.get?.(mapKey) ?? null;
        if (!order || !order.length) {
            return null;
        }

        const existingScene = wram.map_scenes[mapKey] ?? "";
        if (existingScene) {
            const index = order.includes(existingScene) ? order.indexOf(existingScene) : 0;
            wram.map_scenes[mapKey] = existingScene;
            wram.map_scene_indices[mapKey] = index;
            return [existingScene, index];
        }

        let defaultScene = this.dataLoader?.map_default_scene?.[mapKey] ?? order[0];
        if (!order.includes(defaultScene)) {
            defaultScene = order[0];
        }
        const index = order.indexOf(defaultScene);
        wram.map_scenes[mapKey] = defaultScene;
        wram.map_scene_indices[mapKey] = index;
        if (this.overworld?.current_map_name === mapKey) {
            wram.scene_name = defaultScene;
        }
        return [defaultScene, index];
    }

    public _normalise_map_name(mapName: string): string {
        if (!mapName) {
            return mapName;
        }
        if (mapName.includes("_") && mapName.toUpperCase() === mapName) {
            return mapConstantToName(mapName);
        }
        return mapName;
    }

    public jump(scriptName: string, parentScript?: string | null): void {
        const normalized = normalizeScriptName(scriptName);
        const parent = parentScript ?? (normalized.startsWith(".") ? this._find_parent_script_name() : null);
        const traceJump = isDebugEnabled("script:stack") || isDebugEnabled("script");
        if (traceJump) {
            const current = this._script_stack.length
                ? this._script_stack[this._script_stack.length - 1]?.name
                : null;
            pushDebugLog(`[script] jump ${current ?? "<none>"} -> ${normalized}`, {
                parent: parent ?? undefined,
                depth: this._script_stack.length,
            });
        }
        const scriptData = this.dataLoader.get_script(normalized, parent ?? undefined);
        if (!scriptData) {
            if (traceJump) {
                pushDebugLog(`[script] jump FAILED - script not found: ${normalized}`, {
                    parent: parent ?? undefined,
                });
            }
            this._run_standard_script(normalized);
            return;
        }
        const script = this.parse(scriptData);
        const allowFallthrough = shouldAllowFallthrough(scriptData);
        if (traceJump) {
            pushDebugLog(`[script] jump resolved ${scriptData.length} commands for ${normalized}`, {
                parent: parent ?? undefined,
                firstCommand: scriptData[0]?.command ?? undefined,
            });
        }
        const frame = this._script_stack[this._script_stack.length - 1];
        if (frame) {
            frame.name = normalized;
            frame.commands = script;
            frame.index = 0;
            frame.parent = parent ?? undefined;
            frame.allowFallthrough = allowFallthrough;
        } else {
            this._script_stack.push({
                name: normalized,
                commands: script,
                index: 0,
                parent: parent ?? undefined,
                allowFallthrough,
            });
        }
        this.stopExecution = false;
        this._state = ScriptRunnerState.RUNNING;
        this._advance_execution();
    }

    public call(scriptName: string, parentScript?: string | null): void {
        // ASM: engine/overworld/scripting.asm::ScriptCall.
        const normalized = normalizeScriptName(scriptName);
        const parent = parentScript ?? (normalized.startsWith(".") ? this._find_parent_script_name() : null);
        if (isDebugEnabled("script:stack") || isDebugEnabled("script")) {
            const current = this._script_stack.length
                ? this._script_stack[this._script_stack.length - 1]?.name
                : null;
            pushDebugLog(`[script] call ${current ?? "<none>"} -> ${normalized}`, {
                parent: parent ?? undefined,
                depth: this._script_stack.length + 1,
            });
        }
        if (!this._push_script(normalized, parent ?? undefined)) {
            return;
        }
        this.stopExecution = false;
        this._state = ScriptRunnerState.RUNNING;
        this._advance_execution();
    }

    public _terminate_current_script(): void {
        if (!this._script_stack.length) {
            return;
        }
        const frame = this._script_stack[this._script_stack.length - 1];
        frame.index = frame.commands.length;
        frame.allowFallthrough = false;
    }

    private _advance_execution(): void {
        if (this._advancing_execution) {
            return;
        }
        this._advancing_execution = true;
        const traceStack = isDebugEnabled("script:stack") || isDebugEnabled("script");
        const traceCommands = isDebugEnabled("script:cmd") || isDebugEnabled("script:commands");
        const maxCommandsPerAdvance = 50_000;
        let executedThisAdvance = 0;
        const advanceCircuitBreaker = createCircuitBreaker(maxCommandsPerAdvance, "ScriptRunnerImpl._advance_execution");
        try {
            this._paused_frame = null;
            while (true) {
                advanceCircuitBreaker();
                executedThisAdvance += 1;
                if (executedThisAdvance > maxCommandsPerAdvance) {
                    const frame = this._script_stack.length
                        ? this._script_stack[this._script_stack.length - 1]
                        : null;
                    const map = this.overworld?.current_map_name ?? null;
                    const message = `Script runner exceeded ${maxCommandsPerAdvance} iterations without yielding (possible infinite loop).`;
                    pushDebugLog(`[fatal] ${message}`, {
                        map: map ?? undefined,
                        script: frame?.name ?? undefined,
                        index: frame?.index ?? undefined,
                        stackDepth: this._script_stack.length,
                        awaitingResume: this._awaiting_resume,
                        stopExecution: this.stopExecution,
                        lastCommand: this._last_trace?.command ?? undefined,
                        lastScript: this._last_trace?.script ?? undefined,
                        lastIndex: this._last_trace?.index ?? undefined,
                    });
                    // Fallback in case circuit breaker gets bypassed or doesn't throw
                    throw new Error(message);
                }
                if (this._script_stack.length === 0) {
                    if (this._deferred_scripts.length) {
                        const deferred = this._deferred_scripts.shift();
                        if (deferred && this._push_script(deferred)) {
                            continue;
                        }
                    }
                    break;
                }
                const frame = this._script_stack[this._script_stack.length - 1];

                if (frame.index >= frame.commands.length) {
                    const completed = this._script_stack.pop();
                    if (traceStack && completed) {
                        pushDebugLog(`[script] pop ${completed.name}`, {
                            depth: this._script_stack.length,
                            allowFallthrough: completed.allowFallthrough ?? false,
                            awaitingResume: this._awaiting_resume,
                        });
                    }
                    if (this._awaiting_resume > 0) {
                        this._awaiting_resume = Math.max(0, this._awaiting_resume - 1);
                    }
                    if (this._halt_after_frame_pop && this._script_stack.length > 0) {
                        break;
                    }
                    if (completed?.allowFallthrough && this._awaiting_resume === 0) {
                        const successorLookup = this.dataLoader?.get_script_successor;
                        if (typeof successorLookup === "function") {
                            const successor = successorLookup.call(
                                this.dataLoader,
                                completed.name,
                                completed.parent ?? null,
                            );
                            if (successor) {
                                let parent: string | null = null;
                                let name: string;
                                if (Array.isArray(successor) && successor.length >= 2) {
                                    parent = successor[0] ?? null;
                                    name = successor[1];
                                } else {
                                    name = String(successor);
                                }
                                if (name.startsWith(".") && parent === null) {
                                    parent = completed.parent ?? null;
                                }
                                if (traceStack) {
                                    pushDebugLog(`[script] fallthrough ${completed.name} -> ${name}`, {
                                        parent: parent ?? undefined,
                                    });
                                }
                                if (this._push_script(name, parent ?? undefined)) {
                                    continue;
                                }
                            }
                        }
                    }
                    continue;
                }

                const command = frame.commands[frame.index];
                if (traceCommands) {
                    const commandName =
                        (command as CommandWithName).name ?? command?.constructor?.name ?? "Command";
                    pushDebugLog(`[script] cmd ${frame.name}#${frame.index} ${commandName}`, {
                        stackDepth: this._script_stack.length,
                        awaitingResume: this._awaiting_resume,
                    });
                    this._last_trace = { script: frame.name, index: frame.index, command: String(commandName) };
                }
                frame.index++;

                command.runner = this;
                command.execute(this.gameState, this.eventManager, this.overworld);

                const pendingEventManager = this.eventManager as PendingEventManager;
                if (typeof pendingEventManager.process_pending_events === "function") {
                    pendingEventManager.process_pending_events();
                }
                const dialogue = getDialogueState(this.overworld);
                const scriptPaused = Boolean(
                    dialogue?.script_paused ??
                    dialogue?._script_paused ??
                    dialogue?.scriptPaused,
                );
                if (scriptPaused) {
                    this.stopExecution = true;
                    if (traceStack) {
                        pushDebugLog("[script] paused by dialogue", {
                            script: frame.name,
                            index: frame.index,
                            stackDepth: this._script_stack.length,
                        });
                    }
                    break;
                }
                if (this.stopExecution) {
                    if (this._pause_execution || this._awaiting_resume > 0) {
                        this._pause_execution = false;
                        this._paused_frame = frame;
                        if (traceStack) {
                            pushDebugLog("[script] execution paused", {
                                script: frame.name,
                                index: frame.index,
                                awaitingResume: this._awaiting_resume,
                            });
                        }
                        break;
                    }
                    this.stopExecution = false;
                    if (traceStack) {
                        pushDebugLog("[script] stopExecution without pause; popping frame", {
                            script: frame.name,
                            index: frame.index,
                        });
                    }
                    this._script_stack.pop();
                }
            }
            if (this._paused_frame && this._script_stack.length === 0) {
                this._script_stack.push(this._paused_frame);
                this._paused_frame = null;
            }
            this._sync_state();
        } finally {
            this._advancing_execution = false;
        }
    }

    private _suppress_event_flag_refresh_until_idle(): void {
        if (this._event_flag_refresh_suppression_count === 0) {
            this._event_flag_refresh_restore = this.allow_event_flag_refresh ?? true;
        }
        this._event_flag_refresh_suppression_count += 1;
        this.allow_event_flag_refresh = false;
    }

    private _sync_state(): void {
        const runnerIdle = this._script_stack.length === 0 && this._awaiting_resume === 0;
        if (runnerIdle && this._event_flag_refresh_suppression_count > 0) {
            const restoreValue = this._event_flag_refresh_restore ?? true;
            this._event_flag_refresh_suppression_count = 0;
            this._event_flag_refresh_restore = null;
            this.allow_event_flag_refresh = restoreValue;
        }
        if (runnerIdle) {
            this.stopExecution = false;
            this._pause_execution = false;
            this._restore_overworld_music_on_idle();
        } else {
            this._restored_audio_for_idle = false;
        }

        if (this._script_stack.length > 0 || this._awaiting_resume > 0 || this.stopExecution) {
            this._state = ScriptRunnerState.PAUSED;
        } else {
            this._state = ScriptRunnerState.IDLE;
        }
        const pendingEventManager = this.eventManager as PendingEventManager;
        if (this._state === ScriptRunnerState.IDLE && pendingEventManager.has_pending_events) {
            this._state = ScriptRunnerState.PAUSED;
        }
    }

    private _restore_overworld_music_on_idle(): void {
        if (this._restored_audio_for_idle) {
            return;
        }
        const overworld = this.overworld as OverworldWithExtras | null | undefined;
        if (!overworld) {
            this._restored_audio_for_idle = true;
            return;
        }
        const hasTemporaryOverride = overworld.hasTemporaryMusicOverride;
        const restartMapMusic = overworld.restartMapMusic;
        if (
            typeof hasTemporaryOverride === "function" &&
            hasTemporaryOverride.call(overworld) &&
            typeof restartMapMusic === "function"
        ) {
            restartMapMusic.call(overworld);
        }
        this._restored_audio_for_idle = true;
    }

    public parse(scriptData: ScriptData): Command[] {
        const strict = isDebugEnabled("script:strict") || isDebugEnabled("script:parse");
        const commands: Command[] = [];
        for (const entry of scriptData) {
            let commandName: string | undefined;
            let args: string[] = [];

            if (typeof entry === 'string') {
                const cleaned = (entry as string).split(";", 1)[0].trim();
                if (!cleaned) {
                    continue;
                }
                const parts = cleaned.split(/\s+/);
                commandName = parts[0];
                args = parts.slice(1);
            } else if (typeof entry === 'object' && entry !== null && 'command' in entry) {
                commandName = String((entry as { command: unknown }).command);
                const rawArgs = (entry as { args?: unknown }).args;
                if (typeof rawArgs === 'string') {
                    const cleaned = rawArgs.split(";", 1)[0].trim();
                    args = cleaned ? cleaned.split(/\s+/) : [];
                } else if (Array.isArray(rawArgs)) {
                    args = rawArgs.map(String);
                } else if (rawArgs !== undefined && rawArgs !== null) {
                    args = [String(rawArgs)];
                }
            }

            if (!commandName) {
                continue;
            }
            const normalized = commandName.trim().toLowerCase();
            if (normalized === "jumptext" || normalized === "farjumptext") {
                // ASM: engine/overworld/scripting.asm::JumpTextScript.
                const label = args[0] ?? "";
                commands.push(
                    new OpenTextCommand(),
                    new WriteTextCommand(label),
                    new WaitButtonCommand(),
                    new CloseTextCommand(),
                );
                continue;
            }
            if (normalized === "jumptextfaceplayer") {
                // ASM: engine/overworld/scripting.asm::JumpTextFacePlayerScript.
                const label = args[0] ?? "";
                commands.push(
                    new FacePlayerCommand(),
                    new OpenTextCommand(),
                    new WriteTextCommand(label),
                    new WaitButtonCommand(),
                    new CloseTextCommand(),
                );
                continue;
            }
            if (this.commandFactory.commandMap.has(normalized)) {
                const commandFactory = this.commandFactory.commandMap.get(normalized)!;
                const command = commandFactory(...args);
                commands.push(command);
            } else if (strict) {
                throw new Error(`Unsupported script command '${normalized}'`);
            }
        }
        return commands;
    }
}
