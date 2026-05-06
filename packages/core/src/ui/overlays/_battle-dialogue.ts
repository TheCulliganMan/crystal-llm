import { GameState } from '@pokecrystal/core/core/state';
import { isCancelEvent, isConfirmEvent } from '@pokecrystal/core/input/buttons';
import { DialogueWindow } from '../text/dialogue';
import { renderFontText } from '../text/render-font';
import { BattleUI } from './ui-types';
import { BATTLE_TEXT_COLOUR } from './_battle-constants';
import { DEFAULT_TILE_SIZE } from './_battle-layout';
import type { BattleTextWindow } from './_battle-layout';

// Mirrors pokecrystal_disassembly/home/battle.asm (BattleTextbox/StdBattleTextbox).
const DOWN_ARROW = '\u25bc';

export class BattleDialogueBox {
  public readonly ui: BattleUI;
  public readonly window: BattleTextWindow;
  private dialogue: DialogueWindow;
  private queueItems: string[] = [];
  private pending_waits_value = 0;
  private forced_visible = false;

  constructor(ui: BattleUI, window: BattleTextWindow, game_state: GameState) {
    this.ui = ui;
    this.window = window;
    const lines = Math.max(1, window.height_tiles - 2);
    this.dialogue = new DialogueWindow(ui, game_state, lines);
  }

  set_game_state(game_state: GameState): void {
    this.dialogue.game_state = game_state;
  }

  get visible(): boolean {
    return this.forced_visible;
  }

  get queue(): string[] {
    return this.queueItems;
  }

  get pending_waits_count(): number {
    return this.pending_waits_value;
  }

  get pending_waits(): number {
    return this.pending_waits_value;
  }

  get requires_player_ack(): boolean {
    if (this.visible && this.dialogue.is_complete()) {
      return true;
    }
    return this.pending_waits > 0;
  }

  get is_revealing(): boolean {
    return this.visible && !this.dialogue.is_complete();
  }

  get has_pending_messages(): boolean {
    return this.visible || this.queue.length > 0;
  }

  open_box(): void {
    this.forced_visible = true;
    if (!this.dialogue.visible_text) {
      this.dialogue.open('');
    }
  }

  close(): void {
    this.queueItems = [];
    this.pending_waits_value = 0;
    this.forced_visible = false;
    this.dialogue.clear();
  }

  enqueue(text: string): void {
    const normalized = String(text ?? '').replace(/\r/g, '').trim();
    this.queueItems.push(normalized);
    if (!this.visible) {
      this.start_next_message();
    }
  }

  force_message(text: string | null): void {
    this.queueItems = [];
    if (!text) {
      this.close();
      return;
    }
    this.forced_visible = true;
    this.dialogue.open(text);
  }

  push_wait(): void {
    this.pending_waits_value += 1;
    if (!this.visible) {
      this.open_box();
    }
  }

  private start_next_message(): void {
    if (!this.queueItems.length) {
      this.dialogue.clear();
      this.forced_visible = false;
      return;
    }
    const nextText = this.queueItems.shift() ?? '';
    this.dialogue.open(nextText);
    this.forced_visible = true;
  }

  update(): void {
    if (this.visible) {
      this.dialogue.update();
    }
  }

  draw(fill_colour: [number, number, number]): void {
    if (!(this.visible || this.pending_waits_value > 0)) {
      return;
    }
    const tileSize = this.ui.tile_size ?? DEFAULT_TILE_SIZE;
    const [boxX, boxY] = this.window.pixelOrigin(tileSize);
    if (!this.ui.draw_window) {
      throw new Error('BattleDialogueBox requires ui.draw_window');
    }
    this.ui.draw_window(this.ui.screen, boxX, boxY, this.window.width_tiles, this.window.height_tiles, {
      fill: fill_colour,
    });

    const innerX = boxX + tileSize;
    const innerY = boxY + tileSize;
    const widthPx = Math.max(0, (this.window.width_tiles - 2) * tileSize);
    const maxLines = Math.max(1, this.window.height_tiles - 2);
    const text = this.dialogue.visible_text;
    if (text) {
      renderFontText(this.ui.font as any, text, innerX, innerY, this.ui.screen as any, {
        text_width: widthPx,
        max_lines: maxLines,
        uppercase: false,
        color: BATTLE_TEXT_COLOUR,
      });
    }

    if (this.requires_player_ack && this.dialogue.is_complete()) {
      const arrowX = boxX + (this.window.width_tiles - 2) * tileSize;
      const arrowY = boxY + (this.window.height_tiles - 2) * tileSize;
      renderFontText(this.ui.font as any, DOWN_ARROW, arrowX, arrowY, this.ui.screen as any, {
        color: BATTLE_TEXT_COLOUR,
      });
    }
  }

  handle_input(event: KeyboardEvent): boolean {
    if (event.type !== 'keydown') {
      return false;
    }
    if (!(isConfirmEvent(event) || isCancelEvent(event))) {
      return false;
    }
    return this.advance();
  }

  advance(): boolean {
    if (!this.has_pending_messages && this.pending_waits <= 0) {
      return false;
    }

    if (this.visible) {
      if (!this.dialogue.is_complete()) {
        this.dialogue.complete();
        return true;
      }
      if (this.dialogue.has_more_pages()) {
        this.dialogue.advance_page();
        return true;
      }
      this.start_next_message();
      if (this.visible) {
        return true;
      }
    }

    if (this.pending_waits_value > 0) {
      this.pending_waits_value = Math.max(0, this.pending_waits_value - 1);
      if (this.pending_waits_value === 0 && !this.has_pending_messages) {
        this.forced_visible = false;
      }
      return true;
    }
    return false;
  }
}
