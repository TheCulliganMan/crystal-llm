import { createInitialGameState } from "@pokecrystal/core/core/state";
import { Ability, EggGroup, GenderRatio, GrowthRate, ItemEffect, PokemonType, StatusCondition } from "@pokecrystal/core/core/enums";
import { type Item as ItemType, type PokemonSpecies } from "@pokecrystal/core/core/models";
import { createPokemon } from "@pokecrystal/core/engine/systems/pokemon";
import { items } from "@pokecrystal/assets/content/items";
import { FieldItemHandler } from "./field-item-handler";

const DEFAULT_BASE_STATS = {
  hp: 20,
  attack: 10,
  defense: 10,
  speed: 10,
  special_attack: 10,
  special_defense: 10,
};

const speciesCache = new Map<string, PokemonSpecies>();

const ensureSpecies = (id: string): PokemonSpecies => {
  const upperId = id.toUpperCase();
  const cached = speciesCache.get(upperId);
  if (cached) {
    return cached;
  }
  const species: PokemonSpecies = {
    id: upperId,
    int_id: 0,
    base_stats: DEFAULT_BASE_STATS,
    type1: PokemonType.NORMAL,
    type2: PokemonType.NONE,
    catch_rate: 45,
    base_exp: 64,
    item1: undefined,
    item2: undefined,
    gender_ratio: GenderRatio.GENDER_F50,
    unknown1: 0,
    step_cycles_to_hatch: 5120,
    unknown2: 0,
    growth_rate: GrowthRate.GROWTH_MEDIUM_FAST,
    egg_group1: EggGroup.EGG_MONSTER,
    egg_group2: EggGroup.EGG_MONSTER,
    tmhm_learnset: [],
    ability: Ability.NONE,
    pic_size: 0,
    front_pic: 0,
    back_pic: 0,
    evolutions: null,
    weight: 0,
  };
  speciesCache.set(upperId, species);
  return species;
};

describe("FieldItemHandler", () => {
  it("does not allow non-party hold-only items", () => {
    const poisonBerry = items.find((item) => item.script_name === "PSNCURE_BERRY");
    if (!poisonBerry) {
      throw new Error("Missing PSNCUREBERRY item data for test.");
    }

    const heldOnlyBerry = {
      ...poisonBerry,
      effect: ItemEffect.STATUS_HEAL,
      field_menu: "ITEMMENU_NOUSE",
    } as ItemType;
    const handler = new FieldItemHandler({
      itemSystem: { removeItem: jest.fn() },
      dialogue: { open: jest.fn() },
    });

    expect(handler.canHandle(heldOnlyBerry)).toBe(false);
    expect(handler.canHandle({ ...heldOnlyBerry, field_menu: "" })).toBe(true);
  });

  it("heals with a potion even when field menu metadata is missing", () => {
    const potion = items.find((item) => item.script_name === "POTION");
    if (!potion) {
      throw new Error("Missing POTION item data for test.");
    }

    const gameState = createInitialGameState();
    const pokemon = createPokemon(gameState, ensureSpecies("CYNDAQUIL"), 5);
    pokemon.max_hp = 40;
    pokemon.hp = 10;

    const menuState = {
      itemSystem: { removeItem: jest.fn(() => true) },
      pokemonMenu: {
        reset: jest.fn(),
        setAction: jest.fn(),
        requestSelection: jest.fn(),
        clearSelectionRequest: jest.fn(),
      },
      dialogue: { open: jest.fn() },
      queueDialogueCallback: jest.fn((callback: () => void) => callback()),
      currentMenu: "bag_menu",
      current_menu: "bag_menu",
    };

    const handler = new FieldItemHandler(menuState);

    expect(handler.canHandle(potion)).toBe(true);

    handler.begin(potion);
    handler.handleSelection(pokemon);

    expect(pokemon.hp).toBe(30);
    expect(menuState.itemSystem.removeItem).toHaveBeenCalledWith("POTION");
    expect(menuState.dialogue.open).toHaveBeenCalledWith("CYNDAQUIL recovered health!");
    expect(menuState.currentMenu).toBe("bag_menu");
  });

  it("animates party HP when a healing item restores HP", () => {
    const potion = items.find((item) => item.script_name === "POTION");
    if (!potion) {
      throw new Error("Missing POTION item data for test.");
    }

    const gameState = createInitialGameState();
    const pokemon = createPokemon(gameState, ensureSpecies("CYNDAQUIL"), 5);
    pokemon.max_hp = 40;
    pokemon.hp = 10;

    const menuState = {
      itemSystem: { removeItem: jest.fn(() => true) },
      pokemonMenu: {
        reset: jest.fn(),
        setAction: jest.fn(),
        requestSelection: jest.fn(),
        clearSelectionRequest: jest.fn(),
        startHpAnimation: jest.fn(),
      },
      dialogue: { open: jest.fn() },
      queueDialogueCallback: jest.fn((callback: () => void) => callback()),
      playSound: jest.fn(),
      currentMenu: "bag_menu",
      current_menu: "bag_menu",
    };

    const handler = new FieldItemHandler(menuState);

    handler.begin(potion);
    handler.handleSelection(pokemon, 0);

    expect(menuState.pokemonMenu.startHpAnimation).toHaveBeenCalledWith(0, 10, 30, 40);
    expect(menuState.playSound).toHaveBeenCalledWith("SFX_POTION");
  });

  it("uses param-based healing amounts for berries", () => {
    const berry = items.find((item) => item.script_name === "BERRY");
    if (!berry) {
      throw new Error("Missing BERRY item data for test.");
    }

    const gameState = createInitialGameState();
    const pokemon = createPokemon(gameState, ensureSpecies("CYNDAQUIL"), 8);
    pokemon.max_hp = 26;
    pokemon.hp = 4;

    const menuState = {
      itemSystem: { removeItem: jest.fn(() => true) },
      pokemonMenu: {
        reset: jest.fn(),
        setAction: jest.fn(),
        requestSelection: jest.fn(),
        clearSelectionRequest: jest.fn(),
      },
      dialogue: { open: jest.fn() },
      queueDialogueCallback: jest.fn((callback: () => void) => callback()),
      currentMenu: "bag_menu",
      current_menu: "bag_menu",
    };

    const handler = new FieldItemHandler(menuState);

    handler.begin(berry);
    handler.handleSelection(pokemon);

    expect(pokemon.hp).toBe(14);
    expect(menuState.itemSystem.removeItem).toHaveBeenCalledWith("BERRY");
    expect(menuState.dialogue.open).toHaveBeenCalledWith("CYNDAQUIL recovered health!");
  });

  it("cures poisoned Pokémon with a status-heal item", () => {
    const antidote = items.find((item) => item.script_name === "ANTIDOTE");
    if (!antidote) {
      throw new Error("Missing ANTIDOTE item data for test.");
    }

    const gameState = createInitialGameState();
    const pokemon = createPokemon(gameState, ensureSpecies("CYNDAQUIL"), 5);
    pokemon.max_hp = 35;
    pokemon.hp = 35;
    pokemon.status = StatusCondition.POISON;

    const menuState = {
      itemSystem: { removeItem: jest.fn(() => true) },
      pokemonMenu: {
        reset: jest.fn(),
        setAction: jest.fn(),
        requestSelection: jest.fn(),
        clearSelectionRequest: jest.fn(),
      },
      dialogue: { open: jest.fn() },
      queueDialogueCallback: jest.fn((callback: () => void) => callback()),
      currentMenu: "bag_menu",
      current_menu: "bag_menu",
    };

    const handler = new FieldItemHandler(menuState);

    handler.begin(antidote);
    handler.handleSelection(pokemon, 0);

    expect(pokemon.status).toBeUndefined();
    expect(menuState.itemSystem.removeItem).toHaveBeenCalledWith("ANTIDOTE");
    expect(menuState.dialogue.open).toHaveBeenCalledWith("CYNDAQUIL was cured!");
    expect(menuState.currentMenu).toBe("bag_menu");
  });

  it("returns no effect when status-heal has nothing to cure", () => {
    const fullHeal = items.find((item) => item.script_name === "FULL_HEAL");
    if (!fullHeal) {
      throw new Error("Missing FULL_HEAL item data for test.");
    }

    const gameState = createInitialGameState();
    const pokemon = createPokemon(gameState, ensureSpecies("CYNDAQUIL"), 5);
    pokemon.max_hp = 40;
    pokemon.hp = 40;

    const menuState = {
      itemSystem: { removeItem: jest.fn(() => true) },
      pokemonMenu: {
        reset: jest.fn(),
        setAction: jest.fn(),
        requestSelection: jest.fn(),
        clearSelectionRequest: jest.fn(),
      },
      dialogue: { open: jest.fn() },
      queueDialogueCallback: jest.fn((callback: () => void) => callback()),
      currentMenu: "bag_menu",
      current_menu: "bag_menu",
    };

    const handler = new FieldItemHandler(menuState);

    handler.begin(fullHeal);
    handler.handleSelection(pokemon, 0);

    expect(menuState.itemSystem.removeItem).not.toHaveBeenCalled();
    expect(menuState.dialogue.open).toHaveBeenCalledWith("It won't have any effect.");
  });

  it("uses MAX_REVIVE only on fainted Pokémon", () => {
    const maxRevive = items.find((item) => item.script_name === "MAX_REVIVE");
    if (!maxRevive) {
      throw new Error("Missing MAX_REVIVE item data for test.");
    }

    const faintedState = createInitialGameState();
    const faintedPokemon = createPokemon(faintedState, ensureSpecies("CYNDAQUIL"), 5);
    faintedPokemon.max_hp = 50;
    faintedPokemon.hp = 0;
    faintedPokemon.status = StatusCondition.BURN;

    const faintedMenuState = {
      itemSystem: { removeItem: jest.fn(() => true) },
      pokemonMenu: {
        reset: jest.fn(),
        setAction: jest.fn(),
        requestSelection: jest.fn(),
        clearSelectionRequest: jest.fn(),
      },
      dialogue: { open: jest.fn() },
      queueDialogueCallback: jest.fn((callback: () => void) => callback()),
      currentMenu: "bag_menu",
      current_menu: "bag_menu",
    };

    const faintedHandler = new FieldItemHandler(faintedMenuState);
    faintedHandler.begin(maxRevive);
    faintedHandler.handleSelection(faintedPokemon, 0);

    expect(faintedPokemon.hp).toBe(50);
    expect(faintedPokemon.status).toBeUndefined();
    expect(faintedMenuState.itemSystem.removeItem).toHaveBeenCalledWith("MAX_REVIVE");

    const livingState = createInitialGameState();
    const livingPokemon = createPokemon(livingState, ensureSpecies("CYNDAQUIL"), 5);
    livingPokemon.max_hp = 50;
    livingPokemon.hp = 5;

    const livingMenuState = {
      itemSystem: { removeItem: jest.fn(() => true) },
      pokemonMenu: {
        reset: jest.fn(),
        setAction: jest.fn(),
        requestSelection: jest.fn(),
        clearSelectionRequest: jest.fn(),
      },
      dialogue: { open: jest.fn() },
      queueDialogueCallback: jest.fn((callback: () => void) => callback()),
      currentMenu: "bag_menu",
      current_menu: "bag_menu",
    };

    const livingHandler = new FieldItemHandler(livingMenuState);
    livingHandler.begin(maxRevive);
    livingHandler.handleSelection(livingPokemon, 0);

    expect(livingMenuState.itemSystem.removeItem).not.toHaveBeenCalled();
    expect(livingMenuState.dialogue.open).toHaveBeenCalledWith("It won't have any effect.");
  });

  it("fully restores with FULL_RESTORE when needed and does nothing when already max and clear", () => {
    const fullRestore = items.find((item) => item.script_name === "FULL_RESTORE");
    if (!fullRestore) {
      throw new Error("Missing FULL_RESTORE item data for test.");
    }

    const statusState = createInitialGameState();
    const statusPokemon = createPokemon(statusState, ensureSpecies("CYNDAQUIL"), 5);
    statusPokemon.max_hp = 60;
    statusPokemon.hp = 12;
    statusPokemon.status = StatusCondition.SLEEP;
    statusPokemon.sleep_turns = 2;

    const statusMenuState = {
      itemSystem: { removeItem: jest.fn(() => true) },
      pokemonMenu: {
        reset: jest.fn(),
        setAction: jest.fn(),
        requestSelection: jest.fn(),
        clearSelectionRequest: jest.fn(),
      },
      dialogue: { open: jest.fn() },
      queueDialogueCallback: jest.fn((callback: () => void) => callback()),
      currentMenu: "bag_menu",
      current_menu: "bag_menu",
    };

    const statusHandler = new FieldItemHandler(statusMenuState);
    statusHandler.begin(fullRestore);
    statusHandler.handleSelection(statusPokemon, 0);

    expect(statusPokemon.hp).toBe(60);
    expect(statusPokemon.status).toBeUndefined();
    expect(statusPokemon.sleep_turns).toBe(0);
    expect(statusMenuState.itemSystem.removeItem).toHaveBeenCalledWith("FULL_RESTORE");

    const maxState = createInitialGameState();
    const maxPokemon = createPokemon(maxState, ensureSpecies("CYNDAQUIL"), 5);
    maxPokemon.max_hp = 60;
    maxPokemon.hp = 60;

    const maxMenuState = {
      itemSystem: { removeItem: jest.fn(() => true) },
      pokemonMenu: {
        reset: jest.fn(),
        setAction: jest.fn(),
        requestSelection: jest.fn(),
        clearSelectionRequest: jest.fn(),
      },
      dialogue: { open: jest.fn() },
      queueDialogueCallback: jest.fn((callback: () => void) => callback()),
      currentMenu: "bag_menu",
      current_menu: "bag_menu",
    };

    const maxHandler = new FieldItemHandler(maxMenuState);
    maxHandler.begin(fullRestore);
    maxHandler.handleSelection(maxPokemon, 0);

    expect(maxMenuState.itemSystem.removeItem).not.toHaveBeenCalled();
    expect(maxMenuState.dialogue.open).toHaveBeenCalledWith("It won't have any effect.");
  });

  it("rejects eggs", () => {
    const antidote = items.find((item) => item.script_name === "ANTIDOTE");
    if (!antidote) {
      throw new Error("Missing ANTIDOTE item data for test.");
    }

    const gameState = createInitialGameState();
    const pokemon = createPokemon(gameState, ensureSpecies("EGG"), 1);
    pokemon.max_hp = 20;
    pokemon.hp = 20;

    const menuState = {
      itemSystem: { removeItem: jest.fn(() => true) },
      pokemonMenu: {
        reset: jest.fn(),
        setAction: jest.fn(),
        requestSelection: jest.fn(),
        clearSelectionRequest: jest.fn(),
      },
      dialogue: { open: jest.fn() },
      queueDialogueCallback: jest.fn((callback: () => void) => callback()),
      currentMenu: "bag_menu",
      current_menu: "bag_menu",
    };

    const handler = new FieldItemHandler(menuState);

    handler.begin(antidote);
    handler.handleSelection(pokemon, 0);

    expect(menuState.itemSystem.removeItem).not.toHaveBeenCalled();
    expect(menuState.dialogue.open).toHaveBeenCalledWith("Eggs can't use that.");
  });
});
