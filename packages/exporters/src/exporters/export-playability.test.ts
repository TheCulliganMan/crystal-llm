import { buildPlayabilityFromStoryEvents } from "./export-playability";

describe("export-playability", () => {
  it("derives start state, script grants, known items, and completion goals from exported story events", () => {
    const rules = buildPlayabilityFromStoryEvents(
      {
        TestMap: {
          TestScript: [
            { command: "checkevent", args: ["EVENT_ALREADY_DONE"] },
            { command: "setevent", args: ["EVENT_TEST_DONE"] },
            { command: "setflag", args: ["ENGINE_TEST_BADGE"] },
            { command: "verbosegiveitem", args: ["POTION", "1"] },
            { command: "giveitem", args: ["NOT_EXPORTED", "1"] },
            { command: "warp", args: ["NONE", "0", "0"] },
            { command: "warpfacing", args: ["HALL_OF_FAME", "4", "13", "UP"] },
            { command: "halloffame", args: [] },
          ],
          EmptyScript: [{ command: "checkevent", args: ["EVENT_ONLY_CHECKED"] }],
        },
      },
      {
        eventFlags: ["EVENT_INITIAL_VISIBLE"],
        engineFlags: ["ENGINE_INITIAL_FLAG"],
      },
      {
        itemIds: ["POTION"],
        start: { map: "PlayersHouse2F", x: 1, y: 1 },
      }
    );

    expect(rules).toEqual(
      expect.objectContaining({
        start_maps: ["PlayersHouse2F"],
        start_tiles: [{ map: "PlayersHouse2F", tile: { x: 1, y: 1 } }],
        initial_events: ["ENGINE_INITIAL_FLAG", "EVENT_INITIAL_VISIBLE"],
        initial_items: [],
        goal_maps: [],
        goal_events: ["EVENT_HALL_OF_FAME"],
        goal_items: [],
        map_access: [],
        require_all_maps_reachable: false,
        require_walkable_maps: true,
      })
    );
    expect(rules.progression_rules).toEqual([
      {
        id: "script:TestMap:TestScript",
        requires: { events: [], items: [], maps: ["TestMap"] },
        grants: {
          events: ["ENGINE_TEST_BADGE", "EVENT_HALL_OF_FAME", "EVENT_TEST_DONE"],
          items: ["POTION"],
          maps: ["HallOfFame"],
        },
      },
    ]);
    expect(JSON.stringify(rules)).not.toContain("NOT_EXPORTED");
    expect(JSON.stringify(rules)).not.toContain("EVENT_ONLY_CHECKED");
  });

  it("keeps exact event and item ids without case coercion", () => {
    const rules = buildPlayabilityFromStoryEvents(
      {
        ExactMap: {
          ExactScript: [
            { command: "setevent", args: ["EVENT_Mixed_Exact"] },
            { command: "itemball", args: ["Mixed_Item"] },
          ],
        },
      },
      {},
      { itemIds: ["Mixed_Item"] }
    );

    expect(rules.progression_rules).toEqual([
      {
        id: "script:ExactMap:ExactScript",
        requires: { events: [], items: [], maps: ["ExactMap"] },
        grants: {
          events: ["EVENT_Mixed_Exact"],
          items: ["Mixed_Item"],
          maps: [],
        },
      },
    ]);
  });
});
