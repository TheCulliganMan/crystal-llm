import { createInitialGameState } from "@pokecrystal/core/core/state";
import { KEYS } from "@pokecrystal/core/core/keycodes";
import { PokemonStatsScreen } from "./pokemon-stats";
import { updateJoypadState } from "@pokecrystal/core/input/joypad";
import { createTestPokemon } from "@pokecrystal/core/engine/world/story-events/test-utils";
import { calculateExperience } from "@pokecrystal/core/engine/experience";
import { MoveName } from "@pokecrystal/core/core/enums";
import { loadMoveMetadata } from "@pokecrystal/core/ui/overlays/battle-experience";

describe("PokemonStatsScreen input", () => {
  it("responds to DOM keycodes for B button", () => {
    const gameState = createInitialGameState();
    const pokemon = createTestPokemon("TESTMON", 1);
    gameState.sram.party.pokemon[0] = pokemon;

    const statsScreen = new PokemonStatsScreen(
      { screen: null, font: {} },
      gameState
    );
    statsScreen.showPokemon(pokemon);

    const result = statsScreen.handleInput({
      type: KEYS.KEYDOWN,
      key: 88, // DOM keycode for X (B button default)
      is_press: true,
    });

    expect(result).toBe("exit");
  });

  it("exits immediately for MCP-style B button events", () => {
    const gameState = createInitialGameState();
    const pokemon = createTestPokemon("TESTMON", 1);
    gameState.sram.party.pokemon[0] = pokemon;

    const statsScreen = new PokemonStatsScreen(
      { screen: null, font: {} },
      gameState
    );
    statsScreen.showPokemon(pokemon);

    const result = statsScreen.handleInput({
      type: KEYS.KEYDOWN,
      button: "b",
      is_press: true,
    });

    expect(result).toBe("exit");
    expect(statsScreen.getActivePokemon()).toBeNull();
  });

  it("latches page changes when joypad state was pre-updated", () => {
    const gameState = createInitialGameState();
    const pokemon = createTestPokemon("TESTMON", 1);
    gameState.sram.party.pokemon[0] = pokemon;

    const statsScreen = new PokemonStatsScreen(
      { screen: null, font: {} },
      gameState
    );
    statsScreen.showPokemon(pokemon);

    const event = {
      type: KEYS.KEYDOWN,
      key: KEYS.RIGHT,
      is_press: true,
    };
    updateJoypadState(gameState.hram.joypad, event);

    statsScreen.handleInput(event);

    expect(gameState.wram.wStatsScreenFlags & 0x03).toBe(2);
  });
});

describe("PokemonStatsScreen text overlay", () => {
  it("includes detailed status lines on the first page", () => {
    const gameState = createInitialGameState();
    const pokemon = createTestPokemon("TESTMON", 1, {
      level: 5,
      hp: 20,
      max_hp: 20,
      experience: 1234,
      original_trainer_name: "RYAN",
      original_trainer_id: 7,
    });
    gameState.sram.party.pokemon[0] = pokemon;

    const statsScreen = new PokemonStatsScreen({ screen: null, font: {} }, gameState);
    statsScreen.showPokemon(pokemon);

    const overlay = statsScreen.getTextOverlay();

    const findLine = (prefix: string) => overlay.viewportLines.find((line) => line.startsWith(prefix));

    expect(findLine("HP:")).toBeDefined();
    expect(overlay.viewportLines).toEqual(expect.arrayContaining(["STATUS: OK", "TYPE: NORMAL"]));
    expect(overlay.viewportLines).toEqual(expect.arrayContaining(["EXP: 1234"]));
    const nextLevelExp = calculateExperience(pokemon.species.growth_rate, pokemon.level + 1);
    expect(overlay.viewportLines).toContain(`TO NEXT: ${Math.max(0, nextLevelExp - pokemon.experience)}`);
    expect(findLine("EXP FILL:")).toBeDefined();
    expect(findLine("HP BAR:")).toBeDefined();
  });

  it("lists item and moves on the second page", () => {
    const gameState = createInitialGameState();
    const pokemon = createTestPokemon("TESTMON", 1, {
      item: "MIRACLE_SEED",
      moves: [
        { name: MoveName.TACKLE, current_pp: 10 },
        { name: MoveName.GROWL, current_pp: 20 },
      ],
    });
    gameState.sram.party.pokemon[0] = pokemon;

    const statsScreen = new PokemonStatsScreen({ screen: null, font: {} }, gameState);
    statsScreen.showPokemon(pokemon);
    statsScreen.handleInput({ type: KEYS.KEYDOWN, key: KEYS.RIGHT, is_press: true });

    const overlay = statsScreen.getTextOverlay();
    const moveMetadata = loadMoveMetadata();
    const tacklePp = moveMetadata.get(MoveName.TACKLE)?.pp ?? 10;
    const growlPp = moveMetadata.get(MoveName.GROWL)?.pp ?? 20;

    expect(overlay.viewportLines).toEqual(
      expect.arrayContaining(["ITEM: MIRACLE SEED", "MOVE 1: TACKLE", `PP1: 10/${tacklePp}`])
    );
    expect(overlay.viewportLines).toEqual(
      expect.arrayContaining(["MOVE 2: GROWL", `PP2: 20/${growlPp}`])
    );
  });

  it("lists computed stats and DVs on the third page", () => {
    const gameState = createInitialGameState();
    const pokemon = createTestPokemon("TESTMON", 1, {
      dvs: { attack: 3, defense: 4, speed: 5, special: 6, hp: 0 },
      hp_exp: 10,
      attack_exp: 11,
      defense_exp: 12,
      speed_exp: 13,
      special_exp: 14,
    });
    gameState.sram.party.pokemon[0] = pokemon;

    const statsScreen = new PokemonStatsScreen({ screen: null, font: {} }, gameState);
    statsScreen.showPokemon(pokemon);
    statsScreen.handleInput({ type: KEYS.KEYDOWN, key: KEYS.RIGHT, is_press: true });
    statsScreen.handleInput({ type: KEYS.KEYDOWN, key: KEYS.RIGHT, is_press: true });

    const overlay = statsScreen.getTextOverlay();

    const overlayText = overlay.viewportLines.join("\n");
    expect(overlayText).toMatch(/ATTACK:\s+\d+/);
    expect(overlayText).toMatch(/DEFENSE:\s+\d+/);
    expect(overlayText).toMatch(/SPCL ATK:\s+\d+/);
    expect(overlayText).toMatch(/SPCL DEF:\s+\d+/);
    expect(overlayText).toMatch(/SPEED:\s+\d+/);
    expect(overlay.viewportLines).toContain("DVS: ATK 3 DEF 4 SPD 5 SPC 6");
    expect(overlay.viewportLines).toContain("STAT EXP: HP 10 ATK 11 DEF 12");
    expect(overlay.viewportLines).toContain("STAT EXP: SPC 14 SPD 13");
  });
});
