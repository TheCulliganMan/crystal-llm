import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { resolveCollisionValue } from "@pokecrystal/core/engine/world/overworld/collision-data";

describe("OverworldEngine handle_a_button headbutt branch", () => {
  const handler = (OverworldEngine as unknown as { prototype: { handle_a_button: Function } })
    .prototype.handle_a_button;

  const createStub = ({
    hasHeadbutt = true,
  }: {
    hasHeadbutt?: boolean;
  }) => {
    const playSound = jest.fn();
    const handleHeadbutt = jest.fn();
    const floor = resolveCollisionValue("FLOOR");
    const stub = {
      check_for_npc_interaction: () => false,
      get_facing_tile_coords: () => [0, 0],
      _counter_adjusted_tile: (x: number, y: number) => [x, y],
      _bg_event_at: () => null,
      _handle_bg_event: () => false,
      _tile_is_headbutt_tree: () => true,
      _party_has_move: () => hasHeadbutt,
      _play_interaction_sound: playSound,
      handle_headbutt: handleHeadbutt,
      player_x: 0,
      player_y: 0,
      player_direction: "up",
      current_map_name: "TestMap",
      map: {
        width: 1,
        height: 1,
        getMetatileAt: () => 0,
      },
      tileset: {
        metatiles: [{ collision: [floor, floor, floor, floor] }],
      },
    };
    return { stub, playSound, handleHeadbutt };
  };

  it("invokes headbutt with a prompt when the player has the move", () => {
    const { stub, playSound, handleHeadbutt } = createStub({ hasHeadbutt: true });

    handler.call(stub);

    expect(playSound).toHaveBeenCalledTimes(1);
    expect(handleHeadbutt).toHaveBeenCalledWith(null, { prompt: true });
  });

  it("does nothing when the player lacks headbutt", () => {
    const { stub, playSound, handleHeadbutt } = createStub({ hasHeadbutt: false });

    handler.call(stub);

    expect(playSound).not.toHaveBeenCalled();
    expect(handleHeadbutt).not.toHaveBeenCalled();
  });
});

describe("OverworldEngine handle_a_button collision stdscripts", () => {
  const handler = (OverworldEngine as unknown as { prototype: { handle_a_button: Function } })
    .prototype.handle_a_button;

  it("runs PCScript for the Route36 National Park Gate PC collision quadrant", () => {
    const floor = resolveCollisionValue("FLOOR");
    const wall = resolveCollisionValue("WALL");
    const pc = resolveCollisionValue("PC");
    const run = jest.fn();
    const playSound = jest.fn();
    const metatiles = Array.from({ length: 0x2d }, () => ({
      collision: [floor, floor, floor, floor],
    }));
    metatiles[0x2c] = { collision: [wall, wall, floor, pc] };

    const stub = {
      check_for_npc_interaction: () => false,
      get_facing_tile_coords: () => [19, 3],
      _counter_adjusted_tile: (x: number, y: number) => [x, y],
      _bg_event_at: () => null,
      _handle_bg_event: () => false,
      _tile_is_headbutt_tree: () => false,
      _play_interaction_sound: playSound,
      player_x: 19,
      player_y: 5,
      player_direction: "up",
      current_map_name: "Route36NationalParkGate",
      game_state: { wram: { last_talked: 99 } },
      script_runner: { run },
      map: {
        width: 5,
        height: 4,
        getMetatileAt: (x: number, y: number) => (x === 4 && y === 0 ? 0x2c : 0),
      },
      tileset: {
        metatiles,
      },
    };

    handler.call(stub);

    expect(playSound).toHaveBeenCalledTimes(1);
    expect(run).toHaveBeenCalledWith("PCScript");
    expect(stub.game_state.wram.last_talked).toBe(0);
  });
});
