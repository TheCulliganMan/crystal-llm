import { createInitialGameState } from "@pokecrystal/core/core/state";
import type { MenuUI } from "@pokecrystal/core/ui/menus/types";
import { OptionsMenu } from "@pokecrystal/core/ui/menus/options-menu";
import type { BagMenu } from "@pokecrystal/core/ui/menus/bag-menu";
import type { MoveReorderMenu } from "@pokecrystal/core/ui/menus/move-reorder-menu";
import type { PokemonMenu } from "@pokecrystal/core/ui/menus/pokemon-menu";
import {
  buildActiveOptionMenuControlLines,
  buildBagMenuLines,
  buildBagControlLines,
  buildBattleSnapshot,
  buildBattleControlLines,
  buildDialogueControlLines,
  buildContinueScreenControlLines,
  buildGenderSelectionControlLines,
  buildIntroSequenceControlLines,
  buildMoveMenuControlLines,
  buildOakIntroControlLines,
  buildOptionsMenuControlLines,
  buildOptionsMenuLines,
  buildOverworldMetadata,
  buildPokemonMenuControlLines,
  buildPokemonMenuLines,
  buildPokemonStatsControlLines,
  buildPromptControlLines,
  buildStartMenuControlLines,
  buildTitleScreenControlLines,
  buildTrainerCardControlLines,
} from "@pokecrystal/core/ui/text-overlays";
import { createTestPokemon } from "@pokecrystal/core/engine/world/story-events/test-utils";
import { BattleStateEnum } from "@pokecrystal/core/engine/battle/battle/battle-context";
import { BattleMenu } from "@pokecrystal/core/ui/overlays/_battle-menu";
import { MoveName } from "@pokecrystal/core/core/enums";

describe("text overlay helpers", () => {
  it("buildPokemonMenuLines includes party, cancel, and submenu options", () => {
    const pokemon = createTestPokemon("PIKACHU", 25, {
      nickname: "SPARK",
      level: 5,
      hp: 10,
      max_hp: 20,
    });
    const menuStub = {
      getPartyEntries: () => [{ index: 0, pokemon }],
      getCursorIndex: () => 0,
      getMode: () => "submenu",
      getSubmenuChoices: () => [{ label: "STATS" }, { label: "MOVE" }],
      getSubmenuIndex: () => 1,
    } as unknown as PokemonMenu;

    const lines = buildPokemonMenuLines(menuStub);

    expect(lines[0]).toBe("▶ SPARK L5 HP 10/20 OK");
    expect(lines.some((line) => line.trim() === "CANCEL")).toBe(true);
    expect(lines).toContain("SUBMENU:");
    expect(lines).toContain("▶ MOVE");
  });

  it("buildOptionsMenuLines reflects the current selection", () => {
    const gameState = createInitialGameState();
    const ui: MenuUI = {
      screen: null,
      tileSize: 8,
      font: { renderText: jest.fn() },
      drawWindow: jest.fn(),
    };
    const optionsMenu = new OptionsMenu(ui, null, gameState);

    const lines = buildOptionsMenuLines(optionsMenu);

    expect(lines[0]).toBe("▶ TEXT SPEED: FAST");
    expect(lines[lines.length - 1].trim()).toBe("CANCEL");
  });

  it("buildBagControlLines covers list and action modes", () => {
    const listMenu = { getMode: () => "list" } as Pick<BagMenu, "getMode">;
    const actionMenu = { getMode: () => "actions" } as Pick<BagMenu, "getMode">;

    expect(buildBagControlLines(listMenu)).toEqual([
      "D-Pad=Move L/R=Pocket A=Select Select=Register B=Close",
    ]);
    expect(buildBagControlLines(actionMenu)).toEqual(["D-Pad=Move A=Confirm B=Back"]);
  });

  it("buildBagMenuLines marks hidden rows above and below scrollable lists", () => {
    const bagMenu = {
      getCurrentPocketLabel: () => "ITEMS",
      getVisibleItems: () => [["POTION", 1], ["ANTIDOTE", 2]],
      getCurrentItems: () => [["BERRY", 1], ["POTION", 1], ["ANTIDOTE", 2], ["CANCEL", 0]],
      getScrollOffset: () => 1,
      getListIndex: () => 2,
      getMode: () => "list",
    } as unknown as BagMenu;

    expect(buildBagMenuLines(bagMenu)).toEqual([
      "POCKET: ITEMS",
      "▲ more above",
      "   POTION ×01",
      "▶ ANTIDOTE ×02",
      "▼ more below",
    ]);
  });

  it("buildPokemonMenuControlLines matches each mode", () => {
    const makeMenu = (mode: string) => ({ getMode: () => mode } as PokemonMenu);

    expect(buildPokemonMenuControlLines(makeMenu("list"))).toEqual(["D-Pad=Move A=Select B=Back"]);
    expect(buildPokemonMenuControlLines(makeMenu("submenu"))).toEqual(["D-Pad=Move A=Confirm B=Back"]);
    expect(buildPokemonMenuControlLines(makeMenu("switch"))).toEqual(["D-Pad=Move A=Swap B=Cancel"]);
    expect(buildPokemonMenuControlLines(makeMenu("give_take"))).toEqual([
      "Up/Down=Toggle A=Confirm B=Back",
    ]);
  });

  it("buildMoveMenuControlLines reflects swap state", () => {
    const standardMenu = { getSwapOrigin: () => null } as Pick<MoveReorderMenu, "getSwapOrigin">;
    const swapMenu = { getSwapOrigin: () => 1 } as Pick<MoveReorderMenu, "getSwapOrigin">;

    expect(buildMoveMenuControlLines(standardMenu)).toEqual([
      "D-Pad=Move L/R=Pokemon A=Pick B=Back",
    ]);
    expect(buildMoveMenuControlLines(swapMenu)).toEqual(["D-Pad=Move A=Swap B=Cancel"]);
  });

  it("buildPokemonStatsControlLines handles eggs", () => {
    const normal = createTestPokemon("PIKACHU", 25);
    const egg = createTestPokemon("EGG", 0);

    expect(buildPokemonStatsControlLines(normal)).toEqual([
      "L/R/A=Page",
      "Up/Down=Pokemon B=Back",
    ]);
    expect(buildPokemonStatsControlLines(egg)).toEqual(["A/B=Back"]);
  });

  it("buildMenuControlLines constants are stable", () => {
    expect(buildStartMenuControlLines()).toEqual(["D-Pad=Move A/Start=Select B=Close"]);
    expect(buildOptionsMenuControlLines()).toEqual(["D-Pad=Move L/R=Change A=Exit B=Back"]);
    expect(buildTrainerCardControlLines()).toEqual(["L/R=Page A=Toggle B/Start=Exit"]);
    expect(buildActiveOptionMenuControlLines()).toEqual(["D-Pad=Move A=Confirm B=Back"]);
  });

  it("buildScreenControlLines stay consistent across intro/title helpers", () => {
    expect(buildGenderSelectionControlLines(false)).toEqual(["Up/Down=Choose A=Confirm"]);
    expect(buildGenderSelectionControlLines(true)).toEqual(["WAIT: applying choice"]);
    expect(buildContinueScreenControlLines()).toEqual(["A=Continue B=Back"]);
    expect(buildIntroSequenceControlLines(false)).toEqual(["A/START/SELECT/B=Skip intro"]);
    expect(buildIntroSequenceControlLines(true)).toEqual(["WAIT: transitioning to title"]);
    expect(buildOakIntroControlLines({
      waitingForInput: true,
      canRevealText: false,
      allowSkip: true,
    })).toEqual(["A/START=Advance", "B=Skip intro"]);
    expect(buildOakIntroControlLines({
      waitingForInput: false,
      canRevealText: true,
      allowSkip: false,
    })).toEqual(["A/START=Show full text"]);
    expect(buildTitleScreenControlLines("main")).toEqual([
      "A/START=Main menu",
      "Up+B+Select=Delete save",
      "DOWN+B+SELECT arms reset clock",
    ]);
    expect(buildTitleScreenControlLines("timeout")).toEqual(["WAIT: returning to intro"]);
    expect(buildTitleScreenControlLines("entrance")).toEqual(["WAIT: title entrance"]);
  });

  it("buildDialogueControlLines and battle prompts are consistent", () => {
    expect(buildDialogueControlLines()).toEqual(["A=Advance B=Close"]);
    expect(buildPromptControlLines()).toEqual(["Up/Down=Choose A=OK B=Cancel"]);
    expect(buildBattleControlLines({ hasPrompt: true, hasDialogue: true })).toEqual(buildPromptControlLines());
    expect(buildBattleControlLines({ hasPrompt: false, hasDialogue: true })).toEqual(buildDialogueControlLines());
    expect(buildBattleControlLines({ hasPrompt: false, hasDialogue: false })).toEqual([
      "D-Pad=Move A=Confirm B=Back",
    ]);
  });

  it("buildBattleSnapshot hides stale battle menu lines outside player action selection", () => {
    const player = createTestPokemon("CYNDAQUIL", 155, {
      nickname: "CYNDAQUIL",
      level: 8,
      hp: 0,
      max_hp: 26,
      experience: 358,
    });
    const enemy = createTestPokemon("PIDGEY", 16, {
      nickname: "PIDGEY",
      level: 7,
      hp: 16,
      max_hp: 22,
    });
    const snapshot = buildBattleSnapshot({
      context: {
        currentState: BattleStateEnum.POST_TURN_EFFECTS,
        playerPokemon: player,
        enemyPokemon: enemy,
      },
      wram: {
        current_menu: BattleMenu.MAIN,
        wBattleMenuCursorPosition: 0,
      },
      dialogue: {
        forced_visible: false,
        pending_waits: 0,
        queue: [],
        dialogue: {
          visible_text: "",
          is_complete: () => true,
          has_more_pages: () => false,
        },
      },
      yes_no_prompt: {
        active: false,
        prompt: null,
      },
    } as never);

    expect(snapshot?.menuLines).toBeNull();
  });

  it("buildBattleSnapshot lays out the main battle menu as a 2x2 grid", () => {
    const player = createTestPokemon("TOTODILE", 158, {
      nickname: "MENTWO",
      level: 31,
      hp: 85,
      max_hp: 222,
    });
    const enemy = createTestPokemon("DODUO", 84, {
      nickname: "DODUO",
      level: 31,
      hp: 80,
      max_hp: 80,
    });
    const snapshot = buildBattleSnapshot({
      context: {
        currentState: BattleStateEnum.PLAYER_ACTION_SELECT,
        playerPokemon: player,
        enemyPokemon: enemy,
      },
      game_state: {
        wram: {},
      },
      wram: {
        current_menu: BattleMenu.MAIN,
        wBattleMenuCursorPosition: 1,
      },
      dialogue: {
        forced_visible: false,
        pending_waits: 0,
        queue: [],
        dialogue: {
          visible_text: "",
          is_complete: () => true,
          has_more_pages: () => false,
        },
      },
      yes_no_prompt: {
        active: false,
        prompt: null,
      },
    } as never);

    expect(snapshot?.menuLines).toEqual([
      "   FIGHT ▶ <PKMN>",
      "   PACK     RUN",
    ]);
  });

  it("buildBattleSnapshot labels disabled battle moves instead of showing PP", () => {
    const player = createTestPokemon("CROCONAW", 159, {
      nickname: "TOTODILEAA",
      moves: [
        { name: MoveName.BITE, current_pp: 25 },
        { name: MoveName.SCRATCH, current_pp: 35 },
      ],
      disabled_move: MoveName.BITE,
      disable_turns: 6,
    });
    const enemy = createTestPokemon("PIDGEY", 16);

    const snapshot = buildBattleSnapshot({
      context: {
        currentState: BattleStateEnum.PLAYER_ACTION_SELECT,
        playerPokemon: player,
        enemyPokemon: enemy,
      },
      wram: {
        current_menu: BattleMenu.FIGHT,
        wMoveMenuCursorPosition: 0,
      },
      dialogue: {
        forced_visible: false,
        pending_waits: 0,
        queue: [],
        dialogue: {
          visible_text: "",
          is_complete: () => true,
          has_more_pages: () => false,
        },
      },
      yes_no_prompt: {
        active: false,
        prompt: null,
      },
    } as never);

    expect(snapshot?.menuLines).toContain("▶ BITE (DISABLED)");
    expect(snapshot?.menuLines).toContain("   SCRATCH (PP 35/35)");
  });

  it("buildOverworldMetadata includes player position when available", () => {
    const lines = buildOverworldMetadata({ wram: { player_x: 12, player_y: 34 } }, null);

    expect(lines).toEqual([
      "D-Pad=Move A=Talk Start=Menu Select=Item B=Back",
      "Pos: (12,34)",
    ]);
  });

  it("buildOverworldMetadata prefers stable map coords (wXCoord/wYCoord) over player coords", () => {
    const lines = buildOverworldMetadata(
      { wram: { player_x: 40, player_y: 41, wXCoord: 12, wYCoord: 13 } },
      null
    );

    expect(lines).toEqual([
      "D-Pad=Move A=Talk Start=Menu Select=Item B=Back",
      "Pos: (12,13)",
    ]);
  });

  it("buildOverworldMetadata prefers explicit rendered player coords over stale WRAM mirrors", () => {
    const lines = buildOverworldMetadata(
      { wram: { player_x: 40, player_y: 41, wXCoord: 12, wYCoord: 13 } },
      null,
      { x: 47, y: 9 }
    );

    expect(lines).toEqual([
      "D-Pad=Move A=Talk Start=Menu Select=Item B=Back",
      "Pos: (47,9)",
    ]);
  });
});
