import { Surface } from '../surface';
import { SCREEN_TILE_WIDTH } from '../../core/text-constants';
import { loadPhoneContactDirectory } from '../menus/pokegear-contacts';
import { renderFontText } from '../text/render-font';
import { Z_INDEX_DIALOGUE } from '../z-index';

export type UIContextFont = {
  render_text?: (
    text: string,
    x: number,
    y: number,
    surface: Surface,
    options: {
      color: [number, number, number];
      palette: Array<[number, number, number]>;
      text_width: number;
      max_lines: number;
      uppercase: boolean;
    }
  ) => void;
  renderText?: (
    text: string,
    x: number,
    y: number,
    surface: Surface,
    options?: {
      color: [number, number, number];
      palette: Array<[number, number, number]>;
      text_width?: number;
      textWidth?: number;
      max_lines?: number;
      maxLines?: number;
      uppercase: boolean;
    } | boolean
  ) => void;
};

export type PhoneOverlayUI = {
  tile_size: number;
  font: UIContextFont;
  get_context_palette: (name: string) => Array<[number, number, number]>;
  draw_window: (
    surface: Surface,
    x: number,
    y: number,
    widthTiles: number,
    heightTiles: number,
    options: { frame_id: number; fill: [number, number, number] }
  ) => void;
  _record_window_region: (
    surface: Surface,
    x: number,
    y: number,
    widthTiles: number,
    heightTiles: number,
    zIndex: number
  ) => void;
};

export class PhoneCallOverlay {
  private static readonly BOX_HEIGHT_TILES = 4;
  private static readonly TEXT_START_COLUMN = 3;

  private readonly contactDirectory = loadPhoneContactDirectory();
  private activeContact: string | null = null;

  constructor(
    private readonly ui: PhoneOverlayUI,
    private readonly frameIdProvider: () => number
  ) {}

  get active(): boolean {
    return Boolean(this.activeContact);
  }

  show(contactId: string): void {
    const normalized = contactId.trim();
    if (!normalized) {
      throw new Error('PhoneCallOverlay.show requires a contact id.');
    }
    this.activeContact = normalized;
  }

  hide(): void {
    this.activeContact = null;
  }

  draw(surface: Surface | null): void {
    if (!surface || !this.active) {
      return;
    }
    const palette = this.ui.get_context_palette('textbox');
    const fillColor = palette[0];
    const textColor = palette[palette.length - 1];
    const frameId = this.frameIdProvider();
    const width = SCREEN_TILE_WIDTH;
    const height = PhoneCallOverlay.BOX_HEIGHT_TILES;
    const xPx = 0;
    const yPx = 0;

    this.ui.draw_window(surface, xPx, yPx, width, height, {
      frame_id: frameId,
      fill: fillColor,
    });

    const iconX = xPx + this.ui.tile_size;
    const iconY = yPx + this.ui.tile_size;
    const textX = xPx + PhoneCallOverlay.TEXT_START_COLUMN * this.ui.tile_size;
    const textY = yPx + this.ui.tile_size;
    const textWidthPx = Math.max(
      0,
      (width - PhoneCallOverlay.TEXT_START_COLUMN - 1) * this.ui.tile_size
    );

    this.renderIcon(surface, iconX, iconY, textColor, palette);
    this.renderCallerLines(surface, textX, textY, textWidthPx, textColor, palette);
    this.ui._record_window_region(surface, xPx, yPx, width, height, Z_INDEX_DIALOGUE);
  }

  private renderIcon(
    surface: Surface,
    x: number,
    y: number,
    color: [number, number, number],
    palette: Array<[number, number, number]>
  ): void {
    renderFontText(this.ui.font as any, '\u260e', x, y, surface, {
      color,
      palette,
      text_width: this.ui.tile_size,
      max_lines: 1,
      uppercase: false,
    });
  }

  private renderCallerLines(
    surface: Surface,
    x: number,
    y: number,
    textWidth: number,
    color: [number, number, number],
    palette: Array<[number, number, number]>
  ): void {
    const lines = this.contactDirectory.displayLines(this.activeContact ?? '');
    for (const [row, line] of lines.slice(0, 2).entries()) {
      renderFontText(this.ui.font as any, line, x, y + row * this.ui.tile_size, surface, {
        color,
        palette,
        text_width: textWidth,
        max_lines: 1,
        uppercase: false,
      });
    }
  }
}
