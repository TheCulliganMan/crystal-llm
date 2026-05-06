// ASM mapping: pokecrystal_disassembly/engine/pokegear/pokegear.asm::_FlyMap.
import { getPokegearLandmarksSync, type LandmarkEntry } from "@pokecrystal/assets/content/pokegear";
import { PlayerGender } from "@pokecrystal/core/core/enums";
import type { GameState } from "@pokecrystal/core/core/state";
import { isCancelEvent, isConfirmEvent, isKeyDownEvent, type KeyEvent } from "@pokecrystal/core/input/buttons";
import { mapKeyToDirection } from "@pokecrystal/core/input/controls";
import { gameEngine, type GameEngineEventQueue } from "@pokecrystal/core/ui/game-engine";
import { PokegearBackground } from "@pokecrystal/core/ui/menus/pokegear-bg";
import { PokegearCard } from "@pokecrystal/core/ui/menus/pokegear-state";
import { TilemapSurface, _SPACE_TILE } from "@pokecrystal/core/ui/tilemap-surface";
import { nextFrame } from "@pokecrystal/core/ui/async-loop";
import { Surface as UiSurface } from "@pokecrystal/core/ui/surface";
import type { Surface } from "@pokecrystal/core/ui/surface";
import { projectLandmarkToTownMapPixel } from "@pokecrystal/core/ui/overlays/town-map-coords";
import { drawTownMapCursorMarker } from "@pokecrystal/core/ui/overlays/town-map-marker";

const MAP_WIDTH = 160;
const MAP_HEIGHT = 144;
const SPACE_TILE = _SPACE_TILE;

type PromptUI = {
  screen: InstanceType<typeof gameEngine.Surface> | null;
  eventQueue?: GameEngineEventQueue;
  update?: () => void;
  renderSnapshot?: (
    header: string[],
    info: string[],
    title: string,
    legend: string,
    menuLines: string[],
    left: unknown,
    right: unknown,
  ) => void;
};

export type FlyMapPromptOption = {
  label: string;
  landmark: string;
  default: boolean;
};

const LANDMARK_BY_CONSTANT = new Map<string, LandmarkEntry>(
  getPokegearLandmarksSync().map((entry) => [entry.constant, entry]),
);

const toGameEngineSurface = (surface: UiSurface): InstanceType<typeof gameEngine.Surface> => {
  const target = new gameEngine.Surface(surface.width, surface.height);
  const context = target.getContext();
  if (!context) {
    throw new Error("FlyMapPrompt failed to acquire a 2D context.");
  }
  context.putImageData(surface.getImageData(), 0, 0);
  return target;
};

export class FlyMapPrompt {
  private readonly background: PokegearBackground;
  private index: number;
  private finished = false;
  private resultIndex = -1;

  constructor(
    private readonly ui: PromptUI,
    private readonly gameState: GameState,
    private readonly options: FlyMapPromptOption[],
    initialIndex = 0,
  ) {
    if (!options.length) {
      throw new Error("FlyMapPrompt requires at least one option.");
    }
    const gender = this.gameState.sram.player_gender;
    this.background = new PokegearBackground({
      playerGender: typeof gender === "number" ? gender : PlayerGender.MALE,
      mapGroup: this.gameState.wram.wMapGroup,
      mapNumber: this.gameState.wram.wMapNumber,
    });
    this.index = Math.max(0, Math.min(options.length - 1, Math.trunc(initialIndex)));
  }

  private get current(): FlyMapPromptOption {
    return this.options[this.index];
  }

  handleInput(event: KeyEvent): void {
    if (!isKeyDownEvent(event)) {
      return;
    }
    const direction = mapKeyToDirection(event.direction ?? event.code ?? event.key ?? null);
    if (direction === "up") {
      this.index = (this.index + 1) % this.options.length;
    } else if (direction === "down") {
      this.index = (this.index - 1 + this.options.length) % this.options.length;
    } else if (isConfirmEvent(event)) {
      this.resultIndex = this.index;
      this.finished = true;
    } else if (isCancelEvent(event)) {
      this.resultIndex = -1;
      this.finished = true;
    }
  }

  draw(): void {
    const screen = this.ui.screen;
    if (!screen) {
      return;
    }
    const surface = this.renderSurface();
    screen.blit(toGameEngineSurface(surface), [0, 0]);
    this.ui.renderSnapshot?.(
      ["FLY TO WHERE?"],
      ["D-Pad=Move A=Select B=Back"],
      "FLY TO WHERE?",
      "Legend",
      this.options.map((option, index) => `${index === this.index ? ">" : " "} ${option.label}`),
      null,
      null,
    );
  }

  async runAsync(): Promise<number> {
    while (!this.finished) {
      const events = gameEngine.event.get(this.ui.eventQueue);
      for (const event of events) {
        if (event.type === gameEngine.QUIT) {
          gameEngine.quit();
          throw new Error("FlyMapPrompt interrupted by quit event.");
        }
        this.handleInput(event as KeyEvent);
      }
      this.draw();
      this.ui.update?.();
      await nextFrame();
    }
    return this.resultIndex;
  }

  private renderSurface(): UiSurface {
    const region = this.currentRegion();
    this.background.syncHardware({
      mapGroup: this.gameState.wram.wMapGroup,
      mapNumber: this.gameState.wram.wMapNumber,
    });
    const tilemap = new TilemapSurface();
    tilemap.loadTiles(this.background.tilemapForCard(PokegearCard.MAP, region));
    this.writeLabel(tilemap, this.current.label);
    const surface = new UiSurface(MAP_WIDTH, MAP_HEIGHT);
    tilemap.blit(surface, this.background.tileSurfaces());
    this.drawMarkers(surface);
    return surface;
  }

  private writeLabel(tilemap: TilemapSurface, label: string): void {
    tilemap.clearBox(1, 0, 18, 3, SPACE_TILE);
    tilemap.writeText(2, 0, "Where?", { maxLength: 16, pad: false, uppercase: false });
    tilemap.writeText(2, 1, label, { maxLength: 16, pad: true, uppercase: false });
  }

  private currentRegion(): string {
    const entry = LANDMARK_BY_CONSTANT.get(this.current.landmark);
    return String(entry?.region ?? "JOHTO").toUpperCase() === "KANTO" ? "KANTO" : "JOHTO";
  }

  private drawMarkers(surface: Surface): void {
    const current = LANDMARK_BY_CONSTANT.get(this.current.landmark);
    if (current) {
      const [x, y] = this.project(current);
      drawTownMapCursorMarker(surface, [x, y]);
    }
  }

  private project(entry: LandmarkEntry): [number, number] {
    return projectLandmarkToTownMapPixel(entry);
  }
}
