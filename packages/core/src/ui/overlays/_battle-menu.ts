// ASM mapping: engine/battle/menu.asm (BattleMenuHeader, LoadBattleMenu, InterpretBattleMenu).
export const SCREEN_WIDTH = 20;
export const SCREEN_HEIGHT = 18;

export enum BattleMenu {
  MAIN = 'MAIN',
  FIGHT = 'FIGHT',
  POKEMON = 'POKEMON',
  PACK = 'PACK',
}

export enum MenuDirection {
  UP = 'UP',
  DOWN = 'DOWN',
  LEFT = 'LEFT',
  RIGHT = 'RIGHT',
}

export interface MenuCoords {
  left: number;
  top: number;
  right: number;
  bottom: number;
  width: number;
  height: number;
}

export interface BattleMenuHeader {
  coords: MenuCoords;
  rows: number;
  cols: number;
  spacing: number;
  row_spacing: number;
  labels: readonly string[];
  default_option: number;
  disable_b: boolean;
  option_count: number;
  clamp_cursor: (cursor: number) => number;
}

const buildMenuCoords = (left: number, top: number, right: number, bottom: number): MenuCoords => {
  const width = Math.max(0, right - left + 1);
  const height = Math.max(0, bottom - top + 1);
  return { left, top, right, bottom, width, height };
};

const buildHeader = (config: {
  coords: MenuCoords;
  rows: number;
  cols: number;
  spacing: number;
  row_spacing: number;
  labels: readonly string[];
  default_option?: number;
  disable_b?: boolean;
}): BattleMenuHeader => {
  const labels = config.labels.map((label) => label.toUpperCase());
  const optionCount = labels.length;
  return {
    coords: config.coords,
    rows: config.rows,
    cols: config.cols,
    spacing: config.spacing,
    row_spacing: config.row_spacing,
    labels,
    default_option: config.default_option ?? 0,
    disable_b: config.disable_b ?? true,
    option_count: optionCount,
    clamp_cursor: (cursor: number): number => {
      if (!labels.length) {
        return 0;
      }
      return Math.max(0, Math.min(cursor, optionCount - 1));
    },
  };
};

const gridPosition = (header: BattleMenuHeader, cursor: number): [number, number] => {
  const cols = Math.max(1, header.cols);
  const clamped = header.clamp_cursor(cursor);
  return [Math.floor(clamped / cols), clamped % cols];
};

export const cursorFromGrid = (header: BattleMenuHeader, row: number, col: number): number => {
  const cols = Math.max(1, header.cols);
  const cursor = row * cols + col;
  return header.clamp_cursor(cursor);
};

export const moveCursor = (
  header: BattleMenuHeader,
  cursor: number,
  direction: MenuDirection,
): number => {
  const rows = Math.max(1, header.rows);
  const cols = Math.max(1, header.cols);
  const [rowIndex, colIndex] = gridPosition(header, cursor);
  let row = rowIndex;
  let col = colIndex;
  if (direction === MenuDirection.UP) {
    row = (row - 1 + rows) % rows;
  } else if (direction === MenuDirection.DOWN) {
    row = (row + 1) % rows;
  } else if (direction === MenuDirection.LEFT) {
    col = (col - 1 + cols) % cols;
  } else if (direction === MenuDirection.RIGHT) {
    col = (col + 1) % cols;
  }
  return cursorFromGrid(header, row, col);
};

export const tileCoordsForOption = (header: BattleMenuHeader, cursor: number): [number, number] => {
  const [row, col] = gridPosition(header, cursor);
  const x = header.coords.left + 1 + col * header.spacing;
  const y = header.coords.top + 1 + row * header.row_spacing;
  return [x, y];
};

export const tile_coords_for_option = tileCoordsForOption;

export const layoutMatchesHeader = (layout: {
  menu_window?: { tile_x: number; tile_y: number; width_tiles: number; height_tiles: number } | null;
  main_menu_column_spacing: number;
  main_menu_row_spacing: number;
}, header: BattleMenuHeader): boolean => {
  const window = layout.menu_window;
  if (!window) {
    return false;
  }
  if (window.tile_x !== header.coords.left || window.tile_y !== header.coords.top) {
    return false;
  }
  if (window.width_tiles !== header.coords.width || window.height_tiles !== header.coords.height) {
    return false;
  }
  return (
    layout.main_menu_column_spacing === header.spacing &&
    layout.main_menu_row_spacing === header.row_spacing
  );
};

export const layout_matches_header = layoutMatchesHeader;

export const BATTLE_MENU_HEADER = buildHeader({
  coords: buildMenuCoords(8, 12, SCREEN_WIDTH - 1, SCREEN_HEIGHT - 1),
  rows: 2,
  cols: 2,
  spacing: 6,
  row_spacing: 2,
  labels: ['FIGHT', '<PKMN>', 'PACK', 'RUN'],
});

export const YES_NO_MENU_HEADER = buildHeader({
  coords: buildMenuCoords(10, 5, 15, 9),
  rows: 2,
  cols: 1,
  spacing: 0,
  row_spacing: 2,
  labels: ['YES', 'NO'],
});

export const SAFARI_MENU_HEADER = buildHeader({
  coords: buildMenuCoords(0, 12, SCREEN_WIDTH - 1, SCREEN_HEIGHT - 1),
  rows: 2,
  cols: 2,
  spacing: 11,
  row_spacing: 2,
  labels: ['SAFARI BALL\u00d7', 'THROW BAIT', 'THROW ROCK', 'RUN'],
});

export const BUG_CATCHING_MENU_HEADER = buildHeader({
  coords: buildMenuCoords(2, 12, SCREEN_WIDTH - 1, SCREEN_HEIGHT - 1),
  rows: 2,
  cols: 2,
  spacing: 12,
  row_spacing: 2,
  labels: ['FIGHT', '<PKMN>', 'PARKBALL\u00d7', 'RUN'],
});

export const validateMenuLabels = (labels: Iterable<string>): string[] => {
  const result: string[] = [];
  for (const label of labels) {
    result.push(String(label).toUpperCase());
  }
  return result;
};

export const loadBattleMenu = (storedCursor: number, header: BattleMenuHeader = BATTLE_MENU_HEADER): number => {
  if (storedCursor < 0) {
    return header.clamp_cursor(header.default_option);
  }
  return header.clamp_cursor(storedCursor);
};

export const interpretBattleMenu = (
  cursor: number,
  header: BattleMenuHeader = BATTLE_MENU_HEADER,
  direction?: MenuDirection | null,
): number => {
  if (!direction) {
    return header.clamp_cursor(cursor);
  }
  return moveCursor(header, cursor, direction);
};
