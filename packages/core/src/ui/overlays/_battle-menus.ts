import { moveDisplayName } from "@pokecrystal/assets/content/move-names";
import { Pokemon, LearnedMove } from "@pokecrystal/core/core/models";
import { PokemonType } from "@pokecrystal/core/core/enums";
import { renderFontText } from "../text/render-font";

const CURSOR_GLYPH = "\u25B6";

export interface BattleTextWindow {
  tile_x: number;
  tile_y: number;
  width_tiles: number;
  height_tiles: number;
  pixel_origin(tileSize: number): [number, number];
}

export interface BattleUILayout {
  menu_window: BattleTextWindow | null;
  main_menu_origin: [number, number];
  main_menu_column_spacing: number;
  main_menu_row_spacing: number;
  move_selection_window: BattleTextWindow | null;
  move_menu_origin: [number, number];
  move_menu_row_spacing: number;
  move_info_window: BattleTextWindow | null;
}

export interface BattleUIFont {
  render_text?(
    text: string,
    x: number,
    y: number,
    surface: unknown,
    options?: { uppercase?: boolean; color?: [number, number, number] }
  ): void;
  renderText?(
    text: string,
    x: number,
    y: number,
    surface: unknown,
    options?: { uppercase?: boolean; color?: [number, number, number] } | boolean
  ): void;
}

export interface BattleUIRoot {
  screen: unknown;
  font: BattleUIFont;
  draw_window(
    surface: unknown,
    x: number,
    y: number,
    widthTiles: number,
    heightTiles: number,
    options?: { fill?: [number, number, number] }
  ): void;
}

export interface BattleUIState {
  layout: BattleUILayout;
  ui: BattleUIRoot;
  selected_main_menu_index: number;
  selected_move_index: number;
  selected_pokemon_index: number;
  selected_item_index: number;
  BACKGROUND_COLOUR: [number, number, number];
  TEXT_COLOUR: [number, number, number];
  _move_metadata: Map<string, { type: PokemonType; pp: number }>;
  _type_display_name(type: PokemonType): string;
  _selected_move_entry(moves: Array<LearnedMove | null>): LearnedMove | null;
}

export function draw_main_menu(screen: BattleUIState, tile_size: number): void {
  const layout = screen.layout.menu_window;
  let base_x: number;
  let base_y: number;
  if (layout) {
    const [box_x, box_y] = layout.pixel_origin(tile_size);
    screen.ui.draw_window(screen.ui.screen, box_x, box_y, layout.width_tiles, layout.height_tiles, {
      fill: screen.BACKGROUND_COLOUR,
    });
    base_x = box_x + tile_size;
    base_y = box_y + tile_size;
  } else {
    base_x = screen.layout.main_menu_origin[0] * tile_size;
    base_y = screen.layout.main_menu_origin[1] * tile_size;
  }

  const options = ["FIGHT", "PKMN", "PACK", "RUN"];
  const column_spacing = screen.layout.main_menu_column_spacing * tile_size;
  const row_spacing = screen.layout.main_menu_row_spacing * tile_size;
  for (let index = 0; index < options.length; index += 1) {
    const option = options[index];
    const row = Math.floor(index / 2);
    const column = index % 2;
    const cursor = index === screen.selected_main_menu_index ? CURSOR_GLYPH : " ";
    renderFontText(screen.ui.font as any,
      `${cursor}${option}`,
      base_x + column * column_spacing,
      base_y + row * row_spacing,
      screen.ui.screen,
      { color: screen.TEXT_COLOUR }
    );
  }
}

export function draw_move_menu(
  screen: BattleUIState,
  player_pokemon: Pokemon,
  tile_size: number
): void {
  const menu_window = screen.layout.move_selection_window;
  const base_x = screen.layout.move_menu_origin[0] * tile_size;
  const base_y = screen.layout.move_menu_origin[1] * tile_size;
  if (menu_window) {
    const [box_x, box_y] = menu_window.pixel_origin(tile_size);
    screen.ui.draw_window(
      screen.ui.screen,
      box_x,
      box_y,
      menu_window.width_tiles,
      menu_window.height_tiles,
      { fill: screen.BACKGROUND_COLOUR }
    );
  }

  const moves = player_pokemon.moves ?? [];
  const line_height = tile_size * Math.max(1, screen.layout.move_menu_row_spacing);
  for (let index = 0; index < moves.length + 1; index += 1) {
    const move = index < moves.length ? moves[index] : null;
    const label = move ? moveDisplayName(move.name) : "CANCEL";
    const cursor = index === screen.selected_move_index ? CURSOR_GLYPH : " ";
    renderFontText(screen.ui.font as any,
      `${cursor}${label}`,
      base_x,
      base_y + index * line_height,
      screen.ui.screen,
      { color: screen.TEXT_COLOUR }
    );
  }

  const info_window = screen.layout.move_info_window;
  if (info_window) {
    _draw_move_info_box(screen, player_pokemon, info_window, tile_size);
  }
}

function _draw_move_info_box(
  screen: BattleUIState,
  player_pokemon: Pokemon,
  window: BattleTextWindow,
  tile_size: number
): void {
  const [box_x, box_y] = window.pixel_origin(tile_size);
  screen.ui.draw_window(
    screen.ui.screen,
    box_x,
    box_y,
    window.width_tiles,
    window.height_tiles,
    { fill: screen.BACKGROUND_COLOUR }
  );
  const inner_x = box_x + tile_size;
  const inner_y = box_y + tile_size;

  const selected_move = screen._selected_move_entry(player_pokemon.moves ?? []);
  let type_text = "TYPE/----";
  let pp_text = "PP --/--";
  if (selected_move) {
    if (player_pokemon.disable_turns > 0 && player_pokemon.disabled_move === selected_move.name) {
      type_text = "Disabled!";
      pp_text = "";
    } else {
      const move_data = screen._move_metadata.get(selected_move.name);
      const move_type = move_data ? move_data.type : PokemonType.NONE;
      const base_pp = move_data ? move_data.pp : selected_move.current_pp;
      type_text = `TYPE/${screen._type_display_name(move_type)}`;
      pp_text = `PP ${String(selected_move.current_pp).padStart(2, " ")}/${String(base_pp).padStart(2, " ")}`;
    }
  }

  renderFontText(screen.ui.font as any, type_text, inner_x, inner_y, screen.ui.screen, {
    uppercase: false,
    color: screen.TEXT_COLOUR,
  });
  renderFontText(screen.ui.font as any, pp_text, inner_x, inner_y + tile_size, screen.ui.screen, {
    uppercase: false,
    color: screen.TEXT_COLOUR,
  });
}

export function draw_pokemon_menu(
  screen: BattleUIState,
  player_party: Pokemon[],
  base_x: number,
  base_y: number,
  tile_size: number
): void {
  const line_height = tile_size * 2;
  for (let index = 0; index < player_party.length; index += 1) {
    const pokemon = player_party[index];
    const cursor = index === screen.selected_pokemon_index ? CURSOR_GLYPH : " ";
    renderFontText(screen.ui.font as any,
      `${cursor}${pokemon.nickname}`,
      base_x,
      base_y + index * line_height,
      screen.ui.screen,
      { color: screen.TEXT_COLOUR }
    );
  }
}

export function draw_item_menu(
  screen: BattleUIState,
  items: Record<string, unknown>,
  base_x: number,
  base_y: number,
  tile_size: number
): void {
  const item_names = items ? Object.keys(items) : [];
  if (!item_names.length) {
    renderFontText(screen.ui.font as any, "No usable items", base_x, base_y, screen.ui.screen, {
      color: screen.TEXT_COLOUR,
    });
    return;
  }
  const line_height = tile_size * 2;
  for (let index = 0; index < item_names.length; index += 1) {
    const item_name = item_names[index];
    const cursor = index === screen.selected_item_index ? CURSOR_GLYPH : " ";
    renderFontText(screen.ui.font as any,
      `${cursor}${item_name}`,
      base_x,
      base_y + index * line_height,
      screen.ui.screen,
      { color: screen.TEXT_COLOUR }
    );
  }
}
