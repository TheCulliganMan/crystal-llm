import { Surface } from "@pokecrystal/core/ui/surface";
import { TileRegion } from "./pc-layout";
import { getPcCursorTile } from "./pc-wallpaper";
import {
  BILLS_PC_INSERT_CURSOR,
  BILLS_PC_SELECTION_CURSOR,
  BillsPCCursorOAM,
  BillsPCCursorView,
  BillsPCListView,
  OAM_XFLIP,
  OAM_YFLIP,
} from "./pc-views";

const sample = (surface: Surface, x: number, y: number): [number, number, number, number] =>
  surface.get_at([x, y]);

const expectOpaquePixelsAt = (
  screen: Surface,
  x: number,
  y: number,
  expected: Surface,
): void => {
  const transparent = sample(expected, 0, 0);
  for (let row = 0; row < 8; row += 1) {
    for (let col = 0; col < 8; col += 1) {
      const expectedPixel = sample(expected, col, row);
      if (expectedPixel.join(",") === transparent.join(",")) {
        continue;
      }
      expect(sample(screen, x + col, y + row)).toEqual(expectedPixel);
    }
  }
};

describe("BillsPCListView", () => {
  it("keeps the ASM cursor sprite tables byte-for-byte", () => {
    expect(BILLS_PC_SELECTION_CURSOR).toEqual([
      [10, 4, 0, 6, 0x00, 0],
      [11, 4, 0, 6, 0x00, 0],
      [12, 4, 0, 6, 0x00, 0],
      [13, 4, 0, 6, 0x00, 0],
      [14, 4, 0, 6, 0x00, 0],
      [15, 4, 0, 6, 0x00, 0],
      [16, 4, 0, 6, 0x00, 0],
      [17, 4, 0, 6, 0x00, 0],
      [18, 4, 0, 6, 0x00, 0],
      [18, 4, 7, 6, 0x00, 0],
      [10, 7, 0, 1, 0x00, OAM_YFLIP],
      [11, 7, 0, 1, 0x00, OAM_YFLIP],
      [12, 7, 0, 1, 0x00, OAM_YFLIP],
      [13, 7, 0, 1, 0x00, OAM_YFLIP],
      [14, 7, 0, 1, 0x00, OAM_YFLIP],
      [15, 7, 0, 1, 0x00, OAM_YFLIP],
      [16, 7, 0, 1, 0x00, OAM_YFLIP],
      [17, 7, 0, 1, 0x00, OAM_YFLIP],
      [18, 7, 0, 1, 0x00, OAM_YFLIP],
      [18, 7, 7, 1, 0x00, OAM_YFLIP],
      [9, 5, 6, 6, 0x01, 0],
      [9, 6, 6, 1, 0x01, OAM_YFLIP],
      [19, 5, 1, 6, 0x01, OAM_XFLIP],
      [19, 6, 1, 1, 0x01, OAM_XFLIP | OAM_YFLIP],
    ]);
    expect(BILLS_PC_INSERT_CURSOR).toEqual([
      [10, 4, 0, 7, 0x06, 0],
      [11, 5, 0, 3, 0x00, OAM_YFLIP],
      [12, 5, 0, 3, 0x00, OAM_YFLIP],
      [13, 5, 0, 3, 0x00, OAM_YFLIP],
      [14, 5, 0, 3, 0x00, OAM_YFLIP],
      [15, 5, 0, 3, 0x00, OAM_YFLIP],
      [16, 5, 0, 3, 0x00, OAM_YFLIP],
      [17, 5, 0, 3, 0x00, OAM_YFLIP],
      [18, 5, 0, 3, 0x00, OAM_YFLIP],
      [19, 4, 0, 7, 0x07, 0],
    ]);
  });

  it("emits the 24-sprite Crystal cursor OAM box at hardware-adjusted coordinates", () => {
    const oam = new BillsPCCursorOAM();

    oam.update(0, 5);

    expect(oam.entries).toHaveLength(24);
    expect(oam.entries[0]).toEqual({ x: 72, y: 22, tileId: 0x00, attributes: 0x00 });
    expect(oam.entries[9]).toEqual({ x: 143, y: 22, tileId: 0x00, attributes: 0x00 });
    expect(oam.entries[10]).toEqual({ x: 72, y: 41, tileId: 0x00, attributes: 0x40 });
    expect(oam.entries[23]).toEqual({ x: 145, y: 33, tileId: 0x01, attributes: 0x60 });

    oam.update(99, 5);
    expect(oam.entries[0].y).toBe(86);
  });

  it("emits the 10-sprite Crystal insert cursor OAM at hardware-adjusted coordinates", () => {
    const oam = new BillsPCCursorOAM();

    oam.update(2, 5, "insert");

    expect(oam.entries).toHaveLength(10);
    expect(oam.entries[0]).toEqual({ x: 72, y: 55, tileId: 0x06, attributes: 0x00 });
    expect(oam.entries[1]).toEqual({ x: 80, y: 59, tileId: 0x00, attributes: 0x40 });
    expect(oam.entries[9]).toEqual({ x: 144, y: 55, tileId: 0x07, attributes: 0x00 });
  });

  it("draws the asset-backed PC selection cursor instead of the synthetic blue highlight", () => {
    const region = new TileRegion(8, 2, 12, 12);
    const cursorView = new BillsPCCursorView(region, 16, 5);
    const screen = new Surface(160, 144);
    screen.fill([255, 255, 255, 255]);

    cursorView.draw(screen, 0);

    expectOpaquePixelsAt(screen, 72, 22, getPcCursorTile(0x00));
    expectOpaquePixelsAt(screen, 70, 30, getPcCursorTile(0x01));
    expect(sample(screen, 81, 33)).not.toEqual([96, 160, 248, 216]);
  });

  it("treats PC cursor color 0 as transparent like hardware OBJ rendering", () => {
    const region = new TileRegion(8, 2, 12, 12);
    const cursorView = new BillsPCCursorView(region, 16, 5);
    const screen = new Surface(160, 144);
    const underlay: [number, number, number, number] = [42, 99, 123, 255];
    screen.fill(underlay);

    cursorView.draw(screen, 0);

    expect(sample(screen, 72, 22)).toEqual(underlay);
    expect(sample(screen, 72, 29)).toEqual(sample(getPcCursorTile(0x00), 0, 7));
  });

  it("draws the asset-backed PC insert cursor tiles", () => {
    const region = new TileRegion(8, 2, 12, 12);
    const cursorView = new BillsPCCursorView(region, 16, 5);
    const screen = new Surface(160, 144);
    screen.fill([255, 255, 255, 255]);

    cursorView.draw(screen, 0, "insert");

    expectOpaquePixelsAt(screen, 72, 23, getPcCursorTile(0x06));
    expectOpaquePixelsAt(screen, 144, 23, getPcCursorTile(0x07));
  });

  it("leaves rows after CANCEL blank instead of filling them with placeholder dashes", () => {
    const region = new TileRegion(0, 0, 10, 7);
    const cursorView = new BillsPCCursorView(region, 16, 5);
    const view = new BillsPCListView(region, 5, "-----", "CANCEL", cursorView);
    const renderText = jest.fn();
    const ui = {
      font: { renderText },
      drawWindow: jest.fn(),
    };
    const screen = new Surface(160, 144);

    view.draw(
      ui,
      screen,
      [{ nickname: "ALPHA" }, { nickname: "BETA" }],
      0,
      0,
    );

    const labels = renderText.mock.calls.map((call) => call[0]);
    expect(labels).toContain("ALPHA");
    expect(labels).toContain("BETA");
    expect(labels).toContain("CANCEL");
    expect(labels.slice(-2)).toEqual(["", ""]);
    expect(labels).not.toContain("-----");
  });

  it("renders full PC nickname fields instead of six-character party-menu labels", () => {
    const region = new TileRegion(0, 0, 12, 7);
    const cursorView = new BillsPCCursorView(region, 16, 5);
    const view = new BillsPCListView(region, 5, "-----", "CANCEL", cursorView);
    const renderText = jest.fn();
    const ui = {
      font: { renderText },
      drawWindow: jest.fn(),
    };
    const screen = new Surface(160, 144);

    view.draw(ui, screen, [{ nickname: "ELECTRODE" }], 0, 0);

    const labels = renderText.mock.calls.map((call) => call[0]);
    expect(labels).toContain("ELECTRODE");
    expect(labels).not.toContain("ELECTR");
  });
});
