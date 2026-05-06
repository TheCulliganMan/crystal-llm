import { gameEngine, GameEngineEventQueue } from '../game-engine';
import { Surface } from '../surface';
import { TilemapSurface } from '../tilemap-surface';
import { GameState } from '../../core/state';
import logger from '../../core/logger';
import { EngineUnownPuzzle } from '../../engine/games/unown-puzzle';
import { HardwareRNG } from '../../engine/games/rng';
import { AudioEngine } from '../../engine/systems/audio';
import { GameButton, buttonKeys, isKeyDownEvent, isKeyUpEvent, normalizeButtonKey } from '../../input/buttons';
import { GB_FRAME_RATE } from '@pokecrystal/core/core/gb-timing';
import { nextFrame } from '@pokecrystal/core/ui/async-loop';
import * as unownPuzzleAssets from '@pokecrystal/assets/content/data/unown-puzzles/unown-puzzle-assets';

const BG_COLOR: [number, number, number, number] = [248, 248, 248, 255];
const PUZZLE_BORDER = 0xee;
const PUZZLE_VOID = 0xef;
const START_TEXT_TILE = 0xf0;
const START_CANCEL_TEXT_OFFSET = 0xf6;
const SCREEN_WIDTH = 160;
const SCREEN_HEIGHT = 144;
const TILE_SIZE = 8;
const OAM_X_OFFSET = 8;
const OAM_Y_OFFSET = 16;
const A_KEYS = new Set(buttonKeys[GameButton.A]);
const B_KEYS = new Set(buttonKeys[GameButton.B]);
const START_KEYS = new Set(buttonKeys[GameButton.Start]);
const UP_KEYS = new Set(['ArrowUp']);
const DOWN_KEYS = new Set(['ArrowDown']);
const LEFT_KEYS = new Set(['ArrowLeft']);
const RIGHT_KEYS = new Set(['ArrowRight']);
const SFX_MOVE: Record<'true' | 'false', string> = {
  true: 'SFX_MOVE_PUZZLE_PIECE',
  false: 'SFX_POUND',
};
const SFX_INVALID = 'SFX_WRONG';
const SFX_PICKUP = 'SFX_MEGA_KICK';
const SFX_PLACE = 'SFX_PLACE_PUZZLE_PIECE_DOWN';
const SFX_SOLVED = 'SFX_1ST_PLACE';
const BG_WAIT_FRAMES_AFTER_ACTION = 1;
const SFX_WAIT_FRAME_BUDGET = 300;
const TEXT_DELAY_INITIAL = 15;
const TEXT_DELAY_REPEAT = 5;

const DIRECTION_TO_KEY: Record<string, string> = {
  up: "ArrowUp",
  down: "ArrowDown",
  left: "ArrowLeft",
  right: "ArrowRight",
};
const BUTTON_TOKEN_TO_GAME_BUTTON: Record<string, GameButton> = {
  [GameButton.A]: GameButton.A,
  [GameButton.B]: GameButton.B,
  [GameButton.Start]: GameButton.Start,
  [GameButton.Select]: GameButton.Select,
};

const UNOWN_STATE_IDLE = 0;
const UNOWN_STATE_ACTIVE = 1;
const UNOWN_STATE_SOLVED = 2;

class CursorState {
  public position = 0;
}

type PuzzleRunSession = {
  puzzleId: string;
  heldKeys: Set<string>;
  pressedKeys: Set<string>;
  heldButtonCodes: Set<number>;
  pressedButtonCodes: Set<number>;
  solved: boolean;
  screen: Surface;
  previousInMenu: number;
};

export interface UnownPuzzleUI {
  screen: Surface;
  update(): void;
  eventQueue?: GameEngineEventQueue;
}

export class UnownPuzzleOverlay {
  public readonly STATE_IDLE = UNOWN_STATE_IDLE;
  public readonly STATE_ACTIVE = UNOWN_STATE_ACTIVE;
  public readonly STATE_SOLVED = UNOWN_STATE_SOLVED;

  private coords: unownPuzzleAssets.PuzzleCoordinate[] = [];
  private oamTemplates: Record<string, unownPuzzleAssets.OamTemplate[]> = {};
  private cornerTiles: number[] = [];
  private tilemap = new TilemapSurface();
  private cursor = new CursorState();
  private frameCounter = 0;
  private puzzle: EngineUnownPuzzle | null = null;
  private tiles: Record<number, Surface> = {};
  private clock = new gameEngine.time.Clock();
  private awaitingAck = false;
  private state = this.STATE_IDLE;
  private letterIndex = 0;
  private bgMapWaitFrames = 0;
  private sfxWaitBudgetFrames = 0;
  private pendingSfxWaitIds: string[] = [];

  constructor(
    private ui: UnownPuzzleUI,
    private gameState: GameState,
    private audioEngine: AudioEngine | null = null,
  ) {
    const [coords, templates] = unownPuzzleAssets.loadCoordinates();
    this.coords = coords;
    this.oamTemplates = templates;
    this.cornerTiles = unownPuzzleAssets.computeCornerTiles();
  }

  run(puzzleId: string, rng: HardwareRNG, puzzle: EngineUnownPuzzle | null = null): boolean {
    const previousInMenu = this.gameState.hram.hInMenu;
    let session: PuzzleRunSession | null = null;
    logger.debug(`[unown-puzzle] run start: ${puzzleId}`);
    try {
      session = this.beginRunSession(puzzleId, rng, puzzle);
      while (true) {
        if (this.stepRunSession(session)) {
          return session.solved;
        }
        this.clock.tick(GB_FRAME_RATE);
      }
    } finally {
      if (session) {
        logger.debug(`[unown-puzzle] run end: ${session.puzzleId} solved=${session.solved}`);
        this.finishRunSession(session);
      } else {
        this.setState(this.STATE_IDLE);
        this.gameState.hram.hInMenu = previousInMenu;
      }
    }
  }

  async runAsync(
    puzzleId: string,
    rng: HardwareRNG,
    puzzle: EngineUnownPuzzle | null = null,
  ): Promise<boolean> {
    const previousInMenu = this.gameState.hram.hInMenu;
    let session: PuzzleRunSession | null = null;
    logger.debug(`[unown-puzzle] runAsync start: ${puzzleId}`);
    try {
      session = this.beginRunSession(puzzleId, rng, puzzle);
      while (true) {
        if (this.stepRunSession(session)) {
          return session.solved;
        }
        // ASM parity (_UnownPuzzle.loop): DelayFrame yields to the next VBlank.
        // In async mode we mirror that cadence via requestAnimationFrame/setTimeout.
        await nextFrame();
      }
    } finally {
      if (session) {
        logger.debug(`[unown-puzzle] runAsync end: ${session.puzzleId} solved=${session.solved}`);
        this.finishRunSession(session);
      } else {
        this.setState(this.STATE_IDLE);
        this.gameState.hram.hInMenu = previousInMenu;
      }
    }
  }

  private beginRunSession(
    puzzleId: string,
    rng: HardwareRNG,
    puzzle: EngineUnownPuzzle | null,
  ): PuzzleRunSession {
    this.tiles = unownPuzzleAssets.buildTileSurfaces(puzzleId);
    this.puzzle = puzzle ?? new EngineUnownPuzzle(rng);
    if (!puzzle) {
      this.puzzle.shuffle();
    }
    this.cursor = new CursorState();
    this.frameCounter = 0;
    this.awaitingAck = false;
    this.bgMapWaitFrames = 0;
    this.sfxWaitBudgetFrames = 0;
    this.pendingSfxWaitIds = [];
    this.beginPuzzle(puzzleId);
    this.drawUnownPuzzle();
    const previousInMenu = this.gameState.hram.hInMenu;
    this.gameState.hram.hInMenu = 1;
    return {
      puzzleId,
      heldKeys: new Set<string>(),
      pressedKeys: new Set<string>(),
      heldButtonCodes: new Set<number>(),
      pressedButtonCodes: new Set<number>(),
      solved: false,
      screen: new Surface(SCREEN_WIDTH, SCREEN_HEIGHT),
      previousInMenu,
    };
  }

  private finishRunSession(session: PuzzleRunSession): void {
    this.setState(this.STATE_IDLE);
    this.bgMapWaitFrames = 0;
    this.sfxWaitBudgetFrames = 0;
    this.pendingSfxWaitIds = [];
    this.gameState.hram.hInMenu = session.previousInMenu;
  }

  private stepRunSession(session: PuzzleRunSession): boolean {
    session.pressedKeys.clear();
    session.pressedButtonCodes.clear();
    for (const event of gameEngine.event.get(this.ui.eventQueue)) {
      if (event.type === gameEngine.QUIT) {
        gameEngine.quit();
        throw new Error('Unown puzzle interrupted by QUIT event');
      }
      if (isKeyDownEvent(event)) {
        let pressed = false;
        for (const key of this.getEventInputTokens(event)) {
          const keyCode = this.normalizedButtonCode(key);
          const keyText = String(key);
          if (!session.heldKeys.has(keyText)) {
            session.pressedKeys.add(keyText);
            pressed = true;
          }
          session.heldKeys.add(keyText);
          if (keyCode !== null) {
            if (!session.heldButtonCodes.has(keyCode)) {
              session.pressedButtonCodes.add(keyCode);
            }
            session.heldButtonCodes.add(keyCode);
          }
        }
        if (!pressed) {
          logger.debug("[unown-puzzle] ignored key down event payload", {
            event,
          });
        }
      }
      if (isKeyUpEvent(event)) {
        let released = false;
        for (const key of this.getEventInputTokens(event)) {
          const keyText = String(key);
          session.heldKeys.delete(keyText);
          released = true;
          const keyCode = this.normalizedButtonCode(key);
          if (keyCode !== null) {
            session.heldButtonCodes.delete(keyCode);
          }
        }
        if (!released) {
          logger.debug("[unown-puzzle] ignored key up event payload", {
            event,
          });
        }
      }
    }

    const joyLast = this.computeJoyLast(session.heldKeys, session.pressedKeys);
    const joyButtons = this.gameState.hram.hInMenu ? session.heldButtonCodes : session.pressedButtonCodes;
    if (this.isActionBlockedBySfxWait()) {
      session.screen.fill(BG_COLOR);
      this.tilemap.blit(session.screen, this.tiles);
      if (this.shouldDrawCursor) {
        this.drawCursor(session.screen);
      }
      this.ui.screen.blit(session.screen, [0, 0]);
      this.ui.update();
      this.frameCounter += 1;
      return false;
    }
    if (this.awaitingAck) {
      if (
        this.keysContain(session.pressedButtonCodes, A_KEYS) ||
        this.keysContain(session.pressedButtonCodes, B_KEYS) ||
        this.keysContain(joyButtons, A_KEYS) ||
        this.keysContain(joyButtons, B_KEYS)
      ) {
        session.solved = true;
        this.markSolved();
        return true;
      }
    } else {
      if (this.keysContain(session.pressedButtonCodes, START_KEYS) ||
          this.keysContain(session.pressedButtonCodes, B_KEYS)) {
        return true;
      }
      let actionTriggered = false;
      if (this.keysContain(session.pressedButtonCodes, A_KEYS)) {
        actionTriggered = true;
        if (this.handleUnownInput(undefined, true)) {
          session.solved = true;
          this.awaitingAck = true;
        }
      }
      if (!actionTriggered) {
        this.handleUnownInput(joyLast, false);
      }
    }

    session.screen.fill(BG_COLOR);
    this.tilemap.blit(session.screen, this.tiles);
    if (this.shouldDrawCursor) {
      this.drawCursor(session.screen);
    }
    this.ui.screen.blit(session.screen, [0, 0]);
    this.ui.update();
    this.frameCounter += 1;
    return false;
  }

  private beginSfxWait(soundId: string): void {
    this.bgMapWaitFrames = BG_WAIT_FRAMES_AFTER_ACTION;
    this.sfxWaitBudgetFrames = SFX_WAIT_FRAME_BUDGET;
    this.pendingSfxWaitIds = [soundId];
  }

  private isActionBlockedBySfxWait(): boolean {
    if (this.bgMapWaitFrames > 0) {
      this.bgMapWaitFrames -= 1;
      return true;
    }
    if (this.pendingSfxWaitIds.length === 0) {
      return false;
    }
    if (this.sfxWaitBudgetFrames <= 0) {
      this.pendingSfxWaitIds = [];
      return false;
    }
    this.sfxWaitBudgetFrames -= 1;
    if (!this.isSoundPlaying(this.pendingSfxWaitIds)) {
      this.pendingSfxWaitIds = [];
      return false;
    }
    return true;
  }

  private isSoundPlaying(soundIds: string[]): boolean {
    const audioEngine = this.audioEngine;
    if (!audioEngine) {
      return false;
    }
    const isSoundPlaying =
      audioEngine.isSoundPlaying ??
      (audioEngine as unknown as { is_sound_playing?: (name?: string) => boolean }).is_sound_playing;
    if (typeof isSoundPlaying !== "function") {
      return false;
    }
    return soundIds.some((soundId) => isSoundPlaying.call(audioEngine, soundId));
  }

  private getEventInputTokens(
    event: { button?: string | null; direction?: string | null; code?: number | string | null; key?: number | string | null },
  ): Array<number | string> {
    const tokens = new Set<number | string>();
    const addInputToken = (value: number | string | null | undefined) => {
      if (value === null || value === undefined) {
        return;
      }
      tokens.add(value);
      if (typeof value === "string") {
        const directionKey = DIRECTION_TO_KEY[value.trim().toLowerCase()];
        if (directionKey) {
          tokens.add(directionKey);
        }
      }
      const normalizedCode = normalizeButtonKey(value);
      for (const directionKey of Object.values(DIRECTION_TO_KEY)) {
        if (normalizedCode !== null && normalizedCode === normalizeButtonKey(directionKey)) {
          tokens.add(directionKey);
        }
      }
    };
    addInputToken(event.code);
    addInputToken(event.key);
    addInputToken(event.button);
    if (typeof event.direction === "string" && event.direction.trim()) {
      const directionKey = DIRECTION_TO_KEY[event.direction.trim().toLowerCase()];
      if (directionKey) {
        tokens.add(directionKey);
      }
    }
    return Array.from(tokens);
  }

  private normalizedButtonCode(value: number | string): number | null {
    if (typeof value === "string") {
      const trimmed = value.trim();
      if (!trimmed) {
        return null;
      }
      const directionKey = DIRECTION_TO_KEY[trimmed.toLowerCase()];
      if (directionKey) {
        return normalizeButtonKey(directionKey);
      }
      const gameButton = BUTTON_TOKEN_TO_GAME_BUTTON[trimmed.toLowerCase()];
      if (gameButton) {
        const buttonCodes = buttonKeys(gameButton);
        return buttonCodes.length > 0 ? buttonCodes[0] : null;
      }
      return normalizeButtonKey(trimmed);
    }
    return normalizeButtonKey(value);
  }

  private get layout(): number[][] {
    if (!this.puzzle) {
      throw new Error('Puzzle has not been initialised.');
    }
    return this.puzzle.snapshot();
  }

  private get holdingPiece(): boolean {
    return this.puzzle !== null && this.puzzle.status().holding_piece !== null;
  }

  private get shouldDrawCursor(): boolean {
    if (this.awaitingAck) {
      return false;
    }
    if (this.holdingPiece) {
      return true;
    }
    return Boolean(this.frameCounter & 0x10);
  }

  private computeJoyLast(heldKeys: Set<string>, pressedKeys: Set<string>): Set<string> {
    const wram = this.gameState.wram;
    if (wram.wTextDelayFrames > 0) {
      wram.wTextDelayFrames = Math.max(0, wram.wTextDelayFrames - 1);
    }
    const baseKeys = this.gameState.hram.hInMenu ? heldKeys : pressedKeys;
    if (pressedKeys.size) {
      wram.wTextDelayFrames = TEXT_DELAY_INITIAL;
      return new Set(pressedKeys);
    }
    if (wram.wTextDelayFrames > 0) {
      return new Set();
    }
    wram.wTextDelayFrames = TEXT_DELAY_REPEAT;
    return new Set(baseKeys);
  }

  get puzzleLetterIndex(): number {
    return this.letterIndex;
  }

  get unownState(): number {
    return this.state;
  }

  private setState(value: number): void {
    this.state = value;
    this.gameState.wram.wUnownState = value;
  }

  private resolveLetterIndex(puzzleId: string): number {
    const normalized = puzzleId.trim().toUpperCase();
    const index = unownPuzzleAssets.PUZZLE_IDS.indexOf(normalized as typeof unownPuzzleAssets.PUZZLE_IDS[number]);
    if (index === -1) {
      throw new Error(`Unknown Unown puzzle '${puzzleId}'`);
    }
    return index;
  }

  private beginPuzzle(puzzleId: string): void {
    this.letterIndex = this.resolveLetterIndex(puzzleId);
    this.gameState.wram.wUnownLetterIndex = this.letterIndex;
    this.gameState.wram.wSolvedUnownPuzzle = false;
    this.setState(this.STATE_ACTIVE);
  }

  private markSolved(): void {
    this.gameState.wram.wSolvedUnownPuzzle = true;
    this.setState(this.STATE_SOLVED);
  }

  drawUnownPuzzle(): void {
    if (!this.puzzle) {
      return;
    }
    this.rebuildTilemap();
    this.setState(this.state);
  }

  handleUnownInput(heldKeys?: Iterable<string>, action: boolean = false): boolean {
    let solved = false;
    if (heldKeys) {
      this.handleDirection(heldKeys);
    }
    if (action) {
      solved = this.handleAction();
    }
    return solved;
  }

  checkUnownSolution(): boolean {
    if (!this.puzzle) {
      throw new Error('Puzzle has not been initialised.');
    }
    return this.puzzle.isSolved();
  }

  private handleDirection(heldKeys: Iterable<string>): void {
    const keys = new Set(heldKeys);
    if (this.keysContain(keys, UP_KEYS)) {
      this.moveUp();
      return;
    }
    if (this.keysContain(keys, DOWN_KEYS)) {
      this.moveDown();
      return;
    }
    if (this.keysContain(keys, LEFT_KEYS)) {
      this.moveLeft();
      return;
    }
    if (this.keysContain(keys, RIGHT_KEYS)) {
      this.moveRight();
    }
  }

  private handleAction(): boolean {
    if (!this.puzzle) {
      throw new Error('Puzzle has not been initialised.');
    }
    const x = this.cursor.position % 6;
    const y = Math.floor(this.cursor.position / 6);
    const status = this.puzzle.status();
    if (status.holding_piece === null) {
      try {
        this.puzzle.pickup(x, y);
      } catch {
        this.playSound(SFX_INVALID);
        this.beginSfxWait(SFX_INVALID);
        return false;
      }
      this.playSound(SFX_PICKUP);
      this.beginSfxWait(SFX_PICKUP);
      this.drawUnownPuzzle();
      return false;
    }
    try {
      this.puzzle.place(x, y);
    } catch {
      this.playSound(SFX_INVALID);
      this.beginSfxWait(SFX_INVALID);
      return false;
    }
    this.playSound(SFX_PLACE);
    this.beginSfxWait(SFX_PLACE);
    this.drawUnownPuzzle();
    if (this.checkUnownSolution()) {
      this.playSound(SFX_SOLVED);
      this.beginSfxWait(SFX_SOLVED);
      this.setState(this.STATE_SOLVED);
      this.drawUnownPuzzle();
      return true;
    }
    return false;
  }

  private moveUp(): void {
    if (this.cursor.position < 6) {
      return;
    }
    this.cursor.position -= 6;
    this.playSound(SFX_MOVE[String(this.holdingPiece) as 'true' | 'false']);
  }

  private moveDown(): void {
    const pos = this.cursor.position;
    if ([25, 26, 27, 28].includes(pos) || pos >= 30) {
      return;
    }
    this.cursor.position = pos + 6;
    this.playSound(SFX_MOVE[String(this.holdingPiece) as 'true' | 'false']);
  }

  private moveLeft(): void {
    const pos = this.cursor.position;
    if ([0, 6, 12, 18, 24, 30].includes(pos)) {
      return;
    }
    if (pos === 35) {
      this.cursor.position = 30;
    } else {
      this.cursor.position = pos - 1;
    }
    this.playSound(SFX_MOVE[String(this.holdingPiece) as 'true' | 'false']);
  }

  private moveRight(): void {
    const pos = this.cursor.position;
    if ([5, 11, 17, 23, 29, 35].includes(pos)) {
      return;
    }
    if (pos === 30) {
      this.cursor.position = 35;
    } else {
      this.cursor.position = pos + 1;
    }
    this.playSound(SFX_MOVE[String(this.holdingPiece) as 'true' | 'false']);
  }

  private rebuildTilemap(): void {
    if (!this.puzzle) {
      return;
    }
    this.tilemap.fillRect(0, 0, 20, 18, { tile: PUZZLE_BORDER });
    this.tilemap.fillRect(4, 3, 12, 12, { tile: PUZZLE_VOID });
    this.coords.forEach((coord, entryIndex) => {
      const x = entryIndex % 6;
      const y = Math.floor(entryIndex / 6);
      const tileX = coord.tileX;
      const tileY = coord.tileY;
      const value = this.layout[y][x];
      if (value === 0) {
        this.fillVacantBlock(tileX, tileY, coord.vacantTile);
        return;
      }
      const baseTile = this.cornerTiles[value];
      for (let row = 0; row < 3; row += 1) {
        for (let col = 0; col < 3; col += 1) {
          const tileId = baseTile + row * 12 + col;
          this.tilemap.setTile(tileX + col, tileY + row, tileId);
        }
      }
    });
    this.writeStartCancelBox();
  }

  private fillVacantBlock(tileX: number, tileY: number, tileId: number): void {
    for (let row = 0; row < 3; row += 1) {
      for (let col = 0; col < 3; col += 1) {
        this.tilemap.setTile(tileX + col, tileY + row, tileId);
      }
    }
  }

  private writeStartCancelBox(): void {
    this.writeStartCancelBorder();
    if (this.state === this.STATE_SOLVED) {
      return;
    }
    for (let offset = 0; offset < 10; offset += 1) {
      this.tilemap.setTile(5 + offset, 16, START_CANCEL_TEXT_OFFSET + offset);
    }
  }

  private writeStartCancelBorder(): void {
    this.tilemap.setTile(4, 15, START_TEXT_TILE);
    for (let offset = 0; offset < 10; offset += 1) {
      this.tilemap.setTile(5 + offset, 15, START_TEXT_TILE + 1);
    }
    this.tilemap.setTile(15, 15, START_TEXT_TILE + 2);

    this.tilemap.setTile(4, 16, START_TEXT_TILE + 3);
    for (let offset = 0; offset < 10; offset += 1) {
      this.tilemap.setTile(5 + offset, 16, PUZZLE_VOID);
    }
    this.tilemap.setTile(15, 16, START_TEXT_TILE + 3);

    this.tilemap.setTile(4, 17, START_TEXT_TILE + 4);
    for (let offset = 0; offset < 10; offset += 1) {
      this.tilemap.setTile(5 + offset, 17, START_TEXT_TILE + 1);
    }
    this.tilemap.setTile(15, 17, START_TEXT_TILE + 5);
  }

  private drawCursor(surface: Surface): void {
    if (!this.puzzle) {
      return;
    }
    const coord = this.coords[this.cursor.position];
    // ASM: RedrawUnownPuzzlePieces stores raw OAM-space coordinates in UnownPuzzleCoordData.
    // Convert from GB OAM space (x+8, y+16) to screen-space pixels before blitting.
    const cursorBaseX = coord.oamX - OAM_X_OFFSET;
    const cursorBaseY = coord.oamY - OAM_Y_OFFSET;
    const heldPiece = this.puzzle.status().holding_piece;
    const isHoldingPiece = heldPiece !== null;
    const baseTile = this.cornerTiles[isHoldingPiece ? heldPiece : 0];
    const templateKey = isHoldingPiece ? "holding" : "idle";
    for (const template of this.oamTemplates[templateKey] ?? []) {
      let tileSurface = this.resolveTileSurface(baseTile + template.tileOffset);
      const attrs = template.attributes;
      if (attrs & 0x60) {
        tileSurface = flipSurface(tileSurface, Boolean(attrs & 0x20), Boolean(attrs & 0x40));
      }
      surface.blit(tileSurface, [cursorBaseX + template.signedX, cursorBaseY + template.signedY]);
    }
  }

  private resolveTileSurface(tileId: number): Surface {
    const surface = this.tiles[tileId];
    if (!surface) {
      throw new Error(`Missing tile surface for id ${tileId}.`);
    }
    return surface;
  }

  private playSound(sound: string): void {
    if (!this.audioEngine) {
      return;
    }
    this.audioEngine.playSound(sound);
  }

  private keysContain<T>(keys: Set<T>, target: Set<T>): boolean {
    for (const key of target) {
      if (keys.has(key)) {
        return true;
      }
    }
    return false;
  }
}

function flipSurface(source: Surface, flipX: boolean, flipY: boolean): Surface {
  const width = source.width;
  const height = source.height;
  const flipped = new Surface(width, height);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const sourceX = flipX ? width - 1 - x : x;
      const sourceY = flipY ? height - 1 - y : y;
      const color = source.getAt(sourceX, sourceY);
      flipped.setAt(x, y, color);
    }
  }
  return flipped;
}
