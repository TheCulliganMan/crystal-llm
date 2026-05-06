// ASM mapping: pokecrystal_disassembly/engine/evolve/evolve.asm (stone-driven evolutions).
import { MoveName } from "@pokecrystal/core/core/enums/move";
import { PartyMenuAction } from "@pokecrystal/core/core/enums/party-menu";
import { Pokemon, type PokemonSpecies } from "@pokecrystal/core/core/models";
import { Evolution, type EvolutionEvent } from "@pokecrystal/core/engine/systems/evolution";

type EvolutionPokemonEntry = { name?: string };
type EvolutionPokemonData =
  | Map<string, EvolutionPokemonEntry | PokemonSpecies>
  | Record<string, EvolutionPokemonEntry | PokemonSpecies>
  | Map<string, PokemonSpecies>
  | Record<string, PokemonSpecies>;

type EvolutionTextLoader = {
  getText?: (label: string) => string | null;
  get_text?: (label: string) => string | null;
  pokemonData?: EvolutionPokemonData;
  pokemon_data?: EvolutionPokemonData;
};

type EvolutionGameState = {
  wram?: {
    time_of_day?: string | null;
  };
};

type PokemonMenuAdapter = {
  reset?: () => void;
  setAction?: (action: PartyMenuAction) => void;
  setEvolutionItem?: (item: string) => void;
  requestSelection?: (opts: { handler: (pokemon: Pokemon) => boolean; cancelHandler?: () => void }) => void;
  clearSelectionRequest?: () => void;
  set_action?: (action: PartyMenuAction) => void;
  set_evolution_item?: (item: string) => void;
  request_selection?: (opts: { handler: (pokemon: Pokemon) => boolean; cancel_handler?: () => void }) => void;
  clear_selection_request?: () => void;
};

type ItemSystemAdapter = {
  removeItem?: (item: string) => boolean;
  remove_item?: (item: string) => boolean;
};

type MenuStateLike = {
  dataLoader?: EvolutionTextLoader | null;
  data_loader?: EvolutionTextLoader | null;
  itemSystem?: ItemSystemAdapter;
  item_system?: ItemSystemAdapter;
  gameState?: EvolutionGameState;
  game_state?: EvolutionGameState;
  pokemonMenu?: PokemonMenuAdapter | null;
  pokemon_menu?: PokemonMenuAdapter | null;
  dialogue?: { open: (text: string) => void };
  currentMenu?: string;
  current_menu?: string;
};

type DialogueControlState = {
  dialogueVisible?: boolean;
  _dialogue_visible?: boolean;
  queueDialogueCallback?: (callback: () => void) => void;
  _queue_dialogue_callback?: (callback: () => void) => void;
  dialogueAfterClose?: Array<() => void>;
  runDialogueCallbacks?: () => void;
  _run_dialogue_callbacks?: () => void;
  requestMenuClose?: () => void;
  _request_menu_close?: () => void;
  formatMoveName?: (moveId?: string | number) => string;
  _format_move_name?: (moveId?: string | number) => string;
};

type MenuStateInternal = MenuStateLike & DialogueControlState;

type EvolutionOptions = NonNullable<ConstructorParameters<typeof Evolution>[1]>;

type PokemonLike = Pokemon & { nickname?: string };

class EvolutionItemContext {
  public target: Pokemon | null = null;

  constructor(public readonly itemName: string) {}
}

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

const runDialogueCallbacks = (menuState: MenuStateInternal): void => {
  const handler = menuState.runDialogueCallbacks ?? menuState._run_dialogue_callbacks;
  if (typeof handler === "function") {
    handler.call(menuState);
  }
};

const isEgg = (pokemon: Pokemon): boolean => {
  const species = String(pokemon.species?.id ?? "").toUpperCase();
  const nickname = String(pokemon.nickname ?? "").toUpperCase();
  return species === "EGG" || nickname === "EGG";
};

export class EvolutionItemHandler {
  private context: EvolutionItemContext | null = null;

  constructor(private readonly menuState: MenuStateLike) {}

  reset(): void {
    this.context = null;
  }

  begin(itemName: string): void {
    if (this.context) {
      return;
    }
    this.context = new EvolutionItemContext(itemName);
    const menuState = this.menuState;
    const pokemonMenu = (menuState.pokemonMenu ?? menuState.pokemon_menu) as PokemonMenuAdapter | undefined;
    pokemonMenu?.reset?.();
    pokemonMenu?.setAction?.(PartyMenuAction.EVO_STONE);
    pokemonMenu?.set_action?.(PartyMenuAction.EVO_STONE);
    pokemonMenu?.setEvolutionItem?.(itemName);
    pokemonMenu?.set_evolution_item?.(itemName);
    pokemonMenu?.requestSelection?.({
      handler: (pokemon: Pokemon) => {
        this.handleSelection(pokemon);
        return false;
      },
      cancelHandler: () => this.cancel(),
    });
    pokemonMenu?.request_selection?.({
      handler: (pokemon: Pokemon) => {
        this.handleSelection(pokemon);
        return false;
      },
      cancel_handler: () => this.cancel(),
    });
    const internalState = menuState as MenuStateInternal;
    internalState.currentMenu = "pokemon_menu";
    internalState.current_menu = "pokemon_menu";
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

  handleSelection(pokemon: Pokemon): void {
    const ctx = this.context;
    if (!ctx) {
      return;
    }
    const menuState = this.menuState;
    const internalState = menuState as MenuStateInternal;
    if (isEgg(pokemon)) {
      this.showMessage("Eggs can't use that.");
      return;
    }
    const loader = menuState.dataLoader ?? menuState.data_loader;
    const evolutionOptions: EvolutionOptions = {
      data_loader: loader as EvolutionOptions["data_loader"],
      time_of_day:
        menuState.gameState?.wram?.time_of_day ??
        menuState.game_state?.wram?.time_of_day ??
        undefined,
      current_item: ctx.itemName,
      force_evolution: true,
    };
    const evolution = new Evolution(pokemon, evolutionOptions);
    const candidate = evolution.check_for_evolution();
    if (!candidate) {
      this.showMessage("It won't have any effect.");
      return;
    }
    const itemSystem = (menuState.itemSystem ?? menuState.item_system) as ItemSystemAdapter | undefined;
    const removed = itemSystem?.removeItem?.(ctx.itemName) ?? itemSystem?.remove_item?.(ctx.itemName) ?? false;
    if (!removed) {
      throw new Error(`The bag no longer contains ${ctx.itemName.replace(/_/g, " ")}.`);
    }
    const sourceName =
      String((pokemon as PokemonLike).nickname ?? pokemon.species?.id ?? "").trim() || pokemon.species?.id || "";
    const targetSpecies = candidate.species;
    evolution.evolve(true);
    this.showDialogue(sourceName, targetSpecies, evolution.events);
    this.context = null;
    const pokemonMenu = (menuState.pokemonMenu ?? menuState.pokemon_menu) as PokemonMenuAdapter | undefined;
    pokemonMenu?.clearSelectionRequest?.();
    pokemonMenu?.clear_selection_request?.();
    queueDialogueCallback(internalState, () => {
      const requester = internalState.requestMenuClose ?? internalState._request_menu_close;
      if (typeof requester === "function") {
        requester.call(internalState);
      } else {
        internalState.currentMenu = "bag_menu";
        internalState.current_menu = "bag_menu";
      }
    });
    if (!internalState.dialogueVisible && !internalState._dialogue_visible) {
      runDialogueCallbacks(internalState);
    }
  }

  private showMessage(text: string): void {
    this.menuState.dialogue?.open(text);
    setDialogueVisible(this.menuState as MenuStateInternal, true);
  }

  private resolveText(label: string): string {
    const menuState = this.menuState;
    let text = "";
    const loader = menuState.dataLoader ?? menuState.data_loader;
    if (loader) {
      try {
        text =
          loader.getText?.(label) ??
          loader.get_text?.(label) ??
          "";
      } catch (error) {
        throw new Error(
          `Failed to resolve ASM text for label '${label}': ${error instanceof Error ? error.message : String(error)}`
        );
      }
    }
    if (!text) {
      throw new Error(`Missing ASM text for label '${label}'.`);
    }
    return text;
  }

  private formatEvolutionText(textId: string, sourceName: string, targetSpecies: string): string {
    const text = this.resolveText(textId);
    if (!text) {
      return "";
    }
    let targetDisplay = targetSpecies.replace(/_/g, " ").toUpperCase();
    const loader = this.menuState.dataLoader ?? this.menuState.data_loader;
    const data = loader?.pokemonData ?? loader?.pokemon_data;
    if (data) {
      const entry = data instanceof Map ? data.get(targetSpecies) : data[targetSpecies];
      const nameValue =
        entry && typeof entry === "object" && "name" in entry ? (entry as EvolutionPokemonEntry).name : undefined;
      if (typeof nameValue === "string") {
        targetDisplay = nameValue;
      }
    }
    let formatted = text;
    if ((formatted.match(/@/g) ?? []).length >= 2) {
      formatted = formatted.replace("@", sourceName).replace("@", targetDisplay);
    } else if (formatted.includes("@")) {
      formatted = formatted.replace("@", sourceName);
    }
    return formatted;
  }

  private showDialogue(
    sourceName: string,
    targetSpecies: string,
    events: EvolutionEvent[]
  ): void {
    const menuState = this.menuState;
    const menuStateInternal = menuState as MenuStateInternal;
    const messages: string[] = [];
    for (const event of events) {
      if (event.type === "text") {
        const textId = String(event.id ?? "");
        const formatted = this.formatEvolutionText(textId, sourceName, targetSpecies);
        if (formatted) {
          messages.push(formatted);
        }
      } else if (event.type === "move") {
        let moveName = "";
        const formatter = menuStateInternal.formatMoveName ?? menuStateInternal._format_move_name;
        if (typeof formatter === "function" && event.id !== undefined && event.id !== null) {
          const eventId =
            typeof event.id === "string" || typeof event.id === "number"
              ? event.id
              : String(event.id);
          moveName = formatter.call(menuState, eventId);
        } else {
          moveName = String(event.id ?? "");
        }
        messages.push(`${sourceName} learned ${moveName}!`);
      } else if (event.type === "item") {
        messages.push("The held item disappeared!");
      }
    }
    if (!messages.length) {
      return;
    }
    const combined = messages.join("\n\n");
    menuState.dialogue?.open(combined);
    setDialogueVisible(menuStateInternal, true);
  }
}
