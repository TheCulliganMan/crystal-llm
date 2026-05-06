import type { ScriptData, ScriptEntry } from "@pokecrystal/core/core/data-loader";
import { asmStringLoader } from "@pokecrystal/core/core/asm-string-loader";
import { asmTextLoader } from "@pokecrystal/core/core/asm-text-loader";
import { GameState } from "@pokecrystal/core/core/state";
import { Event, EventManager, openText, closeText, showText, waitForInput } from "@pokecrystal/core/engine/events/events";
import type { FieldDialogueManager } from "@pokecrystal/core/ui/text/dialogue";
import { Command, type OverworldContext } from "./base";
import type { ScriptRunner } from "../runner";

type TextDataLoader = {
    get_script?: (label: string, parentScript?: string) => ScriptData | null;
    getScript?: (label: string, parentScript?: string) => ScriptData | null;
    getText?: (label: string) => string | null;
    get_text?: (label: string) => string | null;
};

type ScriptRunnerForText = ScriptRunner & {
    dataLoader?: TextDataLoader | null;
    data_loader?: TextDataLoader | null;
    formatText?: (text: string) => string;
    format_text?: (text: string) => string;
    string_buffers?: Record<string, string>;
};

type ScriptRunnerForString = ScriptRunnerForText & {
    _find_parent_script_name?: () => string | null;
};

type YesNoScriptRunner = ScriptRunnerForText & {
    _consume_script_choice?: (key: string, defaultValue: unknown) => unknown;
    _consumeScriptChoice?: (key: string, defaultValue: unknown) => unknown;
};

type DialogueOverworldContext = OverworldContext & {
    dialogue?: FieldDialogueManager | null;
};

type NpcTrade = {
    dialogSet: "collector" | "happy" | "newbie" | "girl";
    requestedMon: string;
    offeredMon: string;
};

const NPC_TRADES: Record<string, NpcTrade> = {
    NPC_TRADE_MIKE: { dialogSet: "collector", requestedMon: "ABRA", offeredMon: "MACHOP" },
    NPC_TRADE_KYLE: { dialogSet: "collector", requestedMon: "BELLSPROUT", offeredMon: "ONIX" },
    NPC_TRADE_TIM: { dialogSet: "happy", requestedMon: "KRABBY", offeredMon: "VOLTORB" },
    NPC_TRADE_EMY: { dialogSet: "girl", requestedMon: "DRAGONAIR", offeredMon: "DODRIO" },
    NPC_TRADE_CHRIS: { dialogSet: "newbie", requestedMon: "HAUNTER", offeredMon: "XATU" },
    NPC_TRADE_KIM: { dialogSet: "girl", requestedMon: "CHANSEY", offeredMon: "AERODACTYL" },
    NPC_TRADE_FOREST: { dialogSet: "collector", requestedMon: "DUGTRIO", offeredMon: "MAGNETON" },
};

const TRADE_INTRO_LABEL_BY_DIALOG_SET: Record<NpcTrade["dialogSet"], string> = {
    collector: "_NPCTradeIntroText1",
    happy: "_NPCTradeIntroText2",
    newbie: "_NPCTradeIntroText2",
    girl: "_NPCTradeIntroText3",
};

const resolveScriptText = (runner: ScriptRunnerForText | undefined, textLabel: string): string => {
    const loader = runner?.dataLoader ?? runner?.data_loader;
    const rawText =
        loader?.getText?.(textLabel) ??
        loader?.get_text?.(textLabel) ??
        null;
    const normalizedRaw = typeof rawText === "string" ? rawText.trim() : rawText;
    let formatted = normalizedRaw ? rawText! : "";
    if (!formatted || !String(formatted).trim()) {
        throw new Error(`Missing text for label '${textLabel}'.`);
    }
    if (runner?.formatText) {
        formatted = runner.formatText(formatted);
    } else if (runner?.format_text) {
        formatted = runner.format_text(formatted);
    }
    return formatted;
};

const LINE_BREAK_COMMANDS = new Set(["line", "cont", "next"]);
const PARAGRAPH_COMMANDS = new Set(["para"]);
const STRING_COMMANDS = new Set(["db", "text", "text_block", "line", "cont", "next", "para"]);
const TERMINATOR_COMMANDS = new Set(["done", "text_end"]);

const parseStringToken = (token: string): { text: string; terminated: boolean } => {
    const raw = String(token ?? "");
    const trimmed = raw.trim();
    if (!trimmed) {
        return { text: "", terminated: false };
    }
    const quoted = trimmed.includes("\"");
    if (!quoted && !trimmed.includes("@")) {
        return { text: "", terminated: false };
    }
    let text = trimmed;
    if (quoted) {
        const start = trimmed.indexOf("\"");
        const end = trimmed.lastIndexOf("\"");
        if (end > start) {
            text = trimmed.slice(start + 1, end);
        }
    }
    const terminated = text.includes("@") || trimmed.includes("@");
    return { text: text.replace(/@+$/g, ""), terminated };
};

const parseStringArgs = (args: unknown[]): { text: string; terminated: boolean } => {
    let buffer = "";
    for (const arg of args) {
        const { text, terminated } = parseStringToken(String(arg ?? ""));
        if (text) {
            buffer += text;
        }
        if (terminated) {
            return { text: buffer, terminated: true };
        }
    }
    return { text: buffer, terminated: false };
};

export const extractStringFromScript = (scriptData: ScriptData): string | null => {
    let buffer = "";
    let hasString = false;
    for (const entry of scriptData) {
        if (!entry || typeof entry !== "object") {
            continue;
        }
        const scriptEntry = entry as ScriptEntry;
        const command = String(scriptEntry.command ?? "").trim().toLowerCase();
        if (!command) {
            continue;
        }
        if (TERMINATOR_COMMANDS.has(command)) {
            break;
        }
        const args = Array.isArray(scriptEntry.args)
            ? scriptEntry.args
            : scriptEntry.text !== undefined
                ? [scriptEntry.text]
                : [];
        if (!args.length || !STRING_COMMANDS.has(command)) {
            continue;
        }
        if (LINE_BREAK_COMMANDS.has(command)) {
            buffer += "\n";
        } else if (PARAGRAPH_COMMANDS.has(command)) {
            buffer += "\n\n";
        }
        const { text, terminated } = parseStringArgs(args);
        if (text || terminated) {
            hasString = true;
            buffer += text;
        }
        if (terminated) {
            break;
        }
    }
    return hasString ? buffer : null;
};

const resolveStringDataLoader = (
    runner: ScriptRunnerForString | undefined,
    overworld: OverworldContext,
): TextDataLoader | null => {
    const overworldAny = overworld as { data_loader?: TextDataLoader | null; dataLoader?: TextDataLoader | null };
    return runner?.dataLoader ?? runner?.data_loader ?? overworldAny?.dataLoader ?? overworldAny?.data_loader ?? null;
};


export class WriteTextCommand extends Command {
    constructor(public readonly textLabel: string, private autoCloseAfterWait: boolean = false) {
        super();
    }

    public execute(gameState: GameState, eventManager: EventManager, overworld: OverworldContext): void {
        const text = resolveScriptText(this.runner, this.textLabel);
        showText(eventManager, text, { auto_close_after_wait: this.autoCloseAfterWait });
        if (this.runner) {
            this.runner.stopExecution = false;
        }
        if (this.autoCloseAfterWait) {
            waitForInput(eventManager, { pauseRunner: false });
        }
    }
}

export class GetStringCommand extends Command {
    constructor(private readonly bufferName: string, private readonly textLabel: string) {
        super();
    }

    public execute(_gameState: GameState, _eventManager: EventManager, overworld: OverworldContext): void {
        // ASM: pokecrystal_disassembly/macros/scripts/events.asm::getstring
        const runner = this.runner as ScriptRunnerForString | undefined;
        if (!runner) {
            throw new Error("GetStringCommand requires an active ScriptRunner.");
        }
        const label = String(this.textLabel ?? "").trim();
        if (!label) {
            throw new Error("GetStringCommand requires a non-empty label.");
        }
        const loader = resolveStringDataLoader(runner, overworld);
        const parent =
            typeof runner._find_parent_script_name === "function"
                ? runner._find_parent_script_name()
                : null;
        let resolved: string | null = null;
        if (loader?.get_script || loader?.getScript) {
            const scriptData =
                loader?.get_script?.(label, parent ?? undefined)
                ?? loader?.getScript?.(label, parent ?? undefined)
                ?? null;
            if (scriptData) {
                resolved = extractStringFromScript(scriptData);
            }
        }
        if (!resolved && (loader?.get_text || loader?.getText)) {
            resolved = loader?.get_text?.(label) ?? loader?.getText?.(label) ?? null;
        }
        if (!resolved) {
            resolved = asmStringLoader.get(label) || null;
        }
        if (!resolved) {
            const parentInfo = parent ? ` (parent ${parent})` : "";
            throw new Error(`GetStringCommand could not resolve '${label}'${parentInfo}.`);
        }
        if (!runner.string_buffers) {
            runner.string_buffers = {};
        }
        runner.string_buffers[this.bufferName] = resolved;
    }
}

export class OpenTextCommand extends Command {
    public execute(gameState: GameState, eventManager: EventManager, overworld: OverworldContext): void {
        openText(eventManager);
    }
}

export class CloseTextCommand extends Command {
    public execute(gameState: GameState, eventManager: EventManager, overworld: OverworldContext): void {
        closeText(eventManager);
    }
}

export class WaitButtonCommand extends Command {
    public execute(gameState: GameState, eventManager: EventManager, overworld: OverworldContext): void {
        waitForInput(eventManager);
    }
}

export class TradeCommand extends Command {
    constructor(private readonly tradeId: string) {
        super();
    }

    public execute(_gameState: GameState, eventManager: EventManager, _overworld: OverworldContext): void {
        const trade = NPC_TRADES[this.tradeId];
        if (!trade) {
            throw new Error(`Unsupported NPC trade '${this.tradeId}'.`);
        }
        const runner = this.runner as ScriptRunnerForText | undefined;
        if (!runner) {
            throw new Error("TradeCommand requires an active ScriptRunner.");
        }
        if (!runner.string_buffers) {
            runner.string_buffers = {};
        }
        runner.string_buffers.STRING_BUFFER_1 = trade.requestedMon;
        runner.string_buffers.STRING_BUFFER_2 = trade.offeredMon;
        const label = TRADE_INTRO_LABEL_BY_DIALOG_SET[trade.dialogSet];
        const rawText = asmTextLoader.get(label);
        if (!rawText) {
            throw new Error(`Missing ASM text for NPC trade label '${label}'.`);
        }
        const text =
            runner.formatText?.(rawText) ??
            runner.format_text?.(rawText) ??
            rawText;
        showText(eventManager, text);
    }
}

export class YesOrNoCommand extends Command {
    public on_result?: (value: boolean) => void;

    public execute(_gameState: GameState, eventManager: EventManager, overworld: OverworldContext): void {
        const runner = this.runner as YesNoScriptRunner | undefined;
        if (!runner) {
            throw new Error("YesOrNoCommand requires an active ScriptRunner.");
        }

        const consumeChoice = runner._consume_script_choice ?? runner._consumeScriptChoice;
        if (typeof consumeChoice === "function") {
            const value = consumeChoice("_yesorno_choice", null);
            if (value !== null && value !== undefined) {
                const result = Boolean(value);
                runner.last_yes_no_result = result;
                runner.last_condition_result = result;
                return;
            }
        }

        const dialogue = (overworld as DialogueOverworldContext).dialogue ?? null;
        if (!dialogue) {
            throw new Error("YesOrNoCommand requires an overworld dialogue controller.");
        }

        if (!dialogue.active) {
            openText(eventManager);
        }
        if (typeof runner.pause === "function") {
            runner.pause();
        }
        waitForInput(eventManager);
        eventManager.dispatch(
            new Event("prompt_yes_no", {
                callback: typeof this.on_result === "function" ? this.on_result : null,
            })
        );
    }
}
