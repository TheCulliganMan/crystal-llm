export const TILE_SIZE = 8;
export const SCREEN_TILE_WIDTH = 20;
export const SCREEN_TILE_HEIGHT = 18;
export const SCREEN_WIDTH = SCREEN_TILE_WIDTH * TILE_SIZE;
export const SCREEN_HEIGHT = SCREEN_TILE_HEIGHT * TILE_SIZE;

export type RgbColor = [number, number, number];

const DEFAULT_FONT = "8px 'Courier New', monospace";

export function withTextStyle(
  ctx: CanvasRenderingContext2D,
  { font = DEFAULT_FONT, color = "#000" }: { font?: string; color?: string } = {}
): void {
  ctx.font = font;
  ctx.fillStyle = color;
  ctx.textBaseline = "top";
}

export function drawText(
  ctx: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  { lineHeight = TILE_SIZE }: { lineHeight?: number } = {}
): void {
  const lines = text.split("\n");
  lines.forEach((line, idx) => {
    ctx.fillText(line, x, y + idx * lineHeight);
  });
}

export function drawTextBox(
  ctx: CanvasRenderingContext2D,
  text: string,
  xTiles: number,
  yTiles: number,
  widthTiles: number,
  heightTiles: number,
  {
    fill = "#fff",
    stroke = "#000",
    paddingTiles = 1,
  }: { fill?: string; stroke?: string; paddingTiles?: number } = {}
): void {
  drawBox(ctx, xTiles, yTiles, widthTiles, heightTiles, { fill, stroke });
  const xPx = (xTiles + paddingTiles) * TILE_SIZE;
  const yPx = (yTiles + paddingTiles) * TILE_SIZE;
  withTextStyle(ctx);
  drawText(ctx, text, xPx, yPx);
}

export function drawBox(
  ctx: CanvasRenderingContext2D,
  xTiles: number,
  yTiles: number,
  widthTiles: number,
  heightTiles: number,
  {
    fill = "#fff",
    stroke = "#000",
  }: { fill?: string; stroke?: string } = {}
): void {
  const xPx = xTiles * TILE_SIZE;
  const yPx = yTiles * TILE_SIZE;
  const widthPx = widthTiles * TILE_SIZE;
  const heightPx = heightTiles * TILE_SIZE;
  ctx.fillStyle = fill;
  ctx.fillRect(xPx, yPx, widthPx, heightPx);
  ctx.strokeStyle = stroke;
  ctx.strokeRect(xPx + 0.5, yPx + 0.5, widthPx - 1, heightPx - 1);
}

export function fillScreen(ctx: CanvasRenderingContext2D, color: RgbColor): void {
  ctx.fillStyle = `rgb(${color[0]}, ${color[1]}, ${color[2]})`;
  ctx.fillRect(0, 0, SCREEN_WIDTH, SCREEN_HEIGHT);
}
