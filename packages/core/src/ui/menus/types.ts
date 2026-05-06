import { Surface } from "../surface";
import type { BaseFontRenderer } from "../base-ui";
import { GameEngineEventQueue } from "@pokecrystal/core/ui/game-engine";
import type { ScreenUI } from "../screens/screen-types";

export type FontRenderer = BaseFontRenderer & {
  renderText: NonNullable<BaseFontRenderer["renderText"]>;
  fontTiles?: Record<number, Surface>;
  font_tiles?: Record<number, Surface>;
};

export type MenuUI = {
  screen: Surface | null;
  tileSize: number;
  font: FontRenderer;
  getPokemonFrontSurface?: (speciesId: string, frame?: number) => Surface | null;
  eventQueue?: GameEngineEventQueue;
  get_context_palette?: (name: string) => [number, number, number][];
  getContextPalette?: (name: string) => [number, number, number][];
  drawWindow: (
    surface: Surface,
    x: number,
    y: number,
    widthTiles: number,
    heightTiles: number,
    opts?: { frameId?: number | null; fill?: [number, number, number] | null; zIndex?: number },
  ) => void;
  update?: () => void;
  playCry?: (speciesId: string) => void;
  renderSnapshot?: ScreenUI["renderSnapshot"];
};
