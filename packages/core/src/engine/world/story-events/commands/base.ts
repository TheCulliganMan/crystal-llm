import { GameState } from "@pokecrystal/core/core/state";
import { PlayerGender } from "@pokecrystal/core/core/enums";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { ItemSystem } from "@pokecrystal/core/engine/systems/items";
import { OverworldMap } from "@pokecrystal/core/engine/world/overworld";
import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { Item } from "@pokecrystal/core/core/models";
import type { ScriptRunner } from "../runner";

export type OverworldContext = (OverworldMap | OverworldEngine) & { [key: string]: any };

export type EventManagerLike = EventManager | { [key: string]: any };

export abstract class Command {
    public runner?: ScriptRunner;
    protected _childPushed: boolean = false;

    public abstract execute(
        gameState: GameState,
        eventManager: EventManagerLike,
        overworld: OverworldContext
    ): void;
}

export interface ScriptFrame {
    name: string;
    commands: Command[];
    index: number;
    allowFallthrough?: boolean;
    parent?: string;
}

export type ItemSystemLike = ItemSystem;

const looksLikeItemSystem = (value: unknown): value is ItemSystemLike => {
    if (!value || typeof value !== "object") {
        return false;
    }
    const candidate = value as Partial<ItemSystem>;
    return (
        typeof candidate.addItem === "function" ||
        typeof candidate.hasItem === "function" ||
        typeof candidate.removeItem === "function"
    );
};

type ItemSystemHolder = {
    item_system?: unknown;
    itemSystem?: unknown;
};

const getHolderItemSystem = (holder: ItemSystemHolder | null | undefined): ItemSystemLike | null => {
    if (!holder) {
        return null;
    }
    if (looksLikeItemSystem(holder.item_system)) {
        return holder.item_system;
    }
    if (looksLikeItemSystem(holder.itemSystem)) {
        return holder.itemSystem;
    }
    return null;
};

export const resolveItemSystem = (
    runner?: ScriptRunner,
    overworld?: OverworldContext,
): ItemSystemLike | null => {
    return (
        getHolderItemSystem(overworld as ItemSystemHolder | null) ??
        getHolderItemSystem(runner as ItemSystemHolder | null)
    );
};

export const queueStandardScript = (runner: ScriptRunner | undefined, scriptName: string): void => {
    if (!runner) {
        return;
    }
    runner.run(scriptName);
    runner.stopExecution = true;
};

export const selectPackFullScript = (gameState: GameState): string => {
    const gender = gameState.sram.player_gender ?? PlayerGender.MALE;
    if (gender === PlayerGender.FEMALE) {
        return "PackFullFScript";
    }
    return "PackFullMScript";
};

export const hasItemInSystem = (itemSystem: ItemSystem | null, itemName: string): boolean => {
    if (!itemSystem) {
        return false;
    }
    if (typeof itemSystem.hasItem !== "function") {
        throw new Error("ItemSystem implementation missing hasItem().");
    }
    return itemSystem.hasItem(itemName);
};

export const removeItemFromSystem = (itemSystem: ItemSystem | null, itemName: string, quantity: number = 1): boolean => {
    if (!itemSystem) {
        return false;
    }
    if (typeof itemSystem.removeItem !== "function") {
        throw new Error("ItemSystem implementation missing removeItem().");
    }
    return itemSystem.removeItem(itemName, quantity);
};

export const resolveDisplayName = (itemSystem: ItemSystem | null, itemName: string): string => {
    if (itemSystem) {
        try {
            if (typeof itemSystem.getDisplayName === "function") {
                const displayName = itemSystem.getDisplayName(itemName);
                const normalized = String(displayName ?? "").replace(/_/g, " ").trim();
                if (normalized) {
                    return normalized.toUpperCase();
                }
            }
        } catch (error) {
            throw new Error(
                `ItemSystem getDisplayName() failed for '${itemName}': ${error instanceof Error ? error.message : String(error)}`
            );
        }
    }
    return itemName.replace(/_/g, " ").toUpperCase();
};

export const addItemToBag = (
    gameState: GameState,
    itemSystem: ItemSystem | null,
    itemName: string,
    quantity: number = 1
): boolean => {
    if (itemSystem) {
        if (typeof itemSystem.addItem !== "function") {
            throw new Error("ItemSystem implementation missing addItem().");
        }
        return itemSystem.addItem(itemName, quantity);
    }

    const inventory = gameState.sram.items;
    inventory[itemName] = (inventory[itemName] ?? 0) + quantity;
    const keyInventory = gameState.sram.key_items;
    if (itemName in keyInventory) {
        keyInventory[itemName] += quantity;
    } else if (itemName.endsWith("_CARD") || itemName.endsWith("GEAR")) {
        keyInventory[itemName] = quantity;
    }
    return true;
};

export const populateItemStringBuffers = (runner: ScriptRunner | undefined, displayName: string): void => {
    if (!runner || !displayName) {
        return;
    }
    if (!runner.string_buffers) {
        runner.string_buffers = {};
    }
    runner.string_buffers["STRING_BUFFER_1"] = displayName;
    runner.string_buffers["STRING_BUFFER_4"] = displayName;
};

export const normalizeScriptName = (name: string): string => {
    const cleaned = name.split(";", 1)[0].trim();
    if (!cleaned) {
        throw new Error("Script reference resolved to an empty name.");
    }
    return cleaned;
};
