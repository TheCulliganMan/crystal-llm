import {
  buildSnapshotPayload,
  isNonBlockingPcUiSnapshot,
  promptFromSnapshot,
  renderFrameToCompactText,
  renderFrameToText,
} from "./text-render";
import type { TextSnapshotPayload } from "./text-render";

const baseSnapshot = (): TextSnapshotPayload => ({
  viewport: ["OVERWORLD"],
  info: ["LEGEND"],
  menu: null,
  prompt: null,
  dialogue: null,
  titles: { viewport: "Overworld", info: "Info" },
  marker: null,
  action_log: [],
  script: {},
  tasks: [],
});

describe("MCP text rendering prompts", () => {
  it("marks yes/no prompts as pending for waiters", () => {
    const snapshot = baseSnapshot();
    snapshot.prompt = ["Do it?", "▶ YES", "  NO"];

    const status = promptFromSnapshot(snapshot);

    expect(status).toEqual({ pending: true, reason: "prompt" });
  });

  it("does not treat Bill's PC bottom instruction text as a blocking prompt", () => {
    const snapshot = baseSnapshot();
    snapshot.titles.viewport = "Bill's PC";
    snapshot.menu = ["▶ WITHDRAW", "  STATS", "  RELEASE", "  CANCEL"];
    snapshot.prompt = ["What's up?"];

    const status = promptFromSnapshot(snapshot);

    expect(status).toEqual({ pending: false, reason: null });
  });

  it.each([
    ["Choose a <PK><MN>."],
    ["Select a POKéMON."],
    ["What's up?"],
    ["Move to where?"],
  ])("does not treat Bill's PC instruction %s as a blocking prompt", (line) => {
    const snapshot = baseSnapshot();
    snapshot.titles.viewport = "Bill's PC";
    snapshot.prompt = [line];

    const status = promptFromSnapshot(snapshot);

    expect(status).toEqual({ pending: false, reason: null });
  });

  it("keeps real PC cursor prompts pending", () => {
    const snapshot = baseSnapshot();
    snapshot.titles.viewport = "Bill's PC";
    snapshot.prompt = ["Change BOX?", "▶ YES", "  NO"];

    const status = promptFromSnapshot(snapshot);

    expect(status).toEqual({ pending: true, reason: "prompt" });
  });

  it.each([
    ["PC hub", { titles: { viewport: "PC", info: "Legend" }, menu: ["▶ BILL's PC", "  CHRIS's PC", "  TURN OFF"] }],
    ["Bill top menu", { titles: { viewport: "Bill's PC", info: "Legend" }, menu: ["▶ WITHDRAW <PK><MN>", "  DEPOSIT <PK><MN>"] }],
    ["renderer prompt top menu", { titles: { viewport: "Prompt", info: "Info" }, menu: ["▶ WITHDRAW <PK><MN>", "  DEPOSIT <PK><MN>"] }],
    ["Bill action menu", { titles: { viewport: "Bill's PC", info: "Legend" }, menu: ["▶ DEPOSIT", "  STATS"], prompt: ["What's up?"] }],
    ["Bill deposit list", { titles: { viewport: "Bill's PC", info: "Legend" }, prompt: ["Select a Pokémon."] }],
    ["Bill move list", { titles: { viewport: "Bill's PC", info: "Legend" }, prompt: ["Move to where?"] }],
  ])("marks %s as nonblocking PC UI", (_label, partial) => {
    const snapshot = { ...baseSnapshot(), ...partial };

    expect(isNonBlockingPcUiSnapshot(snapshot)).toBe(true);
  });

  it("does not mark real PC cursor prompts as nonblocking UI", () => {
    const snapshot = baseSnapshot();
    snapshot.titles.viewport = "Bill's PC";
    snapshot.prompt = ["Release GEODUDE?", "▶ YES", "  NO"];

    expect(isNonBlockingPcUiSnapshot(snapshot)).toBe(false);
  });

  it("does not treat dialogue-only frames as prompts", () => {
    const snapshot = baseSnapshot();
    snapshot.dialogue = ["Professor Elm: Are you ready?"];

    const status = promptFromSnapshot(snapshot);

    expect(status).toEqual({ pending: false, reason: null });
  });

  it("clears prompt state once prompt and dialogue vanish", () => {
    const snapshot = baseSnapshot();
    snapshot.prompt = ["▶ YES", "  NO"];

    const promptStatus = promptFromSnapshot(snapshot);
    expect(promptStatus.pending).toBe(true);

    snapshot.prompt = null;
    snapshot.dialogue = null;

    const clearedStatus = promptFromSnapshot(snapshot);
    expect(clearedStatus).toEqual({ pending: false, reason: null });
  });
});

describe("renderFrameToText", () => {
  it("preserves map legend lines in compact output", () => {
    const payload = baseSnapshot();
    payload.titles.info = "Legend";
    payload.info = [
      "D-Pad=Move A=Talk Start=Menu Select=Item B=Back",
      "Legend: @=Player .=Floor x=Missing v=Down",
    ];

    const text = renderFrameToText(payload);

    expect(text).toContain("Legend: @=Player");
    expect(text).not.toContain("D-Pad=");
  });

  it("renders legend lines immediately after the map block", () => {
    const payload = baseSnapshot();
    payload.viewport = ["01 ..@.", "02 ####"];
    payload.titles.info = "Legend";
    payload.info = ["Legend: @=Player .=Floor #=Wall", "Pos: (49,21)"];

    const text = renderFrameToText(payload);

    expect(text).toBe([
      "OVERWORLD",
      "01 ..@.",
      "02 ####",
      "Legend: @=Player .=Floor #=Wall",
      "Pos: (49,21)",
    ].join("\n"));
  });

  it("embeds prompt options in the text output", () => {
    const payload = baseSnapshot();
    payload.prompt = ["GO?", "▶ YES", "  NO"];
    payload.mcp = {
      move_summary: {
        direction: "up",
        requested: 2,
        completed: 2,
        start: [0, 0],
        end: [0, 2],
        map: "TestMap",
      },
    };
    const text = renderFrameToText(payload);

    expect(text).toContain("PROMPT");
    expect(text).toContain("GO?");
    expect(text).toContain("YES");
    expect(text).not.toContain("ACTIVE INPUT");
  });

  it("renders MCP notices so whiteouts do not look like random teleports", () => {
    const payload = baseSnapshot();
    payload.notices = ["CHRIS is out of useable POKeMON! CHRIS whited out!"];

    const text = renderFrameToText(payload);

    expect(text).toContain("NOTICE");
    expect(text).toContain("CHRIS is out of useable POKeMON!");
  });

  it("copies snapshots so later mutations do not leak into payload rendering", () => {
    const payload = buildSnapshotPayload(
      {
        viewportLines: ["VIEW"],
        infoLines: ["INFO"],
        menuLines: null,
        promptLines: ["Are you sure?"],
        dialogueLines: ["Press A"],
        viewportTitle: "View",
        infoTitle: "Info",
        marker: null,
        actionLog: [],
      },
      { actionLog: [] }
    );

    const rendered = renderFrameToText(payload);
    payload.prompt?.push("▶ YES");
    payload.dialogue?.push("NO");

    expect(rendered).toContain("Are you sure?");
    expect(rendered).not.toContain("▶ YES");
    expect(rendered).not.toContain("NO");
  });
});

describe("renderFrameToCompactText", () => {
  it("includes map legend lines when present", () => {
    const payload = baseSnapshot();
    payload.info = ["Pos: X=12 Y=34", "Legend: @=Player .=Floor v=Down"];

    const text = renderFrameToCompactText(payload);

    expect(text).toContain("Pos: X=12 Y=34");
    expect(text).toContain("Legend: @=Player .=Floor v=Down");
  });

  it("keeps concise format for non-overworld frames", () => {
    const payload = baseSnapshot();
    payload.info = ["INFO"];
    payload.viewport = ["BATTLE"];
    payload.titles.viewport = "Battle";

    const text = renderFrameToCompactText(payload);

    expect(text).toContain("BATTLE");
  });

  it("includes notices in compact output", () => {
    const payload = baseSnapshot();
    payload.notices = ["CHRIS is out of useable POKeMON! CHRIS whited out!"];

    const text = renderFrameToCompactText(payload);

    expect(text).toContain("NOTICE");
    expect(text).toContain("CHRIS whited out!");
  });

  it("does not truncate dialogue in compact output", () => {
    const payload = baseSnapshot();
    payload.titles.viewport = "Battle";
    payload.dialogue = ["line 1", "line 2", "line 3", "line 4", "line 5", "line 6", "line 7"];

    const text = renderFrameToCompactText(payload);

    expect(text).toContain("line 7");
  });

  it("renders hotspot and flow sections when present", () => {
    const payload = baseSnapshot();
    payload.map = {
      map: "NewBarkTown",
      map_id: "1:1",
      player: {
        coords: { x: 10, y: 10 },
        facing: "up",
      },
      warps: [],
      hotspots: [
        {
          id: "gym-1",
          type: "gym",
          label: "Gym",
          coords: { x: 12, y: 8 },
          visible: true,
          interactable: true,
          approach_tiles: [
            {
              coords: { x: 10, y: 8 },
              facing: "right",
            },
          ],
          token: "G",
        },
      ],
    };
    payload.flow_state = {
      completion_target: { id: "mt-silver", title: "Beat Mt. Silver" },
      summary: "Next goal: Zephyr Badge",
      completed_count: 2,
      total_count: 21,
      completed: [],
      available: [],
      remaining: [],
      remaining_path: [],
      next_goal: { id: "violet-badge", title: "Zephyr Badge" },
    };

    const text = renderFrameToCompactText(payload);

    expect(text).toContain("HOTSPOTS");
    expect(text).toContain("G Gym (2N) face right");
    expect(text).toContain("FLOW");
    expect(text).toContain("Next goal: Zephyr Badge");
    expect(text).toContain("Progress: 2/21");
  });

  it("marks hotspots on the current tile as here", () => {
    const payload = baseSnapshot();
    payload.map = {
      map: "NewBarkTown",
      map_id: "1:1",
      player: {
        coords: { x: 12, y: 8 },
        facing: "down",
      },
      warps: [],
      hotspots: [
        {
          id: "sign-1",
          type: "sign",
          label: "Sign",
          coords: { x: 12, y: 8 },
          visible: true,
          interactable: true,
          token: "S",
        },
      ],
    };

    const text = renderFrameToCompactText(payload);

    expect(text).toContain("S Sign here");
  });

  it("renders hotspot guidance from the nearest approach tile instead of the object tile", () => {
    const payload = baseSnapshot();
    payload.map = {
      map: "PlayersHouse2F",
      map_id: "24:7",
      player: {
        coords: { x: 3, y: 3 },
        facing: "down",
      },
      warps: [],
      hotspots: [
        {
          id: "pc-1",
          type: "utility",
          label: "PC",
          coords: { x: 5, y: 3 },
          visible: true,
          interactable: true,
          approach_tiles: [
            {
              coords: { x: 3, y: 3 },
              facing: "right",
            },
            {
              coords: { x: 5, y: 5 },
              facing: "up",
            },
          ],
          token: "P",
        },
      ],
    };

    const text = renderFrameToCompactText(payload);

    expect(text).toContain("P PC here face right");
    expect(text).not.toContain("P PC (2E)");
  });

  it("shows the nearest stand tile and facing when not already in position", () => {
    const payload = baseSnapshot();
    payload.map = {
      map: "PlayersHouse2F",
      map_id: "24:7",
      player: {
        coords: { x: 3, y: 3 },
        facing: "down",
      },
      warps: [],
      hotspots: [
        {
          id: "sign-1",
          type: "sign",
          label: "Sign",
          coords: { x: 7, y: 3 },
          visible: true,
          interactable: true,
          approach_tiles: [
            {
              coords: { x: 5, y: 3 },
              facing: "right",
            },
          ],
          token: "S",
        },
      ],
    };

    const text = renderFrameToCompactText(payload);

    expect(text).toContain("S Sign (2E) face right");
    expect(text).not.toContain("S Sign (4E)");
  });

  it("scales hotspot offsets by coord stride so directions match move counts", () => {
    const payload = baseSnapshot();
    payload.map = {
      map: "NewBarkTown",
      map_id: "24:4",
      coord_stride: 2,
      player: {
        coords: { x: 27, y: 11 },
        facing: "down",
      },
      warps: [],
      hotspots: [
        {
          id: "warp-1",
          type: "warp",
          label: "Warp: Elms Lab",
          coords: { x: 13, y: 7 },
          visible: true,
          interactable: true,
          token: "D",
        },
      ],
    };

    const text = renderFrameToCompactText(payload);

    expect(text).toContain("D Warp: Elms Lab (2N 7W)");
    expect(text).not.toContain("D Warp: Elms Lab (4N 14W)");
  });

  it("prioritizes nearby warp hotspots over distant map-order signs", () => {
    const payload = baseSnapshot();
    payload.map = {
      map: "PlayersHouse1F",
      map_id: "24:6",
      coord_stride: 2,
      player: {
        coords: { x: 17, y: 15 },
        facing: "down",
      },
      warps: [],
      hotspots: [
        {
          id: "npc-1",
          type: "npc",
          label: "NPC",
          coords: { x: 15, y: 9 },
          visible: true,
          interactable: true,
          token: "N",
          approach_tiles: [{ coords: { x: 17, y: 9 }, facing: "left" }],
        },
        {
          id: "sign-1",
          type: "sign",
          label: "Sign",
          coords: { x: 1, y: 3 },
          visible: true,
          interactable: true,
          token: "S",
          approach_tiles: [{ coords: { x: 1, y: 5 }, facing: "up" }],
        },
        {
          id: "sign-2",
          type: "sign",
          label: "Sign",
          coords: { x: 3, y: 3 },
          visible: true,
          interactable: true,
          token: "S",
          approach_tiles: [{ coords: { x: 3, y: 5 }, facing: "up" }],
        },
        {
          id: "sign-3",
          type: "sign",
          label: "Sign",
          coords: { x: 5, y: 3 },
          visible: true,
          interactable: true,
          token: "S",
          approach_tiles: [{ coords: { x: 5, y: 5 }, facing: "up" }],
        },
        {
          id: "sign-4",
          type: "sign",
          label: "Sign",
          coords: { x: 9, y: 3 },
          visible: true,
          interactable: true,
          token: "S",
          approach_tiles: [{ coords: { x: 9, y: 5 }, facing: "up" }],
        },
        {
          id: "warp-1",
          type: "warp",
          label: "Warp: New Bark Town",
          coords: { x: 13, y: 15 },
          visible: true,
          interactable: true,
          token: "D",
        },
        {
          id: "warp-2",
          type: "warp",
          label: "Warp: New Bark Town",
          coords: { x: 15, y: 15 },
          visible: true,
          interactable: true,
          token: "D",
        },
      ],
    };

    const text = renderFrameToCompactText(payload);

    expect(text).toContain("D Warp: New Bark Town (1W)");
    expect(text).toContain("D Warp: New Bark Town (2W)");
    expect(text.indexOf("D Warp: New Bark Town (1W)")).toBeLessThan(text.indexOf("S Sign (5N 8W) face up"));
  });

  it("keeps Pokemon Center healer guidance visible in crowded rooms", () => {
    const payload = baseSnapshot();
    payload.map = {
      map: "GoldenrodPokecenter1F",
      map_id: "10:14",
      coord_stride: 2,
      player: {
        coords: { x: 19, y: 5 },
        facing: "up",
      },
      warps: [],
      hotspots: [
        ...Array.from({ length: 5 }, (_, index) => ({
          id: `warp-${index + 1}`,
          type: "warp" as const,
          label: `Warp ${index + 1}`,
          coords: { x: 1 + index * 2, y: 13 },
          visible: true,
          interactable: true,
          token: "D",
        })),
        ...Array.from({ length: 4 }, (_, index) => ({
          id: `npc-${index + 1}`,
          type: "npc" as const,
          label: "NPC",
          coords: { x: 13 + index * 2, y: 3 },
          visible: true,
          interactable: true,
          token: "N",
        })),
        {
          id: "npc-nurse",
          type: "heal",
          label: "Healer",
          coords: { x: 7, y: 3 },
          visible: true,
          interactable: true,
          token: "H",
          approach_tiles: [{ coords: { x: 7, y: 7 }, facing: "up" }],
        },
      ],
    };

    const text = renderFrameToCompactText(payload);

    expect(text).toContain("H Healer");
    expect(text).toContain("face up");
  });

  it("does not render non-interactable route triggers in agent-facing hotspot text", () => {
    const payload = baseSnapshot();
    payload.map = {
      map: "PlayersHouse1F",
      map_id: "24:6",
      coord_stride: 2,
      player: {
        coords: { x: 19, y: 11 },
        facing: "right",
      },
      warps: [],
      hotspots: [
        {
          id: "coord-1",
          type: "trigger",
          label: "Route trigger",
          coords: { x: 17, y: 9 },
          visible: true,
          interactable: false,
          token: "!",
        },
        {
          id: "warp-1",
          type: "warp",
          label: "Warp: New Bark Town",
          coords: { x: 13, y: 15 },
          visible: true,
          interactable: true,
          token: "D",
        },
      ],
    };

    const text = renderFrameToCompactText(payload);

    expect(text).toContain("D Warp: New Bark Town (2S 3W)");
    expect(text).not.toContain("Route trigger");
  });
});
