import { TILE_SIZE } from "../engine/world/tile";

export class TileRegion {
  constructor(
    public readonly left: number,
    public readonly top: number,
    public readonly width: number,
    public readonly height: number
  ) {}

  originPx(): [number, number] {
    return [this.left * TILE_SIZE, this.top * TILE_SIZE];
  }

  sizePx(): [number, number] {
    return [this.width * TILE_SIZE, this.height * TILE_SIZE];
  }

  rectPx(): [number, number, number, number] {
    const [x, y] = this.originPx();
    const [w, h] = this.sizePx();
    return [x, y, w, h];
  }

  pointPx(offsetX: number = 0, offsetY: number = 0): [number, number] {
    return [(this.left + offsetX) * TILE_SIZE, (this.top + offsetY) * TILE_SIZE];
  }
}
