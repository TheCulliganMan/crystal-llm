const mockLoadSession = jest.fn();
const mockResolveSessionId = jest.fn(() => "route-render-test");

jest.mock("./common", () => ({
  loadSession: (...args: unknown[]) => mockLoadSession(...args),
  resolveSessionId: (...args: unknown[]) => mockResolveSessionId(...args),
  withRequestIdentity: (_extra: unknown, fn: () => unknown) => fn(),
}));

import { routeRenderHandler } from "./route_render";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";

describe("routeRenderHandler", () => {
  beforeEach(() => {
    mockLoadSession.mockReset();
    mockResolveSessionId.mockClear();
  });

  it("returns a structured unavailable response without fabricating an image", async () => {
    mockLoadSession.mockResolvedValue({
      routeRender: jest.fn().mockResolvedValue({
        available: false,
        reason: "route_render is only available in overworld mode; current mode is battle.",
        map: "Battle",
        map_id: "battle",
        legend: [],
        warps: [],
        hotspots: [],
      }),
    });

    const response = await routeRenderHandler({ include_image: true }, {});
    const payload = response.content[0]?.type === "text" ? JSON.parse(response.content[0].text) : null;

    expect(response.content).toHaveLength(1);
    expect(payload).toMatchObject({
      available: false,
      reason: expect.stringContaining("overworld mode"),
    });
  });

  it("returns text JSON and an annotated PNG when image output is requested", async () => {
    mockLoadSession.mockResolvedValue({
      routeRender: jest.fn().mockResolvedValue({
        available: true,
        map: "Route29",
        map_id: "24:3",
        coord_stride: 2,
        size: { width: 4, height: 4 },
        player: { coords: { x: 1, y: 1 }, facing: "right" },
        grid: {
          origin: { x: 0, y: 0 },
          rows: ["....", ".@D.", ".~#.", "...."],
        },
        legend: [{ token: "@", label: "Player" }],
        warps: [{
          index: 1,
          coords: { x: 2, y: 1 },
          target: { map_constant: "NEXT", map_name: "Next", warp_id: 1 },
        }],
        hotspots: [],
      }),
    });

    const response = await routeRenderHandler(
      { include_image: true, image_scale: 1, cell_size: 8, detail: "compact" },
      {}
    );
    const image = response.content.find((entry) => entry.type === "image");
    const payload = response.content[0]?.type === "text" ? JSON.parse(response.content[0].text) : null;

    expect(payload).toMatchObject({ available: true, map: "Route29" });
    expect(image).toMatchObject({ type: "image", mimeType: "image/png" });
    expect(Buffer.from(image?.type === "image" ? image.data : "", "base64").subarray(0, 8)).toEqual(
      Buffer.from("89504e470d0a1a0a", "hex")
    );
  });

  it("uses the high-fidelity tile renderer only when tile image style is requested", async () => {
    const snapshot = {
      available: true,
      map: "Route29",
      map_id: "24:3",
      coord_stride: 2,
      size: { width: 4, height: 4 },
      player: { coords: { x: 1, y: 1 }, facing: "right" },
      grid: {
        origin: { x: 0, y: 0 },
        rows: ["....", ".@D.", ".~#.", "...."],
      },
      legend: [{ token: "@", label: "Player" }],
      warps: [],
      hotspots: [],
    };
    const surface = new gameEngine.Surface(16, 16);
    surface.fill([16, 32, 64, 255]);
    const routeRenderImage = jest.fn().mockResolvedValue(surface);
    mockLoadSession.mockResolvedValue({
      routeRender: jest.fn().mockResolvedValue(snapshot),
      routeRenderImage,
    });

    const response = await routeRenderHandler(
      { include_image: true, image_style: "tiles", image_scale: 1, cell_size: 8 },
      {}
    );
    const image = response.content.find((entry) => entry.type === "image");

    expect(routeRenderImage).toHaveBeenCalledWith(snapshot, { cellSize: 8 });
    expect(image).toMatchObject({ type: "image", mimeType: "image/png" });
  });
});
