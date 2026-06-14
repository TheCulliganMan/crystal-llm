import { statusSchema } from "./types.js";

describe("status schema", () => {
  it("parses the richer krabbyclaw status guidance fields", () => {
    const parsed = statusSchema.parse({
      mode: "overworld",
      map: "PlayersHouse2F",
      location: "Player's House",
      mapId: "24:7",
      coords: [3, 9],
      interactionTile: [3, 11],
      interactionSetup: {
        hotspot: {
          coords: [3, 11],
          label: "Stairs",
          token: "D>",
          hotspotType: "warp",
        },
        recommendedApproach: {
          coords: [3, 11],
          facing: "down",
          setupFrom: [3, 9],
        },
      },
      localFocus: {
        source: "status",
        target: {
          kind: "hotspot",
          coords: [3, 11],
          label: "Stairs",
          token: "D>",
          hotspotType: "warp",
        },
      },
      localMovement: {
        openDirections: [{ direction: "down", tile: "." }],
        blockedDirections: [{ direction: "right", tile: "#" }],
      },
      facing: "down",
      badges: 0,
      inMenu: false,
      inBattle: false,
      inDialog: false,
      textBoxOpen: false,
      promptPending: false,
      movementLocked: false,
      scriptBusy: false,
      canMove: true,
      blockedReason: "terrain",
      partyCount: 0,
      flowSummary: "Next goal: Starter + Pokédex",
      flowNextGoal: "Starter + Pokédex",
      flowCompletionTarget: "Beat Mt. Silver",
    });

    expect(parsed.interactionSetup?.recommendedApproach?.setupFrom).toEqual([3, 9]);
    expect(parsed.localFocus?.target?.label).toBe("Stairs");
    expect(parsed.localMovement?.openDirections[0]?.direction).toBe("down");
    expect(parsed.blockedReason).toBe("terrain");
  });

  it("parses non-overworld MCP status without stale facing or coords", () => {
    const parsed = statusSchema.parse({
      mode: "title",
      surface: {
        kind: "title",
        title: "Title",
        state: "entrance",
        primaryText: "TITLE SCREEN",
      },
      map: "TITLE",
      location: "TITLE",
      mapId: "title",
      badges: 0,
      canMove: false,
      blockedReason: "title_screen",
      partyCount: 0,
      flowSummary: "Next goal: Starter + Pokédex",
      flowNextGoal: "Starter + Pokédex",
      flowCompletionTarget: "Beat Mt. Silver",
    });

    expect(parsed.facing).toBeUndefined();
    expect(parsed.coords).toBeUndefined();
    expect(parsed.map).toBe("TITLE");
    expect(parsed.surface?.primaryText).toBe("TITLE SCREEN");
  });
});
