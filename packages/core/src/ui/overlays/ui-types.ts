import type { Surface } from '@pokecrystal/core/ui/surface';

export interface UIFont {
  render_text: (
    text: string,
    x: number,
    y: number,
    surface: Surface | import("@pokecrystal/core/ui/font-renderer").SurfaceLike,
    options?: import("@pokecrystal/core/ui/font-renderer").RenderTextOptions,
  ) => void;
  renderText?: (
    text: string,
    x: number,
    y: number,
    surface: Surface | import("@pokecrystal/core/ui/font-renderer").SurfaceLike,
    options?: boolean | import("@pokecrystal/core/ui/font-renderer").RenderTextOptions,
  ) => void;
}

export interface BattleUI {
  screen: Surface;
  tile_size?: number;
  font: UIFont;
  draw_window?: (
    surface: Surface,
    x: number,
    y: number,
    widthTiles: number,
    heightTiles: number,
    options?: { fill?: [number, number, number] },
  ) => void;
  get_sprite_surface?: (
    spriteId: string,
    spriteType: string,
    frame?: number,
  ) => Surface | null;
  get_pokemon_frame_count?: (speciesId: string, sprite_type: string) => number;
  draw_sprite?: (
    speciesId: string,
    x: number,
    y: number,
    options?: { sprite_type?: string; frame?: number },
  ) => void;
  _apply_colorkey_transparency?: (surface: Surface) => Surface;
}
