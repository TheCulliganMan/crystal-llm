// ASM mapping: pokecrystal_disassembly/engine/overworld/map_sign.asm (Map name sign rendering).
import fs from "fs";
import path from "path";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { POKEGEAR_LANDMARKS } from "@pokecrystal/assets/content/data/pokegear-landmarks";
import { LANDMARK_SPECIAL } from "@pokecrystal/core/core/constants";
import { getWorldMapLocation } from "@pokecrystal/core/core/home";
import { GameState } from "@pokecrystal/core/core/state";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import { TextRenderer } from "@pokecrystal/core/ui/text/text-renderer";

const TIME_OF_DAY_ALIASES: Record<string, string> = {
  morn: "morn",
  morning: "morn",
  day: "day",
  nite: "nite",
  night: "nite",
  dark: "dark",
  indoor: "indoor",
};
const BG_TIME_SECTIONS = new Set(["morn", "day", "nite", "dark", "indoor"]);
const MAP_SIGN_PALETTE_INDEX = 0x07;
const DEFAULT_TIME_OF_DAY = "day";

const SCREEN_TILE_WIDTH = 20;
const SCREEN_TILE_HEIGHT = 18;

type Surface = InstanceType<typeof gameEngine.Surface>;

function load_bg_tiles_palettes(): Record<string, Array<Array<[number, number, number]>>> {
  const palette_path = getAssetPath("gfx", "tilesets", "bg_tiles.pal");
  const sections: Record<string, Array<Array<[number, number, number]>>> = {};
  BG_TIME_SECTIONS.forEach((key) => {
    sections[key] = [];
  });
  let current_section: string | null = null;
  const content = fs.readFileSync(palette_path, "utf-8");
  for (const raw_line of content.split("\n")) {
    const line = raw_line.trim();
    if (!line) {
      continue;
    }
    if (line.startsWith(";")) {
      const label = line.slice(1).trim().toLowerCase();
      const normalized = label.split(" ")[0] ?? "";
      current_section = BG_TIME_SECTIONS.has(normalized) ? normalized : null;
      continue;
    }
    if (!current_section || !line.toUpperCase().startsWith("RGB")) {
      continue;
    }
    const rgb_part = line.split("RGB", 2)[1] ?? "";
    const rgb_values = rgb_part
      .replace(";", ",")
      .split(",")
      .map((token) => token.trim())
      .filter(Boolean);
    const colors: Array<[number, number, number]> = [];
    for (let idx = 0; idx < 12; idx += 3) {
      const r = Number(rgb_values[idx]);
      const g = Number(rgb_values[idx + 1]);
      const b = Number(rgb_values[idx + 2]);
      if (Number.isNaN(r) || Number.isNaN(g) || Number.isNaN(b)) {
        break;
      }
      colors.push([r * 8, g * 8, b * 8]);
    }
    if (colors.length === 4) {
      sections[current_section].push(colors);
    }
  }

  for (const key of BG_TIME_SECTIONS) {
    const count = sections[key].length;
    if (count !== 8) {
      throw new Error(
        `Palette section '${key}' in ${palette_path} produced ${count} entries; expected 8.`
      );
    }
  }

  return sections;
}

const BG_TILES_PALETTES = load_bg_tiles_palettes();

export class MapNameSignController {
  public static readonly SIGN_TILE_WIDTH = SCREEN_TILE_WIDTH;
  public static readonly SIGN_TILE_HEIGHT = 4;
  public static readonly SIGN_DURATION_FRAMES = 60;
  public static readonly WINDOW_Y_PIXELS = (SCREEN_TILE_HEIGHT - MapNameSignController.SIGN_TILE_HEIGHT) * TILE_SIZE;
  public static readonly SHOWN_FLAG = 0x01;

  private static readonly NATIONAL_PARK_GATES = new Set([
    "Route35NationalParkGate",
    "Route36NationalParkGate",
  ]);
  private static readonly SPECIAL_LANDMARK_CONSTANTS = new Set([
    "LANDMARK_RADIO_TOWER",
    "LANDMARK_LAV_RADIO_TOWER",
    "LANDMARK_UNDERGROUND_PATH",
    "LANDMARK_INDIGO_PLATEAU",
    "LANDMARK_POWER_PLANT",
  ]);
  private static readonly SENTINEL_LANDMARK = -1;

  private game_state: GameState;
  private text_renderer: TextRenderer;
  private tiles: Surface[];
  private frame_templates: Map<string, Surface> = new Map();
  private tinted_tiles_cache: Map<string, Surface[]> = new Map();
  private active_surface: Surface | null = null;
  private timer: number = 0;
  private landmark_id_by_constant: Map<string, number> = new Map();
  private landmark_name_by_id: Map<number, string> = new Map();
  private special_landmarks: Set<number> = new Set();

  constructor(game_state: GameState, tiles: Surface[], text_renderer: TextRenderer) {
    this.game_state = game_state;
    this.text_renderer = text_renderer;
    this.tiles = tiles;
    for (const entry of POKEGEAR_LANDMARKS) {
      this.landmark_id_by_constant.set(entry.constant, entry.id);
      this.landmark_name_by_id.set(entry.id, entry.name);
    }
    for (const name of MapNameSignController.SPECIAL_LANDMARK_CONSTANTS) {
      const id = this.landmark_id_by_constant.get(name);
      if (id !== undefined) {
        this.special_landmarks.add(id);
      }
    }
  }

  public static async create(game_state: GameState): Promise<MapNameSignController> {
    const text_renderer = new TextRenderer();
    await text_renderer.load();
    const tiles = await MapNameSignController.load_tileset();
    return new MapNameSignController(game_state, tiles, text_renderer);
  }

  public getTiles(): ReadonlyArray<Surface> {
    return this.tiles;
  }

  public on_map_loaded(map_name: string): void {
    const wram = this.game_state.wram;
    let landmark_id = getWorldMapLocation(wram.wMapGroup, wram.wMapNumber);
    landmark_id = this.override_for_gate(map_name, landmark_id);
    wram.wCurLandmark = landmark_id;

    const flags = wram.wMapNameSignFlags ?? 0;
    if (flags & MapNameSignController.SHOWN_FLAG) {
      wram.wMapNameSignFlags = flags & ~MapNameSignController.SHOWN_FLAG;
      wram.wPrevLandmark = landmark_id;
      this.hide();
      return;
    }

    const previous = wram.wPrevLandmark;
    wram.wPrevLandmark = landmark_id;
    const moved_within_landmark = previous === landmark_id || previous === LANDMARK_SPECIAL;
    if (moved_within_landmark || this.should_skip_landmark(landmark_id)) {
      this.hide();
      return;
    }

    const label = this.landmark_name_by_id.get(landmark_id);
    if (!label) {
      this.hide();
      return;
    }

    this.timer = MapNameSignController.SIGN_DURATION_FRAMES;
    wram.wLandmarkSignTimer = MapNameSignController.SIGN_DURATION_FRAMES;
    this.active_surface = this.compose_surface(label);
  }

  public update(): void {
    if (this.timer <= 0) {
      this.hide();
      return;
    }
    this.timer = Math.max(0, this.timer - 1);
    this.game_state.wram.wLandmarkSignTimer = this.timer;
    if (this.timer === 0) {
      this.hide();
    }
  }

  public draw(target: Surface): void {
    if (!this.active_surface || this.timer <= 0) {
      return;
    }
    target.blit(this.active_surface, [0, MapNameSignController.WINDOW_Y_PIXELS]);
  }

  private hide(): void {
    this.timer = 0;
    this.game_state.wram.wLandmarkSignTimer = 0;
    this.active_surface = null;
  }

  private override_for_gate(map_name: string, landmark_id: number): number {
    if (MapNameSignController.NATIONAL_PARK_GATES.has(map_name)) {
      return MapNameSignController.SENTINEL_LANDMARK;
    }
    return landmark_id;
  }

  private should_skip_landmark(landmark_id: number): boolean {
    if (landmark_id === MapNameSignController.SENTINEL_LANDMARK || landmark_id === LANDMARK_SPECIAL) {
      return true;
    }
    return this.special_landmarks.has(landmark_id);
  }

  private compose_surface(text: string): Surface {
    const palette_key = this.current_palette_key();
    const frame = this.frame_template_for_palette(palette_key);
    const surface = frame.copy();
    const glyph_count = this.measure_tiles(text);
    const padding_tiles = Math.max(0, Math.trunc((MapNameSignController.SIGN_TILE_WIDTH - glyph_count) / 2));
    const x_px = padding_tiles * TILE_SIZE;
    const y_px = 2 * TILE_SIZE;
    this.text_renderer.drawText(surface, text, x_px, y_px, [0, 0, 0]);
    return surface;
  }

  private measure_tiles(text: string): number {
    const cleaned = Array.from(split_control_sequences(text)).join("");
    return cleaned.length;
  }

  private current_palette_key(): string {
    const label = this.game_state.wram.time_of_day;
    return normalize_time_of_day(label);
  }

  private frame_template_for_palette(palette_key: string): Surface {
    const cached = this.frame_templates.get(palette_key);
    if (cached) {
      return cached;
    }
    const tiles = this.tinted_tiles_for_palette(palette_key);
    const frame = this.build_frame(tiles);
    this.frame_templates.set(palette_key, frame);
    return frame;
  }

  private tinted_tiles_for_palette(palette_key: string): Surface[] {
    const cached = this.tinted_tiles_cache.get(palette_key);
    if (cached) {
      return cached;
    }
    const palette_section = BG_TILES_PALETTES[palette_key] ?? BG_TILES_PALETTES[DEFAULT_TIME_OF_DAY];
    const palette = palette_section[MAP_SIGN_PALETTE_INDEX];
    const tinted_tiles = this.tiles.map((tile) => colorize_map_sign_tile(tile, palette));
    this.tinted_tiles_cache.set(palette_key, tinted_tiles);
    return tinted_tiles;
  }

  private build_frame(tiles: Surface[]): Surface {
    const width_px = MapNameSignController.SIGN_TILE_WIDTH * TILE_SIZE;
    const height_px = MapNameSignController.SIGN_TILE_HEIGHT * TILE_SIZE;
    const surface = new gameEngine.Surface(width_px, height_px);
    this.blit_tile(surface, tiles, 0, 0, 1);
    this.fill_gradient(surface, tiles, 1, 0, 2);
    this.blit_tile(surface, tiles, width_px - TILE_SIZE, 0, 4);

    let row_y = TILE_SIZE;
    this.blit_tile(surface, tiles, 0, row_y, 5);
    this.fill_middle(surface, tiles, row_y);
    this.blit_tile(surface, tiles, width_px - TILE_SIZE, row_y, 11);

    row_y += TILE_SIZE;
    this.blit_tile(surface, tiles, 0, row_y, 6);
    this.fill_middle(surface, tiles, row_y);
    this.blit_tile(surface, tiles, width_px - TILE_SIZE, row_y, 12);

    row_y += TILE_SIZE;
    this.blit_tile(surface, tiles, 0, row_y, 7);
    this.fill_gradient(surface, tiles, 1, row_y, 8);
    this.blit_tile(surface, tiles, width_px - TILE_SIZE, row_y, 10);
    return surface;
  }

  private fill_gradient(surface: Surface, tiles: Surface[], start_column: number, y_px: number, base_index: number): void {
    let x_px = start_column * TILE_SIZE;
    const interior_tiles = MapNameSignController.SIGN_TILE_WIDTH - 2;
    const high_tile = tiles[base_index + 1];
    const low_tile = tiles[base_index];
    const pattern = [high_tile, high_tile, low_tile, low_tile];
    for (let idx = 0; idx < interior_tiles; idx += 1) {
      const tile = pattern[idx % pattern.length];
      surface.blit(tile, [x_px, y_px]);
      x_px += TILE_SIZE;
    }
  }

  private fill_middle(surface: Surface, tiles: Surface[], y_px: number): void {
    let x_px = TILE_SIZE;
    const interior_tiles = MapNameSignController.SIGN_TILE_WIDTH - 2;
    const filler = tiles[13];
    for (let idx = 0; idx < interior_tiles; idx += 1) {
      surface.blit(filler, [x_px, y_px]);
      x_px += TILE_SIZE;
    }
  }

  private blit_tile(surface: Surface, tiles: Surface[], x: number, y: number, tile_index: number): void {
    const tile = tiles[tile_index];
    surface.blit(tile, [x, y]);
  }

  private static async load_tileset(): Promise<Surface[]> {
    const frame_path = getAssetPath("gfx", "frames", "map_entry_sign.png");
    if (!fs.existsSync(frame_path)) {
      throw new Error(`Missing map sign tilesheet at ${frame_path}`);
    }
    const source = await gameEngine.image.load(frame_path);
    const [width, height] = source.get_size();
    const expected: [number, number] = [56, 16];
    if (width !== expected[0] || height !== expected[1]) {
      throw new Error(
        `map_entry_sign.png must be 56x16 pixels to match ASM tile order; found ${width}x${height}`
      );
    }
    const tiles: Surface[] = [];
    for (let y = 0; y < height; y += TILE_SIZE) {
      for (let x = 0; x < width; x += TILE_SIZE) {
        const tile_rect = new gameEngine.Rect(x, y, TILE_SIZE, TILE_SIZE);
        const tile = source.subsurface(tile_rect).copy();
        tiles.push(tile);
      }
    }
    return tiles;
  }
}

function* split_control_sequences(text: string): IterableIterator<string> {
  const fragment: string[] = [];
  let in_control = false;
  for (const char of text) {
    if (char === "<") {
      if (fragment.length) {
        yield fragment.join("");
        fragment.length = 0;
      }
      in_control = true;
      continue;
    }
    if (char === ">" && in_control) {
      in_control = false;
      continue;
    }
    if (!in_control) {
      fragment.push(char);
    }
  }
  if (fragment.length) {
    yield fragment.join("");
  }
}

function normalize_time_of_day(label?: string | null): string {
  if (!label) {
    return DEFAULT_TIME_OF_DAY;
  }
  const key = label.trim().toLowerCase();
  return TIME_OF_DAY_ALIASES[key] ?? DEFAULT_TIME_OF_DAY;
}

function colorize_map_sign_tile(source: Surface, palette: Array<[number, number, number]>): Surface {
  const tinted = new gameEngine.Surface(source.get_width(), source.get_height());
  const width = source.get_width();
  const height = source.get_height();
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const [r, g, b, a] = source.get_at([x, y]);
      if (a === 0) {
        continue;
      }
      const color = palette[greyscale_level([r, g, b, a])];
      tinted.set_at([x, y], [color[0], color[1], color[2], 255]);
    }
  }
  return tinted;
}

function greyscale_level(pixel: [number, number, number, number]): number {
  const value = Math.trunc((pixel[0] + pixel[1] + pixel[2]) / 3);
  if (value >= 213) {
    return 0;
  }
  if (value >= 160) {
    return 1;
  }
  if (value >= 96) {
    return 2;
  }
  return 3;
}
