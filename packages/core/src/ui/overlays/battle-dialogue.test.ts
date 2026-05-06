import { createInitialGameState } from "@pokecrystal/core/core/state";
import { getDebugLogEntries } from "@pokecrystal/core/core/debug-log";
import { Surface } from "@pokecrystal/core/ui/surface";
import { DialogueWindow } from "../text/dialogue";
import { BattleTextWindowSchema } from "./_battle-layout";
import { enqueue_text, type DialogueState } from "./battle-dialogue";
import { buildBattleSnapshot } from "../text-overlays";
import { BattleStateEnum } from "../../engine/battle/battle/battle-context";

describe("battle dialogue logging", () => {
  it("pushes debug log entries when text is queued and started", () => {
    const initialCount = getDebugLogEntries().length;
    const gameState = createInitialGameState();
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn(), charWidth: 8 },
      drawTextBox: jest.fn(),
      draw_window: jest.fn(),
    };
    const window = BattleTextWindowSchema.parse({
      tile_x: 0,
      tile_y: 12,
      width_tiles: 20,
      height_tiles: 6,
    });
    const dialogue = new DialogueWindow(ui, gameState, 2);
    const state: DialogueState = {
      window,
      dialogue,
      queue: [],
      pending_waits: 0,
      forced_visible: false,
      auto_close_after_display: false,
    };

    enqueue_text(state, "Wild SENTRET appeared!");

    const entries = getDebugLogEntries().slice(initialCount);
    expect(entries.length).toBeGreaterThanOrEqual(2);
    expect(entries[entries.length - 1]?.message).toContain("[battle] dialogue start");
  });

  it("publishes the full current page to text snapshots before typewriter reveal completes", () => {
    const gameState = createInitialGameState();
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn(), charWidth: 8 },
      drawTextBox: jest.fn(),
      draw_window: jest.fn(),
    };
    const window = BattleTextWindowSchema.parse({
      tile_x: 0,
      tile_y: 12,
      width_tiles: 20,
      height_tiles: 6,
    });
    const dialogue = new DialogueWindow(ui, gameState, 2);
    const state = {
      context: {
        currentState: BattleStateEnum.BATTLE_END,
        playerPokemon: {
          nickname: "TOTODILEAA",
          level: 35,
          hp: 97,
          max_hp: 100,
          status: "OK",
          experience: 0,
          moves: [],
        },
        enemyPokemon: {
          nickname: "POLIWHIRL",
          level: 18,
          hp: 0,
          max_hp: 70,
          status: "OK",
          moves: [],
        },
      },
      dialogue: {
        window,
        dialogue,
        queue: [],
        pending_waits: 1,
        forced_visible: true,
        auto_close_after_display: false,
      },
      yes_no_prompt: { active: false, prompt: null },
      active_move_learn: null,
    };

    dialogue.open("TOTODILEAA grew to level 35!");

    expect(dialogue.visible_text).toBe("");
    expect(buildBattleSnapshot(state as never)?.dialogueLines).toEqual([
      "TOTODILEAA grew to",
      "level 35!",
    ]);
  });
});
