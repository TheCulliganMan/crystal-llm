import { createInitialGameState } from "@pokecrystal/core/core/state";
import { getMapMetadataByConstant } from "@pokecrystal/core/engine/world/maps";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { FlyMapPrompt } from "@pokecrystal/core/ui/overlays/fly-map-prompt";

const placeOnMap = (mapConstant: string, gameState: ReturnType<typeof createInitialGameState>): void => {
  const metadata = getMapMetadataByConstant(mapConstant);
  if (!metadata) {
    throw new Error(`Missing metadata for ${mapConstant}`);
  }
  gameState.wram.wMapGroup = metadata.groupId;
  gameState.wram.wMapNumber = metadata.mapId;
};

describe("FlyMapPrompt", () => {
  it("renders an opaque town-map screen instead of overlaying text on the overworld", () => {
    const gameState = createInitialGameState();
    placeOnMap("NEW_BARK_TOWN", gameState);
    const screen = new gameEngine.Surface(160, 144);
    screen.fill([0, 0, 0, 255]);
    const prompt = new FlyMapPrompt(
      {
        screen,
        eventQueue: gameEngine.event.createQueue(),
        update: jest.fn(),
        renderSnapshot: jest.fn(),
      },
      gameState,
      [
        { label: "NEW BARK TOWN", landmark: "LANDMARK_NEW_BARK_TOWN", default: true },
        { label: "CHERRYGROVE CITY", landmark: "LANDMARK_CHERRYGROVE_CITY", default: false },
      ],
      0,
    );

    prompt.draw();

    let changedPixels = 0;
    for (let y = 0; y < 144; y += 16) {
      for (let x = 0; x < 160; x += 16) {
        const [r, g, b] = screen.getAt(x, y);
        if (r !== 0 || g !== 0 || b !== 0) {
          changedPixels += 1;
        }
      }
    }
    expect(changedPixels).toBeGreaterThan(20);
  });

  it("returns -1 on B like ASM _FlyMap", async () => {
    const gameState = createInitialGameState();
    placeOnMap("NEW_BARK_TOWN", gameState);
    const eventQueue = gameEngine.event.createQueue();
    gameEngine.event.post({ type: "keydown", button: "b", is_press: true }, eventQueue);
    const prompt = new FlyMapPrompt(
      {
        screen: new gameEngine.Surface(160, 144),
        eventQueue,
        update: jest.fn(),
      },
      gameState,
      [{ label: "NEW BARK TOWN", landmark: "LANDMARK_NEW_BARK_TOWN", default: true }],
      0,
    );

    await expect(prompt.runAsync()).resolves.toBe(-1);
  });

  it("accepts first-use D-pad and A input on the Fly map event queue", async () => {
    const gameState = createInitialGameState();
    placeOnMap("NEW_BARK_TOWN", gameState);
    const eventQueue = gameEngine.event.createQueue();
    gameEngine.event.post({ type: "keydown", direction: "up", is_press: true }, eventQueue);
    gameEngine.event.post({ type: "keydown", button: "a", is_press: true }, eventQueue);
    const renderSnapshot = jest.fn();
    const prompt = new FlyMapPrompt(
      {
        screen: new gameEngine.Surface(160, 144),
        eventQueue,
        update: jest.fn(),
        renderSnapshot,
      },
      gameState,
      [
        { label: "NEW BARK TOWN", landmark: "LANDMARK_NEW_BARK_TOWN", default: true },
        { label: "CHERRYGROVE CITY", landmark: "LANDMARK_CHERRYGROVE_CITY", default: false },
      ],
      0,
    );

    await expect(prompt.runAsync()).resolves.toBe(1);
    expect(renderSnapshot).toHaveBeenCalledWith(
      ["FLY TO WHERE?"],
      ["D-Pad=Move A=Select B=Back"],
      "FLY TO WHERE?",
      "Legend",
      ["  NEW BARK TOWN", "> CHERRYGROVE CITY"],
      null,
      null,
    );
  });
});
