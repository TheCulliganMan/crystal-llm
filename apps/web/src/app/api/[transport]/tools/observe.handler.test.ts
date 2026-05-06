const mockLoadSession = jest.fn();
const mockResolveSessionId = jest.fn(() => "observe-compact-map-test");

jest.mock("./common", () => ({
  MAX_ADVANCE_FRAMES: 8,
  getObserveSnapshotCache: jest.fn(() => undefined),
  invalidateObserveSnapshotCache: jest.fn(),
  loadSession: (...args: unknown[]) => mockLoadSession(...args),
  setObserveSnapshotCache: jest.fn(),
  resolveSessionId: (...args: unknown[]) => mockResolveSessionId(...args),
  withRequestIdentity: (_extra: unknown, fn: () => unknown) => fn(),
}));

import { observeHandler } from "./observe";

const createFakePngBase64 = (width = 160, height = 144): string => {
  const bytes = Buffer.alloc(24);
  Buffer.from("89504e470d0a1a0a", "hex").copy(bytes, 0);
  bytes.writeUInt32BE(13, 8);
  bytes.write("IHDR", 12, "ascii");
  bytes.writeUInt32BE(width, 16);
  bytes.writeUInt32BE(height, 20);
  return bytes.toString("base64");
};

describe("observeHandler compact map metadata", () => {
  beforeEach(() => {
    mockLoadSession.mockReset();
    mockResolveSessionId.mockClear();
  });

  it("preserves interactable starter poke balls without exposing event-tile triggers", async () => {
    const map = {
      map: "ElmsLab",
      map_id: "24:5",
      coord_stride: 2,
      player: { coords: { x: 13, y: 5 }, facing: "down" as const },
      warps: [],
      hotspots: [
        {
          id: "npc-3",
          type: "objective" as const,
          label: "Cyndaquil Poke Ball",
          coords: { x: 13, y: 7 },
          visible: true,
          interactable: true,
          token: "!",
        },
      ],
    };
    mockLoadSession.mockResolvedValue({
      getFrameCount: jest.fn(() => 42),
      observeText: jest.fn(() => "OVERWORLD\n00 . @v I"),
      playerContext: jest.fn().mockResolvedValue({
        map: "ElmsLab",
        coords: { x: 13, y: 5 },
        facing: "down",
      }),
      status: jest.fn().mockResolvedValue({
        mode: "overworld",
        map: "ElmsLab",
        coords: { x: 13, y: 5 },
        facing: "down",
        can_move: true,
      }),
      recentEvents: jest.fn().mockResolvedValue({
        total: 0,
        recap: "no_events",
        truncated: false,
        events: [],
      }),
      observePayload: jest.fn(() => ({
        viewport: ["OVERWORLD"],
        info: ["Pos: (13,5)"],
        menu: null,
        prompt: null,
        dialogue: null,
        titles: { viewport: "Overworld", info: "Info" },
        marker: null,
        action_log: [],
        script: {},
        tasks: [],
        map,
      })),
      mapInfo: jest.fn().mockResolvedValue(map),
    });

    const response = await observeHandler({ include_snapshot_text: false }, {});
    const payload = response.content[0]?.type === "text" ? JSON.parse(response.content[0].text ?? "{}") : null;

    expect(payload?.map?.hs).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          t: "objective",
          l: "Cyndaquil Poke Ball",
          tk: "!",
          i: 1,
        }),
      ])
    );
    expect(payload?.map?.hs).not.toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          t: "trigger",
          l: "Route trigger",
        }),
      ])
    );
  });

  it("returns the same PNG payload produced by the web session tilemap image renderer", async () => {
    const imageData = createFakePngBase64(320, 288);
    const observeTilemapImage = jest.fn(async () => ({
      data: imageData,
      width: 320,
      height: 288,
    }));
    const map = {
      map: "NewBarkTown",
      map_id: "24:4",
      coord_stride: 1,
      player: { coords: { x: 4, y: 7 }, facing: "down" as const },
      warps: [],
      hotspots: [],
    };
    mockLoadSession.mockResolvedValue({
      getFrameCount: jest.fn(() => 43),
      observeText: jest.fn(() => "OVERWORLD\n00 . @v ."),
      playerContext: jest.fn().mockResolvedValue({
        map: "NewBarkTown",
        coords: { x: 4, y: 7 },
        facing: "down",
      }),
      status: jest.fn().mockResolvedValue({
        mode: "overworld",
        map: "NewBarkTown",
        coords: { x: 4, y: 7 },
        facing: "down",
        can_move: true,
      }),
      recentEvents: jest.fn().mockResolvedValue({
        total: 0,
        recap: "no_events",
        truncated: false,
        events: [],
      }),
      observePayload: jest.fn(() => ({
        viewport: ["OVERWORLD"],
        info: ["Pos: (4,7)"],
        menu: null,
        prompt: null,
        dialogue: null,
        titles: { viewport: "Overworld", info: "Info" },
        marker: null,
        action_log: [],
        script: {},
        tasks: [],
        map,
      })),
      mapInfo: jest.fn().mockResolvedValue(map),
      observeTilemapImage,
    });

    const response = await observeHandler(
      { include_snapshot_text: false, include_image: true, image_scale: 2 },
      {},
    );
    const image = response.content.find((entry) => entry.type === "image");

    expect(observeTilemapImage).toHaveBeenCalledWith({ scale: 2 });
    expect(image).toEqual({
      type: "image",
      data: imageData,
      mimeType: "image/png",
    });
  });

  it("includes explicit direction and warp guidance in compact observe JSON", async () => {
    const map = {
      map: "PlayersHouse2F",
      map_id: "24:1",
      coord_stride: 1,
      player: { coords: { x: 16, y: 9 }, facing: "right" as const },
      warps: [],
      hotspots: [],
    };
    mockLoadSession.mockResolvedValue({
      getFrameCount: jest.fn(() => 44),
      observeText: jest.fn(() => [
        "OVERWORLD",
        "15 16 17 18",
        "08 . . . .",
        "09 . @> . DP",
        "10 . . . x",
      ].join("\n")),
      playerContext: jest.fn().mockResolvedValue({
        map: "PlayersHouse2F",
        coords: { x: 16, y: 9 },
        facing: "right",
      }),
      status: jest.fn().mockResolvedValue({
        mode: "overworld",
        map: "PlayersHouse2F",
        coords: { x: 16, y: 9 },
        facing: "right",
        can_move: true,
      }),
      recentEvents: jest.fn().mockResolvedValue({
        total: 0,
        recap: "no_events",
        truncated: false,
        events: [],
      }),
      observePayload: jest.fn(() => ({
        viewport: ["OVERWORLD"],
        info: ["Pos: (16,9)"],
        menu: null,
        prompt: null,
        dialogue: null,
        titles: { viewport: "Overworld", info: "Info" },
        marker: null,
        action_log: [],
        script: {},
        tasks: [],
        map,
      })),
      mapInfo: jest.fn().mockResolvedValue(map),
    });

    const response = await observeHandler({ include_snapshot_text: false }, {});
    const payload = response.content[0]?.type === "text" ? JSON.parse(response.content[0].text ?? "{}") : null;

    expect(payload?.dir?.move).toEqual({
      up: [0, -1],
      down: [0, 1],
      left: [-1, 0],
      right: [1, 0],
    });
    expect(payload?.ow?.w?.[0]).toEqual(
      expect.objectContaining({
        at: [18, 9],
        ap: [18, 8],
        go: "down",
        stand: [18, 8],
        move: "down",
        note: "stand at 18,8; move down to enter",
      })
    );
  });
});
