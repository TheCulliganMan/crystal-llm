import { createInitialGameState } from "@pokecrystal/core/core/state";
import { PlayerState } from "@pokecrystal/core/core/enums/overworld";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { OverworldAudioController } from "@pokecrystal/core/engine/world/overworld/audio-controller";
import { ScriptRunnerImpl } from "@pokecrystal/core/engine/world/story-events/runner";
import { snorlax_awake } from "./snorlax";

describe("SnorlaxAwake", () => {
  it("wakes Snorlax only when the Poke Flute radio channel is still the map music", () => {
    const gameState = createInitialGameState();
    gameState.wram.wXCoord = 34;
    gameState.wram.wYCoord = 10;

    gameState.wram.wMapMusic = "MUSIC_VERMILION_CITY";
    expect(snorlax_awake(gameState)).toBe(false);

    gameState.wram.wMapMusic = "MUSIC_POKE_FLUTE_CHANNEL";
    expect(snorlax_awake(gameState)).toBe(true);
  });

  it("accepts the doubled outdoor runtime coordinates used by Vermilion Snorlax", () => {
    const gameState = createInitialGameState();
    gameState.wram.wMapMusic = "MUSIC_POKE_FLUTE_CHANNEL";
    gameState.wram.wXCoord = 69;
    gameState.wram.wYCoord = 21;

    expect(snorlax_awake(gameState)).toBe(true);

    gameState.wram.wXCoord = 71;
    expect(snorlax_awake(gameState)).toBe(true);
  });

  it("keeps saved Poke Flute radio active through map music startup for VermilionSnorlax", () => {
    const gameState = createInitialGameState();
    gameState.wram.wMapMusic = "MUSIC_POKE_FLUTE_CHANNEL";
    gameState.wram.wXCoord = 69;
    gameState.wram.wYCoord = 21;
    gameState.sram.party.pokemon = [
      {
        species: "CYNDAQUIL",
        level: 20,
        moves: ["TACKLE"],
      } as never,
    ];
    const transport = {
      playMusic: jest.fn(),
      stopMusic: jest.fn(),
      canResolveMusicToken: jest.fn(() => true),
      canResolveSoundToken: jest.fn(() => true),
    };
    const audio = new OverworldAudioController(gameState, transport);
    audio.requestMapMusic("VermilionCity", PlayerState.NORMAL);
    audio.update();

    const dataLoader = new DataLoader();
    dataLoader.get_script = (name: string, parent?: string | null) => {
      if (name === "VermilionSnorlax") {
        return [
          { command: "special", args: ["SnorlaxAwake"] },
          { command: "iftrue", args: [".Awake"] },
          { command: "loadwildmon", args: ["SPEAROW", "17"] },
          { command: "end", args: [] },
        ];
      }
      if (name === ".Awake" && parent === "VermilionSnorlax") {
        return [
          { command: "loadwildmon", args: ["SNORLAX", "50"] },
          { command: "end", args: [] },
        ];
      }
      return null;
    };
    const eventManager = new EventManager(gameState);
    const overworld = { current_map_name: "VermilionCity" };
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld as never);

    runner.run("VermilionSnorlax");

    expect(transport.playMusic).toHaveBeenCalledWith("MUSIC_POKE_FLUTE_CHANNEL", "radio");
    expect(gameState.wram.wMapMusic).toBe("MUSIC_POKE_FLUTE_CHANNEL");
    expect(gameState.wram.wild_pokemon).toEqual({ species: "SNORLAX", level: 50 });
  });
});
