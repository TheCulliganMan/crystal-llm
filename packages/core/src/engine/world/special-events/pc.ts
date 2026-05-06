
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import type { DataLoader } from "@pokecrystal/core/core/data-loader";
import { showText, waitForInput, type EventManager } from "@pokecrystal/core/engine/events/events";
import type { ScriptRunner } from "@pokecrystal/core/engine/world/story-events/runner";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { GameState } from "@pokecrystal/core/core/state";
import { resolveText } from "@pokecrystal/core/engine/world/story-events/text-helpers";
import { PCHubMenu } from "@pokecrystal/core/ui/menus/pc-hub-prompt";
import { PokemonCenterPCSession } from "@pokecrystal/core/ui/menus/pc-menu";
import { PlayerPCMenu, PlayerPCMenuActionResult, PlayerPCUI } from "@pokecrystal/core/ui/menus/pc-player-menu";
import type { BaseUI } from "@pokecrystal/core/ui/base-ui";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { pcHubEntries } from './pc-helpers';

type DialogueState = {
    visible?: boolean;
    waiting_for_input?: boolean;
    pending_waits?: number;
    handle_input?: (event: unknown) => boolean;
    handleInput?: (event: unknown) => boolean;
};

type PlayersPcHandlerResult = boolean | Record<string, unknown> | [unknown, boolean] | null;
type PlayersPcHandler = (options: {
    game_state: GameState;
    runner?: Runner | null;
    overworld?: Overworld | null;
    event_manager?: EventManager | null;
}) => PlayersPcHandlerResult | Promise<PlayersPcHandlerResult>;

type Runner = ScriptRunner & {
    players_pc_handler?: PlayersPcHandler;
    playersPcHandler?: PlayersPcHandler;
    data_loader?: DataLoader | null;
    dataLoader?: DataLoader | null;
};

type Overworld = OverworldEngine & {
    ui?: PlayerPCUI | BaseUI | null;
    dialogue?: DialogueState | null;
    audio_engine?: AudioEngine | null;
    open_players_pc?: PlayersPcHandler;
    openPlayersPc?: PlayersPcHandler;
    draw?: () => void;
    update?: () => void;
    input_capture_active?: boolean;
};

type PCSelectionLogEntry =
    | { selection: string; mailbox: Record<string, unknown> }
    | { selection: string; status: "decorations_unavailable" }
    | { selection: string; actions: PlayerPCMenuActionResult[] };

type PCMenuSelectionResult = {
    selection: string;
    selection_index: number;
    actions: PCSelectionLogEntry[];
    changed_decorations: false;
};

const PLAYERS_HOUSE_PC_OPTIONS = [
    "WITHDRAW ITEM",
    "DEPOSIT ITEM",
    "TOSS ITEM",
    "MAIL BOX",
    "DECORATION",
    "TURN OFF",
];

function _pc_result_label(selection: string): string | null {
    const normalized = selection.trim().toUpperCase();
    if (normalized.startsWith("BILL")) {
        return "PokecenterBillsPCText";
    }
    if (normalized.endsWith("'S PC") && normalized.includes("OAK")) {
        return "PokecenterOaksPCText";
    }
    if (normalized.endsWith("'S PC")) {
        return "PokecenterPlayersPCText";
    }
    if (normalized === "TURN OFF") {
        return "PokecenterPCOaksClosedText";
    }
    return null;
}

function _resolve_pc_result_text(
    selection: string,
    {
        runner,
        data_loader,
        strict = false,
    }: { runner?: Runner | null; data_loader?: DataLoader | null; strict?: boolean } = {},
): string {
    const label = _pc_result_label(selection);
    if (label) {
        return resolveText(
            runner
                ? {
                    dataLoader: runner.data_loader ?? runner.dataLoader ?? null,
                    format_text: runner.format_text?.bind(runner),
                    formatText: runner.formatText?.bind(runner),
                }
                : {
                    dataLoader: data_loader
                        ? {
                            get_text: data_loader.get_text?.bind(data_loader),
                            getText: data_loader.getText?.bind(data_loader),
                        }
                        : null,
                },
            null,
            label,
        );
    }
    if (strict) {
        throw new Error(`Missing ASM PC result text for selection '${selection}'.`);
    }
    return String(selection || "PC");
}

function _dialogue_allows_input(dialogue?: DialogueState | null): boolean {
    if (!dialogue) {
        return true;
    }
    const visible = Boolean(dialogue.visible);
    const waiting = Boolean(dialogue.waiting_for_input);
    const pending = Number(dialogue.pending_waits ?? 0);
    return !(visible && (waiting || pending > 0));
}

function _dialogue_is_waiting(dialogue?: DialogueState | null): boolean {
    if (!dialogue) {
        return false;
    }
    return Boolean(dialogue.waiting_for_input) || Number(dialogue.pending_waits ?? 0) > 0;
}

function _dialogue_aware_event_provider<T>(dialogue: DialogueState | null | undefined, provider: () => T[]): () => T[] {
    return () => {
        const events = provider();
        if (!_dialogue_is_waiting(dialogue)) {
            return events;
        }
        const handleInput = dialogue?.handle_input ?? dialogue?.handleInput;
        if (handleInput) {
            for (const event of events) {
                handleInput.call(dialogue, event);
            }
        }
        return [];
    };
}

function _wrap_overworld_draw_callback(overworld?: Overworld | null): (() => void) | null {
    if (!overworld || typeof overworld.draw !== "function") {
        return null;
    }
    const draw = overworld.draw.bind(overworld);
    const update = typeof overworld.update === "function" ? overworld.update.bind(overworld) : null;
    if (!update) {
        return draw;
    }
    return () => {
        try {
            update();
        } catch {
            // Ignore update errors to keep UI responsive.
        }
        draw();
    };
}

const _format_item_name = (itemId: unknown): string => {
    if (!itemId) {
        return "ITEM";
    }
    return String(itemId).replace(/_/g, " ").trim().replace(/\b\w/g, (c) => c.toUpperCase());
};

type EventManagerAdapter = {
    showText?: (text: string) => void;
    waitForInput?: () => void;
};

const _event_manager_adapter = (
    event_manager?: EventManager | null,
): EventManagerAdapter | undefined => {
    if (!event_manager) {
        return undefined;
    }
    return {
        showText: (text: string) => showText(event_manager, text),
        waitForInput: () => waitForInput(event_manager),
    };
};

const _announce_player_action = (
    event_manager: EventManagerAdapter | null,
    result: PlayerPCMenuActionResult | null,
): PlayerPCMenuActionResult | null => {
    if (!result || !event_manager) {
        return result;
    }
    const action = result.action;
    const status = result.status;
    const display = (result.display as string) || _format_item_name(result.item);
    const quantity = Number(result.quantity ?? 0);
    let text: string | null = null;
    if (action === "deposit") {
        if (status === "ok") {
            text = `Stored ${quantity} ${display}.`;
        } else if (status === "empty") {
            text = `You have no ${display}.`;
        } else if (status === "pc_full") {
            text = "The PC is full.";
        } else if (status === "bag_error" || status === "invalid") {
            text = "You can't store that item.";
        }
    } else if (action === "withdraw") {
        if (status === "ok") {
            text = `Withdrew ${quantity} ${display} from the PC.`;
        } else if (status === "empty") {
            text = `No ${display} stored.`;
        } else if (status === "bag_full") {
            text = "You can't hold the item.";
        }
    } else if (action === "toss") {
        if (status === "ok") {
            text = `Tossed ${quantity} ${display}.`;
        } else if (status === "empty") {
            text = `You have no ${display}.`;
        }
    }
    if (text) {
        event_manager.showText?.(text);
        event_manager.waitForInput?.();
    }
    return result;
};

function _extract_pc_change_flag(result: unknown): boolean {
    if (typeof result === 'boolean') {
        return result;
    }
    if (typeof result === 'object' && result !== null) {
        const candidate = result as { changed_decorations?: unknown };
        return !!candidate.changed_decorations;
    }
    return false;
}

const _is_promise_like = (value: unknown): value is Promise<unknown> =>
    !!value && typeof (value as { then?: unknown }).then === "function";

const isPlayerPcUi = (ui: unknown): ui is PlayerPCUI => {
    if (!ui || typeof ui !== "object") {
        return false;
    }
    const candidate = ui as PlayerPCUI;
    return typeof candidate.drawWindow === "function" && typeof candidate.update === "function";
};

async function _run_players_house_pc_menu(
    game_state: GameState,
    {
        ui,
        data_loader,
        event_manager,
        dialogue,
        audio_engine,
        draw_callback,
    }: {
        ui: PlayerPCUI;
        data_loader?: DataLoader | null;
        event_manager?: EventManager | null;
        dialogue?: DialogueState | null;
        audio_engine?: AudioEngine | null;
        draw_callback?: (() => void) | null;
    },
): Promise<PCMenuSelectionResult> {
    const event_adapter = _event_manager_adapter(event_manager);
    const session = new PokemonCenterPCSession(game_state, {
        ui,
        dataLoader: data_loader
            ? {
                getText: (label: string) =>
                    data_loader.getText?.(label) ?? data_loader.get_text?.(label) ?? "",
            }
            : undefined,
        eventManager: event_adapter ?? undefined,
        dialogue: dialogue ?? undefined,
        audioEngine: audio_engine ?? undefined,
        drawCallback: draw_callback ?? undefined,
        playersPc: true,
    });

    if (!_dialogue_allows_input(dialogue)) {
        throw new Error("Dialogue is blocking Player's PC input.");
    }

    const options = [...PLAYERS_HOUSE_PC_OPTIONS];
    const selection_log: PCSelectionLogEntry[] = [];
    let last_index = 0;

    const pollEvents = ui.pollEvents;
    const rawEventProvider =
        typeof pollEvents === "function"
            ? pollEvents.bind(ui)
            : () => gameEngine.event.get(ui.eventQueue ?? undefined);
    const eventProvider = _dialogue_aware_event_provider(dialogue, rawEventProvider);

    while (true) {
        const prompt = new PCHubMenu(ui, options, audio_engine ?? null, { eventProvider });
        prompt.index = last_index;
        const choice_index = await prompt.runAsync(draw_callback ?? undefined);
        const selection = options[choice_index];
        last_index = choice_index;

        if (selection === "TURN OFF") {
            break;
        }
        if (selection === "MAIL BOX") {
            const mailbox_result = await session.runMailboxInteractiveAsync();
            selection_log.push({ selection, mailbox: mailbox_result });
            continue;
        }
        if (selection === "DECORATION") {
            const message = _resolve_pc_result_text(selection, { runner: null, data_loader });
            if (event_adapter) {
                event_adapter.showText?.(message || "The decoration feature is not available yet.");
                event_adapter.waitForInput?.();
            }
            selection_log.push({ selection, status: "decorations_unavailable" });
            continue;
        }

        const menu = new PlayerPCMenu(ui, game_state, data_loader ?? undefined, audio_engine ?? undefined);
        menu.jumpToAction(selection, { openList: true });
        const actions = await menu.runInteractiveAsync({
            actionHandler: (result) => _announce_player_action(event_adapter ?? null, result),
            drawCallback: draw_callback ?? undefined,
            eventProvider,
        });
        selection_log.push({ selection, actions });
    }

    return {
        selection: options[last_index],
        selection_index: last_index,
        actions: selection_log,
        changed_decorations: false,
    };
}

export function players_house_pc(
    game_state: GameState,
    { runner, overworld, event_manager }: { runner?: Runner; overworld?: Overworld | null; event_manager?: EventManager | null },
): boolean | Promise<boolean> {
    const overworld_instance = (overworld ?? runner?.overworld ?? null) as Overworld | null;

    if (!event_manager && runner) {
        event_manager = runner.event_manager ?? null;
    }

    const boot_text = resolveText(
        runner
            ? {
                dataLoader: runner.data_loader ?? runner.dataLoader ?? null,
                format_text: runner.format_text?.bind(runner),
                formatText: runner.formatText?.bind(runner),
            }
            : null,
        overworld_instance
            ? {
                dataLoader: (overworld_instance as { dataLoader?: DataLoader | null }).dataLoader ?? null,
            }
            : null,
        "PlayersPCTurnOnText",
    );

    let handler = runner?.players_pc_handler ?? runner?.playersPcHandler ?? null;
    if (!handler && overworld_instance) {
        handler = overworld_instance.open_players_pc ?? overworld_instance.openPlayersPc ?? null;
    }

    let handler_result: PlayersPcHandlerResult | Promise<PlayersPcHandlerResult> | null = null;
    let changed_decorations = false;

    const overworld_ui = overworld_instance?.ui ?? null;
    if (typeof handler === "function") {
        handler_result = handler({
            game_state,
            runner,
            overworld: overworld_instance,
            event_manager,
        });
    } else if (overworld_instance && isPlayerPcUi(overworld_ui)) {
        const draw_callback = _wrap_overworld_draw_callback(overworld_instance);
        const previous_capture = overworld_instance.input_capture_active;
        overworld_instance.input_capture_active = true;
        const menu_result = _run_players_house_pc_menu(game_state, {
            ui: overworld_ui,
            data_loader: runner?.data_loader ?? runner?.dataLoader ?? null,
            event_manager: event_manager ?? null,
            dialogue: overworld_instance?.dialogue ?? null,
            audio_engine: overworld_instance?.audio_engine ?? null,
            draw_callback,
        });
        handler_result = menu_result.finally(() => {
            overworld_instance.input_capture_active = previous_capture;
        });
    } else if (runner) {
        changed_decorations = !!runner.variables?.["_players_house_pc_changed_decorations"];
    }

    const finalize = (result: PlayersPcHandlerResult): boolean => {
        if (result !== null && result !== undefined) {
            if (Array.isArray(result) && result.length === 2 && typeof result[1] === "boolean") {
                const [payload, decorations] = result;
                handler_result = payload as PlayersPcHandlerResult;
                changed_decorations = decorations;
            } else {
                changed_decorations = _extract_pc_change_flag(result);
                handler_result = result;
            }
        } else {
            changed_decorations = Boolean(changed_decorations);
            handler_result = result;
        }

        if (changed_decorations) {
            game_state.wram.maptile_decorations_visible = false;
        }

        if (runner) {
            runner.last_condition_result = Boolean(changed_decorations);
            const stack = runner._script_stack ?? [];
            if (stack.length > 0) {
                stack[stack.length - 1].allowFallthrough = Boolean(changed_decorations);
            }
            const details: Record<string, unknown> = {
                boot_text,
                changed_decorations: Boolean(changed_decorations),
            };
            if (handler_result && typeof handler_result === "object" && !Array.isArray(handler_result)) {
                Object.assign(details, handler_result as Record<string, unknown>);
            } else if (handler_result !== null && handler_result !== details.changed_decorations) {
                details.result = handler_result;
            }
            runner.last_value = { pc: details };
        }

        return Boolean(changed_decorations);
    };

    if (_is_promise_like(handler_result)) {
        return handler_result.then((resolved) => finalize(resolved));
    }

    return finalize(handler_result);
}

export function BillPC(game_state: GameState, { overworld }: { overworld?: Overworld | null }): Promise<Record<string, unknown>> {
    const runner = overworld?.script_runner ?? undefined;
    const event_manager = overworld?.event_manager;
    return pokemon_center_pc(game_state, { runner, overworld, event_manager });
}

export async function pokemon_center_pc(
    game_state: GameState,
    { runner, overworld, event_manager }: { runner?: Runner; overworld?: Overworld | null; event_manager?: EventManager | null },
): Promise<Record<string, unknown>> {
    const overworld_instance = overworld;
    const ui = overworld_instance?.ui;
    const dialogue = overworld_instance?.dialogue;
    const audio_engine = overworld_instance?.audio_engine;
    const draw_callback = _wrap_overworld_draw_callback(overworld_instance);
    const data_loader = runner?.data_loader ?? runner?.dataLoader ?? null;
    const entries = pcHubEntries(game_state, { include_hall_of_fame: false });
    const option_labels = entries.map(entry => entry.label);

    let selection: string | undefined;
    const scripted_actions: { [key: string]: any } = {};

    if (runner) {
        const variables = runner.variables || {};
        const rawSelection = variables["_pokemon_center_pc_selection"];
        selection = typeof rawSelection === "string" ? rawSelection : undefined;
        if (!selection) {
            // consume_script_choice logic here if needed
        }
        scripted_actions["bill"] = variables["_pc_bill_actions"] || null;
        scripted_actions["mail"] = variables["_pc_mail_actions"] || null;
        scripted_actions["player"] = variables["_pc_player_actions"] || null;
    }

    const textLoader = data_loader
        ? { getText: (label: string) => data_loader.getText?.(label) ?? "" }
        : undefined;
    const session = new PokemonCenterPCSession(game_state, {
        ui,
        dataLoader: textLoader,
        eventManager: _event_manager_adapter(event_manager),
        dialogue: dialogue ?? undefined,
        audioEngine: audio_engine,
        drawCallback: draw_callback ?? undefined,
    });
    if (typeof session.setHubOptions === "function") {
        session.setHubOptions(entries);
    }

    const previous_capture = overworld_instance?.input_capture_active ?? false;
    if (overworld_instance) {
        overworld_instance.input_capture_active = true;
    }
    let summary: Record<string, unknown>;
    try {
        summary = await session.runAsync({
            selection,
            scriptedActions: scripted_actions,
        });
    } finally {
        if (overworld_instance) {
            overworld_instance.input_capture_active = previous_capture;
        }
    }

    summary["options"] = option_labels;

    const chosen_index = summary["selection_index"] as number || 0;
    const clamped_index = Math.max(0, Math.min(chosen_index, option_labels.length - 1));
    const selection_name = (summary["selection_name"] as string || option_labels[clamped_index]).toUpperCase();
    summary["selection_index"] = clamped_index;
    summary["selection_name"] = selection_name;
    summary["result_text"] = _resolve_pc_result_text(selection_name, { runner, data_loader, strict: true });

    if (runner) {
        runner.last_condition_result = selection_name !== "TURN OFF";
        const party = game_state.sram.party.pokemon.filter(p => p);
        const pc_details = {
            available: party.length > 0,
            selection_index: clamped_index,
            selection: selection_name,
            summary: summary,
            result_text: summary["result_text"],
            options: summary["options"],
        };
        runner.last_value = { pc: pc_details };
    }

    return summary;
}

export async function hall_of_fame_terminal(
    game_state: GameState,
    { runner, overworld, event_manager, actions }: { runner?: Runner; overworld?: Overworld | null; event_manager?: EventManager | null, actions?: any },
): Promise<Record<string, unknown>> {
    const overworld_instance = overworld || runner?.overworld;
    const ui = overworld_instance?.ui;
    const dialogue = overworld_instance?.dialogue;
    const audio_engine = overworld_instance?.audio_engine;
    const draw_callback = _wrap_overworld_draw_callback(overworld_instance);
    const data_loader = runner?.data_loader ?? runner?.dataLoader ?? null;

    const textLoader = data_loader
        ? { getText: (label: string) => data_loader.getText?.(label) ?? "" }
        : undefined;
    const session = new PokemonCenterPCSession(game_state, {
        ui,
        dataLoader: textLoader,
        eventManager: _event_manager_adapter(event_manager),
        dialogue: dialogue ?? undefined,
        audioEngine: audio_engine,
        drawCallback: draw_callback ?? undefined,
    });

    const summary = actions
        ? await session.runHallOfFame(actions)
        : await session.runHallOfFameInteractiveAsync();

    if (runner) {
        runner.last_value = { hall_of_fame: summary };
    }

    return summary;
}
