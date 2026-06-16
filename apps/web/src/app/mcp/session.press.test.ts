import { getMcpSession, __testing, type Direction } from "./session";
import type { McpMapInfoSnapshot } from "./map-info";

const pokecenterHealPointCases = [
  ["AzaleaPokecenter1F", "AzaleaPokecenter1FNurseScript"],
  ["BlackthornPokecenter1F", "BlackthornPokecenter1FNurseScript"],
  ["CeladonPokecenter1F", "CeladonPokecenter1FNurseScript"],
  ["CeruleanPokecenter1F", "CeruleanPokecenter1FNurseScript"],
  ["CherrygrovePokecenter1F", "CherrygrovePokecenter1FNurseScript"],
  ["CianwoodPokecenter1F", "CianwoodPokecenter1FNurseScript"],
  ["CinnabarPokecenter1F", "CinnabarPokecenter1FNurseScript"],
  ["EcruteakPokecenter1F", "EcruteakPokecenter1FNurseScript"],
  ["FuchsiaPokecenter1F", "FuchsiaPokecenter1FNurseScript"],
  ["GoldenrodPokecenter1F", "GoldenrodPokecenter1FNurseScript"],
  ["IndigoPlateauPokecenter1F", "IndigoPlateauPokecenter1FNurseScript"],
  ["LavenderPokecenter1F", "LavenderPokecenter1FNurseScript"],
  ["MahoganyPokecenter1F", "MahoganyPokecenter1FNurseScript"],
  ["OlivinePokecenter1F", "OlivinePokecenter1FNurseScript"],
  ["PewterPokecenter1F", "PewterPokecenter1FNurseScript"],
  ["Route10Pokecenter1F", "Route10Pokecenter1FNurseScript"],
  ["Route32Pokecenter1F", "Route32Pokecenter1FNurseScript"],
  ["SaffronPokecenter1F", "SaffronPokecenter1FNurseScript"],
  ["SilverCavePokecenter1F", "SilverCavePokecenter1FNurseScript"],
  ["VermilionPokecenter1F", "VermilionPokecenter1FNurseScript"],
  ["VioletPokecenter1F", "VioletPokecenterNurse"],
  ["ViridianPokecenter1F", "ViridianPokecenter1FNurseScript"],
] as const;

describe("McpGameSession interactive battle presses", () => {
  beforeEach(() => {
    __testing.clearSessions();
  });

  afterEach(() => {
    __testing.clearSessions();
  });

  it.each([
    ["wild", false],
    ["trainer", true],
  ] as const)("keeps instant %s battle menu A presses to the minimum frame budget", async (_kind, trainerBattle) => {
    const session = getMcpSession(`press-instant-${_kind}-battle-menu-confirm`);
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: {
        viewport?: string[];
        info?: string[];
        titles?: { viewport: string; info: string; menu?: string; prompt?: string };
        menu?: string[] | null;
        prompt?: string[] | null;
        dialogue?: string[] | null;
        marker?: string[] | null;
      } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { instant_mode: boolean; player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getBattle: () => { context: { trainerBattle: boolean } };
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("BATTLE\n\nMENU\nFIGHT  PKMN\nPACK   RUN");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      titles: { viewport: "Battle", info: "Legend" },
      viewport: ["BATTLE"],
      info: [],
      prompt: null,
      dialogue: null,
      menu: ["▶ FIGHT", "  <PKMN>", "  PACK", "  RUN"],
      marker: null,
    };
    const game = {
      getGameState: () => ({ wram: { instant_mode: true, player_x: 95, player_y: 31 } }),
      getMapName: () => "Route30",
      isMenuOpen: () => true,
      isBattleActive: () => true,
      getBattle: () => ({ context: { trainerBattle } }),
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    const frameCounts: number[] = [];
    sessionAny.stepFrames = jest.fn((count: number) => {
      frameCounts.push(count);
      sessionAny.lastSnapshot = {
        prompt: null,
        dialogue: null,
        menu: ["▶ TACKLE (PP 35/35)", "  LEER (PP 30/30)", "  CANCEL"],
      };
    });

    const action = await session.press("a", 1);

    expect(action.result.ok).toBe(true);
    expect(frameCounts).toEqual([2]);
    expect(sessionAny.stepFrames).toHaveBeenCalledTimes(1);
  });

  it("returns compact instant battle press snapshots without passive blank-handoff settling", async () => {
    const session = getMcpSession("press-instant-battle-compact-no-passive-settle");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      getModalUiState: jest.Mock;
      getGame: jest.Mock;
      isMenuOpenForSession: jest.Mock;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
      captureSnapshot: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      prompt: null,
      dialogue: null,
      menu: ["▶ FIGHT", "  <PKMN>", "  PACK", "  RUN"],
    };
    const blankBattleModal = {
      in_battle: true,
      in_menu: false,
      in_dialog: false,
      text_box_open: false,
      prompt_pending: false,
      movement_locked: false,
      script_busy: false,
      input_blocked_reason: "battle",
      can_move: false,
    };
    const game = {
      getGameState: () => ({ wram: { instant_mode: true, player_x: 95, player_y: 31 } }),
      getMapName: () => "Route30",
      isMenuOpen: () => false,
      isBattleActive: () => true,
      getBattle: () => ({ context: { trainerBattle: true } }),
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.isMenuOpenForSession = jest.fn(() => false);
    sessionAny.readFacingDirection = jest.fn(() => "down");
    sessionAny.readBestMapName = jest.fn(() => "Route30");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "Route30", group: 26, number: 1 }));
    sessionAny.getModalUiState = jest.fn(() => blankBattleModal);
    sessionAny.scheduleKeyPress = jest.fn();
    const frameCounts: number[] = [];
    sessionAny.stepFrames = jest.fn((count: number) => {
      frameCounts.push(count);
      sessionAny.lastSnapshot = {
        titles: { viewport: "Battle", info: "Legend" },
        viewport: ["BATTLE"],
        info: [],
        prompt: null,
        dialogue: null,
        menu: null,
        marker: null,
      };
    });
    sessionAny.captureSnapshot = jest.fn(() => {
      sessionAny.lastSnapshot = {
        titles: { viewport: "Battle", info: "Legend" },
        viewport: ["BATTLE"],
        info: [],
        prompt: null,
        dialogue: null,
        menu: null,
        marker: null,
      };
    });

    const action = await session.press("a", 1, { settleSnapshot: false });

    expect(action.result.ok).toBe(true);
    expect(frameCounts).toEqual([2]);
  });

  it("gives battle-menu A presses enough settle frames to carry move confirms forward", async () => {
    const session = getMcpSession("press-battle-menu-confirm-targeted");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      prompt: null,
      dialogue: null,
      menu: ["▶ TACKLE (PP 28/35)", "  LEER (PP 30/30)", "  SMOKESCREEN (PP 20/20)", "  CANCEL"],
    };
    const game = {
      getGameState: () => ({ wram: { player_x: 81, player_y: 35 } }),
      getMapName: () => "Route29",
      isMenuOpen: () => true,
      isBattleActive: () => true,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn((count: number) => {
      if (count >= 5) {
        sessionAny.lastSnapshot = {
          prompt: null,
          dialogue: ["CYNDAQUIL used TACKLE!"],
          menu: null,
        };
      }
    });

    const action = await session.press("a", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.stepFrames).toHaveBeenCalledWith(5);
  });

  it("gives battle-menu A one extra bounded settle pass when the move menu lands late", async () => {
    const session = getMcpSession("press-battle-menu-late-open");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      prompt: null,
      dialogue: null,
      menu: ["▶ FIGHT", "  <PKMN>", "  PACK", "  RUN"],
    };
    let steppedFrames = 0;
    const game = {
      getGameState: () => ({ wram: { player_x: 95, player_y: 31 } }),
      getMapName: () => "Route29",
      isMenuOpen: () => true,
      isBattleActive: () => true,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn((count: number) => {
      steppedFrames += count;
      if (steppedFrames >= 9) {
        sessionAny.lastSnapshot = {
          prompt: null,
          dialogue: null,
          menu: ["▶ TACKLE (PP 11/35)", "  LEER (PP 30/30)", "  SMOKESCREEN (PP 20/20)", "  CANCEL"],
        };
      }
    });

    const action = await session.press("a", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(1, 5);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(2, 4);
  });

  it("waits through delayed Fly destination prompt resolution before classifying the press", async () => {
    const session = getMcpSession("press-fly-destination-delayed-resolution");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: {
        viewport?: string[];
        info?: string[];
        titles?: { viewport: string; info: string };
        menu?: string[] | null;
        prompt?: string[] | null;
        dialogue?: string[] | null;
      } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      waitForInputOwningSurfaceSettle: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      viewport: ["FLY TO WHERE?"],
      info: ["D-Pad=Move A=Select B=Back"],
      menu: ["> NEW BARK TOWN", "  CHERRYGROVE CITY"],
      prompt: null,
      dialogue: null,
      titles: { viewport: "FLY TO WHERE?", info: "Legend" },
    };
    let mapName = "RuinsOfAlphOutside";
    const game = {
      getGameState: () => ({ wram: { player_x: 29, player_y: 17 } }),
      getMapName: () => mapName,
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    let settleCount = 0;
    sessionAny.waitForInputOwningSurfaceSettle = jest.fn(async () => {
      settleCount += 1;
      if (settleCount >= 3) {
        mapName = "NewBarkTown";
        sessionAny.lastSnapshot = {
          viewport: ["OVERWORLD"],
          info: ["D-Pad=Move A=Talk Start=Menu Select=Item B=Back"],
          menu: null,
          prompt: null,
          dialogue: null,
          titles: { viewport: "Overworld", info: "Legend" },
        };
      }
    });

    const action = await session.press("a", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.waitForInputOwningSurfaceSettle).toHaveBeenCalledTimes(4);
  });

  it("treats runner interaction state changes as real overworld press progress even before dialogue text lands", async () => {
    const session = getMcpSession("press-overworld-runner-interaction-progress");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      requestAutosave: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number; last_talked: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: {
            is_busy?: boolean;
            _script_stack?: unknown[];
            _awaiting_resume?: number;
            _queued_overworld_task_count?: number;
            stop_execution?: boolean;
            last_interaction_object_index?: number | null;
          } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.requestAutosave = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { prompt: null, dialogue: null, menu: null };
    const overworld = {
      script_runner: {
        is_busy: false,
        _script_stack: [],
        _awaiting_resume: 0,
        _queued_overworld_task_count: 0,
        stop_execution: false,
        last_interaction_object_index: null,
      },
      player_movement_locked: () => false,
      script_tasks_active: () => false,
    };
    const wram = { player_x: 15, player_y: 5, last_talked: 0 };
    const game = {
      getGameState: () => ({ wram }),
      getMapName: () => "ElmsLab",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => overworld,
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn(() => {
      overworld.script_runner = {
        ...overworld.script_runner,
        _script_stack: [{ name: "CyndaquilPokeBallScript" }],
        _awaiting_resume: 1,
        stop_execution: true,
        last_interaction_object_index: 3,
      };
      wram.last_talked = 3;
    });

    const action = await session.press("a", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.requestAutosave).toHaveBeenCalledWith({ force: true });
  });

  it("clears stale queued directional input before an overworld A press", async () => {
    const session = getMcpSession("press-overworld-clears-stale-directional-input");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      requestAutosave: jest.Mock;
      clearStaleDirectionalInput: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.requestAutosave = jest.fn().mockResolvedValue(undefined);
    sessionAny.clearStaleDirectionalInput = jest.fn(() => true);
    sessionAny.lastSnapshot = { prompt: null, dialogue: null, menu: null };
    const game = {
      getGameState: () => ({ wram: { player_x: 13, player_y: 5 } }),
      getMapName: () => "ElmsLab",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn(() => {
      sessionAny.lastSnapshot = {
        prompt: null,
        dialogue: ["ELM: Go on, choose one!"],
        menu: null,
      };
    });

    const action = await session.press("a", 1);

    expect(sessionAny.clearStaleDirectionalInput).toHaveBeenCalledWith(game);
    expect(action.result.changed).toBe(true);
    expect(action.result.events).toContain("stale_input_cleared");
  });

  it("gives confirmed overworld object interactions enough settle time to land late script state", async () => {
    const session = getMcpSession("press-overworld-confirmed-object-late-settle");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      requestAutosave: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number; last_talked: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          current_map_name: string;
          player_direction: string;
          script_runner: {
            is_busy?: boolean;
            _script_stack?: unknown[];
            _awaiting_resume?: number;
            _queued_overworld_task_count?: number;
            stop_execution?: boolean;
            last_interaction_object_index?: number | null;
          } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
          get_facing_tile_coords: () => [number, number];
          _counter_adjusted_tile: (x: number, y: number) => [number, number];
          _npc_on_tile: () => null;
          _nearest_npc_covering_subtile: () => null;
          _bg_event_at: () => null;
          npcs: Array<{
            objectIndex: number;
            x: number;
            y: number;
            hidden?: boolean;
            name?: string;
            event?: { script?: string };
          }>;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.requestAutosave = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { prompt: null, dialogue: null, menu: null };
    const wram = { player_x: 15, player_y: 5, last_talked: 0 };
    const overworld = {
      current_map_name: "ElmsLab",
      player_direction: "down",
      script_runner: {
        is_busy: false,
        _script_stack: [],
        _awaiting_resume: 0,
        _queued_overworld_task_count: 0,
        stop_execution: false,
        last_interaction_object_index: null,
      },
      player_movement_locked: () => false,
      script_tasks_active: () => false,
      get_facing_tile_coords: () => [15, 7] as [number, number],
      _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
      _npc_on_tile: () => null,
      _nearest_npc_covering_subtile: () => null,
      _bg_event_at: () => null,
      npcs: [
        {
          objectIndex: 3,
          x: 15,
          y: 7,
          event: { script: "TotodilePokeBallScript" },
          name: "TOTODILE BALL",
        },
      ],
    };
    const game = {
      getGameState: () => ({ wram }),
      getMapName: () => "ElmsLab",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => overworld,
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    let steppedFrames = 0;
    sessionAny.stepFrames = jest.fn((count: number) => {
      steppedFrames += count;
      if (steppedFrames >= 69) {
        overworld.script_runner = {
          ...overworld.script_runner,
          _script_stack: [{ name: "TotodilePokeBallScript" }],
          _awaiting_resume: 1,
          stop_execution: true,
          last_interaction_object_index: 3,
        };
        wram.last_talked = 3;
      }
    });

    const action = await session.press("a", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(1, 3);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(2, 4);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(3, 8);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(4, 16);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(5, 32);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(6, 64);
  });

  it("retries confirmed Pokecenter healer scripts when a stale hardware press only changes the viewport", async () => {
    const session = getMcpSession("press-healer-viewport-only-retry");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { viewport?: string[]; prompt?: string[] | null; dialogue?: string[] | null; menu?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      requestAutosave: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number; last_talked: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          current_map_name: string;
          player_direction: string;
          script_runner: {
            is_busy?: boolean;
            run: jest.Mock;
            _script_stack?: unknown[];
            _awaiting_resume?: number;
            _queued_overworld_task_count?: number;
            stop_execution?: boolean;
            last_interaction_object_index?: number | null;
          };
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
          get_facing_tile_coords: () => [number, number];
          _counter_adjusted_tile: (x: number, y: number) => [number, number];
          _npc_on_tile: (x: number, y: number) => {
            objectIndex: number;
            x: number;
            y: number;
            walking: boolean;
            jumping: boolean;
            event: { script: string };
          } | null;
          _nearest_npc_covering_subtile: () => null;
          _bg_event_at: () => null;
          _play_interaction_sound: jest.Mock;
        };
      };
      getGame: jest.Mock;
      buildSnapshotMapInfo: jest.Mock;
      captureSceneSignal: jest.Mock;
      buildStateFingerprint: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OVERWORLD");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.requestAutosave = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { viewport: ["old"], prompt: null, dialogue: null, menu: null };
    const wram = { player_x: 7, player_y: 7, last_talked: 0 };
    const scriptRunner = {
      is_busy: false,
      run: jest.fn(() => {
        sessionAny.lastSnapshot = {
          viewport: ["nurse"],
          dialogue: ["We can heal your POKEMON."],
          prompt: null,
          menu: null,
        };
      }),
      _script_stack: [],
      _awaiting_resume: 0,
      _queued_overworld_task_count: 0,
      stop_execution: false,
      last_interaction_object_index: null as number | null,
    };
    const overworld = {
      current_map_name: "OlivinePokecenter1F",
      player_direction: "up",
      script_runner: scriptRunner,
      player_movement_locked: () => false,
      script_tasks_active: () => false,
      get_facing_tile_coords: () => [7, 5] as [number, number],
      _counter_adjusted_tile: () => [7, 3] as [number, number],
      _npc_on_tile: (x: number, y: number) =>
        x === 7 && y === 3
          ? {
              objectIndex: 1,
              x: 7,
              y: 3,
              walking: false,
              jumping: false,
              event: { script: "OlivinePokecenter1FNurseScript" },
            }
          : null,
      _nearest_npc_covering_subtile: () => null,
      _bg_event_at: () => null,
      _play_interaction_sound: jest.fn(),
    };
    const game = {
      getGameState: () => ({ wram }),
      getMapName: () => "OlivinePokecenter1F",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => overworld,
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.buildSnapshotMapInfo = jest.fn(() => ({
      map: "OlivinePokecenter1F",
      map_id: null,
      coord_stride: 2,
      player: { coords: { x: 7, y: 7 }, facing: "up" },
      warps: [],
      hotspots: [
        {
          id: "heal-olivine",
          type: "heal",
          label: "Healer",
          coords: { x: 7, y: 3 },
          visible: true,
          interactable: true,
          token: "H",
          approach_tiles: [{ coords: { x: 7, y: 7 }, facing: "up" }],
        },
      ],
    }));
    sessionAny.scheduleKeyPress = jest.fn();
    let stepCall = 0;
    sessionAny.stepFrames = jest.fn(() => {
      stepCall += 1;
      if (stepCall === 1) {
        sessionAny.lastSnapshot = { viewport: ["animated npc"], prompt: null, dialogue: null, menu: null };
      }
    });
    const blankOverworld = {
      mode: "overworld",
      menu: false,
      promptReason: null,
      dialogueText: "",
      viewportText: "",
      menuText: "",
      promptText: "",
      markerText: "",
    };
    sessionAny.captureSceneSignal = jest.fn(() =>
      sessionAny.lastSnapshot?.dialogue?.length
        ? { ...blankOverworld, dialogueText: sessionAny.lastSnapshot.dialogue.join("\n") }
        : blankOverworld
    );
    sessionAny.buildStateFingerprint = jest.fn(() =>
      sessionAny.lastSnapshot?.dialogue?.length
        ? "dialogue"
        : stepCall > 0
          ? "viewport-animation"
          : "before"
    );

    const action = await session.press("a", 1);

    expect(scriptRunner.run).toHaveBeenCalledWith("OlivinePokecenter1FNurseScript");
    expect(wram.last_talked).toBe(1);
    expect(scriptRunner.last_interaction_object_index).toBe(1);
    expect(action.result.events).toContain("confirmed_heal_interaction_retried");
    expect(action.result.changed).toBe(true);
  });

  it("retries blueprint-backed Pokecenter healer hotspots when the live NPC lookup misses", async () => {
    const session = getMcpSession("press-healer-blueprint-hotspot-retry");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { viewport?: string[]; prompt?: string[] | null; dialogue?: string[] | null; menu?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      requestAutosave: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number; last_talked: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          current_map_name: string;
          player_direction: string;
          script_runner: {
            is_busy?: boolean;
            run: jest.Mock;
            _script_stack?: unknown[];
            _awaiting_resume?: number;
            _queued_overworld_task_count?: number;
            stop_execution?: boolean;
            last_interaction_object_index?: number | null;
          };
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
          get_facing_tile_coords: () => [number, number];
          _counter_adjusted_tile: (x: number, y: number) => [number, number];
          _npc_on_tile: () => null;
          _nearest_npc_covering_subtile: () => null;
          _bg_event_at: () => null;
          _npc_blueprints: Map<string, Map<string, [unknown, number]>>;
          _play_interaction_sound: jest.Mock;
        };
      };
      getGame: jest.Mock;
      buildSnapshotMapInfo: jest.Mock;
      captureSceneSignal: jest.Mock;
      buildStateFingerprint: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OVERWORLD");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.requestAutosave = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { viewport: ["old"], prompt: null, dialogue: null, menu: null };
    const wram = { player_x: 7, player_y: 7, last_talked: 0 };
    const scriptRunner = {
      is_busy: false,
      run: jest.fn(() => {
        sessionAny.lastSnapshot = {
          viewport: ["nurse"],
          dialogue: ["We can heal your POKEMON."],
          prompt: null,
          menu: null,
        };
      }),
      _script_stack: [],
      _awaiting_resume: 0,
      _queued_overworld_task_count: 0,
      stop_execution: false,
      last_interaction_object_index: null as number | null,
    };
    const overworld = {
      current_map_name: "EcruteakPokecenter1F",
      player_direction: "up",
      script_runner: scriptRunner,
      player_movement_locked: () => false,
      script_tasks_active: () => false,
      get_facing_tile_coords: () => [7, 5] as [number, number],
      _counter_adjusted_tile: () => [7, 3] as [number, number],
      _npc_on_tile: () => null,
      _nearest_npc_covering_subtile: () => null,
      _bg_event_at: () => null,
      _npc_blueprints: new Map([
        [
          "EcruteakPokecenter1F",
          new Map([
            [
              "ECRUTEAKPOKECENTER1F_NURSE",
              [{ x: 7, y: 3, script: "EcruteakPokecenter1FNurseScript" }, 1],
            ],
          ]),
        ],
      ]),
      _play_interaction_sound: jest.fn(),
    };
    const game = {
      getGameState: () => ({ wram }),
      getMapName: () => "EcruteakPokecenter1F",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => overworld,
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.buildSnapshotMapInfo = jest.fn(() => ({
      map: "EcruteakPokecenter1F",
      map_id: null,
      coord_stride: 2,
      player: { coords: { x: 7, y: 7 }, facing: "up" },
      warps: [],
      hotspots: [
        {
          id: "heal-ecruteak",
          type: "heal",
          label: "Healer",
          coords: { x: 7, y: 3 },
          visible: true,
          interactable: true,
          token: "H",
          approach_tiles: [{ coords: { x: 7, y: 7 }, facing: "up" }],
        },
      ],
    }));
    sessionAny.scheduleKeyPress = jest.fn();
    let stepCall = 0;
    sessionAny.stepFrames = jest.fn(() => {
      stepCall += 1;
    });
    const blankOverworld = {
      mode: "overworld",
      menu: false,
      promptReason: null,
      dialogueText: "",
      viewportText: "",
      menuText: "",
      promptText: "",
      markerText: "",
    };
    sessionAny.captureSceneSignal = jest.fn(() =>
      sessionAny.lastSnapshot?.dialogue?.length
        ? { ...blankOverworld, dialogueText: sessionAny.lastSnapshot.dialogue.join("\n") }
        : blankOverworld
    );
    sessionAny.buildStateFingerprint = jest.fn(() =>
      sessionAny.lastSnapshot?.dialogue?.length ? "dialogue" : stepCall > 0 ? "after-press" : "before"
    );

    const action = await session.press("a", 1);

    expect(scriptRunner.run).toHaveBeenCalledWith("EcruteakPokecenter1FNurseScript");
    expect(wram.last_talked).toBe(1);
    expect(scriptRunner.last_interaction_object_index).toBe(1);
    expect(action.result.events).toContain("confirmed_heal_interaction_retried");
    expect(action.result.changed).toBe(true);
  });

  it("waits for async Pokecenter healer retry dialogue before reporting the A press result", async () => {
    const session = getMcpSession("press-healer-async-dialogue-retry");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { viewport?: string[]; prompt?: string[] | null; dialogue?: string[] | null; menu?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      requestAutosave: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number; last_talked: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          current_map_name: string;
          player_direction: string;
          script_runner: {
            is_busy?: boolean;
            run: jest.Mock;
            _script_stack?: unknown[];
            _awaiting_resume?: number;
            _queued_overworld_task_count?: number;
            stop_execution?: boolean;
            last_interaction_object_index?: number | null;
          };
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
          get_facing_tile_coords: () => [number, number];
          _counter_adjusted_tile: (x: number, y: number) => [number, number];
          _npc_on_tile: () => null;
          _nearest_npc_covering_subtile: () => null;
          _bg_event_at: () => null;
          _npc_blueprints: Map<string, Map<string, [unknown, number]>>;
          _play_interaction_sound: jest.Mock;
        };
      };
      getGame: jest.Mock;
      buildSnapshotMapInfo: jest.Mock;
      captureSceneSignal: jest.Mock;
      buildStateFingerprint: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OVERWORLD");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.requestAutosave = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { viewport: ["old"], prompt: null, dialogue: null, menu: null };
    const wram = { player_x: 7, player_y: 7, last_talked: 0 };
    const scriptRunner = {
      is_busy: false,
      run: jest.fn(() => {
        void Promise.resolve().then(() => {
          sessionAny.lastSnapshot = {
            viewport: ["nurse"],
            dialogue: ["We can heal your POKEMON."],
            prompt: null,
            menu: null,
          };
        });
      }),
      _script_stack: [],
      _awaiting_resume: 0,
      _queued_overworld_task_count: 0,
      stop_execution: false,
      last_interaction_object_index: null as number | null,
    };
    const overworld = {
      current_map_name: "EcruteakPokecenter1F",
      player_direction: "up",
      script_runner: scriptRunner,
      player_movement_locked: () => false,
      script_tasks_active: () => false,
      get_facing_tile_coords: () => [7, 5] as [number, number],
      _counter_adjusted_tile: () => [7, 3] as [number, number],
      _npc_on_tile: () => null,
      _nearest_npc_covering_subtile: () => null,
      _bg_event_at: () => null,
      _npc_blueprints: new Map([
        [
          "EcruteakPokecenter1F",
          new Map([
            [
              "ECRUTEAKPOKECENTER1F_NURSE",
              [{ x: 7, y: 3, script: "EcruteakPokecenter1FNurseScript" }, 1],
            ],
          ]),
        ],
      ]),
      _play_interaction_sound: jest.fn(),
    };
    const game = {
      getGameState: () => ({ wram }),
      getMapName: () => "EcruteakPokecenter1F",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => overworld,
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.buildSnapshotMapInfo = jest.fn(() => ({
      map: "EcruteakPokecenter1F",
      map_id: null,
      coord_stride: 2,
      player: { coords: { x: 7, y: 7 }, facing: "up" },
      warps: [],
      hotspots: [
        {
          id: "heal-ecruteak",
          type: "heal",
          label: "Healer",
          coords: { x: 7, y: 3 },
          visible: true,
          interactable: true,
          token: "H",
          approach_tiles: [{ coords: { x: 7, y: 7 }, facing: "up" }],
        },
      ],
    }));
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    const blankOverworld = {
      mode: "overworld",
      menu: false,
      promptReason: null,
      dialogueText: "",
      viewportText: "",
      menuText: "",
      promptText: "",
      markerText: "",
    };
    sessionAny.captureSceneSignal = jest.fn(() =>
      sessionAny.lastSnapshot?.dialogue?.length
        ? { ...blankOverworld, dialogueText: sessionAny.lastSnapshot.dialogue.join("\n") }
        : blankOverworld
    );
    sessionAny.buildStateFingerprint = jest.fn(() =>
      sessionAny.lastSnapshot?.dialogue?.length ? "dialogue" : "before"
    );

    const action = await session.press("a", 1);

    expect(scriptRunner.run).toHaveBeenCalledWith("EcruteakPokecenter1FNurseScript");
    expect(wram.last_talked).toBe(1);
    expect(scriptRunner.last_interaction_object_index).toBe(1);
    expect(action.result.events).toContain("confirmed_heal_interaction_retried");
    expect(action.result.changed).toBe(true);
  });

  it("retries confirmed PC background-event scripts when a stale hardware press only changes the viewport", async () => {
    const session = getMcpSession("press-pc-bg-event-viewport-only-retry");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { viewport?: string[]; prompt?: string[] | null; dialogue?: string[] | null; menu?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      requestAutosave: jest.Mock;
      game: unknown;
      getGame: jest.Mock;
      buildSnapshotMapInfo: jest.Mock;
      captureSceneSignal: jest.Mock;
      buildStateFingerprint: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OVERWORLD");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.requestAutosave = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { viewport: ["old"], prompt: null, dialogue: null, menu: null };

    const pcEvent = { event_type: "BGEVENT_DOWN", script: "PCScript", x: 7, y: 11 };
    const scriptRunner = {
      is_busy: false,
      run: jest.fn(),
      _script_stack: [],
      _awaiting_resume: 0,
      _queued_overworld_task_count: 0,
      stop_execution: false,
      last_interaction_object_index: null as number | null,
    };
    const overworld = {
      current_map_name: "CherrygrovePokecenter1F",
      player_direction: "down",
      script_runner: scriptRunner,
      player_movement_locked: () => false,
      script_tasks_active: () => false,
      get_facing_tile_coords: () => [7, 11] as [number, number],
      _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
      _npc_on_tile: () => null,
      _nearest_npc_covering_subtile: () => null,
      _bg_event_at: (x: number, y: number) => (x === 7 && y === 11 ? pcEvent : null),
      _handle_bg_event: jest.fn(() => {
        sessionAny.lastSnapshot = {
          viewport: ["PC"],
          menu: ["▶ BILL's PC", "  CHRIS's PC", "  TURN OFF"],
          prompt: null,
          dialogue: null,
        };
        return true;
      }),
    };
    const game = {
      getGameState: () => ({ wram: { player_x: 7, player_y: 9, last_talked: 0 } }),
      getMapName: () => "CherrygrovePokecenter1F",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => overworld,
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.buildSnapshotMapInfo = jest.fn(() => ({
      map: "CherrygrovePokecenter1F",
      map_id: null,
      coord_stride: 2,
      player: { coords: { x: 7, y: 9 }, facing: "down" },
      warps: [],
      hotspots: [
        {
          id: "bg-pc",
          type: "utility",
          label: "PC",
          coords: { x: 7, y: 11 },
          visible: true,
          interactable: true,
          token: "P",
          approach_tiles: [{ coords: { x: 7, y: 9 }, facing: "down" }],
        },
      ],
    }));
    sessionAny.scheduleKeyPress = jest.fn();
    let stepCall = 0;
    sessionAny.stepFrames = jest.fn(() => {
      stepCall += 1;
      if (!sessionAny.lastSnapshot?.menu?.length) {
        sessionAny.lastSnapshot = { viewport: ["viewport changed"], prompt: null, dialogue: null, menu: null };
      }
    });
    const blankOverworld = {
      mode: "overworld",
      menu: false,
      promptReason: null,
      dialogueText: "",
      viewportText: "",
      menuText: "",
      promptText: "",
      markerText: "",
    };
    sessionAny.captureSceneSignal = jest.fn(() =>
      sessionAny.lastSnapshot?.menu?.length
        ? { ...blankOverworld, menu: true, menuText: sessionAny.lastSnapshot.menu.join("\n") }
        : blankOverworld
    );
    sessionAny.buildStateFingerprint = jest.fn(() =>
      sessionAny.lastSnapshot?.menu?.length ? "pc-menu" : stepCall > 0 ? "viewport-animation" : "before"
    );

    const action = await session.press("a", 1);

    expect(overworld._handle_bg_event).toHaveBeenCalledWith(pcEvent);
    expect(action.result.events).toContain("confirmed_scripted_interaction_retried");
    expect(action.result.changed).toBe(true);
  });

  it("retries confirmed Gym Leader NPC scripts when a stale hardware press only changes the viewport", async () => {
    const session = getMcpSession("press-gym-leader-npc-viewport-only-retry");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { viewport?: string[]; prompt?: string[] | null; dialogue?: string[] | null; menu?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      requestAutosave: jest.Mock;
      game: unknown;
      getGame: jest.Mock;
      buildSnapshotMapInfo: jest.Mock;
      captureSceneSignal: jest.Mock;
      buildStateFingerprint: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OVERWORLD");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.requestAutosave = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { viewport: ["old"], prompt: null, dialogue: null, menu: null };

    const wram = { player_x: 9, player_y: 7, last_talked: 0 };
    const leader = {
      objectIndex: 2,
      x: 9,
      y: 5,
      walking: false,
      jumping: false,
      event: { script: "VioletGymFalknerScript", object_type: "OBJECTTYPE_TRAINER" },
      facePlayer: jest.fn(),
    };
    const scriptRunner = {
      is_busy: false,
      run: jest.fn(() => {
        sessionAny.lastSnapshot = {
          viewport: ["GYM"],
          dialogue: ["I'm FALKNER, the VIOLET POKEMON GYM leader!"],
          prompt: null,
          menu: null,
        };
      }),
      _script_stack: [],
      _awaiting_resume: 0,
      _queued_overworld_task_count: 0,
      stop_execution: false,
      last_interaction_object_index: null as number | null,
    };
    const overworld = {
      current_map_name: "VioletGym",
      player_x: 9,
      player_y: 7,
      player_direction: "up",
      player_object: { x: 9, y: 7 },
      script_runner: scriptRunner,
      player_movement_locked: () => false,
      script_tasks_active: () => false,
      get_facing_tile_coords: () => [9, 5] as [number, number],
      _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
      _npc_on_tile: (x: number, y: number) => (x === 9 && y === 5 ? leader : null),
      _nearest_npc_covering_subtile: () => null,
      _bg_event_at: () => null,
      _play_interaction_sound: jest.fn(),
    };
    const game = {
      getGameState: () => ({ wram }),
      getMapName: () => "VioletGym",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => overworld,
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.buildSnapshotMapInfo = jest.fn(() => ({
      map: "VioletGym",
      map_id: null,
      coord_stride: 2,
      player: { coords: { x: 9, y: 7 }, facing: "up" },
      warps: [],
      hotspots: [
        {
          id: "npc-2",
          type: "gym",
          label: "Gym Leader",
          coords: { x: 9, y: 5 },
          visible: true,
          interactable: true,
          token: "G",
          approach_tiles: [{ coords: { x: 9, y: 7 }, facing: "up" }],
        },
      ],
    }));
    sessionAny.scheduleKeyPress = jest.fn();
    let stepCall = 0;
    sessionAny.stepFrames = jest.fn(() => {
      stepCall += 1;
      if (!sessionAny.lastSnapshot?.dialogue?.length) {
        sessionAny.lastSnapshot = { viewport: ["viewport changed"], prompt: null, dialogue: null, menu: null };
      }
    });
    const blankOverworld = {
      mode: "overworld",
      menu: false,
      promptReason: null,
      dialogueText: "",
      viewportText: "",
      menuText: "",
      promptText: "",
      markerText: "",
    };
    sessionAny.captureSceneSignal = jest.fn(() =>
      sessionAny.lastSnapshot?.dialogue?.length
        ? { ...blankOverworld, dialogueText: sessionAny.lastSnapshot.dialogue.join("\n") }
        : blankOverworld
    );
    sessionAny.buildStateFingerprint = jest.fn(() =>
      sessionAny.lastSnapshot?.dialogue?.length ? "gym-dialogue" : stepCall > 0 ? "viewport-animation" : "before"
    );

    const action = await session.press("a", 1);

    expect(wram.last_talked).toBe(2);
    expect(scriptRunner.last_interaction_object_index).toBe(2);
    expect(leader.facePlayer).toHaveBeenCalledWith(9, 7);
    expect(overworld._play_interaction_sound).toHaveBeenCalledTimes(1);
    expect(scriptRunner.run).toHaveBeenCalledWith("VioletGymFalknerScript", { allow_fallthrough: false });
    expect(action.result.events).toContain("confirmed_scripted_interaction_retried");
    expect(action.result.changed).toBe(true);
  });

  it("settles from closed overworld dialogue into a script-owned prompt snapshot", async () => {
    const session = getMcpSession("press-overworld-dialogue-to-prompt-settle");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      requestAutosave: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.requestAutosave = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = {
      prompt: null,
      dialogue: ["You mustn't forget", "that!"],
      menu: null,
    };
    sessionAny.observeText = jest.fn(() => JSON.stringify(sessionAny.lastSnapshot));

    let movementLocked = false;
    const overworld = {
      script_runner: { is_busy: false },
      player_movement_locked: () => movementLocked,
      script_tasks_active: () => false,
    };
    const game = {
      getGameState: () => ({ wram: { player_x: 19, player_y: 9 } }),
      getMapName: () => "PlayersHouse1F",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => overworld,
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    let settleFrames = 0;
    sessionAny.stepFrames = jest.fn((count: number) => {
      settleFrames += count;
      if (settleFrames === count) {
        movementLocked = true;
        overworld.script_runner = { is_busy: true };
        sessionAny.lastSnapshot = { prompt: null, dialogue: null, menu: null };
        return;
      }
      movementLocked = false;
      overworld.script_runner = { is_busy: false };
      sessionAny.lastSnapshot = {
        prompt: ["What day is it?", ">> SUNDAY", " MONDAY"],
        dialogue: null,
        menu: null,
      };
    });

    const action = await session.press("a", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(action.snapshotText).toContain("What day is it?");
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(1, 3);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(2, 1);
  });

  it("holds blank-overworld A for an extra frame on a confirmed live interaction target", async () => {
    const session = getMcpSession("press-overworld-confirmed-object-extra-hold");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      requestAutosave: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          current_map_name: string;
          player_direction: string;
          script_runner: null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
          get_facing_tile_coords: () => [number, number];
          _counter_adjusted_tile: (x: number, y: number) => [number, number];
          _npc_on_tile: () => null;
          _nearest_npc_covering_subtile: () => null;
          _bg_event_at: () => { event_type: string; script: string };
          _bg_event_allowed_by_flags: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.requestAutosave = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { prompt: null, dialogue: null, menu: null };
    const game = {
      getGameState: () => ({ wram: { player_x: 13, player_y: 9 } }),
      getMapName: () => "ElmsLab",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_direction: "up",
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        get_facing_tile_coords: () => [13, 8] as [number, number],
        _counter_adjusted_tile: () => [13, 7] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => ({ event_type: "BGEVENT_READ", script: "CyndaquilPokeBallScript" }),
        _bg_event_allowed_by_flags: () => true,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();

    await session.press("a", 1);

    expect(sessionAny.scheduleKeyPress).toHaveBeenCalledWith({
      key: "KeyZ",
      button: "a",
      direction: undefined,
      holdFrames: 2,
      repeatPressFrames: false,
    });
  });

  it("treats scripted overlay and text-lock changes as real overworld press progress before dialogue text lands", async () => {
    const session = getMcpSession("press-overworld-overlay-progress");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      requestAutosave: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number; last_talked: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          _text_lock_active: boolean;
          pokepic_overlay: { isVisible: boolean };
          script_runner: {
            is_busy?: boolean;
            _script_stack?: unknown[];
            _awaiting_resume?: number;
            _queued_overworld_task_count?: number;
            stop_execution?: boolean;
            last_interaction_object_index?: number | null;
          } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.requestAutosave = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { prompt: null, dialogue: null, menu: null };
    const overworld = {
      _text_lock_active: false,
      pokepic_overlay: { isVisible: false },
      script_runner: {
        is_busy: false,
        _script_stack: [],
        _awaiting_resume: 0,
        _queued_overworld_task_count: 0,
        stop_execution: false,
        last_interaction_object_index: null,
      },
      player_movement_locked: () => false,
      script_tasks_active: () => false,
    };
    const wram = { player_x: 13, player_y: 9, last_talked: 0 };
    const game = {
      getGameState: () => ({ wram }),
      getMapName: () => "ElmsLab",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => overworld,
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn(() => {
      overworld._text_lock_active = true;
      overworld.pokepic_overlay.isVisible = true;
    });

    const action = await session.press("a", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.requestAutosave).toHaveBeenCalledWith({ force: true });
  });

  it("treats blocking script-task handoff changes as real overworld press progress before dialogue text lands", async () => {
    const session = getMcpSession("press-overworld-blocking-task-progress");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      requestAutosave: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number; last_talked: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          _text_lock_active: boolean;
          _blocking_task_count: number;
          _blocking_movement_lock_active: boolean;
          _active_script_task: { kind: string } | null;
          _script_task_queue: Array<{ kind: string }>;
          pokepic_overlay: { isVisible: boolean };
          script_runner: {
            is_busy?: boolean;
            _script_stack?: unknown[];
            _awaiting_resume?: number;
            _queued_overworld_task_count?: number;
            stop_execution?: boolean;
            last_interaction_object_index?: number | null;
          } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.requestAutosave = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { prompt: null, dialogue: null, menu: null };
    const overworld = {
      _text_lock_active: false,
      _blocking_task_count: 0,
      _blocking_movement_lock_active: false,
      _active_script_task: null,
      _script_task_queue: [] as Array<{ kind: string }>,
      pokepic_overlay: { isVisible: false },
      script_runner: {
        is_busy: false,
        _script_stack: [],
        _awaiting_resume: 0,
        _queued_overworld_task_count: 0,
        stop_execution: false,
        last_interaction_object_index: null,
      },
      player_movement_locked: () => false,
      script_tasks_active: () => false,
    };
    const wram = { player_x: 13, player_y: 9, last_talked: 0 };
    const game = {
      getGameState: () => ({ wram }),
      getMapName: () => "ElmsLab",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => overworld,
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn(() => {
      overworld._blocking_task_count = 1;
      overworld._blocking_movement_lock_active = true;
      overworld._active_script_task = { kind: "dialogue" };
      overworld._script_task_queue = [{ kind: "wait_for_input" }];
    });

    const action = await session.press("a", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.requestAutosave).toHaveBeenCalledWith({ force: true });
  });

  it("keeps settling a real overworld interaction lane long enough for late script state to land", async () => {
    const session = getMcpSession("press-overworld-late-interaction-target");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null; marker?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      requestAutosave: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number; last_talked: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          get_facing_tile_coords: () => [number, number];
          _counter_adjusted_tile: (x: number, y: number) => [number, number];
          _bg_event_at: (x: number, y: number) => { event_type: string; script: string } | null;
          _bg_event_allowed_by_flags: (eventType: string, scriptName: string) => boolean;
          _text_lock_active: boolean;
          pokepic_overlay: { isVisible: boolean };
          script_runner: {
            is_busy?: boolean;
            _script_stack?: unknown[];
            _awaiting_resume?: number;
            _queued_overworld_task_count?: number;
            stop_execution?: boolean;
            last_interaction_object_index?: number | null;
          } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.requestAutosave = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { prompt: null, dialogue: null, menu: null, marker: null };
    let settledFrames = 0;
    const overworld = {
      get_facing_tile_coords: () => [13, 7] as [number, number],
      _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
      _bg_event_at: (x: number, y: number) =>
        x === 13 && y === 7 ? { event_type: "ITEM", script: "CyndaquilPokeBallScript" } : null,
      _bg_event_allowed_by_flags: () => true,
      _text_lock_active: false,
      pokepic_overlay: { isVisible: false },
      script_runner: {
        is_busy: false,
        _script_stack: [],
        _awaiting_resume: 0,
        _queued_overworld_task_count: 0,
        stop_execution: false,
        last_interaction_object_index: null,
      },
      player_movement_locked: () => false,
      script_tasks_active: () => false,
    };
    const wram = { player_x: 11, player_y: 7, last_talked: 0 };
    const game = {
      getGameState: () => ({ wram }),
      getMapName: () => "ElmsLab",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => overworld,
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn((count: number) => {
      settledFrames += count;
      if (settledFrames >= 31) {
        overworld._text_lock_active = true;
        overworld.pokepic_overlay.isVisible = true;
        overworld.script_runner = {
          ...overworld.script_runner,
          _script_stack: [{ name: "CyndaquilPokeBallScript" }],
          _awaiting_resume: 1,
          stop_execution: true,
        };
      }
    });

    const action = await session.press("a", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(1, 3);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(2, 4);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(3, 8);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(4, 16);
    expect(sessionAny.requestAutosave).toHaveBeenCalledWith({ force: true });
  });

  it("gives battle-menu directional presses additional bounded settle passes when the main cursor lands very late", async () => {
    const session = getMcpSession("press-battle-menu-direction-very-late");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      prompt: null,
      dialogue: null,
      menu: ["▶ FIGHT", "  <PKMN>", "  PACK", "  RUN"],
    };
    let steppedFrames = 0;
    const game = {
      getGameState: () => ({ wram: { player_x: 37, player_y: 31 } }),
      getMapName: () => "Route30",
      isMenuOpen: () => true,
      isBattleActive: () => true,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn((count: number) => {
      steppedFrames += count;
      if (steppedFrames >= 17) {
        sessionAny.lastSnapshot = {
          prompt: null,
          dialogue: null,
          menu: ["FIGHT", "<PKMN>", "▶ PACK", "RUN"],
        };
      }
    });

    const action = await session.press("down", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(1, 4);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(2, 4);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(3, 4);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(4, 8);
  });

  it("gives battle-menu A additional bounded settle passes when the move result lands very late", async () => {
    const session = getMcpSession("press-battle-menu-confirm-very-late");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      prompt: null,
      dialogue: null,
      menu: ["TACKLE (PP 28/35)", "LEER (PP 30/30)", "▶ SMOKESCREEN (PP 19/20)", "EMBER (PP 25/25)", "CANCEL"],
    };
    let steppedFrames = 0;
    const game = {
      getGameState: () => ({ wram: { player_x: 105, player_y: 19 } }),
      getMapName: () => "Route29",
      isMenuOpen: () => true,
      isBattleActive: () => true,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn((count: number) => {
      steppedFrames += count;
      if (steppedFrames >= 65) {
        sessionAny.lastSnapshot = {
          prompt: ["A=Advance B=Close"],
          dialogue: ["CYNDAQUIL used", "SMOKESCREEN!"],
          menu: null,
        };
      }
    });

    const action = await session.press("a", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(1, 5);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(2, 4);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(3, 8);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(4, 16);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(5, 32);
  });

  it("gives battle dialogue-close A presses one extra bounded settle pass when the next prompt lands late", async () => {
    const session = getMcpSession("press-battle-dialogue-close-late");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      prompt: ["A=Advance B=Close"],
      dialogue: ["SENTRET used", "TACKLE!"],
      menu: null,
    };
    let steppedFrames = 0;
    const game = {
      getGameState: () => ({ wram: { player_x: 105, player_y: 25 } }),
      getMapName: () => "Route29",
      isMenuOpen: () => false,
      isBattleActive: () => true,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn((count: number) => {
      steppedFrames += count;
      if (steppedFrames >= 3) {
        sessionAny.lastSnapshot = {
          prompt: null,
          dialogue: null,
          menu: null,
        };
      }
      if (steppedFrames >= 7) {
        sessionAny.lastSnapshot = {
          prompt: null,
          dialogue: null,
          menu: ["▶ FIGHT", "  <PKMN>", "  PACK", "  RUN"],
        };
      }
    });

    const action = await session.press("a", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(1, 3);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(2, 4);
  });

  it("gives battle dialogue-close A presses extra bounded settle passes when catch text lands late from a blank handoff", async () => {
    const session = getMcpSession("press-battle-dialogue-close-catch-late");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      prompt: ["A=Advance B=Close"],
      dialogue: ["DUDE used the POKe", "BALL."],
      menu: null,
    };
    let steppedFrames = 0;
    const game = {
      getGameState: () => ({ wram: { player_x: 13, player_y: 57 } }),
      getMapName: () => "Route30",
      isMenuOpen: () => false,
      isBattleActive: () => true,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn((count: number) => {
      steppedFrames += count;
      if (steppedFrames >= 29) {
        sessionAny.lastSnapshot = {
          prompt: ["A=Advance B=Close"],
          dialogue: ["Gotcha! PIDGEY", "was caught!"],
          menu: null,
        };
        return;
      }
      if (steppedFrames >= 3) {
        sessionAny.lastSnapshot = {
          prompt: null,
          dialogue: null,
          menu: null,
        };
      }
    });

    const action = await session.press("a", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(1, 3);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(2, 4);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(3, 8);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(4, 16);
  });

  it("waits through battle intro transition before applying the first interactive battle press", async () => {
    const session = getMcpSession("press-battle-intro-transition-settle");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: {
        viewport?: string[] | null;
        menu?: string[] | null;
        prompt?: string[] | null;
        dialogue?: string[] | null;
      } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      viewport: ["BATTLE TRANSITION", "The battle is starting...", "Wait: battle intro animation"],
      prompt: null,
      dialogue: null,
      menu: null,
    };
    let steppedFrames = 0;
    const game = {
      getGameState: () => ({ wram: { player_x: 13, player_y: 57 } }),
      getMapName: () => "Route30",
      isMenuOpen: () => Boolean(sessionAny.lastSnapshot?.menu?.length),
      isBattleActive: () => true,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn((count: number) => {
      steppedFrames += count;
      if (steppedFrames >= 24 && !sessionAny.lastSnapshot?.menu) {
        sessionAny.lastSnapshot = {
          viewport: ["BATTLE"],
          prompt: null,
          dialogue: null,
          menu: ["▶ FIGHT", "  <PKMN>", "  PACK", "  RUN"],
        };
        return;
      }
      if (steppedFrames >= 28 && sessionAny.lastSnapshot?.menu) {
        sessionAny.lastSnapshot = {
          viewport: ["BATTLE"],
          prompt: null,
          dialogue: null,
          menu: ["  FIGHT", "  <PKMN>", "▶ PACK", "  RUN"],
        };
      }
    });

    const action = await session.press("down", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(1, 8);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(2, 16);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(3, 4);
  });

  it("stops at the first main battle menu after intro settle instead of carrying A through to FIGHT", async () => {
    const session = getMcpSession("press-battle-intro-first-a-stops-at-main-menu");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: {
        viewport?: string[] | null;
        menu?: string[] | null;
        prompt?: string[] | null;
        dialogue?: string[] | null;
      } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      viewport: ["BATTLE TRANSITION", "The battle is starting...", "Wait: battle intro animation"],
      prompt: null,
      dialogue: null,
      menu: null,
    };
    let steppedFrames = 0;
    const game = {
      getGameState: () => ({ wram: { player_x: 15, player_y: 57 } }),
      getMapName: () => "Route30",
      isMenuOpen: () => Boolean(sessionAny.lastSnapshot?.menu?.length),
      isBattleActive: () => true,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn((count: number) => {
      steppedFrames += count;
      if (steppedFrames >= 24) {
        sessionAny.lastSnapshot = {
          viewport: ["BATTLE"],
          prompt: null,
          dialogue: null,
          menu: ["▶ FIGHT", "  <PKMN>", "  PACK", "  RUN"],
        };
      }
    });

    const action = await session.press("a", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.ok).toBe(true);
    expect(sessionAny.scheduleKeyPress).not.toHaveBeenCalled();
    expect(sessionAny.stepFrames).toHaveBeenCalledTimes(2);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(1, 8);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(2, 16);
  });

  it("sends interactive battle d-pad presses as directional inputs so menu cursors can move", async () => {
    const session = getMcpSession("press-battle-menu-direction");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      prompt: null,
      dialogue: null,
      menu: ["▶ FIGHT", "  <PKMN>", "  PACK", "  RUN"],
    };
    const game = {
      getGameState: () => ({ wram: { player_x: 93, player_y: 29 } }),
      getMapName: () => "Route29",
      isMenuOpen: () => true,
      isBattleActive: () => true,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    let scheduledDirection: string | null = null;
    sessionAny.scheduleKeyPress = jest.fn((options: { direction?: string | null }) => {
      scheduledDirection = options.direction ?? null;
    });
    sessionAny.stepFrames = jest.fn(() => {
      if (scheduledDirection === "down") {
        sessionAny.lastSnapshot = {
          prompt: null,
          dialogue: null,
          menu: ["  FIGHT", "  <PKMN>", "▶ PACK", "  RUN"],
        };
      }
    });

    const action = await session.press("down", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.scheduleKeyPress).toHaveBeenCalledWith(
      expect.objectContaining({
        key: expect.anything(),
        direction: "down",
        button: undefined,
      })
    );
  });

  it("gives battle-menu d-pad presses enough settle frames to move the main cursor", async () => {
    const session = getMcpSession("press-battle-menu-direction-settle");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      prompt: null,
      dialogue: null,
      menu: ["▶ FIGHT", "  <PKMN>", "  PACK", "  RUN"],
    };
    const game = {
      getGameState: () => ({ wram: { player_x: 95, player_y: 29 } }),
      getMapName: () => "Route29",
      isMenuOpen: () => true,
      isBattleActive: () => true,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn((count: number) => {
      if (count >= 4) {
        sessionAny.lastSnapshot = {
          prompt: null,
          dialogue: null,
          menu: ["  FIGHT", "  <PKMN>", "▶ PACK", "  RUN"],
        };
      }
    });

    const action = await session.press("down", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.stepFrames).toHaveBeenCalledWith(4);
  });

  it("gives PC prompt d-pad presses enough settle frames to move from withdraw to deposit", async () => {
    const session = getMcpSession("press-pc-top-menu-direction-settle");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("PROMPT\n▶ WITHDRAW <PK><MN>\n  DEPOSIT <PK><MN>");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      prompt: null,
      dialogue: null,
      menu: ["> WITHDRAW <PK><MN>", "  DEPOSIT <PK><MN>", "  CHANGE BOX", "  SEE YA!"],
    };
    const game = {
      getGameState: () => ({ wram: { player_x: 19, player_y: 5 } }),
      getMapName: () => "CHERRYGROVE_POKECENTER_1F",
      isMenuOpen: () => true,
      isBattleActive: () => false,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn((count: number) => {
      if (count >= 4) {
        sessionAny.lastSnapshot = {
          prompt: null,
          dialogue: null,
          menu: ["  WITHDRAW <PK><MN>", "> DEPOSIT <PK><MN>", "  CHANGE BOX", "  SEE YA!"],
        };
      }
    });

    const action = await session.press("down", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.stepFrames).toHaveBeenCalledWith(4);
  });

  it("does not repeat A across non-blocking PC prompt hold frames", async () => {
    const session = getMcpSession("press-pc-top-menu-a-no-repeat");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("PROMPT\n▶ WITHDRAW <PK><MN>\n  DEPOSIT <PK><MN>");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      prompt: null,
      dialogue: null,
      menu: ["> WITHDRAW <PK><MN>", "  DEPOSIT <PK><MN>", "  CHANGE BOX", "  SEE YA!"],
    };
    const game = {
      getGameState: () => ({ wram: { player_x: 19, player_y: 5 } }),
      getMapName: () => "CHERRYGROVE_POKECENTER_1F",
      isMenuOpen: () => true,
      isBattleActive: () => false,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();

    await session.press("a", 1);

    expect(sessionAny.scheduleKeyPress).toHaveBeenCalledWith(
      expect.objectContaining({
        button: "a",
        holdFrames: 4,
        repeatPressFrames: false,
      }),
    );
  });

  it("gives battle-menu d-pad presses one extra bounded settle pass when the main cursor lands late", async () => {
    const session = getMcpSession("press-battle-menu-direction-late");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      prompt: null,
      dialogue: null,
      menu: ["▶ FIGHT", "  <PKMN>", "  PACK", "  RUN"],
    };
    let steppedFrames = 0;
    const game = {
      getGameState: () => ({ wram: { player_x: 79, player_y: 29 } }),
      getMapName: () => "Route29",
      isMenuOpen: () => true,
      isBattleActive: () => true,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn((count: number) => {
      steppedFrames += count;
      if (steppedFrames >= 8) {
        sessionAny.lastSnapshot = {
          prompt: null,
          dialogue: null,
          menu: ["  FIGHT", "  <PKMN>", "▶ PACK", "  RUN"],
        };
      }
    });

    const action = await session.press("down", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(1, 4);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(2, 4);
  });

  it("gives battle-menu d-pad presses one final bounded settle pass when the main cursor lands very late", async () => {
    const session = getMcpSession("press-battle-menu-direction-very-late");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      prompt: null,
      dialogue: null,
      menu: ["▶ FIGHT", "  <PKMN>", "  PACK", "  RUN"],
    };
    let steppedFrames = 0;
    const game = {
      getGameState: () => ({ wram: { player_x: 25, player_y: 71 } }),
      getMapName: () => "Route30",
      isMenuOpen: () => true,
      isBattleActive: () => true,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn((count: number) => {
      steppedFrames += count;
      if (steppedFrames >= 12) {
        sessionAny.lastSnapshot = {
          prompt: null,
          dialogue: null,
          menu: ["  FIGHT", "  <PKMN>", "  PACK", "▶ RUN"],
        };
      }
    });

    const action = await session.press("down", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(1, 4);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(2, 4);
    expect(sessionAny.stepFrames).toHaveBeenNthCalledWith(3, 4);
  });

  it.each(pokecenterHealPointCases)(
    "pivots toward the %s heal counter before pressing A",
    async (mapName, scriptName) => {
      const session = getMcpSession(`press-${mapName}-heal-pivot`);
      const sessionAny = session as unknown as {
        ensureReady: jest.Mock;
        observeText: jest.Mock;
        actionLimiter: { consume: jest.Mock };
        holdFrames: number;
        lastSnapshot: {
          viewport?: string[];
          info?: string[];
          prompt?: string[] | null;
          dialogue?: string[] | null;
          menu?: string[] | null;
          marker?: string[] | null;
        } | null;
        scheduleKeyPress: jest.Mock;
        stepFrames: jest.Mock;
        game: unknown;
        getGame: jest.Mock;
        buildSnapshotMapInfo: jest.Mock;
      };

      const wram = {
        instant_mode: true,
        player_x: 5,
        player_y: 7,
        wXCoord: 5,
        wYCoord: 7,
        last_talked: 0,
      };
      const nurseObjectIndex = 1;
      const overworld = {
        current_map_name: mapName,
        player_x: 5,
        player_y: 7,
        player_direction: "down" as Direction,
        player_object: {
          x: 5,
          y: 7,
          direction: "down" as Direction,
          updatePixelPosition: jest.fn(),
        },
        script_runner: {
          is_busy: false,
          run: jest.fn((startedScriptName: string) => {
            overworld.script_runner._script_stack = [{ name: startedScriptName }];
            overworld.script_runner._awaiting_resume = 1;
            overworld.script_runner.stop_execution = true;
            sessionAny.lastSnapshot = {
              prompt: null,
              dialogue: ["We can heal your POKEMON."],
              menu: null,
            };
          }),
          _script_stack: [] as Array<{ name: string }>,
          _awaiting_resume: 0,
          stop_execution: false,
          last_interaction_object_index: null as number | null,
        },
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        get_facing_tile_coords: () =>
          overworld.player_direction === "up" ? [5, 5] as [number, number] : [5, 9] as [number, number],
        _counter_adjusted_tile: (tileX: number, tileY: number) =>
          overworld.player_direction === "up" && tileX === 5 && tileY === 5
            ? [5, 3] as [number, number]
            : [tileX, tileY] as [number, number],
        _npc_on_tile: (tileX: number, tileY: number) =>
          tileX === 5 && tileY === 3
            ? {
                objectIndex: nurseObjectIndex,
                x: 5,
                y: 3,
                walking: false,
                jumping: false,
                event: { script: scriptName },
              }
            : null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => null,
        _bg_event_allowed_by_flags: () => true,
        npcs: [
          {
            objectIndex: nurseObjectIndex,
            x: 5,
            y: 3,
            event: { script: scriptName },
          },
        ],
      };
      const game = {
        getGameState: () => ({ wram }),
        getMapName: () => mapName,
        isMenuOpen: () => false,
        isBattleActive: () => false,
        getOverworld: () => overworld,
      };
      const mapInfo: McpMapInfoSnapshot = {
        map: mapName,
        map_id: null,
        coord_stride: 2,
        player: { coords: { x: 5, y: 7 }, facing: "down" },
        warps: [],
        hotspots: [
          {
            id: `heal-${mapName}`,
            type: "heal",
            label: "Pokemon Center nurse",
            coords: { x: 5, y: 3 },
            visible: true,
            interactable: true,
            token: "H",
            approach_tiles: [
              {
                coords: { x: 5, y: 7 },
                facing: "up",
              },
            ],
          },
        ],
      };

      sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
      sessionAny.observeText = jest.fn().mockReturnValue("OK");
      sessionAny.actionLimiter = { consume: jest.fn() };
      sessionAny.holdFrames = 1;
      sessionAny.lastSnapshot = {
        viewport: ["POKECENTER"],
        info: [],
        prompt: null,
        dialogue: null,
        menu: null,
        marker: null,
      };
      sessionAny.game = game;
      sessionAny.getGame = jest.fn(() => game);
      sessionAny.buildSnapshotMapInfo = jest.fn(() => mapInfo);
      sessionAny.scheduleKeyPress = jest.fn();
      sessionAny.stepFrames = jest.fn();

      const action = await session.press("a", 1);

      expect(overworld.player_direction).toBe("up");
      expect(overworld.player_object.direction).toBe("up");
      expect(overworld.player_object.updatePixelPosition).toHaveBeenCalledTimes(1);
      expect(wram.last_talked).toBe(nurseObjectIndex);
      expect(overworld.script_runner.last_interaction_object_index).toBe(nurseObjectIndex);
      expect(overworld.script_runner.run).toHaveBeenCalledWith(scriptName);
      expect(overworld.script_runner._script_stack).toEqual([{ name: scriptName }]);
      expect(action.result.ok).toBe(true);
      expect(action.result.changed).toBe(true);
      expect(action.result.events).toContain("confirmed_heal_interaction_retried");
      expect(sessionAny.scheduleKeyPress).toHaveBeenCalledWith({
        key: "KeyZ",
        button: "a",
        direction: undefined,
        holdFrames: 2,
        repeatPressFrames: false,
      });
    }
  );
});
