import { MapEvents } from "@/core/models/map";
import { OverworldMapManagerMixin } from "@/engine/world/overworld/overworld-map-manager";

class TestMapManager extends OverworldMapManagerMixin {
  public TILES_PER_COLLISION = 2;
  public _map_events = MapEvents.parse({ warps: [], coord_events: [], bg_events: [] });

  public findBgEvent(tileX: number, tileY: number) {
    return this._bg_event_at(tileX, tileY);
  }
}

describe("OverworldMapManagerMixin._bg_event_at", () => {
  it("matches bg events at stride-offset tile coordinates", () => {
    const manager = new TestMapManager();
    const stride = manager.TILES_PER_COLLISION;
    const offset = stride - 1;
    const event = { x: 3, y: 4, event_type: "BGEVENT_READ", script: "TestScript" };
    manager._map_events = MapEvents.parse({ warps: [], coord_events: [], bg_events: [event] });

    const match = manager.findBgEvent(event.x * stride + offset, event.y * stride + offset);
    expect(match?.script).toBe("TestScript");

    const offByOne = manager.findBgEvent(event.x * stride, event.y * stride);
    expect(offByOne).toBeNull();
  });
});
