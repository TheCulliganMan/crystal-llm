import { Rect, Surface } from "./surface";

export type RGB = [number, number, number];
export type RGBA = [number, number, number, number];
export type Palette = ReadonlyArray<RGB>;

type SurfaceDest = [number, number] | Rect | { x: number; y: number };
type SurfaceArea = Rect | { x: number; y: number; width: number; height: number };

export type SurfaceLike = {
  blit?: (source: Surface | SurfaceLike, dest: SurfaceDest, area?: SurfaceArea) => void;
  get_width?: () => number;
  width?: number;
  get_height?: () => number;
  height?: number;
  get_size?: () => [number, number];
  getAt?: (x: number, y: number) => [number, number, number, number];
  get_at?: (pos: [number, number]) => [number, number, number, number];
  setAt?: (x: number, y: number, color: [number, number, number, number]) => void;
  set_at?: (pos: [number, number], color: [number, number, number, number]) => void;
  copy?: () => SurfaceLike;
  fill?: (color: [number, number, number, number], rect?: Rect) => void;
  set_colorkey?: (color: [number, number, number]) => void;
  getCanvasImageSource?: () => CanvasImageSource | null;
  get_flags?: () => number;
};

export type RenderTextOptions = {
  color?: RGB;
  palette?: Palette;
  textWidth?: number;
  text_width?: number;
  maxLines?: number;
  max_lines?: number;
  uppercase?: boolean;
};
