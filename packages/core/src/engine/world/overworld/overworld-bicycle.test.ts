import { PlayerState } from "@pokecrystal/core/core/enums/overworld";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { OverworldEngine } from "./overworld";

const useKeyItem = (OverworldEngine as unknown as { prototype: { use_key_item: Function } })
  .prototype.use_key_item;

const applyMapEntryState = (OverworldEngine as unknown as {
  prototype: { _apply_map_entry_player_state: Function };
}).prototype._apply_map_entry_player_state;

const isDownhillCoastDirection = (OverworldEngine as unknown as {
  prototype: { _is_downhill_coast_direction: Function };
}).prototype._is_downhill_coast_direction;

const queueDownhillIdleStep = (OverworldEngine as unknown as {
  prototype: { _queue_downhill_idle_step: Function };
}).prototype._queue_downhill_idle_step;

const startPendingAutoStep = (OverworldEngine as unknown as {
  prototype: { _start_pending_auto_step: Function };
}).prototype._start_pending_auto_step;

const startIdleHeldDirectionStep = (OverworldEngine as unknown as {
  prototype: { _start_idle_held_direction_step: Function };
}).prototype._start_idle_held_direction_step;

const tickTurning = (OverworldEngine as unknown as {
  prototype: { _tick_turning: Function };
}).prototype._tick_turning;

const updateOverworld = (OverworldEngine as unknown as {
  prototype: { update: Function };
}).prototype.update;

const handleInput = (OverworldEngine as unknown as {
  prototype: { handle_input: Function };
}).prototype.handle_input;

describe("OverworldEngine bicycle key item parity", () => {
  const createBikeStub = ({
    mapName = "Route17",
    state = PlayerState.NORMAL,
    permission = 0,
    alwaysOnBike = false,
  }: {
    mapName?: string;
    state?: PlayerState;
    permission?: number;
    alwaysOnBike?: boolean;
  }) => {
    const gameState = createInitialGameState();
    gameState.wram.engine_flags.ENGINE_ALWAYS_ON_BIKE = alwaysOnBike;
    return {
      current_map_name: mapName,
      game_state: gameState,
      player_state: state,
      player_sprite_id: "chris",
      player_palette_id: 0,
      player_animations: {},
      _create_player_animations: () => ({}),
      _use_bicycle: OverworldEngine.prototype._use_bicycle,
      _can_toggle_bicycle_here: OverworldEngine.prototype._can_toggle_bicycle_here,
      _current_tile_permission: () => permission,
      _show_field_move_text: jest.fn(),
      _show_field_move_text_async: jest.fn(async () => undefined),
      start_map_music: jest.fn(),
    };
  };

  it("gets on the bike on valid bike tiles", async () => {
    const stub = createBikeStub({ state: PlayerState.NORMAL, permission: 0 });

    const used = await useKeyItem.call(stub, "BICYCLE");

    expect(used).toBe(true);
    expect(stub.player_state).toBe(PlayerState.BIKE);
    expect(stub._show_field_move_text_async).toHaveBeenCalledWith("_GotOnBikeText");
    expect(stub.start_map_music).toHaveBeenCalledTimes(1);
  });

  it("gets off the bike when always-on-bike is not active", async () => {
    const stub = createBikeStub({ state: PlayerState.BIKE, permission: 0 });

    const used = await useKeyItem.call(stub, "BICYCLE");

    expect(used).toBe(true);
    expect(stub.player_state).toBe(PlayerState.NORMAL);
    expect(stub._show_field_move_text_async).toHaveBeenCalledWith("_GotOffBikeText");
    expect(stub.start_map_music).toHaveBeenCalledTimes(1);
  });

  it("blocks getting off the bike when always-on-bike is active", async () => {
    const stub = createBikeStub({
      state: PlayerState.BIKE,
      permission: 0,
      alwaysOnBike: true,
    });

    const used = await useKeyItem.call(stub, "BICYCLE");

    expect(used).toBe(true);
    expect(stub.player_state).toBe(PlayerState.BIKE);
    expect(stub._show_field_move_text_async).toHaveBeenCalledWith("_CantGetOffBikeText");
    expect(stub.start_map_music).not.toHaveBeenCalled();
  });

  it("does not use the synchronous dialogue wait when toggling the bike", async () => {
    const stub = createBikeStub({ state: PlayerState.NORMAL, permission: 0 });
    stub._show_field_move_text.mockImplementation(() => {
      throw new Error("sync dialogue wait used");
    });

    await expect(useKeyItem.call(stub, "BICYCLE")).resolves.toBe(true);

    expect(stub._show_field_move_text).not.toHaveBeenCalled();
    expect(stub._show_field_move_text_async).toHaveBeenCalledWith("_GotOnBikeText");
  });

  it("rejects bike use on disallowed maps or non-floor tiles", () => {
    const indoorStub = createBikeStub({
      mapName: "PlayersHouse1F",
      state: PlayerState.NORMAL,
      permission: 0,
    });
    const nonFloorStub = createBikeStub({
      mapName: "Route17",
      state: PlayerState.NORMAL,
      permission: 1,
    });

    expect(useKeyItem.call(indoorStub, "BICYCLE")).toBe(false);
    expect(useKeyItem.call(nonFloorStub, "BICYCLE")).toBe(false);
    expect(indoorStub._show_field_move_text).not.toHaveBeenCalled();
    expect(nonFloorStub._show_field_move_text).not.toHaveBeenCalled();
  });
});

describe("OverworldEngine registered key item input", () => {
  it("uses the registered key item when Select is pressed", () => {
    const gameState = createInitialGameState();
    gameState.wram.wRegisteredItem = "BICYCLE";
    const stub = {
      game_state: gameState,
      dialogue: null,
      input_capture_active: false,
      _blocking_task_count: 0,
      _held_directions: new Map(),
      _queued_direction: null,
      _ignore_a_until_release: false,
      _ignore_select_until_release: false,
      prev_player_x: 0,
      prev_player_y: 0,
      player_x: 0,
      player_y: 0,
      _town_map_overlay: null,
      _direction_from_key: () => null,
      player_movement_locked: () => false,
      use_key_item: jest.fn(() => true),
      _show_field_move_text: jest.fn(),
    };

    handleInput.call(stub, { type: "keydown", button: "select" });
    handleInput.call(stub, { type: "keydown", button: "select" });

    expect(stub.use_key_item).toHaveBeenCalledTimes(1);
    expect(stub.use_key_item).toHaveBeenCalledWith("BICYCLE");
    expect(stub._show_field_move_text).not.toHaveBeenCalled();
  });

  it("shows the cannot-use text when the registered key item is invalid here", () => {
    const gameState = createInitialGameState();
    gameState.wram.wRegisteredItem = "ITEMFINDER";
    const stub = {
      game_state: gameState,
      dialogue: null,
      input_capture_active: false,
      _blocking_task_count: 0,
      _held_directions: new Map(),
      _queued_direction: null,
      _ignore_a_until_release: false,
      _ignore_select_until_release: false,
      prev_player_x: 0,
      prev_player_y: 0,
      player_x: 0,
      player_y: 0,
      _town_map_overlay: null,
      _direction_from_key: () => null,
      player_movement_locked: () => false,
      use_key_item: jest.fn(() => false),
      _show_field_move_text: jest.fn(),
    };

    handleInput.call(stub, { type: "keydown", button: "select" });

    expect(stub.use_key_item).toHaveBeenCalledWith("ITEMFINDER");
    expect(stub._show_field_move_text).toHaveBeenCalledWith("CantUseItemText");
  });
});

describe("OverworldEngine map-entry bike state parity", () => {
  const createMapEntryStub = ({
    mapName,
    state,
    alwaysOnBike,
  }: {
    mapName: string;
    state: PlayerState;
    alwaysOnBike: boolean;
  }) => {
    const gameState = createInitialGameState();
    gameState.wram.engine_flags.ENGINE_ALWAYS_ON_BIKE = alwaysOnBike;
    return {
      current_map_name: mapName,
      game_state: gameState,
      player_state: state,
      player_sprite_id: "chris",
      player_palette_id: 0,
      player_animations: {},
      _create_player_animations: () => ({}),
      _map_disallows_bike: OverworldEngine.prototype._map_disallows_bike,
    };
  };

  it("forces bike state when map callback enables always-on-bike", () => {
    const stub = createMapEntryStub({
      mapName: "Route17",
      state: PlayerState.NORMAL,
      alwaysOnBike: true,
    });

    applyMapEntryState.call(stub);

    expect(stub.player_state).toBe(PlayerState.BIKE);
  });

  it("clears bike state in indoor maps", () => {
    const stub = createMapEntryStub({
      mapName: "PlayersHouse1F",
      state: PlayerState.BIKE,
      alwaysOnBike: false,
    });

    applyMapEntryState.call(stub);

    expect(stub.player_state).toBe(PlayerState.NORMAL);
  });
});

describe("OverworldEngine downhill coast direction", () => {
  it("requires downhill flag and bike/skate state", () => {
    const gameState = createInitialGameState();
    const stub = {
      game_state: gameState,
      player_state: PlayerState.NORMAL,
    };

    expect(isDownhillCoastDirection.call(stub, "down")).toBe(false);

    stub.player_state = PlayerState.BIKE;
    expect(isDownhillCoastDirection.call(stub, "down")).toBe(false);

    gameState.wram.engine_flags.ENGINE_DOWNHILL = true;
    expect(isDownhillCoastDirection.call(stub, "left")).toBe(false);
    expect(isDownhillCoastDirection.call(stub, "down")).toBe(true);
  });
});

describe("OverworldEngine downhill auto-step queue", () => {
  it("queues and starts downhill auto-steps without held input", () => {
    const gameState = createInitialGameState();
    gameState.wram.engine_flags.ENGINE_DOWNHILL = true;
    const stub = {
      game_state: gameState,
      player_state: PlayerState.BIKE,
      is_moving: false,
      _pending_auto_step: null as [string, boolean] | null,
      _held_directions: new Map<string, null>(),
      player_movement_locked: () => false,
      move_player: jest.fn(),
      _is_downhill_coast_direction: OverworldEngine.prototype._is_downhill_coast_direction,
    };

    queueDownhillIdleStep.call(stub);
    expect(stub._pending_auto_step).toEqual(["down", true]);

    startPendingAutoStep.call(stub);
    expect(stub.move_player).toHaveBeenCalledWith("down", true);
    expect(stub._pending_auto_step).toBeNull();
  });
});

describe("OverworldEngine deferred player step scheduling", () => {
  it("starts an idle held-direction step from the tick instead of keydown", () => {
    const stub = {
      is_moving: false,
      _pending_auto_step: null as [string, boolean] | null,
      _held_directions: new Map<string, null>([["right", null]]),
      _queued_direction: "right" as string | null,
      _turn_frames_remaining: 0,
      player_movement_locked: () => false,
      move_player: jest.fn(),
    };

    startIdleHeldDirectionStep.call(stub);

    expect(stub.move_player).toHaveBeenCalledWith("right");
  });

  it("queues the post-turn step until the post-NPC phase of the update", () => {
    const stub = {
      _turn_frames_remaining: 1,
      _turning_direction: "right" as string | null,
      _turn_should_force_step: false,
      is_moving: false,
      _held_directions: new Map<string, null>([["right", null]]),
      _pending_auto_step: null as [string, boolean] | null,
      player_movement_locked: () => false,
      move_player: jest.fn(),
    };

    tickTurning.call(stub);

    expect(stub.move_player).not.toHaveBeenCalled();
    expect(stub._pending_auto_step).toEqual(["right", false]);

    startPendingAutoStep.call(stub);
    expect(stub.move_player).toHaveBeenCalledWith("right", false);
  });

  it("blocks the player when an NPC enters the destination tile earlier in the same update", () => {
    let npcOccupiesDestination = false;
    const stub = {
      audio_controller: { update: jest.fn() },
      _overworld_time_system: null,
      _tick_tile_animation_timer: jest.fn(),
      _tick_emotes: jest.fn(),
      _tick_grass_rustle: jest.fn(),
      _tick_field_move_states: jest.fn(),
      _warp_cooldown: 0,
      _tick_field_move_animation_queue: jest.fn(),
      _tick_fishing_session: jest.fn(),
      _tick_turning: jest.fn(),
      _field_move_animation_renderer: null,
      _egg_hatch_animation: null,
      is_moving: false,
      _update_follower_movement: jest.fn(),
      _tileset_animator: null,
      _refresh_warp_state: jest.fn(),
      _map_sign: null,
      _npc_autonomous_controller: {
        update: jest.fn(() => {
          npcOccupiesDestination = true;
        }),
      },
      _pending_auto_step: null as [string, boolean] | null,
      player_movement_locked: () => false,
      _held_directions: new Map<string, null>([["right", null]]),
      _queued_direction: "right" as string | null,
      _turn_frames_remaining: 0,
      move_player: jest.fn(function (this: { is_moving: boolean; blocked?: boolean }) {
        if (npcOccupiesDestination) {
          this.blocked = true;
          return;
        }
        this.is_moving = true;
      }),
      player_object: { walking: false, jumping: false },
      player_direction: "right",
      player_animations: {},
      _apply_animation_state: jest.fn(),
      _map_has_tall_grass: false,
      npcs: [],
      _process_player_events: jest.fn(),
      _process_script_tasks: jest.fn(),
      _process_pending_event_flag_updates: jest.fn(),
      _update_earthquake_state: jest.fn(),
      _update_elevator_state: jest.fn(),
      _update_dialogue_and_scripts: jest.fn(() => false),
      _update_fade: jest.fn(),
      _update_poison_flash: jest.fn(),
      _start_idle_held_direction_step: (OverworldEngine.prototype as any)._start_idle_held_direction_step,
      _queue_downhill_idle_step: jest.fn(),
      _start_pending_auto_step: jest.fn(),
    };

    updateOverworld.call(stub);

    expect(stub._npc_autonomous_controller.update).toHaveBeenCalledTimes(1);
    expect(stub.move_player).toHaveBeenCalledWith("right");
    expect(stub.is_moving).toBe(false);
    expect((stub as { blocked?: boolean }).blocked).toBe(true);
  });
});
