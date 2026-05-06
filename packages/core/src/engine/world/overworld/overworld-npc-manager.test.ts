import { createInitialGameState, type GameState } from "@pokecrystal/core/core/state";
import fs from "fs";
import os from "os";
import path from "path";
import type { ObjectEvent } from "@pokecrystal/core/core/models/map";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { ScriptRunnerImpl } from "@pokecrystal/core/engine/world/story-events/runner";
import { Surface } from "@pokecrystal/core/ui/game-engine";
import { FieldDialogueManager } from "@pokecrystal/core/ui/text/dialogue";
import { OverworldNpcManagerMixin } from "./overworld-npc-manager";
import { OverworldObject } from "./overworld-object";
import { resolveCollisionValue } from "./collision-data";
import { MovementTask } from "./script-tasks/movement-task";

class TestOverworld extends OverworldNpcManagerMixin {
  protected _sprite_asset_exists(): boolean {
    return true;
  }

  public setGameState(state: GameState): void {
    this.game_state = state;
  }

  public shouldSpawn(npc: OverworldObject, opts: { ignore_event_flag: boolean }): boolean {
    return this._object_should_spawn(npc, opts);
  }

  public setNpcData(mapName: string, events: ObjectEvent[]): void {
    this.data_loader = { npc_data: new Map([[mapName, events]]) } as any;
    this._npc_blueprints = new Map();
  }

  public buildBlueprint(mapName: string): Map<string, [ObjectEvent, number]> {
    return this._build_blueprint(mapName);
  }
}

class TestOverworldSpriteAssets extends OverworldNpcManagerMixin {
  public setGameState(state: GameState): void {
    this.game_state = state;
  }

  public setSpriteRoot(root: string): void {
    this._sprite_root = root;
  }

  public assetExists(spriteId: string): boolean {
    return this._sprite_asset_exists(spriteId);
  }

  public shouldSpawn(npc: OverworldObject, opts: { ignore_event_flag: boolean }): boolean {
    return this._object_should_spawn(npc, opts);
  }
}

class TestOverworldInteraction extends OverworldNpcManagerMixin {
  public facingCoords: [number, number] = [0, 0];
  public script_runner: { is_busy?: boolean; run?: jest.Mock } | null = null;

  public setGameState(state: GameState): void {
    this.game_state = state;
  }

  public getGameState(): GameState | null {
    return this.game_state ?? null;
  }

  public setPlayer(x: number, y: number): void {
    this.player_x = x;
    this.player_y = y;
  }

  public setMap(map: unknown): void {
    this.map = map as any;
  }

  public setTilesPerCollision(value: number): void {
    this.TILES_PER_COLLISION = value;
  }

  public get_facing_tile_coords(): [number, number] {
    return this.facingCoords;
  }

  public _counter_adjusted_tile(x: number, y: number): [number, number] {
    return [x, y];
  }

  public _play_interaction_sound(): void {
    // Test stub.
  }

  public npcOccupyingSubtile(x: number, y: number): OverworldObject | null {
    return this._npc_occupying_subtile(x, y);
  }

  public npcOccupancyLookup(): (x: number, y: number) => OverworldObject | null {
    return this._npc_occupancy_lookup();
  }
}

class TestOverworldCollision extends OverworldNpcManagerMixin {
  public is_moving = false;
  public target_tile_x = 0;
  public target_tile_y = 0;
  public _pending_auto_step: [string, boolean] | null = null;

  public setPlayer(x: number, y: number): void {
    this.player_x = x;
    this.player_y = y;
  }

  public setMap(map: unknown, tileset: unknown): void {
    this.map = map as any;
    this.tileset = tileset as any;
  }

  public setTilesPerCollision(value: number): void {
    this.TILES_PER_COLLISION = value;
  }

  public isNpcStepBlocked(
    npc: OverworldObject,
    direction: string,
    targetTileX: number,
    targetTileY: number,
    options?: { is_player_target?: boolean; player_only?: boolean; suppress_blocked_log?: boolean },
  ): boolean {
    return this._npc_step_blocked(npc, direction, targetTileX, targetTileY, options);
  }
}

type WramWithTrainerFlags = GameState["wram"] & {
  trainer_flags: Record<string, boolean>;
};

type TestGameState = GameState & {
  wram: WramWithTrainerFlags;
};

const createTestGameState = (timeOfDay = "day"): TestGameState => {
  const base = createInitialGameState();
  return {
    ...base,
    wram: {
      ...base.wram,
      time_of_day: timeOfDay,
      event_flags: {},
      trainer_flags: {},
    },
  };
};

const buildEvent = (hram_y: number | string): ObjectEvent => {
  let normalizedHramY: number | string = hram_y;
  if (typeof hram_y === "string") {
    const trimmed = hram_y.trim();
    normalizedHramY = /^-?\d+$/.test(trimmed) ? Number(trimmed) : trimmed;
  }
  return {
    sprite: "SPRITE_MOM",
    x: 0,
    y: 0,
    spritemovedata: "SPRITEMOVEDATA_STANDING_LEFT",
    move_range_x: 0,
    move_range_y: 0,
    hram_x: 0,
    hram_y: normalizedHramY,
    pal: 0,
    object_type: "OBJECTTYPE_SCRIPT",
    radius: 0,
    script: "MomScript",
    event_flag: "",
    object_identifier: null,
    sightline_direction_override: null,
  } as unknown as ObjectEvent;
};

const buildNpcEvent = (x: number, y: number): ObjectEvent => ({
  sprite: "SPRITE_MOM",
  x,
  y,
  spritemovedata: "SPRITEMOVEDATA_STANDING_LEFT",
  move_range_x: 0,
  move_range_y: 0,
  hram_x: 0,
  hram_y: 0,
  pal: 0,
  object_type: "OBJECTTYPE_SCRIPT",
  radius: 0,
  script: "",
  event_flag: "",
  object_identifier: null,
  sightline_direction_override: null,
} as unknown as ObjectEvent);

describe("OverworldNpcManagerMixin._object_should_spawn", () => {
  it("resolves numeric object ids by objectIndex instead of live NPC array order", () => {
    const overworld = new TestOverworldInteraction();
    const first = new OverworldObject({
      ...buildNpcEvent(5, 5),
      sprite: "SPRITE_BLACK_BELT",
      object_identifier: "TEST_BLACK_BELT",
    });
    first.objectIndex = 4;
    const second = new OverworldObject({
      ...buildNpcEvent(3, 7),
      sprite: "SPRITE_BOULDER",
      object_identifier: "TEST_BOULDER",
    });
    second.objectIndex = 8;
    overworld.npcs = [second, first];

    expect(overworld.get_object_by_id(4)).toBe(first);
    expect(overworld.get_object_by_id(8)).toBe(second);
    expect(overworld.get_object_by_id("8")).toBe(second);
  });

  it("keeps explicit object constants ahead of broad sprite aliases", () => {
    const overworld = new TestOverworld();
    overworld.setNpcData("TestMap", [
      {
        ...buildNpcEvent(1, 1),
        sprite: "SPRITE_PLACEHOLDER",
        script: "ObjectEvent",
        object_identifier: "TESTMAP_RIVAL",
      } as ObjectEvent,
      {
        ...buildNpcEvent(2, 2),
        sprite: "SPRITE_RIVAL",
        script: "OtherNpcScript",
        object_identifier: "TESTMAP_OTHER",
      } as ObjectEvent,
    ]);

    const blueprint = overworld.buildBlueprint("TestMap");

    expect(blueprint.get("TESTMAP_RIVAL")?.[1]).toBe(1);
    expect(blueprint.get("TESTMAP_OTHER")?.[1]).toBe(2);
  });

  it("resolves lowercase sprite filenames without lowercasing the asset root path", () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "PokeCrystalSprites-"));
    fs.writeFileSync(path.join(root, "poke_ball.png"), "");
    try {
      const overworld = new TestOverworldSpriteAssets();
      overworld.setSpriteRoot(root);

      expect(overworld.assetExists("POKE_BALL")).toBe(true);
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  it("spawns starter poke ball objects when their canonical lowercase sprite asset exists", () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "PokeCrystalSprites-"));
    fs.writeFileSync(path.join(root, "poke_ball.png"), "");
    try {
      const overworld = new TestOverworldSpriteAssets();
      overworld.setGameState(createTestGameState());
      overworld.setSpriteRoot(root);

      const npc = new OverworldObject({
        ...buildNpcEvent(6, 3),
        sprite: "SPRITE_POKE_BALL",
        object_type: "OBJECTTYPE_SCRIPT",
        script: "CyndaquilPokeBallScript",
        object_identifier: "ELMSLAB_POKE_BALL1",
      } as ObjectEvent);

      expect(overworld.shouldSpawn(npc, { ignore_event_flag: false })).toBe(true);
      expect(npc.spriteId).toBe("POKE_BALL");
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  it("resolves Pokemon sprite constants through menu icons instead of a player fallback", () => {
    const root = fs.mkdtempSync(path.join(os.tmpdir(), "PokeCrystalSprites-"));
    fs.writeFileSync(path.join(root, "chris.png"), "");
    try {
      const overworld = new TestOverworldSpriteAssets();
      overworld.setGameState(createTestGameState());
      overworld.setSpriteRoot(root);

      const npc = new OverworldObject({
        ...buildNpcEvent(9, 4),
        sprite: "SPRITE_BUTTERFREE",
        spritemovedata: "SPRITEMOVEDATA_POKEMON",
        pal: 8,
        object_type: "OBJECTTYPE_SCRIPT",
        script: "Route34IlexForestGateButterfreeScript",
        object_identifier: "ROUTE34ILEXFORESTGATE_BUTTERFREE",
      } as ObjectEvent);

      expect(overworld.shouldSpawn(npc, { ignore_event_flag: false })).toBe(true);
      expect(npc.spriteId).toBe("ICON_MOTH");
      expect(npc.baseSprite).toBe("SPRITE_BUTTERFREE");
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  it("filters time-of-day strings against the current time mask", () => {
    const overworld = new TestOverworld();
    const state = createTestGameState();
    overworld.setGameState(state);

    const npc = new OverworldObject(buildEvent("MORN"));
    expect(overworld.shouldSpawn(npc, { ignore_event_flag: true })).toBe(false);

    state.wram.time_of_day = "morn";
    expect(overworld.shouldSpawn(npc, { ignore_event_flag: true })).toBe(true);
  });

  it("accepts numeric strings for time-of-day masks", () => {
    const overworld = new TestOverworld();
    overworld.setGameState(createTestGameState());

    const npc = new OverworldObject(buildEvent("-1"));
    expect(overworld.shouldSpawn(npc, { ignore_event_flag: true })).toBe(true);
  });

  it("treats the ASM -1 event flag sentinel as unflagged even when state contains it", () => {
    const overworld = new TestOverworld();
    const state = createTestGameState();
    state.wram.event_flags["-1"] = true;
    overworld.setGameState(state);

    const npc = new OverworldObject({
      ...buildNpcEvent(10, 46),
      sprite: "SPRITE_POKEFAN_F",
      script: "CianwoodCityChucksWife",
      event_flag: "-1",
      object_identifier: "CIANWOODCITY_POKEFAN_F",
    } as ObjectEvent);

    expect(overworld.shouldSpawn(npc, { ignore_event_flag: false })).toBe(true);
  });

  it("keeps defeated trainers visible unless their object event flag hides them", () => {
    const overworld = new TestOverworld();
    const state = createTestGameState();
    state.wram.event_flags.EVENT_BEAT_ROCKET_GRUNTM_16 = true;
    state.wram.trainer_flags.GRUNTM_16 = true;
    overworld.setGameState(state);

    const npc = new OverworldObject({
      ...buildNpcEvent(2, 4),
      sprite: "SPRITE_ROCKET",
      object_type: "OBJECTTYPE_TRAINER",
      radius: 3,
      script: "TrainerGruntM16",
      event_flag: "EVENT_TEAM_ROCKET_BASE_POPULATION",
      object_identifier: "TEAMROCKETBASEB1F_ROCKET2",
      trainer_type: "GRUNTM",
      trainer_id: "GRUNTM_16",
    } as unknown as ObjectEvent);

    expect(overworld.shouldSpawn(npc, { ignore_event_flag: false })).toBe(true);

    state.wram.event_flags.EVENT_TEAM_ROCKET_BASE_POPULATION = true;
    expect(overworld.shouldSpawn(npc, { ignore_event_flag: false })).toBe(false);
  });
});

describe("OverworldNpcManagerMixin.check_for_npc_interaction", () => {
  it("turns the NPC to face the player before running scripts", () => {
    const overworld = new TestOverworldInteraction();
    overworld.setGameState(createTestGameState());
    overworld.setPlayer(2, 3);
    overworld.facingCoords = [2, 2];
    overworld.setMap({} as any);
    overworld.setTilesPerCollision(2);
    overworld.script_runner = { is_busy: false, run: jest.fn() } as any;

    const npc = new OverworldObject(buildNpcEvent(2, 2));
    npc.objectIndex = 1;
    npc.direction = "up";
    overworld.npcs = [npc];

    expect(overworld.check_for_npc_interaction()).toBe(true);
    expect(npc.direction).toBe("down");
  });

  it("does not interact with walking NPCs", () => {
    const overworld = new TestOverworldInteraction();
    overworld.setGameState(createTestGameState());
    overworld.setPlayer(2, 3);
    overworld.facingCoords = [2, 2];
    overworld.setMap({} as any);
    overworld.setTilesPerCollision(2);
    overworld.script_runner = { is_busy: false, run: jest.fn() } as any;

    const npc = new OverworldObject(buildNpcEvent(2, 2));
    npc.objectIndex = 1;
    npc.direction = "up";
    npc.walking = true;
    overworld.npcs = [npc];

    expect(overworld.check_for_npc_interaction()).toBe(false);
    const runner = overworld.script_runner;
    expect(runner?.run).not.toHaveBeenCalled();
    expect(overworld.getGameState()?.wram.last_talked).toBe(0);
  });

  it("requires the NPC to be on the facing tile", () => {
    const overworld = new TestOverworldInteraction();
    overworld.setGameState(createTestGameState());
    overworld.setPlayer(2, 3);
    overworld.facingCoords = [2, 2];
    overworld.setMap({} as any);
    overworld.setTilesPerCollision(2);
    overworld.script_runner = { is_busy: false, run: jest.fn() } as any;

    const npc = new OverworldObject(buildNpcEvent(4, 2));
    npc.objectIndex = 1;
    npc.direction = "up";
    overworld.npcs = [npc];

    expect(overworld.check_for_npc_interaction()).toBe(false);
    const runner = overworld.script_runner;
    expect(runner?.run).not.toHaveBeenCalled();
    expect(overworld.getGameState()?.wram.last_talked).toBe(0);
  });

  it("interacts with an NPC whose collision footprint covers the facing tile", () => {
    const overworld = new TestOverworldInteraction();
    overworld.setGameState(createTestGameState());
    overworld.setPlayer(2, 3);
    overworld.facingCoords = [2, 2];
    overworld.setMap({} as any);
    overworld.setTilesPerCollision(2);
    overworld.script_runner = { is_busy: false, run: jest.fn() } as any;

    const npc = new OverworldObject(buildNpcEvent(3, 2));
    npc.objectIndex = 1;
    npc.direction = "up";
    overworld.npcs = [npc];

    expect(overworld.check_for_npc_interaction()).toBe(true);
    const runner = overworld.script_runner;
    expect(runner?.run).not.toHaveBeenCalled();
    expect(overworld.getGameState()?.wram.last_talked).toBe(1);
  });

  it("does not start the New Bark rival shove script from below the lab wall", () => {
    const overworld = new TestOverworldInteraction();
    overworld.setGameState(createTestGameState());
    overworld.setPlayer(7, 7);
    overworld.facingCoords = [7, 5];
    overworld.setMap({} as any);
    overworld.setTilesPerCollision(2);
    overworld.current_map_name = "NewBarkTown";
    overworld.script_runner = { is_busy: false, run: jest.fn() } as any;

    const rival = new OverworldObject({
      ...buildNpcEvent(3, 2),
      sprite: "SPRITE_RIVAL",
      spritemovedata: "SPRITEMOVEDATA_STANDING_RIGHT",
      script: "NewBarkTownRivalScript",
      object_identifier: "NEWBARKTOWN_RIVAL",
    } as ObjectEvent);
    rival.objectIndex = 3;
    rival.setCollisionStride(2);
    rival.x = 7;
    rival.y = 5;
    overworld.npcs = [rival];

    expect(overworld.check_for_npc_interaction()).toBe(false);
    expect(overworld.script_runner?.run).not.toHaveBeenCalled();
    expect(overworld.getGameState()?.wram.last_talked).toBe(0);
  });

  it("starts the New Bark rival shove script from the west-side approach", () => {
    const overworld = new TestOverworldInteraction();
    overworld.setGameState(createTestGameState());
    overworld.setPlayer(5, 5);
    overworld.facingCoords = [7, 5];
    overworld.setMap({} as any);
    overworld.setTilesPerCollision(2);
    overworld.current_map_name = "NewBarkTown";
    overworld.script_runner = { is_busy: false, run: jest.fn() } as any;

    const rival = new OverworldObject({
      ...buildNpcEvent(3, 2),
      sprite: "SPRITE_RIVAL",
      spritemovedata: "SPRITEMOVEDATA_STANDING_RIGHT",
      script: "NewBarkTownRivalScript",
      object_identifier: "NEWBARKTOWN_RIVAL",
    } as ObjectEvent);
    rival.objectIndex = 3;
    rival.setCollisionStride(2);
    rival.x = 7;
    rival.y = 5;
    overworld.npcs = [rival];

    expect(overworld.check_for_npc_interaction()).toBe(true);
    expect(overworld.script_runner?.run).toHaveBeenCalledWith("NewBarkTownRivalScript");
    expect(overworld.getGameState()?.wram.last_talked).toBe(3);
  });

  it("prefers a direct NPC interaction before counter redirection", () => {
    const overworld = new TestOverworldInteraction();
    overworld.setGameState(createTestGameState());
    overworld.setPlayer(11, 13);
    overworld.facingCoords = [13, 13];
    overworld.setMap({} as any);
    overworld.setTilesPerCollision(2);
    overworld.script_runner = { is_busy: false, run: jest.fn() } as any;
    overworld._counter_adjusted_tile = jest.fn(() => [15, 13]);

    const npc = new OverworldObject(buildNpcEvent(13, 13));
    npc.objectIndex = 1;
    npc.direction = "up";
    overworld.npcs = [npc];

    expect(overworld.check_for_npc_interaction()).toBe(true);
    expect(overworld._counter_adjusted_tile).not.toHaveBeenCalled();
    expect(overworld.getGameState()?.wram.last_talked).toBe(1);
  });

  it("runs scripted NPC interactions with ASM fallthrough enabled", () => {
    const overworld = new TestOverworldInteraction();
    overworld.setGameState(createTestGameState());
    overworld.setPlayer(2, 3);
    overworld.facingCoords = [2, 2];
    overworld.setMap({} as any);
    overworld.setTilesPerCollision(2);
    overworld.script_runner = { is_busy: false, run: jest.fn() } as any;

    const npc = new OverworldObject({
      ...buildNpcEvent(2, 2),
      script: "SudowoodoScript",
    } as ObjectEvent);
    npc.objectIndex = 1;
    overworld.npcs = [npc];

    expect(overworld.check_for_npc_interaction()).toBe(true);
    const runner = overworld.script_runner;
    expect(runner?.run).toHaveBeenCalledWith("SudowoodoScript");
  });

  it("runs direct trainer interactions without script fallthrough", () => {
    const overworld = new TestOverworldInteraction();
    overworld.setGameState(createTestGameState());
    overworld.setPlayer(2, 3);
    overworld.facingCoords = [2, 2];
    overworld.setMap({} as any);
    overworld.setTilesPerCollision(2);
    overworld.script_runner = { is_busy: false, run: jest.fn() } as any;

    const npc = new OverworldObject({
      ...buildNpcEvent(2, 2),
      object_type: "OBJECTTYPE_TRAINER",
      script: "TrainerScript",
    } as ObjectEvent);
    npc.objectIndex = 1;
    overworld.npcs = [npc];

    expect(overworld.check_for_npc_interaction()).toBe(true);
    const runner = overworld.script_runner;
    expect(runner?.run).toHaveBeenCalledWith("TrainerScript", { allow_fallthrough: false });
  });

  it("continues the direct Sudowoodo YES prompt into WateredWeirdTreeScript", () => {
    const overworld = new TestOverworldInteraction();
    const gameState = createTestGameState();
    gameState.sram.key_items.SQUIRTBOTTLE = 1;
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const scriptLoads: string[] = [];
    dataLoader.get_script = (name: string, parent?: string) => {
      scriptLoads.push(parent ? `${parent}::${name}` : name);
      if (name === "SudowoodoScript") {
        return [
          { command: "checkitem", args: ["SQUIRTBOTTLE"] },
          { command: "iftrue", args: [".Fight"] },
          { command: "end", args: [] },
        ];
      }
      if (name === ".Fight" && parent === "SudowoodoScript") {
        return [
          { command: "opentext", args: [] },
          { command: "writetext", args: ["UseSquirtbottleText"] },
          { command: "yesorno", args: [] },
          { command: "iffalse", args: ["DidntUseSquirtbottleScript"] },
          { command: "closetext", args: [] },
        ];
      }
      if (name === "WateredWeirdTreeScript") {
        return [
          { command: "loadwildmon", args: ["SUDOWOODO", "20"] },
          { command: "end", args: [] },
        ];
      }
      if (name === "DidntUseSquirtbottleScript") {
        return [{ command: "end", args: [] }];
      }
      return null;
    };
    dataLoader.get_script_successor = (name: string, parent?: string | null) =>
      name === ".Fight" && parent === "SudowoodoScript" ? [null, "WateredWeirdTreeScript"] : null;
    dataLoader.get_text = () => "Use SQUIRTBOTTLE?";

    overworld.setGameState(gameState);
    overworld.setPlayer(2, 3);
    overworld.facingCoords = [2, 2];
    overworld.setMap({} as any);
    overworld.setTilesPerCollision(2);
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld as any);
    runner._consume_script_choice = (key: string) => (key === "_yesorno_choice" ? true : null);
    overworld.script_runner = runner as any;

    const npc = new OverworldObject({
      ...buildNpcEvent(2, 2),
      spritemovedata: "SPRITEMOVEDATA_SUDOWOODO",
      script: "SudowoodoScript",
    } as ObjectEvent);
    npc.objectIndex = 1;
    overworld.npcs = [npc];

    expect(overworld.check_for_npc_interaction()).toBe(true);
    expect(scriptLoads).toEqual([
      "SudowoodoScript",
      "SudowoodoScript::.Fight",
      "WateredWeirdTreeScript",
    ]);
    expect(gameState.wram.wild_pokemon).toEqual({ species: "SUDOWOODO", level: 20 });
  });

  it("continues the direct Sudowoodo script after real yes/no prompt input", () => {
    const overworld = new TestOverworldInteraction();
    const gameState = createTestGameState();
    gameState.sram.key_items.SQUIRTBOTTLE = 1;
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const scriptLoads: string[] = [];
    dataLoader.get_script = (name: string, parent?: string) => {
      scriptLoads.push(parent ? `${parent}::${name}` : name);
      if (name === "SudowoodoScript") {
        return [
          { command: "checkitem", args: ["SQUIRTBOTTLE"] },
          { command: "iftrue", args: [".Fight"] },
          { command: "end", args: [] },
        ];
      }
      if (name === ".Fight" && parent === "SudowoodoScript") {
        return [
          { command: "opentext", args: [] },
          { command: "writetext", args: ["UseSquirtbottleText"] },
          { command: "yesorno", args: [] },
          { command: "iffalse", args: ["DidntUseSquirtbottleScript"] },
          { command: "closetext", args: [] },
        ];
      }
      if (name === "WateredWeirdTreeScript") {
        return [
          { command: "loadwildmon", args: ["SUDOWOODO", "20"] },
          { command: "end", args: [] },
        ];
      }
      if (name === "DidntUseSquirtbottleScript") {
        return [{ command: "end", args: [] }];
      }
      return null;
    };
    dataLoader.get_script_successor = (name: string, parent?: string | null) =>
      name === ".Fight" && parent === "SudowoodoScript" ? [null, "WateredWeirdTreeScript"] : null;
    dataLoader.get_text = () => "Use SQUIRTBOTTLE?";

    overworld.setGameState(gameState);
    overworld.setPlayer(2, 3);
    overworld.facingCoords = [2, 2];
    overworld.setMap({} as any);
    overworld.setTilesPerCollision(2);
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld as any);
    overworld.script_runner = runner as any;
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn(), charWidth: 8 },
      drawTextBox: jest.fn(),
      draw_window: jest.fn(),
    };
    const dialogue = new FieldDialogueManager(ui, gameState, runner);
    (overworld as unknown as { dialogue: FieldDialogueManager }).dialogue = dialogue;
    for (const eventName of ["open_text", "close_text", "show_text", "wait_for_input", "prompt_yes_no"]) {
      eventManager.on(eventName, dialogue.handle_event.bind(dialogue));
    }

    const npc = new OverworldObject({
      ...buildNpcEvent(2, 2),
      spritemovedata: "SPRITEMOVEDATA_SUDOWOODO",
      script: "SudowoodoScript",
    } as ObjectEvent);
    npc.objectIndex = 1;
    overworld.npcs = [npc];

    expect(overworld.check_for_npc_interaction()).toBe(true);
    (dialogue as unknown as { window: { complete: () => void } }).window.complete();
    dialogue.update();
    expect(dialogue._yes_no_prompt).not.toBeNull();

    dialogue.handle_input({ type: "keydown", key: "KeyZ", code: "KeyZ" });

    expect(dialogue._yes_no_prompt).toBeNull();
    expect(scriptLoads).toEqual([
      "SudowoodoScript",
      "SudowoodoScript::.Fight",
      "WateredWeirdTreeScript",
    ]);
    expect(gameState.wram.wild_pokemon).toEqual({ species: "SUDOWOODO", level: 20 });
  });
});

describe("OverworldNpcManagerMixin._npc_on_tile", () => {
  it("ignores stale last-map coordinates once an NPC has stopped moving", () => {
    const overworld = new TestOverworldInteraction();
    overworld.setGameState(createTestGameState());
    overworld.setMap({} as any);
    overworld.setTilesPerCollision(2);

    const npc = new OverworldObject(buildNpcEvent(4, 4));
    npc.objectIndex = 1;
    (npc as any).prev_x = 2;
    (npc as any).prev_y = 4;
    npc.prevX = 2;
    npc.prevY = 4;
    overworld.npcs = [npc];

    expect(overworld._npc_on_tile(2, 4)).toBeNull();
    expect(overworld._npc_on_tile(4, 4)).toBe(npc);
  });

  it("prefers the lowest object index when multiple NPCs share a tile", () => {
    const overworld = new TestOverworldInteraction();
    overworld.setGameState(createTestGameState());
    overworld.setMap({} as any);
    overworld.setTilesPerCollision(2);

    const laterNpc = new OverworldObject(buildNpcEvent(3, 3));
    laterNpc.objectIndex = 5;
    const earlierNpc = new OverworldObject(buildNpcEvent(3, 3));
    earlierNpc.objectIndex = 2;

    overworld.npcs = [laterNpc, earlierNpc];

    expect(overworld._npc_on_tile(3, 3)).toBe(earlierNpc);
  });
});

describe("OverworldNpcManagerMixin._npc_occupying_subtile", () => {
  it("uses a walking NPC's visible pixel footprint for occupancy", () => {
    const overworld = new TestOverworldInteraction();
    overworld.setGameState(createTestGameState());
    overworld.setMap({} as any);
    overworld.setTilesPerCollision(2);

    const npc = new OverworldObject(buildNpcEvent(4, 4));
    npc.objectIndex = 1;
    npc.prevX = 2;
    npc.prevY = 4;
    (npc as any).prev_x = 2;
    (npc as any).prev_y = 4;
    npc.walking = true;
    npc.pixelX = 8;
    npc.pixelY = 24;
    overworld.npcs = [npc];

    expect(overworld.npcOccupyingSubtile(1, 3)).toBe(npc);
    expect(overworld.npcOccupyingSubtile(4, 4)).toBeNull();
  });

  it("lookup snapshot preserves direct lookup ordering for overlapping footprints", () => {
    const overworld = new TestOverworldInteraction();
    overworld.setGameState(createTestGameState());
    overworld.setMap({} as any);
    overworld.setTilesPerCollision(2);

    const first = new OverworldObject(buildNpcEvent(4, 4));
    first.objectIndex = 7;
    const second = new OverworldObject(buildNpcEvent(4, 4));
    second.objectIndex = 2;
    overworld.npcs = [first, second];

    const lookup = overworld.npcOccupancyLookup();
    expect(lookup(4, 4)).toBe(overworld.npcOccupyingSubtile(4, 4));
  });
});

describe("TDD repro: NPC visual collision boundaries", () => {
  const createFlatMap = () => ({
    width: 6,
    height: 2,
    getMetatileAt: () => 0,
  });

  const createFlatTileset = () => {
    const floor = resolveCollisionValue("FLOOR");
    return {
      tilesetName: "TEST",
      metatiles: [{ collision: [floor, floor, floor, floor] }],
      renderMetatile: jest.fn(),
      renderPriorityMetatile: jest.fn(),
    };
  };

  it("does not leave a stopped NPC solid at stale last-map coordinates where no sprite is drawn", () => {
    const overworld = new TestOverworldInteraction();
    overworld.setGameState(createTestGameState());
    overworld.setMap({} as any);
    overworld.setTilesPerCollision(2);

    const npc = new OverworldObject(buildNpcEvent(0, 0));
    npc.objectIndex = 1;
    npc.x = 9;
    npc.y = 1;
    npc.prevX = 3;
    npc.prevY = 1;
    (npc as any).prev_x = 3;
    (npc as any).prev_y = 1;
    npc.walking = false;
    overworld.npcs = [npc];

    expect(overworld.npcOccupyingSubtile(3, 1)).toBeNull();
    expect(overworld.npcOccupyingSubtile(9, 1)).toBe(npc);
  });

  it("does not block NPC movement against another stopped NPC's stale last-map footprint", () => {
    const overworld = new TestOverworldCollision();
    overworld.setTilesPerCollision(2);
    overworld.setPlayer(0, 0);
    overworld.setMap(createFlatMap(), createFlatTileset());

    const mover = new OverworldObject(buildNpcEvent(0, 0));
    mover.objectIndex = 1;
    mover.x = 1;
    mover.y = 1;
    mover.prevX = 1;
    mover.prevY = 1;

    const blocker = new OverworldObject(buildNpcEvent(0, 0));
    blocker.objectIndex = 2;
    blocker.x = 9;
    blocker.y = 1;
    blocker.prevX = 3;
    blocker.prevY = 1;
    (blocker as any).prev_x = 3;
    (blocker as any).prev_y = 1;
    blocker.walking = false;

    overworld.npcs = [mover, blocker];

    expect(overworld.isNpcStepBlocked(mover, "right", 3, 1)).toBe(false);
  });

  it("does not claim a walking NPC destination before the sprite visually reaches it", () => {
    const overworld = new TestOverworldInteraction();
    overworld.setGameState(createTestGameState());
    overworld.setMap({} as any);
    overworld.setTilesPerCollision(2);

    const npc = new OverworldObject(buildNpcEvent(0, 0));
    npc.objectIndex = 1;
    npc.x = 1;
    npc.y = 1;
    npc.prevX = 1;
    npc.prevY = 1;
    npc.pixelX = 0;
    npc.pixelY = 0;
    npc.targetPixelX = 0;
    npc.targetPixelY = 0;
    overworld.npcs = [npc];

    const task = new MovementTask(npc, ["step RIGHT", "step_end"], { blocking: false });
    task.start({ WALK_FRAMES: 4, TILES_PER_COLLISION: 2 } as any);

    expect(npc.walking).toBe(true);
    expect(npc.pixelX).toBe(0);
    expect(npc.x).toBe(3);
    expect(overworld.npcOccupyingSubtile(3, 1)).toBeNull();
    expect(overworld.npcOccupyingSubtile(1, 1)).toBe(npc);
  });

  it("treats every subtile covered by a large rendered NPC frame as occupied", () => {
    const overworld = new TestOverworldInteraction();
    overworld.setGameState(createTestGameState());
    overworld.setMap({} as any);
    overworld.setTilesPerCollision(2);

    const npc = new OverworldObject(buildNpcEvent(0, 0));
    npc.objectIndex = 1;
    npc.x = 3;
    npc.y = 3;
    npc.prevX = 3;
    npc.prevY = 3;
    npc.pixelX = 16;
    npc.pixelY = 16;
    npc.direction = "down";
    npc.animations = {
      down: { currentFrame: new Surface(32, 32) } as any,
    };
    overworld.npcs = [npc];

    expect(overworld.npcOccupyingSubtile(2, 2)).toBe(npc);
    expect(overworld.npcOccupyingSubtile(4, 3)).toBe(npc);
    expect(overworld.npcOccupyingSubtile(5, 5)).toBe(npc);
  });

  it("blocks player-sized movement into any subtile covered by a large rendered NPC frame", () => {
    const overworld = new TestOverworldCollision();
    overworld.setTilesPerCollision(2);
    overworld.setPlayer(0, 0);
    overworld.setMap(createFlatMap(), createFlatTileset());

    const mover = new OverworldObject(buildNpcEvent(0, 0));
    mover.objectIndex = 1;
    mover.x = 1;
    mover.y = 3;
    mover.prevX = 1;
    mover.prevY = 3;

    const blocker = new OverworldObject(buildNpcEvent(0, 0));
    blocker.objectIndex = 2;
    blocker.x = 3;
    blocker.y = 3;
    blocker.prevX = 3;
    blocker.prevY = 3;
    blocker.pixelX = 16;
    blocker.pixelY = 16;
    blocker.direction = "down";
    blocker.animations = {
      down: { currentFrame: new Surface(32, 32) } as any,
    };

    overworld.npcs = [mover, blocker];

    expect(overworld.isNpcStepBlocked(mover, "right", 5, 3)).toBe(true);
  });
});

describe("OverworldNpcManagerMixin._npc_step_blocked", () => {
  it("can probe only player occupancy for scripted NPC movement", () => {
    const overworld = new TestOverworldCollision();
    overworld.setTilesPerCollision(2);
    overworld.setPlayer(3, 1);

    const npc = new OverworldObject(buildNpcEvent(0, 0));
    npc.x = 1;
    npc.y = 1;
    overworld.npcs = [npc];

    expect(
      overworld.isNpcStepBlocked(npc, "right", 3, 1, { player_only: true })
    ).toBe(true);
    expect(
      overworld.isNpcStepBlocked(npc, "right", 5, 1, { player_only: true })
    ).toBe(false);
  });

  it("blocks NPC movement into the player's destination tile while the player is moving", () => {
    const overworld = new TestOverworldCollision();
    overworld.setTilesPerCollision(2);
    overworld.setPlayer(2, 2);
    overworld.is_moving = true;
    overworld.target_tile_x = 4;
    overworld.target_tile_y = 2;

    const map = {
      width: 10,
      height: 10,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
      renderMetatile: jest.fn(),
      renderPriorityMetatile: jest.fn(),
    };
    overworld.setMap(map, tileset);

    const npc = new OverworldObject(buildNpcEvent(6, 2));
    overworld.npcs = [npc];

    expect(overworld.isNpcStepBlocked(npc, "left", 4, 2)).toBe(true);
  });

  it("blocks NPC movement into the player's destination tile when the player object is walking", () => {
    const overworld = new TestOverworldCollision();
    overworld.setTilesPerCollision(2);
    overworld.setPlayer(11, 13);
    overworld.target_tile_x = 11;
    overworld.target_tile_y = 11;
    overworld.player_object = { walking: true, jumping: false } as any;

    const map = {
      width: 8,
      height: 8,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
      renderMetatile: jest.fn(),
      renderPriorityMetatile: jest.fn(),
    };
    overworld.setMap(map, tileset);

    const npc = new OverworldObject(buildNpcEvent(5, 4));
    npc.x = 11;
    npc.y = 9;
    overworld.npcs = [npc];

    expect(overworld.isNpcStepBlocked(npc, "down", 11, 11)).toBe(true);
  });

  it("blocks NPC movement when leaving a down-wall collision tile", () => {
    const overworld = new TestOverworldCollision();
    overworld.setTilesPerCollision(2);
    overworld.setPlayer(0, 0);

    const downWall = resolveCollisionValue("DOWN_WALL");
    const floor = resolveCollisionValue("FLOOR");
    const map = {
      width: 1,
      height: 2,
      getMetatileAt: (_x: number, y: number) => (y === 0 ? 0 : 1),
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [
        { collision: [downWall, downWall, downWall, downWall] },
        { collision: [floor, floor, floor, floor] },
      ],
      renderMetatile: jest.fn(),
      renderPriorityMetatile: jest.fn(),
    };
    overworld.setMap(map, tileset);

    const npc = new OverworldObject(buildNpcEvent(0, 0));
    npc.x = 1;
    npc.y = 3;
    overworld.npcs = [npc];

    expect(overworld.isNpcStepBlocked(npc, "down", 1, 5)).toBe(true);
  });

  it("blocks NPC movement instead of throwing when collision samples reference an invalid metatile", () => {
    const overworld = new TestOverworldCollision();
    overworld.setTilesPerCollision(2);
    overworld.setPlayer(0, 0);

    const map = {
      width: 5,
      height: 5,
      getMetatileAt: () => 10,
    };
    const tileset = {
      tilesetName: "players_house",
      metatiles: [{ collision: [0, 0, 0, 0] }],
      renderMetatile: jest.fn(),
      renderPriorityMetatile: jest.fn(),
    };
    overworld.setMap(map, tileset);

    const npc = new OverworldObject(buildNpcEvent(4, 4));

    expect(() => overworld.isNpcStepBlocked(npc, "up", 1, 1)).not.toThrow();
    expect(overworld.isNpcStepBlocked(npc, "up", 1, 1)).toBe(true);
  });

  it("suppresses blocked-movement debug logs when requested for scripted movement probes", () => {
    const overworld = new TestOverworldCollision();
    overworld.setTilesPerCollision(2);
    overworld.setPlayer(0, 0);

    const downWall = resolveCollisionValue("DOWN_WALL");
    const floor = resolveCollisionValue("FLOOR");
    const map = {
      width: 1,
      height: 2,
      getMetatileAt: (_x: number, y: number) => (y === 0 ? 0 : 1),
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [
        { collision: [downWall, downWall, downWall, downWall] },
        { collision: [floor, floor, floor, floor] },
      ],
      renderMetatile: jest.fn(),
      renderPriorityMetatile: jest.fn(),
    };
    overworld.setMap(map, tileset);

    const debug = jest.fn();
    overworld._logger = { isEnabledFor: () => true, debug } as any;

    const npc = new OverworldObject(buildNpcEvent(0, 0));
    npc.x = 1;
    npc.y = 3;
    overworld.npcs = [npc];

    expect(
      overworld.isNpcStepBlocked(npc, "down", 1, 5, { suppress_blocked_log: true })
    ).toBe(true);
    expect(debug).not.toHaveBeenCalled();
  });

  it("allows NPC movement when leaving tile walls do not face the step direction", () => {
    const overworld = new TestOverworldCollision();
    overworld.setTilesPerCollision(2);
    overworld.setPlayer(0, 0);

    const downWall = resolveCollisionValue("DOWN_WALL");
    const floor = resolveCollisionValue("FLOOR");
    const map = {
      width: 1,
      height: 2,
      getMetatileAt: (_x: number, y: number) => (y === 0 ? 0 : 1),
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [
        { collision: [floor, floor, floor, floor] },
        { collision: [downWall, downWall, downWall, downWall] },
      ],
      renderMetatile: jest.fn(),
      renderPriorityMetatile: jest.fn(),
    };
    overworld.setMap(map, tileset);

    const npc = new OverworldObject(buildNpcEvent(0, 0));
    npc.x = 1;
    npc.y = 5;
    overworld.npcs = [npc];

    expect(overworld.isNpcStepBlocked(npc, "up", 1, 3)).toBe(false);
  });

  it("blocks NPC movement into the player pending auto-step destination", () => {
    const overworld = new TestOverworldCollision();
    overworld.setTilesPerCollision(2);
    overworld.setPlayer(1, 1);
    overworld._pending_auto_step = ["right", true];

    const map = {
      width: 2,
      height: 1,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
      renderMetatile: jest.fn(),
      renderPriorityMetatile: jest.fn(),
    };
    overworld.setMap(map, tileset);

    const npc = new OverworldObject(buildNpcEvent(0, 0));
    npc.x = 5;
    npc.y = 1;
    overworld.npcs = [npc];

    expect(overworld.isNpcStepBlocked(npc, "left", 3, 1)).toBe(true);
  });

  it("blocks NPC movement into another NPC's visible moving footprint", () => {
    const overworld = new TestOverworldCollision();
    overworld.setTilesPerCollision(2);
    overworld.setPlayer(0, 0);

    const map = {
      width: 6,
      height: 2,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
      renderMetatile: jest.fn(),
      renderPriorityMetatile: jest.fn(),
    };
    overworld.setMap(map, tileset);

    const mover = new OverworldObject(buildNpcEvent(0, 0));
    mover.x = 5;
    mover.y = 1;

    const blocker = new OverworldObject(buildNpcEvent(0, 0));
    blocker.x = 9;
    blocker.y = 1;
    blocker.prevX = 3;
    blocker.prevY = 1;
    (blocker as any).prev_x = 3;
    (blocker as any).prev_y = 1;
    blocker.walking = true;
    blocker.pixelX = 16;
    blocker.pixelY = 0;

    overworld.npcs = [mover, blocker];

    expect(overworld.isNpcStepBlocked(mover, "left", 3, 1)).toBe(true);
  });

  it("does not block a second NPC from entering a destination before the first sprite reaches it", () => {
    const overworld = new TestOverworldCollision();
    overworld.setTilesPerCollision(2);
    overworld.setPlayer(0, 0);

    const map = {
      width: 4,
      height: 2,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
      renderMetatile: jest.fn(),
      renderPriorityMetatile: jest.fn(),
    };
    overworld.setMap(map, tileset);

    const first = new OverworldObject(buildNpcEvent(0, 0));
    first.x = 1;
    first.y = 1;
    first.pixelX = 0;
    first.pixelY = 0;
    first.targetPixelX = 0;
    first.targetPixelY = 0;
    first.objectIndex = 1;

    const second = new OverworldObject(buildNpcEvent(0, 0));
    second.x = 5;
    second.y = 1;
    second.objectIndex = 2;

    overworld.npcs = [first, second];

    const firstTask = new MovementTask(first, ["step RIGHT", "step_end"], { blocking: false });
    firstTask.start(overworld as any);

    expect(first.x).toBe(3);
    expect(first.prevX).toBe(1);
    expect(overworld.isNpcStepBlocked(second, "left", 3, 1)).toBe(false);
  });

  it("blocks NPC movement into the player's last-map tile during scripted player steps", () => {
    const overworld = new TestOverworldCollision();
    overworld.setTilesPerCollision(2);
    overworld.setPlayer(4, 2);
    (overworld as any).prev_player_x = 2;
    (overworld as any).prev_player_y = 2;
    overworld.player_object = { walking: true, jumping: false } as any;
    overworld.target_tile_x = 0;
    overworld.target_tile_y = 0;

    const map = {
      width: 4,
      height: 2,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
      renderMetatile: jest.fn(),
      renderPriorityMetatile: jest.fn(),
    };
    overworld.setMap(map, tileset);

    const npc = new OverworldObject(buildNpcEvent(0, 0));
    npc.x = 0;
    npc.y = 2;
    overworld.npcs = [npc];

    expect(overworld.isNpcStepBlocked(npc, "right", 2, 2)).toBe(true);
  });
});
