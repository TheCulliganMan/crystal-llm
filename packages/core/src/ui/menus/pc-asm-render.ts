// ASM mapping: pokecrystal_disassembly/engine/pokemon/bills_pc_top.asm top menu prompt.
import type { BaseUI } from "../base-ui";
import type { ScreenUI } from "../screens/screen-types";
import { renderFontText } from "../text/render-font";

const TILE_SIZE = 8;

export const BILLS_PC_TOP_MENU_WINDOW = {
  x: 0,
  y: 0,
  width: 20,
  height: 18,
} as const;

const BILLS_PC_TOP_MENU_TEXT_X = TILE_SIZE;
const BILLS_PC_TOP_MENU_TEXT_Y = TILE_SIZE;
const BILLS_PC_TOP_MENU_ROW_HEIGHT = TILE_SIZE;
const BILLS_PC_TOP_MENU_FILL: [number, number, number] = [255, 255, 255];

export type BillsPcTopMenuEntry = string | readonly [label: string, action: string];

const labelForEntry = (entry: BillsPcTopMenuEntry): string => {
  if (typeof entry === "string") {
    return entry;
  }
  return entry[0];
};

export const billsPcTopMenuLabels = (
  entries: ReadonlyArray<BillsPcTopMenuEntry>
): string[] => entries.map(labelForEntry);

export const renderBillsPcTopMenu = (
  ui: BaseUI & { renderSnapshot?: ScreenUI["renderSnapshot"] },
  entries: ReadonlyArray<BillsPcTopMenuEntry>,
  { selectedIndex = 0 }: { selectedIndex?: number } = {}
): void => {
  const font = ui.font;
  if (!font) {
    throw new Error("Bill PC top menu render requires a BaseUI font renderer.");
  }

  ui.clearScreen(BILLS_PC_TOP_MENU_FILL);
  ui.drawWindow(
    ui.screen,
    BILLS_PC_TOP_MENU_WINDOW.x * TILE_SIZE,
    BILLS_PC_TOP_MENU_WINDOW.y * TILE_SIZE,
    BILLS_PC_TOP_MENU_WINDOW.width,
    BILLS_PC_TOP_MENU_WINDOW.height,
    { fill: BILLS_PC_TOP_MENU_FILL }
  );

  billsPcTopMenuLabels(entries).forEach((label, index) => {
    const cursor = index === selectedIndex ? "▶" : " ";
    renderFontText(
      font,
      `${cursor}${label}`,
      BILLS_PC_TOP_MENU_TEXT_X,
      BILLS_PC_TOP_MENU_TEXT_Y + index * BILLS_PC_TOP_MENU_ROW_HEIGHT,
      ui.screen,
      { uppercase: true }
    );
  });
  ui.renderSnapshot?.(
    ["BILL'S PC"],
    ["D-Pad=Move A=Select B=Back"],
    "Bill's PC",
    "Legend",
    billsPcTopMenuLabels(entries).map((label, index) => `${index === selectedIndex ? "▶" : " "} ${label}`),
    null,
    null,
  );
};
