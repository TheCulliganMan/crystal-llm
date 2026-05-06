// ASM mapping: pokecrystal_disassembly/engine/menus/pack.asm (field item selection flow).
import { Item, Pokemon } from "@pokecrystal/core/core/models";
import { ItemEffect, StatusCondition } from "@pokecrystal/core/core/enums";
import { PartyMenuAction } from "@pokecrystal/core/core/enums/party-menu";

type MenuStateLike = {
  gameState?: { sram?: { party?: { pokemon: Array<PokemonLike | null> } } };
  game_state?: { sram?: { party?: { pokemon: Array<PokemonLike | null> } } };
  dataLoader?: unknown;
  data_loader?: unknown;
  itemSystem?: { removeItem: (item: string) => boolean };
  item_system?: { remove_item: (item: string) => boolean };
  pokemonMenu?: {
    reset: () => void;
    setAction?: (action: PartyMenuAction) => void;
    requestSelection?: (opts: { handler: (pokemon: Pokemon, index?: number) => boolean; cancelHandler?: () => void }) => void;
    clearSelectionRequest?: () => void;
    startHpAnimation?: (partyIndex: number, fromHp: number, toHp: number, maxHp: number) => void;
  } | null;
  pokemon_menu?: {
    reset: () => void;
    set_action?: (action: PartyMenuAction) => void;
    request_selection?: (opts: { handler: (pokemon: Pokemon, index?: number) => boolean; cancel_handler?: () => void }) => void;
    clear_selection_request?: () => void;
    start_hp_animation?: (partyIndex: number, fromHp: number, toHp: number, maxHp: number) => void;
  } | null;
  dialogue?: { open: (text: string) => void };
  currentMenu?: string;
  current_menu?: string;
  playSound?: (token: string) => void;
  play_sound?: (token: string) => void;
};

type ItemLookup = {
  itemData?: Map<string, Item> | Record<string, Item>;
  item_data?: Record<string, Item>;
};

type DialogueControlState = {
  dialogueVisible?: boolean;
  _dialogue_visible?: boolean;
  queueDialogueCallback?: (callback: () => void) => void;
  _queue_dialogue_callback?: (callback: () => void) => void;
  dialogueAfterClose?: Array<() => void>;
};

type MenuStateInternal = MenuStateLike & DialogueControlState;

type PokemonMenuAdapter = {
  reset?: () => void;
  setAction?: (action: PartyMenuAction) => void;
  requestSelection?: (opts: { handler: (pokemon: Pokemon, index?: number) => boolean; cancelHandler?: () => void }) => void;
  clearSelectionRequest?: () => void;
  set_action?: (action: PartyMenuAction) => void;
  request_selection?: (opts: { handler: (pokemon: Pokemon, index?: number) => boolean; cancel_handler?: () => void }) => void;
  clear_selection_request?: () => void;
  startHpAnimation?: (partyIndex: number, fromHp: number, toHp: number, maxHp: number) => void;
  start_hp_animation?: (partyIndex: number, fromHp: number, toHp: number, maxHp: number) => void;
};

type ItemSystemAdapter = {
  removeItem?: (item: string) => boolean;
  remove_item?: (item: string) => boolean;
};

type PokemonLike = {
  species?: { id?: string | null } | null;
  nickname?: string | null;
  hp: number;
  max_hp?: number;
  status?: StatusCondition | null;
  sleep_turns?: number;
  confusion_turns?: number;
};

class FieldItemContext {
  constructor(public readonly item: Item) {}
}

const STATUS_CLEAR_ALL = new Set(["FULL_HEAL", "FULL_RESTORE", "HEAL_POWDER", "MIRACLEBERRY"]);
const STATUS_ITEM_MAP: Record<string, StatusCondition> = {
  ANTIDOTE: StatusCondition.POISON,
  BURN_HEAL: StatusCondition.BURN,
  ICE_HEAL: StatusCondition.FREEZE,
  AWAKENING: StatusCondition.SLEEP,
  PARLYZ_HEAL: StatusCondition.PARALYSIS,
  PSNCUREBERRY: StatusCondition.POISON,
  PRZCUREBERRY: StatusCondition.PARALYSIS,
  BURNT_BERRY: StatusCondition.BURN,
  ICE_BERRY: StatusCondition.FREEZE,
  MINT_BERRY: StatusCondition.SLEEP,
};
const REVIVE_FULL_ITEMS = new Set(["MAX_REVIVE", "REVIVAL_HERB"]);
const HP_FULL_ITEMS = new Set(["MAX_POTION", "FULL_RESTORE"]);

const isEgg = (pokemon: Pokemon): boolean => {
  const species = String(pokemon.species?.id ?? "").toUpperCase();
  const nickname = String(pokemon.nickname ?? "").toUpperCase();
  return species === "EGG" || nickname === "EGG";
};

const setDialogueVisible = (menuState: MenuStateInternal, visible: boolean): void => {
  menuState.dialogueVisible = visible;
  menuState._dialogue_visible = visible;
};

const queueDialogueCallback = (menuState: MenuStateInternal, callback: () => void): void => {
  const handler = menuState.queueDialogueCallback ?? menuState._queue_dialogue_callback;
  if (typeof handler === "function") {
    handler.call(menuState, callback);
    return;
  }
  if (Array.isArray(menuState.dialogueAfterClose)) {
    menuState.dialogueAfterClose.push(callback);
  }
};

export class FieldItemHandler {
  private context: FieldItemContext | null = null;

  constructor(private readonly menuState: MenuStateLike) {}

  reset(): void {
    this.context = null;
  }

  findItemDefinition(canonical: string): Item | null {
    const loader = (this.menuState.dataLoader ?? this.menuState.data_loader) as ItemLookup | undefined;
    if (!loader) {
      return null;
    }
    const items = loader.itemData ?? loader.item_data;
    if (!items) {
      return null;
    }
    if (items instanceof Map) {
      for (const item of items.values()) {
        if (item.script_name === canonical) {
          return item;
        }
      }
    } else {
      for (const item of Object.values(items)) {
        if (item.script_name === canonical) {
          return item;
        }
      }
    }
    return null;
  }

  canHandle(item: Item | null): boolean {
    if (!item) {
      return false;
    }
    const fieldMenu = String(item.field_menu ?? "").toUpperCase();
    if (fieldMenu) {
      return fieldMenu === "ITEMMENU_PARTY";
    }
    return (
      item.effect === ItemEffect.STATUS_HEAL ||
      item.effect === ItemEffect.FULL_RESTORE ||
      item.effect === ItemEffect.RESTORE_HP ||
      item.effect === ItemEffect.REVIVE
    );
  }

  begin(item: Item): void {
    if (this.context) {
      return;
    }
    this.context = new FieldItemContext(item);
    const pokemonMenu = (this.menuState.pokemonMenu ?? this.menuState.pokemon_menu) as PokemonMenuAdapter | undefined;
    pokemonMenu?.reset?.();
    pokemonMenu?.setAction?.(PartyMenuAction.HEALING_ITEM);
    pokemonMenu?.set_action?.(PartyMenuAction.HEALING_ITEM);
    pokemonMenu?.requestSelection?.({
      handler: (pokemon: Pokemon, index?: number) => this.handleSelectionWrapper(pokemon, index),
      cancelHandler: () => this.cancel(),
    });
    pokemonMenu?.request_selection?.({
      handler: (pokemon: Pokemon, index?: number) => this.handleSelectionWrapper(pokemon, index),
      cancel_handler: () => this.cancel(),
    });
    const internalState = this.menuState as MenuStateInternal;
    internalState.currentMenu = "pokemon_menu";
    internalState.current_menu = "pokemon_menu";
  }

  private handleSelectionWrapper(pokemon: Pokemon, index?: number): boolean {
    this.handleSelection(pokemon, index);
    return false;
  }

  handleSelection(pokemon: Pokemon, index?: number): void {
    const ctx = this.context;
    if (!ctx) {
      return;
    }
    if (isEgg(pokemon)) {
      this.menuState.dialogue?.open("Eggs can't use that.");
      setDialogueVisible(this.menuState as MenuStateInternal, true);
      return;
    }
    const result = this.applyFieldItemEffect(pokemon as PokemonLike, ctx.item);
    if (result.used) {
      const itemSystem = (this.menuState.itemSystem ?? this.menuState.item_system) as ItemSystemAdapter | undefined;
      const removed = itemSystem?.removeItem?.(ctx.item.script_name) ?? itemSystem?.remove_item?.(ctx.item.script_name) ?? false;
      if (!removed) {
        throw new Error(`Bag no longer contains ${ctx.item.script_name} for field use.`);
      }
    }
    if (result.hpAnimation && result.used) {
      const pokemonMenu =
        (this.menuState.pokemonMenu ?? this.menuState.pokemon_menu) as PokemonMenuAdapter | undefined;
      const partyIndex = typeof index === "number" ? index : -1;
      pokemonMenu?.startHpAnimation?.(
        partyIndex,
        result.hpAnimation.from,
        result.hpAnimation.to,
        result.hpAnimation.maxHp,
      );
      pokemonMenu?.start_hp_animation?.(
        partyIndex,
        result.hpAnimation.from,
        result.hpAnimation.to,
        result.hpAnimation.maxHp,
      );
      const playSound = this.menuState.playSound ?? this.menuState.play_sound;
      playSound?.("SFX_POTION");
    }
    this.menuState.dialogue?.open(result.message);
    const internalState = this.menuState as MenuStateInternal;
    setDialogueVisible(internalState, true);
    queueDialogueCallback(internalState, () => this.finish());
  }

  cancel(): void {
    this.context = null;
    const menuState = this.menuState;
    const pokemonMenu = (menuState.pokemonMenu ?? menuState.pokemon_menu) as PokemonMenuAdapter | undefined;
    pokemonMenu?.clearSelectionRequest?.();
    pokemonMenu?.clear_selection_request?.();
    const internalState = menuState as MenuStateInternal;
    internalState.currentMenu = "bag_menu";
    internalState.current_menu = "bag_menu";
  }

  finish(): void {
    this.context = null;
    const menuState = this.menuState;
    const pokemonMenu =
      (menuState.pokemonMenu ?? menuState.pokemon_menu) as PokemonMenuAdapter | undefined;
    pokemonMenu?.clearSelectionRequest?.();
    pokemonMenu?.clear_selection_request?.();
    const internalState = menuState as MenuStateInternal;
    internalState.currentMenu = "bag_menu";
    internalState.current_menu = "bag_menu";
  }

  private applyFieldItemEffect(
    pokemon: PokemonLike,
    item: Item,
  ): { used: boolean; message: string; hpAnimation?: { from: number; to: number; maxHp: number } } {
    const name = String(pokemon.nickname ?? pokemon.species?.id ?? "").trim() || pokemon.species?.id || "";
    const scriptName = String(item.script_name ?? item.name ?? "")
      .replace(/ /g, "_")
      .toUpperCase();
    const maxHp = Math.max(0, pokemon.max_hp ?? pokemon.hp ?? 0);
    const currentHp = Math.max(0, Math.min(pokemon.hp, maxHp));

    if (item.effect === ItemEffect.REVIVE) {
      if (currentHp > 0) {
        return { used: false, message: "It won't have any effect." };
      }
      const restored = Math.max(1, REVIVE_FULL_ITEMS.has(scriptName) ? maxHp : Math.floor(maxHp / 2));
      pokemon.hp = Math.min(maxHp, restored);
      pokemon.status = undefined;
      pokemon.sleep_turns = 0;
      return {
        used: true,
        message: `${name} was revived!`,
        hpAnimation: { from: currentHp, to: pokemon.hp, maxHp },
      };
    }

    if (item.effect === ItemEffect.FULL_RESTORE) {
      if (currentHp <= 0) {
        return { used: false, message: "It won't have any effect." };
      }
      const hasStatus = Boolean(pokemon.status);
      const hasSleep = (pokemon.sleep_turns ?? 0) > 0;
      const hasConfusion = (pokemon.confusion_turns ?? 0) > 0;
      if (currentHp === maxHp && !hasStatus && !hasSleep && !hasConfusion) {
        return { used: false, message: "It won't have any effect." };
      }
      pokemon.hp = maxHp;
      pokemon.status = undefined;
      pokemon.sleep_turns = 0;
      pokemon.confusion_turns = 0;
      return {
        used: true,
        message: `${name} was fully restored!`,
        hpAnimation: { from: currentHp, to: pokemon.hp, maxHp },
      };
    }

    if (item.effect === ItemEffect.RESTORE_HP) {
      if (currentHp <= 0) {
        return { used: false, message: "It won't have any effect." };
      }
      if (currentHp >= maxHp) {
        return { used: false, message: "It won't have any effect." };
      }
      const amount = this.resolveHealingAmount(item, maxHp);
      const healAmount = amount === null ? maxHp : amount;
      pokemon.hp = Math.min(maxHp, currentHp + healAmount);
      return {
        used: true,
        message: `${name} recovered health!`,
        hpAnimation: { from: currentHp, to: pokemon.hp, maxHp },
      };
    }

    if (item.effect === ItemEffect.STATUS_HEAL) {
      if (currentHp <= 0) {
        return { used: false, message: "It won't have any effect." };
      }
      const hasStatus = Boolean(pokemon.status);
      const hasSleep = (pokemon.sleep_turns ?? 0) > 0 || pokemon.status === StatusCondition.SLEEP;
      const hasConfusion = (pokemon.confusion_turns ?? 0) > 0;
      if (STATUS_CLEAR_ALL.has(scriptName)) {
        if (!hasStatus && !hasSleep && !hasConfusion) {
          return { used: false, message: "It won't have any effect." };
        }
        pokemon.status = undefined;
        pokemon.sleep_turns = 0;
        pokemon.confusion_turns = 0;
        return { used: true, message: `${name} was cured!` };
      }
      const expected = STATUS_ITEM_MAP[scriptName];
      if (expected === undefined) {
        return { used: false, message: "It won't have any effect." };
      }
      if (expected === StatusCondition.SLEEP) {
        if (!hasSleep) {
          return { used: false, message: "It won't have any effect." };
        }
        pokemon.status = undefined;
        pokemon.sleep_turns = 0;
        return { used: true, message: `${name} was cured!` };
      }
      if (!hasStatus || pokemon.status !== expected) {
        return { used: false, message: "It won't have any effect." };
      }
      pokemon.status = undefined;
      pokemon.sleep_turns = 0;
      return { used: true, message: `${name} was cured!` };
    }

    return { used: false, message: "Can't use that here." };
  }

  private resolveHealingAmount(item: Item, maxHp: number): number | null {
    const rawAmount = typeof item.parameter === "number" ? item.parameter : undefined;
    if (rawAmount === -1 || HP_FULL_ITEMS.has(item.script_name)) {
      return null;
    }
    if (typeof rawAmount === "number" && rawAmount > 0) {
      return rawAmount;
    }
    const match = item.description?.match(/(\d+)/);
    if (match) {
      return Math.min(maxHp, Number(match[1]));
    }
    return null;
  }
}
