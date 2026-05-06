// ASM mapping: pokecrystal_disassembly/engine/menus/main_menu.asm (main menu layout + cursor bobbing).
import { MenuUI } from "./types";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { DAY_HOUR, MORN_HOUR, NITE_HOUR, syncGameClock } from "@pokecrystal/core/engine/systems/time";
import { GameState } from "@pokecrystal/core/core/state";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import { isConfirmEvent, isKeyDownEvent, type KeyEvent } from "@pokecrystal/core/input/buttons";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { Surface } from "@pokecrystal/core/ui/surface";
import { renderFontText } from "@pokecrystal/core/ui/text/render-font";
import { renderTextSnapshot, type TextSnapshotPayload } from "../text-overlays";

const BACKGROUND_COLOR: [number, number, number] = [255, 255, 255];

class TileBox {
  constructor(
    public readonly left: number,
    public readonly top: number,
    public readonly width: number,
    public readonly height: number
  ) {}

  toPixels(): [number, number] {
    return [this.left * TILE_SIZE, this.top * TILE_SIZE];
  }
}

const MENU_BOX = new TileBox(0, 0, 17, 8);
const TIME_BOX = new TileBox(0, 14, 20, 4);
const DAY_STRINGS = ["SUN", "MON", "TUES", "WEDNES", "THURS", "FRI", "SATUR"];
const isTextOnlyMenuUi = (ui: MenuUI): boolean =>
  typeof (ui as { renderSnapshot?: unknown }).renderSnapshot === "function" &&
  typeof (ui as { getChildren?: () => unknown[] }).getChildren !== "function";

export class MainMenu {
  static readonly CURSOR_PERIOD = 16;
  static readonly CURSOR_OFFSET = 1;
  static readonly FADE_SPEED = 24;

  private readonly screenWidth = 20 * TILE_SIZE;
  private readonly screenHeight = 18 * TILE_SIZE;
  private menuOptions: string[] = [];
  private selectedOption = 0;
  private cursorTimer = 0;
  private cursorBob = 0;
  private fadeAlpha = 255;
  private fadeState: "in" | "out" | null = "in";
  private saveExists: boolean;

  constructor(
    private readonly ui: MenuUI,
    private readonly audioEngine: AudioEngine,
    private readonly gameState: GameState,
    saveExists: boolean
  ) {
    this.saveExists = saveExists;
    this.initializeMenuOptions();
  }

  refresh(saveExists: boolean): void {
    this.saveExists = saveExists;
    const previous = this.selectedOption;
    this.initializeMenuOptions();
    if (this.menuOptions.length) {
      this.selectedOption = Math.min(previous, this.menuOptions.length - 1);
    } else {
      this.selectedOption = 0;
    }
  }

  startFadeIn(): void {
    this.fadeState = "in";
    this.fadeAlpha = 255;
  }

  skipFade(): void {
    this.fadeState = null;
    this.fadeAlpha = 0;
  }

  startFadeOut(): void {
    this.fadeState = "out";
    this.fadeAlpha = 0;
  }

  isFading(): boolean {
    return this.fadeState !== null;
  }

  update(): void {
    this.cursorTimer = (this.cursorTimer + 1) % MainMenu.CURSOR_PERIOD;
    const halfPeriod = MainMenu.CURSOR_PERIOD / 2;
    this.cursorBob = this.cursorTimer < halfPeriod ? 0 : MainMenu.CURSOR_OFFSET;

    if (this.fadeState === "in") {
      this.fadeAlpha = Math.max(0, this.fadeAlpha - MainMenu.FADE_SPEED);
      if (this.fadeAlpha === 0) {
        this.fadeState = null;
      }
    } else if (this.fadeState === "out") {
      this.fadeAlpha = Math.min(255, this.fadeAlpha + MainMenu.FADE_SPEED);
      if (this.fadeAlpha >= 255) {
        this.fadeState = null;
      }
    }
  }

  draw(): void {
    syncGameClock(this.gameState);
    if (isTextOnlyMenuUi(this.ui)) {
      renderTextSnapshot(this.ui, this.getTextSnapshot());
      return;
    }
    if (!this.ui.screen) {
      return;
    }
    const frame = new Surface(this.screenWidth, this.screenHeight);
    frame.fill([BACKGROUND_COLOR[0], BACKGROUND_COLOR[1], BACKGROUND_COLOR[2], 255]);

    this.drawMenuBox(frame);
    if (this.saveExists) {
      this.drawTimeBox(frame);
    }

    this.ui.screen.blit(frame, [0, 0]);
    if (this.fadeAlpha > 0) {
      const overlay = new Surface(this.screenWidth, this.screenHeight);
      overlay.fill([0, 0, 0, this.fadeAlpha]);
      this.ui.screen.blit(overlay, [0, 0]);
    }
    renderTextSnapshot(this.ui, this.getTextSnapshot());
  }

  handleInput(event: KeyEvent): string | null {
    if (!isKeyDownEvent(event) || !this.menuOptions.length) {
      return null;
    }
    if (event.key === gameEngine.K_UP) {
      this.selectedOption = (this.selectedOption - 1 + this.menuOptions.length) % this.menuOptions.length;
      return null;
    }
    if (event.key === gameEngine.K_DOWN) {
      this.selectedOption = (this.selectedOption + 1) % this.menuOptions.length;
      return null;
    }
    if (isConfirmEvent(event)) {
      this.audioEngine.playSound("menu_option");
      return this.handleSelection();
    }
    return null;
  }

  private initializeMenuOptions(): void {
    const options: string[] = [];
    if (this.saveExists) {
      options.push("CONTINUE");
      options.push("NEW GAME", "OPTION");
      if (this.mysteryGiftUnlocked()) {
        options.push("MYSTERY GIFT");
      }
    } else {
      // ASM MainMenu_GetWhichMenu returns MAINMENU_NEW_GAME before checking mystery gift.
      options.push("NEW GAME", "OPTION");
    }
    this.menuOptions = options;
  }

  private mysteryGiftUnlocked(): boolean {
    return Boolean(this.gameState.sram.mystery_gift_unlocked);
  }

  private drawMenuBox(surface: Surface): void {
    const [boxX, boxY] = MENU_BOX.toPixels();
    this.ui.drawWindow(surface, boxX, boxY, MENU_BOX.width, MENU_BOX.height, {
      fill: BACKGROUND_COLOR,
    });
    const textX = boxX + TILE_SIZE * 2;
    const textY = boxY + TILE_SIZE * 2;
    this.menuOptions.forEach((option, index) => {
      const lineY = textY + index * TILE_SIZE;
      if (index === this.selectedOption) {
        const cursorX = boxX + TILE_SIZE;
        const cursorY = lineY + this.cursorBob;
        renderFontText(this.ui.font, "\u25b6", cursorX, cursorY, surface);
      }
      renderFontText(this.ui.font, option, textX, lineY, surface);
    });
  }

  private drawTimeBox(surface: Surface): void {
    const [boxX, boxY] = TIME_BOX.toPixels();
    this.ui.drawWindow(surface, boxX, boxY, TIME_BOX.width, TIME_BOX.height, {
      fill: BACKGROUND_COLOR,
    });
    const [dayText, timeText] = this.clockStrings();
    renderFontText(this.ui.font, dayText, boxX + TILE_SIZE, boxY + TILE_SIZE, surface);
    renderFontText(this.ui.font, timeText, boxX + 4 * TILE_SIZE, boxY + 2 * TILE_SIZE, surface);
  }

  private handleSelection(): string | null {
    const selectedText = this.menuOptions[this.selectedOption];
    if (selectedText === "CONTINUE") {
      return "show_continue_screen";
    }
    if (selectedText === "NEW GAME") {
      return "new_game";
    }
    if (selectedText === "OPTION") {
      return "options_menu";
    }
    if (selectedText === "MYSTERY GIFT") {
      return "mystery_gift";
    }
    throw new Error(`MainMenu selected unsupported ASM option '${selectedText ?? "<missing>"}'`);
  }

  private clockStrings(): [string, string] {
    const dayIndex = this.gameState.sram.day_of_week % DAY_STRINGS.length;
    const dayString = `${DAY_STRINGS[dayIndex]}DAY`;
    const hour = this.gameState.hram.hHours % 24;
    const minute = Math.max(0, Math.min(this.gameState.hram.hMinutes, 59));
    const period = MainMenu.timeOfDayLabel(hour);
    const hour12 = hour % 12 === 0 ? 12 : hour % 12;
    const hourField = String(hour12).padEnd(2, " ");
    const timeString = `${period}${hourField}:${minute.toString().padStart(2, "0")}`;
    return [dayString, timeString];
  }

  private static timeOfDayLabel(hour: number): string {
    const hourMod = ((hour % 24) + 24) % 24;
    if (hourMod < MORN_HOUR) {
      return "NITE";
    }
    if (hourMod < DAY_HOUR) {
      return "MORN";
    }
    if (hourMod < NITE_HOUR) {
      return "DAY";
    }
    return "NITE";
  }

  private getTextSnapshot(): TextSnapshotPayload {
    const menuLines = this.menuOptions.map((option, index) =>
      `${index === this.selectedOption ? "▶" : " "} ${option}`
    );
    const [dayText, timeText] = this.saveExists ? this.clockStrings() : ["---", "---"];
    return {
      viewportLines: ["MAIN MENU"],
      infoLines: [
        "STATE: main_menu",
        `SAVE EXISTS: ${this.saveExists ? "yes" : "no"}`,
        `DAY: ${dayText}`,
        `TIME: ${timeText}`,
        `FADE: ${this.fadeState ?? "none"}`,
        "Up/Down=Move A=Confirm",
      ],
      viewportTitle: "Main Menu",
      infoTitle: "Main Menu",
      menuLines,
      promptLines: null,
      dialogueLines: null,
    };
  }
}
