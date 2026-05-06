import { PlayerState } from "@pokecrystal/core/core/enums/overworld";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import type { EventManager } from "@pokecrystal/core/engine/events/events";
import {
  CutCommand,
  FlashCommand,
  FlyCommand,
  StrengthCommand,
  SurfCommand,
  WaterfallCommand,
  WhirlpoolCommand,
} from "./movement";

describe("field move script commands", () => {
  it("invokes every HM handler with the overworld as this", () => {
    const gameState = createInitialGameState();
    const calls: Array<[string, unknown, unknown[]]> = [];
    const overworld = {
      player_state: PlayerState.SURF,
      handle_cut(...args: unknown[]) {
        calls.push(["CUT", this, args]);
        return true;
      },
      handle_surf(...args: unknown[]) {
        calls.push(["SURF", this, args]);
        return true;
      },
      _handle_hm(...args: unknown[]) {
        calls.push([String(args[0]).toUpperCase(), this, args]);
        return true;
      },
      handle_flash(...args: unknown[]) {
        calls.push(["FLASH", this, args]);
        return true;
      },
      handle_fly(...args: unknown[]) {
        calls.push(["FLY", this, args]);
        return true;
      },
    };

    new CutCommand(1, 2).execute(gameState, {} as EventManager, overworld as never);
    new SurfCommand(3, 4).execute(gameState, {} as EventManager, overworld as never);
    new StrengthCommand(5, 6).execute(gameState, {} as EventManager, overworld as never);
    new WhirlpoolCommand(7, 8).execute(gameState, {} as EventManager, overworld as never);
    new WaterfallCommand(9, 10).execute(gameState, {} as EventManager, overworld as never);
    new FlashCommand().execute(gameState, {} as EventManager, overworld as never);
    new FlyCommand(11, 12).execute(gameState, {} as EventManager, overworld as never);

    expect(calls).toEqual([
      ["CUT", overworld, [1, 2]],
      ["SURF", overworld, [3, 4]],
      ["STRENGTH", overworld, ["Strength", 5, 6, PlayerState.SURF]],
      ["WHIRLPOOL", overworld, ["Whirlpool", 7, 8, PlayerState.SURF]],
      ["WATERFALL", overworld, ["Waterfall", 9, 10, PlayerState.SURF]],
      ["FLASH", overworld, []],
      ["FLY", overworld, [11, 12]],
    ]);
  });
});
