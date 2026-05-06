import { createInitialGameState } from "@pokecrystal/core/core/state";
import { _CHAR_MAP } from "@pokecrystal/core/ui/tilemap-surface";
import { PokegearScreen } from "./pokegear";
import { PokegearCard } from "./pokegear-state";

const createPokegearUi = () => ({ screen: null, font: {} });

const buildRadioScreen = () => {
  const gameState = createInitialGameState();
  gameState.wram.engine_flags.ENGINE_POKEGEAR = true;
  gameState.wram.engine_flags.ENGINE_RADIO_CARD = true;
  const audioEngine = {
    playSound: jest.fn(),
    startRadioChannel: jest.fn(),
    stopRadioChannel: jest.fn(),
  };
  const screen = new PokegearScreen(createPokegearUi(), gameState, { audioEngine } as any);
  screen.logic.forceCard(PokegearCard.RADIO);
  return { gameState, screen, audioEngine };
};

describe("Pokegear radio audio", () => {
  it("starts the tuned radio station through the audio engine", () => {
    const { screen, audioEngine } = buildRadioScreen();

    screen.draw();

    expect(audioEngine.startRadioChannel).toHaveBeenCalledWith("OAKS_POKEMON_TALK", 0);
  });

  it("retunes audio when the radio frequency changes", () => {
    const { screen, audioEngine } = buildRadioScreen();

    screen.draw();
    screen.handleInput({ type: "keydown", key: "ArrowUp" });

    expect(audioEngine.startRadioChannel).toHaveBeenNthCalledWith(1, "OAKS_POKEMON_TALK", 0);
    expect(audioEngine.startRadioChannel).toHaveBeenNthCalledWith(2, "POKEMON_MUSIC", 0);
  });

  it("keeps a tuned station playing when leaving the radio card", () => {
    const { screen, audioEngine } = buildRadioScreen();

    screen.draw();
    screen.handleInput({ type: "keydown", key: "ArrowRight" });

    expect(audioEngine.stopRadioChannel).not.toHaveBeenCalled();
  });

  it("keeps a tuned station playing when closing Pokegear", () => {
    const { screen, audioEngine } = buildRadioScreen();

    screen.draw();
    screen.handleInput({ type: "keydown", key: "Escape" });

    expect(audioEngine.stopRadioChannel).not.toHaveBeenCalled();
  });
});

describe("Pokegear radio tilemap", () => {
  it("places the tuned station name in the ASM radio station area", () => {
    const { screen } = buildRadioScreen();
    screen.logic.setRadioIndex(3);

    const tilemap = screen.buildTilemap();

    expect(tilemap.getTile(4, 6)).not.toBe(_CHAR_MAP.T);
    expect(tilemap.getTile(8, 6)).not.toBe(_CHAR_MAP["1"]);
    expect(tilemap.getTile(2, 9)).toBe(_CHAR_MAP.B);
    expect(tilemap.getTile(3, 9)).toBe(_CHAR_MAP.U);
  });
});
