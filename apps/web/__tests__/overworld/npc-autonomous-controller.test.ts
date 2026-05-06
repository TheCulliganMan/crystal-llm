import type { ObjectEvent } from "@/core/models/map";
import { OverworldNpcManagerMixin } from "@/engine/world/overworld/overworld-npc-manager";
import { OverworldObject } from "@/engine/world/overworld/overworld-object";

type NpcControllerLike = {
  states: Map<string, boolean>;
  rebuild(npcs: OverworldObject[]): void;
};

const createTestEvent = (): ObjectEvent => ({
  sprite: "SPRITE_TEST",
  x: 0,
  y: 0,
  spritemovedata: "SPRITEMOVEDATA_STANDING_DOWN",
  move_range_x: 0,
  move_range_y: 0,
  hram_x: 0,
  hram_y: 0,
  pal: 0,
  object_type: "OBJECTTYPE_SCRIPT",
  radius: 0,
  script: "TestScript",
  event_flag: "EVENT_TEST",
  object_identifier: null,
  sightline_direction_override: null,
});

class TestNpcManager extends OverworldNpcManagerMixin {
  constructor() {
    super();
    this.current_map_name = "TestMap";
    this.npcs = [];
    this._npc_index_lookup = new Map();
  }

  public _npc_autonomous_controller?: NpcControllerLike;

  protected _build_blueprint(): Map<string, [ObjectEvent, number]> {
    const event = createTestEvent();
    return new Map([["npc_1", [event, 1]]]);
  }

  protected _add_map_sprites(): OverworldObject[] {
    const event = createTestEvent();
    const npc = new OverworldObject(event);
    npc.objectIndex = 1;
    return [npc];
  }

  protected _initialise_npc_object_safe(): void {}
}

describe("OverworldNpcManagerMixin.refresh_map_sprites", () => {
  it("preserves controller binding for rebuild", () => {
    const manager = new TestNpcManager();
    const controller: NpcControllerLike = {
      states: new Map<string, boolean>(),
      rebuild(npcs: OverworldObject[]) {
        this.states.set("called", true);
        this.states.set(`count:${npcs.length}`, true);
      },
    };
    manager._npc_autonomous_controller = controller;

    expect(() => manager.refresh_map_sprites()).not.toThrow();
    expect(controller.states.get("called")).toBe(true);
    expect(controller.states.get("count:1")).toBe(true);
  });
});
