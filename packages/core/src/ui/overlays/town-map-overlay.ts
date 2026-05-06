// ASM: engine/pokegear/pokegear.asm (Town Map tilemap + landmark label render).
import { LandmarkEntry } from "@pokecrystal/assets/content/pokegear";
import { PlayerGender } from "@pokecrystal/core/core/enums";
import { GameState } from "@pokecrystal/core/core/state";
import { Event, EventManager } from "@pokecrystal/core/engine/events/events";
import { KeyEvent, isCancelEvent, isConfirmEvent } from "@pokecrystal/core/input/buttons";
import { PokegearBackground } from "@pokecrystal/core/ui/menus/pokegear-bg";
import { PokegearCard, PokegearStateMachine } from "@pokecrystal/core/ui/menus/pokegear-state";
import { resolveLandmarkText } from "@pokecrystal/core/ui/menus/pokegear-labels";
import { TilemapSurface, _SPACE_TILE } from "@pokecrystal/core/ui/tilemap-surface";
import { Surface as UiSurface } from "@pokecrystal/core/ui/surface";
import { gameEngine, Surface, type GameEngineEvent } from "@pokecrystal/core/ui/game-engine";
import type { EventManagerLike } from "@pokecrystal/core/engine/events/event-manager-like";
import { projectLandmarkToTownMapPixel } from "@pokecrystal/core/ui/overlays/town-map-coords";
import { drawTownMapCursorMarker } from "@pokecrystal/core/ui/overlays/town-map-marker";

const SPACE_TILE_ID = _SPACE_TILE;
const MAP_SIZE: [number, number] = [160, 144];

const MAP_LABEL_ICON_TILE = 0x34;
const MAP_LABEL_TEXT_X = 9;
const MAP_LABEL_TEXT_WIDTH = 11;

type ScriptRunner = {
  pause(): void;
  resume(): void;
  _awaiting_resume?: number;
};

type TownMapEventPayload = {
  runner?: ScriptRunner | null;
};

export type TownMapOverlayLike = {
  handle_input?: (event: GameEngineEvent) => boolean;
  register?: (event_manager: EventManager) => void;
  visible?: boolean;
  drawToGameEngine?: (surface: Surface | null) => void;
};

const toGameEngineSurface = (surface: UiSurface): Surface => {
  const target = new gameEngine.Surface(surface.width, surface.height);
  const context = target.getContext();
  if (!context) {
    throw new Error("TownMapOverlay failed to acquire 2D context for map surface.");
  }
  context.putImageData(surface.getImageData(), 0, 0);
  return target;
};

const drawCircle = (
  surface: UiSurface,
  color: [number, number, number],
  center: [number, number],
  radius: number,
  width = 0,
): void => {
  const [cx, cy] = center;
  const r2 = radius * radius;
  const inner = Math.max(0, radius - width);
  const inner2 = inner * inner;
  for (let y = cy - radius; y <= cy + radius; y += 1) {
    for (let x = cx - radius; x <= cx + radius; x += 1) {
      const dx = x - cx;
      const dy = y - cy;
      const dist2 = dx * dx + dy * dy;
      if (dist2 > r2) {
        continue;
      }
      if (width > 0 && dist2 < inner2) {
        continue;
      }
      surface.setAt(x, y, [color[0], color[1], color[2], 255]);
    }
  }
};

export class TownMapOverlay {
  private readonly gameState: GameState;
  private readonly stateMachine: PokegearStateMachine;
  private readonly background: PokegearBackground;
  private readonly lockMovement: () => void;
  private readonly unlockMovement: () => void;
  private visibleFlag = false;
  private mapSurface: UiSurface;
  private mapSurfaceEngine: Surface | null = null;
  private awaitingRunner: ScriptRunner | null = null;
  private resumeRunnerOnClose = false;
  private eventAttached = false;

  constructor(
    _ui: unknown,
    game_state: GameState,
    options: {
      script_runner?: ScriptRunner | null;
      lock_movement: () => void;
      unlock_movement: () => void;
    },
  ) {
    this.gameState = game_state;
    this.stateMachine = new PokegearStateMachine(game_state);
    this.background = new PokegearBackground({
      playerGender: this.resolvePlayerGender(),
      mapGroup: this.gameState.wram.wMapGroup,
      mapNumber: this.gameState.wram.wMapNumber,
    });
    this.lockMovement = options.lock_movement;
    this.unlockMovement = options.unlock_movement;
    this.mapSurface = new UiSurface(MAP_SIZE[0], MAP_SIZE[1]);
    if (options.script_runner) {
      this.awaitingRunner = options.script_runner;
    }
  }

  get visible(): boolean {
    return this.visibleFlag;
  }

  register(event_manager: EventManager): void {
    if (this.eventAttached) {
      return;
    }
    this.eventAttached = true;
    event_manager.on<TownMapEventPayload>("show_town_map", (event) => this.handleShowEvent(event));
  }

  private resolvePlayerGender(): PlayerGender {
    const gender = this.gameState.sram.player_gender ?? PlayerGender.MALE;
    return typeof gender === "number" ? gender : PlayerGender.MALE;
  }

  private ensureGenderAssets(): void {
    this.background.setPlayerGender(this.resolvePlayerGender());
  }

  private handleShowEvent(event: Event<TownMapEventPayload>): void {
    if (this.visibleFlag) {
      return;
    }
    this.ensureGenderAssets();
    this.stateMachine.refresh();
    this.renderMapSurface();
    const runner = event.data.runner ?? null;
    this.captureRunner(runner);
    this.lockMovement();
    this.visibleFlag = true;
  }

  show(runner: ScriptRunner | null = null): void {
    if (this.visibleFlag) {
      return;
    }
    this.ensureGenderAssets();
    this.stateMachine.refresh();
    this.renderMapSurface();
    this.captureRunner(runner);
    this.lockMovement();
    this.visibleFlag = true;
  }

  private renderMapSurface(): void {
    this.background.syncHardware({
      mapGroup: this.gameState.wram.wMapGroup,
      mapNumber: this.gameState.wram.wMapNumber,
    });
    const tilemap = new TilemapSurface();
    const tiles = this.background.tilemapForCard(PokegearCard.MAP, this.stateMachine.mapRegion);
    tilemap.loadTiles(tiles);
    const label = resolveLandmarkText(this.stateMachine.mapCursorEntry as LandmarkEntry);
    this.writeMapLabel(tilemap, label);

    const surface = new UiSurface(MAP_SIZE[0], MAP_SIZE[1]);
    tilemap.blit(surface, this.background.tileSurfaces());
    this.drawMarkerDots(surface);
    this.mapSurface = surface;
    this.mapSurfaceEngine = null;
  }

  private writeMapLabel(tilemap: TilemapSurface, label: string): void {
    tilemap.clearBox(8, 0, 12, 2, SPACE_TILE_ID);
    tilemap.setTile(8, 0, MAP_LABEL_ICON_TILE);
    const [first, second] = label.split("\n", 2);
    tilemap.writeText(MAP_LABEL_TEXT_X, 0, first ?? "", {
      maxLength: MAP_LABEL_TEXT_WIDTH,
      pad: true,
      uppercase: false,
    });
    if (second) {
      tilemap.writeText(MAP_LABEL_TEXT_X, 1, second, {
        maxLength: MAP_LABEL_TEXT_WIDTH,
        pad: true,
        uppercase: false,
      });
    }
  }

  private drawMarkerDots(surface: UiSurface): void {
    const playerEntry = this.stateMachine.mapPlayerEntry;
    const [playerX, playerY] = this.project(playerEntry);
    drawCircle(surface, [224, 0, 64], [playerX, playerY], 3);

    const cursorEntry = this.stateMachine.mapCursorEntry;
    const [cursorX, cursorY] = this.project(cursorEntry);
    drawTownMapCursorMarker(surface, [cursorX, cursorY]);
  }

  private project(entry: LandmarkEntry): [number, number] {
    return projectLandmarkToTownMapPixel(entry);
  }

  handle_input(event: KeyEvent): boolean {
    if (!this.visibleFlag) {
      return false;
    }
    if (isConfirmEvent(event) || isCancelEvent(event)) {
      this.close();
      return true;
    }
    return true;
  }

  public close(): void {
    if (!this.visibleFlag) {
      return;
    }
    this.visibleFlag = false;
    this.unlockMovement();
    const runner = this.awaitingRunner;
    this.awaitingRunner = null;
    const shouldResume = this.resumeRunnerOnClose;
    this.resumeRunnerOnClose = false;
    if (runner && shouldResume) {
      runner.resume();
    }
  }

  private captureRunner(runner: ScriptRunner | null): void {
    this.awaitingRunner = runner;
    this.resumeRunnerOnClose = false;
    if (!runner) {
      return;
    }
    const awaitingResume = Number(runner._awaiting_resume ?? 0);
    if (awaitingResume <= 0) {
      runner.pause();
    }
    this.resumeRunnerOnClose = true;
  }

  draw(surface: UiSurface | null): void {
    if (!surface || !this.visibleFlag) {
      return;
    }
    surface.blit(this.mapSurface, [0, 0]);
  }

  drawToGameEngine(surface: Surface | null): void {
    if (!surface || !this.visibleFlag) {
      return;
    }
    if (!this.mapSurfaceEngine) {
      this.mapSurfaceEngine = toGameEngineSurface(this.mapSurface);
    }
    surface.blit(this.mapSurfaceEngine, [0, 0]);
  }
}
