import { OverworldInputMixin } from "./overworld-input";
import type { GameEngineEvent } from "@pokecrystal/core/ui/game-engine";
import type { TownMapOverlayLike } from "@pokecrystal/core/ui/overlays/town-map-overlay";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { Event } from "@pokecrystal/core/engine/world/events";
import { OverworldObject } from "@pokecrystal/core/engine/world/overworld/overworld-object";
import { JOY_A } from "@pokecrystal/core/core/constants";
import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { isDebugEnabled } from "@pokecrystal/core/core/debug-flags";
import { pushDebugLog } from "@pokecrystal/core/core/debug-log";
import { createInitialGameState, type GameState } from "@pokecrystal/core/core/state";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import type { ObjectEvent } from "@pokecrystal/core/core/models/map";
import { OverworldMap } from "@pokecrystal/core/engine/world/overworld/overworld-map";
import { OverworldTileset } from "@pokecrystal/core/engine/world/overworld/overworld-tileset";

jest.mock("@pokecrystal/core/core/debug-flags", () => {
  const actual = jest.requireActual("@pokecrystal/core/core/debug-flags");
  return {
    ...actual,
    isDebugEnabled: jest.fn(),
  };
});

jest.mock("@pokecrystal/core/core/debug-log", () => {
  const actual = jest.requireActual("@pokecrystal/core/core/debug-log");
  return {
    ...actual,
    pushDebugLog: jest.fn(),
  };
});

class TestOverworldInput extends OverworldInputMixin {
  public _town_map_overlay?: TownMapOverlayLike;
  public facingCoords: [number, number] = [0, 0];
  public script_runner: { is_busy?: boolean; run?: jest.Mock; last_interaction_object_index?: number | null } | null =
    null;

  public invokeInteractionSound(): void {
    this._play_interaction_sound();
  }

  public setGameState(state: GameState): void {
    this.game_state = state;
  }

  public setPlayer(x: number, y: number): void {
    this.player_x = x;
    this.player_y = y;
  }

  public setMap(map: unknown): void {
    this.map = map as never;
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

  public _npc_occupying_subtile(subtileX: number, subtileY: number): OverworldObject | null {
    for (const npc of this.npcs ?? []) {
      const stride = Math.max(1, npc.collisionStride ?? this.TILES_PER_COLLISION);
      const footprint = stride - 1;
      const originX = npc.x - footprint;
      const originY = npc.y - footprint;
      if (
        subtileX >= originX &&
        subtileX < originX + stride &&
        subtileY >= originY &&
        subtileY < originY + stride
      ) {
        return npc;
      }
    }
    return null;
  }
}

const createTestGameState = (): GameState => createInitialGameState();

const buildNpcEvent = (x: number, y: number, script = "NpcScript"): ObjectEvent =>
  ({
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
    script,
    event_flag: "",
    sight_range: 0,
    hour: -1,
    time_group: -1,
    object_identifier: null,
  } as unknown as ObjectEvent);

describe("OverworldInputMixin audio guards", () => {
  const isDebugEnabledMock = jest.mocked(isDebugEnabled);
  const pushDebugLogMock = jest.mocked(pushDebugLog);

  beforeEach(() => {
    isDebugEnabledMock.mockReset();
    isDebugEnabledMock.mockReturnValue(false);
    pushDebugLogMock.mockReset();
  });

  it("plays interaction sounds via playSound when play_sound is missing", () => {
    const input = new TestOverworldInput();
    const playSound = jest.fn();
    class TestAudioEngine extends AudioEngine {
      public override playSound(name: string): void {
        playSound(name);
      }
    }
    input.audio_engine = new TestAudioEngine();

    expect(() => input.invokeInteractionSound()).not.toThrow();
    expect(playSound).toHaveBeenCalledWith("SFX_READ_TEXT_2");
  });

  it("routes input to the town map overlay before overworld handlers", () => {
    const input = new TestOverworldInput();
    const overlayHandle = jest.fn().mockReturnValue(true);
    const describeEvent = jest.fn(() => "keydown:KeyZ");
    const handleA = jest.fn();
    input._town_map_overlay = { handle_input: overlayHandle };
    input._describe_input_event = describeEvent;
    input._direction_from_key = jest.fn(() => null);
    input.player_movement_locked = jest.fn(() => false);
    input.handle_a_button = handleA;
    input.dialogue = null;
    input._blocking_task_count = 0;

    input.handle_input({ type: "keydown", key: "KeyZ" });

    expect(overlayHandle).toHaveBeenCalledTimes(1);
    expect(describeEvent).not.toHaveBeenCalled();
    expect(handleA).not.toHaveBeenCalled();
  });

  it("traces dialogue input consumption only when debug input is enabled", () => {
    const input = new TestOverworldInput();
    const dialogueHandle = jest.fn(() => true);
    input.dialogue = {
      handle_input: dialogueHandle,
      active: true,
      waiting_for_input: false,
    } as TestOverworldInput["dialogue"];
    input._describe_input_event = jest.fn(() => "keydown:KeyZ");

    input.handle_input({ type: "keydown", key: "KeyZ" });
    expect(dialogueHandle).toHaveBeenCalledTimes(1);
    expect(pushDebugLogMock).not.toHaveBeenCalled();

    isDebugEnabledMock.mockImplementation((channel) => channel === "overworld:input");
    input.handle_input({ type: "keydown", key: "KeyZ" });
    expect(pushDebugLogMock).toHaveBeenCalledWith("Dialogue consumed keydown:KeyZ");
  });

  it("suppresses A presses until the key is released", () => {
    const input = new TestOverworldInput();
    const handleA = jest.fn();
    input._direction_from_key = jest.fn(() => null);
    input.player_movement_locked = jest.fn(() => false);
    input.handle_a_button = handleA;
    input.dialogue = null;
    input._blocking_task_count = 0;
    input._ignore_a_until_release = true;

    input.handle_input({ type: "keydown", key: "KeyZ" });
    expect(handleA).not.toHaveBeenCalled();

    input.handle_input({ type: "keyup", key: "KeyZ" });
    input.handle_input({ type: "keydown", key: "KeyZ" });
    expect(handleA).toHaveBeenCalledTimes(1);
  });

  it("handles only one A interaction while the key is held", () => {
    const input = new TestOverworldInput();
    const handleA = jest.fn();
    input._direction_from_key = jest.fn(() => null);
    input.player_movement_locked = jest.fn(() => false);
    input.handle_a_button = handleA;
    input.dialogue = null;
    input._blocking_task_count = 0;
    input._ignore_a_until_release = false;

    input.handle_input({ type: "keydown", key: "KeyZ" });
    input.handle_input({ type: "keydown", key: "KeyZ" });
    input.handle_input({ type: "keydown", key: "KeyZ" });

    expect(handleA).toHaveBeenCalledTimes(1);
    expect(input._ignore_a_until_release).toBe(true);

    input.handle_input({ type: "keyup", key: "KeyZ" });
    input.handle_input({ type: "keydown", key: "KeyZ" });
    expect(handleA).toHaveBeenCalledTimes(2);
  });

  it("ignores non-dialogue key events while input capture is active", () => {
    const input = new TestOverworldInput();
    const handleA = jest.fn();
    const movePlayer = jest.fn();
    input._direction_from_key = jest.fn(() => null);
    input.player_movement_locked = jest.fn(() => false);
    input.handle_a_button = handleA;
    input.move_player = movePlayer;
    input.dialogue = null;
    input._blocking_task_count = 0;
    input.input_capture_active = true;

    input.handle_input({ type: "keydown", key: "KeyZ" });
    input.handle_input({ type: "keydown", key: "ArrowDown" });
    input.handle_input({ type: "keyup", key: "KeyZ" });

    expect(handleA).not.toHaveBeenCalled();
    expect(movePlayer).not.toHaveBeenCalled();
  });

  it("still routes captured key events to dialogue while input capture is active", () => {
    const input = new TestOverworldInput();
    const dialogueHandle = jest.fn(() => true);
    const handleA = jest.fn();
    const movePlayer = jest.fn();
    input.dialogue = {
      handle_input: dialogueHandle,
      active: true,
      waiting_for_input: true,
    } as TestOverworldInput["dialogue"];
    input._direction_from_key = jest.fn(() => null);
    input.player_movement_locked = jest.fn(() => false);
    input.handle_a_button = handleA;
    input.move_player = movePlayer;
    input._blocking_task_count = 0;
    input.input_capture_active = true;

    input.handle_input({ type: "keydown", key: "KeyZ" });

    expect(dialogueHandle).toHaveBeenCalledTimes(1);
    expect(handleA).not.toHaveBeenCalled();
    expect(movePlayer).not.toHaveBeenCalled();
  });

  it("sets A suppression when text closes while A is held", () => {
    const unlockMovement = jest.fn();
    const context = {
      _text_lock_active: true,
      unlock_player_movement: unlockMovement,
      _pending_poison_whiteout: false,
      _perform_poison_whiteout: jest.fn(),
      _ignore_a_until_release: false,
      game_state: {
        hram: {
          joypad: {
            hJoyDown: JOY_A,
            hJoypadDown: 0,
          },
        },
      },
    } as unknown as OverworldEngine;

    const event = new Event("close_text", {});
    const handleTextVisibility = (
      OverworldEngine.prototype as unknown as {
        _handle_text_visibility_event: (event: GameEngineEvent, state: OverworldEngine["game_state"]) => void;
      }
    )._handle_text_visibility_event;
    handleTextVisibility.call(context, event, context.game_state);

    expect(unlockMovement).toHaveBeenCalledTimes(1);
    expect(context._ignore_a_until_release).toBe(true);
  });

  it("defers direction movement until the overworld tick", () => {
    const input = new TestOverworldInput();
    const movePlayer = jest.fn();
    input._direction_from_key = jest.fn(() => "right");
    input.player_movement_locked = jest.fn(() => false);
    input.move_player = movePlayer;
    input.dialogue = null;
    input._blocking_task_count = 0;

    input.handle_input({ type: "keydown", key: "ArrowRight" });

    expect(movePlayer).not.toHaveBeenCalled();
    expect(input._queued_direction).toBe("right");
    expect(Array.from(input._held_directions.keys())).toEqual(["right"]);
  });

  it("interacts with scripted NPCs whose footprint covers the facing tile and preserves script fallthrough", () => {
    const input = new TestOverworldInput();
    input.setGameState(createTestGameState());
    input.setPlayer(2, 3);
    input.facingCoords = [2, 2];
    input.setMap({});
    input.setTilesPerCollision(2);
    input.script_runner = { is_busy: false, run: jest.fn(), last_interaction_object_index: null };

    const npc = new OverworldObject(buildNpcEvent(3, 2));
    npc.objectIndex = 7;
    input.npcs = [npc];

    expect(input.check_for_npc_interaction()).toBe(true);
    expect(input.script_runner?.run).toHaveBeenCalledWith("NpcScript");
    expect(input.game_state.wram.last_talked).toBe(7);
  });

  it("prefers a direct NPC interaction before counter redirection", () => {
    const input = new TestOverworldInput();
    input.setGameState(createTestGameState());
    input.setPlayer(11, 13);
    input.facingCoords = [13, 13];
    input.setMap({});
    input.setTilesPerCollision(2);
    input.script_runner = { is_busy: false, run: jest.fn(), last_interaction_object_index: null };
    input._counter_adjusted_tile = jest.fn(() => [15, 13]);

    const npc = new OverworldObject(buildNpcEvent(13, 13, "DirectNpcScript"));
    npc.objectIndex = 7;
    input.npcs = [npc];

    expect(input.check_for_npc_interaction()).toBe(true);
    expect(input._counter_adjusted_tile).not.toHaveBeenCalled();
    expect(input.script_runner?.run).toHaveBeenCalledWith("DirectNpcScript");
    expect(input.game_state.wram.last_talked).toBe(7);
  });

  it("interacts with Pokecenter nurses across the counter lane", () => {
    const input = new TestOverworldInput();
    input.setGameState(createTestGameState());
    input.setPlayer(7, 7);
    input.facingCoords = [7, 5];
    input.setMap({});
    input.setTilesPerCollision(2);
    input.script_runner = { is_busy: false, run: jest.fn(), last_interaction_object_index: null };
    input._counter_adjusted_tile = jest.fn(() => [7, 3]);

    const npc = new OverworldObject({
      ...buildNpcEvent(7, 3, "EcruteakPokecenter1FNurseScript"),
      sprite: "SPRITE_NURSE",
      spritemovedata: "SPRITEMOVEDATA_STANDING_DOWN",
    });
    npc.objectIndex = 1;
    input.npcs = [npc];

    expect(input.check_for_npc_interaction()).toBe(true);
    expect(input._counter_adjusted_tile).toHaveBeenCalledWith(7, 5);
    expect(input.script_runner?.run).toHaveBeenCalledWith("EcruteakPokecenter1FNurseScript");
    expect(input.game_state.wram.last_talked).toBe(1);
  });

  it("interacts with the real Olivine Pokecenter nurse across the counter lane", async () => {
    const loader = new DataLoader();
    loader.load_map_attributes();
    loader.load_map_dimensions();
    loader.load_npc_data();
    const attributes = loader.map_attributes.get("OlivinePokecenter1F");
    const dimensions = loader.map_dimensions.get("OLIVINE_POKECENTER_1F");
    const nurseEvent = loader.npc_data
      .get("OlivinePokecenter1F")
      ?.find((event) => event.script === "OlivinePokecenter1FNurseScript");
    expect(attributes?.tileset_name).toBe("pokecenter");
    expect(dimensions).toBeTruthy();
    expect(nurseEvent).toBeTruthy();

    const input = new TestOverworldInput();
    input.setGameState(createTestGameState());
    input.setPlayer(7, 7);
    input.facingCoords = [7, 5];
    input.setMap(new OverworldMap("OlivinePokecenter1F", dimensions!.width, dimensions!.height, dimensions!.blocks));
    input.setTilesPerCollision(2);
    input.script_runner = { is_busy: false, run: jest.fn(), last_interaction_object_index: null };
    const tileset = new OverworldTileset(attributes!.tileset_name, "day");
    await tileset.ready;
    input.tileset = tileset;
    input._counter_adjusted_tile = OverworldEngine.prototype._counter_adjusted_tile.bind(
      input as unknown as OverworldEngine
    );

    const npc = new OverworldObject(nurseEvent!);
    npc.objectIndex = 1;
    npc.setCollisionStride(2);
    npc.x = nurseEvent!.x * 2 + 1;
    npc.y = nurseEvent!.y * 2 + 1;
    npc.prevX = npc.x;
    npc.prevY = npc.y;
    input.npcs = [npc];

    expect(input.check_for_npc_interaction()).toBe(true);
    expect(input.script_runner?.run).toHaveBeenCalledWith("OlivinePokecenter1FNurseScript");
    expect(input.game_state.wram.last_talked).toBe(1);
  });

  it("starts direct trainer interactions without script fallthrough", () => {
    const input = new TestOverworldInput();
    input.setGameState(createTestGameState());
    input.setPlayer(2, 3);
    input.facingCoords = [2, 2];
    input.setMap({});
    input.setTilesPerCollision(2);
    input.script_runner = { is_busy: false, run: jest.fn(), last_interaction_object_index: null };

    const npc = new OverworldObject({
      ...buildNpcEvent(3, 2, "TrainerScript"),
      object_type: "OBJECTTYPE_TRAINER",
    });
    npc.objectIndex = 8;
    input.npcs = [npc];

    expect(input.check_for_npc_interaction()).toBe(true);
    expect(input.script_runner?.run).toHaveBeenCalledWith("TrainerScript", { allow_fallthrough: false });
    expect(input.game_state.wram.last_talked).toBe(8);
  });
});
