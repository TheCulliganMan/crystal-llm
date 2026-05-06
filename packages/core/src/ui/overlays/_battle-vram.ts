export class TileSpan {
  constructor(
    public start_tile: number,
    public tile_count: number,
    public source: string,
    public bank: number = 0,
    public target: string = 'vTiles2'
  ) {}

  end_tile(): number {
    return this.start_tile + this.tile_count - 1;
  }
}

export class BattleHardwareRegisters {
  public scx = 0;
  public scy = 0;
  public wx = 0;
  public wy = 0;
  public lcdc_pointer: number | null = null;

  set_scroll(scx: number, scy: number): void {
    this.scx = scx & 0xff;
    this.scy = scy & 0xff;
  }

  set_window(wx: number, wy: number): void {
    this.wx = wx & 0xff;
    this.wy = wy & 0xff;
  }

  clear_pointer(): void {
    this.lcdc_pointer = null;
  }
}

export class BattleVRAMAllocator {
  public spans: TileSpan[] = [];
  public scx = 0;
  public scy = 0;
  public oam_enabled = true;

  record_tiles(options: {
    start_tile: number;
    tile_count: number;
    source: string;
    bank?: number;
    target?: string;
  }): void {
    const { start_tile, tile_count, source } = options;
    const bank = options.bank ?? 0;
    const target = options.target ?? 'vTiles2';
    if (tile_count <= 0) {
      return;
    }
    const span = new TileSpan(start_tile, tile_count, source, bank, target);
    this.verify_no_overlap(span);
    this.spans.push(span);
  }

  record_scroll(scx: number, scy: number): void {
    this.scx = scx & 0xff;
    this.scy = scy & 0xff;
  }

  toggle_oam(enabled: boolean): void {
    this.oam_enabled = Boolean(enabled);
  }

  private verify_no_overlap(span: TileSpan): void {
    for (const existing of this.spans) {
      if (existing.target !== span.target || existing.bank !== span.bank) {
        continue;
      }
      if (
        existing.start_tile === span.start_tile &&
        existing.tile_count === span.tile_count &&
        existing.source === span.source
      ) {
        return;
      }
      if (!BattleVRAMAllocator.ranges_disjoint(
        [existing.start_tile, existing.end_tile()],
        [span.start_tile, span.end_tile()]
      )) {
        throw new Error(
          `VRAM span ${span.start_tile.toString(16)}-${span.end_tile().toString(16)} from ${span.source} ` +
          `overlaps ${existing.start_tile.toString(16)}-${existing.end_tile().toString(16)} (${existing.source})`
        );
      }
    }
  }

  private static ranges_disjoint(a: [number, number], b: [number, number]): boolean {
    return a[1] < b[0] || b[1] < a[0];
  }
}
