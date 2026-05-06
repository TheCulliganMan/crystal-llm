import type { TextSnapshotPayload } from "@/app/mcp/text-render";
import type { McpMapInfoSnapshot } from "@/app/mcp/map-info";
import type { McpStatusSnapshot } from "@/app/mcp/session";
import { __testables } from "./observe";

const buildFrame = (viewport: string[]): TextSnapshotPayload => ({
  viewport,
  info: ["Pos: (13,5)"],
  dialogue: null,
  prompt: null,
  menu: null,
  map: null,
  flow_state: null,
  titles: {
    viewport: "Overworld",
    info: "Info",
    dialogue: "Dialogue",
    prompt: "Prompt",
    menu: "Menu",
  },
});

const buildStatus = (overrides: Partial<McpStatusSnapshot> = {}): McpStatusSnapshot => ({
  mode: "overworld",
  menu: false,
  facing: "down",
  coords: { x: 13, y: 5 },
  map: "ElmsLab",
  ...overrides,
});

describe("buildVisibleScreen", () => {
  it("keeps Elm's Lab starter poke balls interactable without exposing event-tile triggers", () => {
    const compactMap = __testables.buildCompactMapSummary(
      {
        map: "ElmsLab",
        map_id: "24:5",
        coord_stride: 2,
        player: { coords: { x: 13, y: 5 }, facing: "down" },
        warps: [],
        hotspots: [
          {
            id: "npc-3",
            type: "objective",
            label: "Cyndaquil Poke Ball",
            coords: { x: 13, y: 7 },
            visible: true,
            interactable: true,
            token: "!",
          },
        ],
      },
      "overworld"
    );

    expect(compactMap?.hs).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          t: "objective",
          l: "Cyndaquil Poke Ball",
          tk: "!",
          i: 1,
        }),
      ])
    );
    expect(compactMap?.hs).not.toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          t: "trigger",
          l: "Route trigger",
        }),
      ])
    );
  });

  it("parses scrolled overworld windows with absolute player and warp coordinates", () => {
    const parsed = __testables.parseOverworldWindow([
      "OVERWORLD",
      "15 16 17 18",
      "08 . . . .",
      "09 . @> . DP",
    ].join("\n"));

    expect(parsed?.origin).toEqual({ x: 15, y: 8 });
    expect(parsed?.player).toEqual({ x: 16, y: 9, token: "@>" });
    expect(parsed?.warps[0]?.threshold).toEqual({ x: 18, y: 9 });
  });

  it("documents all four movement directions in screen-coordinate terms", () => {
    expect(__testables.DIRECTION_CONVENTION.move).toEqual({
      up: [0, -1],
      down: [0, 1],
      left: [-1, 0],
      right: [1, 0],
    });
    expect(__testables.DIRECTION_CONVENTION.coord).toBe("x+ right, x- left, y+ down, y- up");
  });

  it("does not report a tall item-ball sprite as ahead when the adjacent world tile is not actually interactable", () => {
    const frame = buildFrame([
      "OVERWORLD",
      "00 01 02 03 04 05 06 07 08 09",
      "02 . . . . . . @v . . .",
      "03 . . . . . . I I I #",
    ]);
    const status = buildStatus({
      interaction_tile: { x: 13, y: 9 },
    });
    const overworldWindow = {
      grid: [
        [".", ".", ".", ".", ".", ".", ".", ".", ".", "."],
        [".", ".", ".", ".", ".", ".", "@v", ".", ".", "."],
        [".", ".", ".", ".", ".", ".", "I", "I", "I", "#"],
      ],
      origin: { x: 7, y: 4 },
      player: { x: 13, y: 5, token: "@v" },
      warps: [],
    };
    const mapInfo: McpMapInfoSnapshot = {
      map: "ElmsLab",
      map_id: "24:5",
      coord_stride: 2,
      player: { coords: { x: 13, y: 5 }, facing: "down" },
      warps: [],
      hotspots: [
        {
          id: "starter-cyndaquil",
          type: "objective",
          label: "Cyndaquil Poke Ball",
          coords: { x: 13, y: 7 },
          visible: true,
          interactable: true,
          token: "!",
        },
      ],
    };

    const visible = __testables.buildVisibleScreen(frame, status, overworldWindow, mapInfo);

    expect(visible?.ahead).toBeUndefined();
  });

  it("uses the structured interaction-tile hotspot token even when the runtime has no explicit interaction_target", () => {
    const frame = buildFrame([
      "OVERWORLD",
      "00 01 02 03 04 05 06 07 08 09",
      "02 . . . . . . @v . . .",
      "03 . . . . . . I I I #",
    ]);
    const status = buildStatus({
      interaction_tile: { x: 13, y: 7 },
    });
    const overworldWindow = {
      grid: [
        [".", ".", ".", ".", ".", ".", ".", ".", ".", "."],
        [".", ".", ".", ".", ".", ".", "@v", ".", ".", "."],
        [".", ".", ".", ".", ".", ".", "I", "I", "I", "#"],
      ],
      origin: { x: 7, y: 4 },
      player: { x: 13, y: 5, token: "@v" },
      warps: [],
    };
    const mapInfo: McpMapInfoSnapshot = {
      map: "ElmsLab",
      map_id: "24:5",
      coord_stride: 2,
      player: { coords: { x: 13, y: 5 }, facing: "down" },
      warps: [],
      hotspots: [
        {
          id: "pc",
          type: "objective",
          label: "Elm Lab Console",
          coords: { x: 13, y: 7 },
          visible: true,
          interactable: true,
          token: "!",
        },
      ],
    };

    const visible = __testables.buildVisibleScreen(frame, status, overworldWindow, mapInfo);

    expect(visible?.ahead).toBe("!");
  });

  it("keeps name-entry cursor metadata in the visible payload", () => {
    const frame = buildFrame(["NAME ENTRY", "LAST INPUT: ArrowRight"]);
    frame.info = [
      "STATE: name_entry",
      "PROMPT: NAME",
      "NAME: (blank)",
      "LENGTH: 0/5",
      "CURSOR: row 0 col 1",
    ];

    const visible = __testables.buildVisibleScreen(frame, buildStatus({ mode: "name_entry" }), null);

    expect(visible?.viewport).toEqual(["NAME ENTRY", "LAST INPUT: ArrowRight"]);
    expect(visible?.info).toEqual(expect.arrayContaining(["CURSOR: row 0 col 1"]));
  });

  it("keeps all dialogue lines in the MCP-visible payload", () => {
    const frame = buildFrame(["BATTLE"]);
    frame.dialogue = [
      "line 1",
      "line 2",
      "line 3",
      "line 4",
      "line 5",
      "line 6",
      "line 7",
    ];

    const visible = __testables.buildVisibleScreen(frame, buildStatus({ mode: "battle" }), null);

    expect(visible?.dialogue).toEqual(frame.dialogue);
  });

  it("keeps a player row below the old twelve-line viewport cap", () => {
    const frame = buildFrame([
      "   00 01 02 03 04 05 06 07 08 09",
      "00 #  #  #  #  #  #  #  #  #  #",
      "01 #  #  #  #  #  #  #  #  #  #",
      "02 #  #  .  N> #  #  #  #  #  #",
      "03 #  #  .  S  #  #  D  #  #  #",
      "04 #  #  .  .  .  .  .  .  .  .",
      "05 #  #  .  .  .  .  .  .  .  .",
      "06 #  #  .  .  .  .  .  .  .  .",
      "07 #  #  .  .  .  .  .  .  .  .",
      "08 .  .  .  .  .  .  Nv .  S  .",
      "09 .  .  .  .  .  .  .  .  .  .",
      "10 #  #  #  #  #  #  .  .  .  .",
      "11 #  #  #  D  #  #  .  .  .  .",
      "12 .  .  .  .  .  .  .  .  .  .",
      "13 .  .  .  .  .  .  .  .  .  S",
      "14 #  #  .  .  .  .  .  .  .  .",
      "15 #  #  H  H  H  H  .  @v .  .",
      "16 #  #  #  #  #  #  #  #  #  #",
      "17 #  #  #  #  #  #  #  #  #  #",
    ]);
    frame.info = ["Pos: (7,15)"];

    const visible = __testables.buildVisibleScreen(frame, buildStatus({ coords: { x: 7, y: 15 } }), null);

    expect(visible?.viewport?.join("\n")).toContain("@v");
    expect(visible?.viewport?.map((line) => line.slice(0, 2))).toContain("15");
    expect(visible?.viewport?.length).toBeGreaterThan(12);
  });
});
