import { createInitialGameState } from "@/core/state";
import { DataLoader } from "@/core/data-loader";
import { PlayerState } from "@/core/enums/overworld";
import {
  DoFishing,
  FishingBite,
  FishingRodState,
  roll_fishing_encounter,
} from "@/engine/world/overworld/fishing";
import { OverworldFieldMoveMixin } from "@/engine/world/overworld/overworld-field-moves";
import type { HardwareRNG } from "@/engine/games/rng";

const createRng = (...bytes: number[]): HardwareRNG => {
  let index = 0;
  return {
    nextByte: () => {
      if (index >= bytes.length) {
        throw new Error("RNG byte sequence exhausted.");
      }
      const value = bytes[index];
      index += 1;
      return value;
    },
  } as HardwareRNG;
};

const createLoaderWithGroup = (group: string): DataLoader => {
  const loader = new DataLoader();
  loader.map_attributes.set("TEST_MAP", { fishing_group: group } as never);
  return loader;
};

describe("fishing encounter selection", () => {
  it("uses night time group entries when time_of_day is NIGHT", () => {
    const gameState = createInitialGameState();
    gameState.wram.time_of_day = "NIGHT";
    const loader = createLoaderWithGroup("FISHGROUP_SHORE");
    const rng = createRng(0, 231);

    const outcome = roll_fishing_encounter(gameState, loader, "TEST_MAP", "GOOD_ROD", rng);

    expect(outcome.encounter).toEqual({ species: "STARYU", level: 20 });
  });

  it("resolves swarm groups when the swarm flag is set", () => {
    const gameState = createInitialGameState();
    gameState.wram.daily_flags1 = 1 << 2;
    gameState.wram.wFishingSwarmFlag = 1;
    const loader = createLoaderWithGroup("FISHGROUP_QWILFISH");
    const rng = createRng(0, 10);

    const outcome = roll_fishing_encounter(gameState, loader, "TEST_MAP", "OLD_ROD", rng);

    expect(outcome.group).toBe("FISHGROUP_QWILFISH_SWARM");
  });
});

describe("fishing timing", () => {
  it("triggers a bite after the cast delay", () => {
    const gameState = createInitialGameState();
    const loader = createLoaderWithGroup("FISHGROUP_SHORE");
    const rng = createRng(0, 0);

    const session = DoFishing(gameState, loader, "TEST_MAP", "OLD_ROD", rng);

    expect(session.bite_delay_frames).toBe(0);
    expect(FishingBite(gameState, session, { current_frame: 39 })).toBeNull();
    expect(FishingBite(gameState, session, { current_frame: 40 })).toBe(true);
    expect(gameState.wram.wFishingRodState).toBe(FishingRodState.BITE);
    expect(gameState.wram.wFishingResult).toBe(1);
  });
});

describe("fishing field move checks", () => {
  class FishingTestOverworld extends OverworldFieldMoveMixin {
    public shownText: string | null = null;
    public player_state: PlayerState = PlayerState.SURF;

    protected override _show_field_move_text(label: string): void {
      this.shownText = label;
    }

    protected override async _show_field_move_text_async(label: string): Promise<void> {
      this.shownText = label;
    }
  }

  it("refuses fishing while surfing", async () => {
    const overworld = new FishingTestOverworld();

    await expect(overworld.handle_fishing("OLD_ROD")).resolves.toBe(false);
    expect(overworld.shownText).toBe("CantFishHereText");
  });
});
