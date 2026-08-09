import fs from "fs";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { SCREEN_TILE_WIDTH, TEXTBOX_HEIGHT_TILES, TEXTBOX_Y_TILES } from "@pokecrystal/core/core/text-constants";
import { gbc5To8 } from "@pokecrystal/core/core/gbc-colors";
import { PlayerGender } from "@pokecrystal/core/core/enums";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { Surface } from "@pokecrystal/core/ui/game-engine";
import type { FontRenderer as TextboxFontRenderer } from "@pokecrystal/core/ui/textbox";
import {
  isConfirmEvent,
  isKeyDownEvent,
  mapKeyToDirection,
  type InputEventLike,
} from "@pokecrystal/core/input/controls";
import {
  fillScreen,
  SCREEN_HEIGHT,
  SCREEN_WIDTH,
  TILE_SIZE,
} from "./rendering";
import { BootTextboxRenderer } from "./boot-textbox-renderer";
import {
  type TextSnapshotPayload,
} from "../../text-overlays";
import { buildGenderSelectionControlLines } from "../../control-lines";

// ASM reference: engine/menus/init_gender.asm::InitGender

export class GenderSelectionScreen {
  static readonly QUESTION_TEXT = "ARE YOU A BOY? OR ARE YOU A GIRL?";
  static readonly BOX_TOP_LEFT: [number, number] = [6, 4];
  static readonly BOX_BOTTOM_RIGHT: [number, number] = [12, 9];
  static readonly POST_CONFIRM_DELAY = 10;
  static readonly FADE_IN_FRAMES = 8;

  private readonly options: PlayerGender[] = [
    PlayerGender.MALE,
    PlayerGender.FEMALE,
  ];
  private readonly optionLabels = ["BOY", "GIRL"];
  private readonly backgroundColor: [number, number, number];
  private readonly textboxRenderer: BootTextboxRenderer;
  private readonly audioEngine?: AudioEngine | null;

  private fadeCounter = 0;
  private selectedIndex = 0;
  private selectedGender = this.options[0];
  private confirmed = false;
  private confirmCountdown = 0;

  constructor(font?: TextboxFontRenderer, audioEngine?: AudioEngine | null) {
    if (!font) {
      throw new Error("GenderSelectionScreen requires the shared textbox font renderer.");
    }
    this.textboxRenderer = new BootTextboxRenderer(font, TILE_SIZE);
    this.backgroundColor = this.loadBackgroundColor();
    this.audioEngine = audioEngine ?? null;
  }

  reset(): void {
    this.fadeCounter = 0;
    this.selectedIndex = 0;
    this.selectedGender = this.options[0];
    this.confirmed = false;
    this.confirmCountdown = 0;
  }

  update(): boolean {
    if (!this.confirmed) {
      if (this.fadeCounter < GenderSelectionScreen.FADE_IN_FRAMES) {
        this.fadeCounter += 1;
      }
      return false;
    }

    if (this.confirmCountdown > 0) {
      this.confirmCountdown -= 1;
      return false;
    }
    return true;
  }

  handleInput(event: InputEventLike): void {
    if (!isKeyDownEvent(event) || this.confirmed) {
      return;
    }
    const direction = mapKeyToDirection(event.direction ?? event.key ?? event.code ?? null);
    if (direction === "up") {
      this.selectedIndex = (this.selectedIndex + this.options.length - 1) %
        this.options.length;
    } else if (direction === "down") {
      this.selectedIndex = (this.selectedIndex + 1) % this.options.length;
    } else if (isConfirmEvent(event)) {
      this.confirmSelection();
    }
  }

  draw(ctx: CanvasRenderingContext2D): void {
    if (ctx.canvas.width === SCREEN_WIDTH && ctx.canvas.height === SCREEN_HEIGHT) {
      this.drawNative(ctx);
      return;
    }

    const surface = new Surface(SCREEN_WIDTH, SCREEN_HEIGHT);
    this.drawNative(surface.getContext() as CanvasRenderingContext2D);

    ctx.fillStyle = `rgb(${this.backgroundColor[0]}, ${this.backgroundColor[1]}, ${this.backgroundColor[2]})`;
    ctx.fillRect(0, 0, ctx.canvas.width, ctx.canvas.height);

    const scale = Math.min(ctx.canvas.width / SCREEN_WIDTH, ctx.canvas.height / SCREEN_HEIGHT);
    const width = Math.round(SCREEN_WIDTH * scale);
    const height = Math.round(SCREEN_HEIGHT * scale);
    const x = Math.floor((ctx.canvas.width - width) / 2);
    const y = Math.floor((ctx.canvas.height - height) / 2);
    const source = surface.getCanvasImageSource();
    if (source) {
      ctx.imageSmoothingEnabled = false;
      ctx.drawImage(source, x, y, width, height);
    }
  }

  private drawNative(ctx: CanvasRenderingContext2D): void {
    fillScreen(ctx, this.backgroundColor);
    this.drawQuestion(ctx);
    this.drawMenuBox(ctx);

    if (this.fadeCounter < GenderSelectionScreen.FADE_IN_FRAMES) {
      const alpha = 1 - this.fadeCounter / GenderSelectionScreen.FADE_IN_FRAMES;
      ctx.fillStyle = `rgba(255, 255, 255, ${alpha})`;
      ctx.fillRect(0, 0, ctx.canvas.width, ctx.canvas.height);
    }
  }

  getSelectedGender(): PlayerGender {
    return this.selectedGender;
  }

  setSelectedGender(gender: PlayerGender): void {
    const normalized = gender === PlayerGender.FEMALE ? PlayerGender.FEMALE : PlayerGender.MALE;
    const selectedIndex = this.options.indexOf(normalized);
    this.selectedIndex = selectedIndex >= 0 ? selectedIndex : 0;
    this.selectedGender = this.options[this.selectedIndex];
  }

  isConfirmed(): boolean {
    return this.confirmed;
  }

  private confirmSelection(): void {
    this.audioEngine?.playSound?.("menu_option");
    this.confirmed = true;
    this.selectedGender = this.options[this.selectedIndex];
    this.confirmCountdown = GenderSelectionScreen.POST_CONFIRM_DELAY;
  }

  private drawQuestion(ctx: CanvasRenderingContext2D): void {
    this.textboxRenderer.drawTextBox(
      ctx,
      GenderSelectionScreen.QUESTION_TEXT,
      0,
      TEXTBOX_Y_TILES,
      SCREEN_TILE_WIDTH,
      TEXTBOX_HEIGHT_TILES
    );
  }

  private drawMenuBox(ctx: CanvasRenderingContext2D): void {
    const [xTiles, yTiles] = GenderSelectionScreen.BOX_TOP_LEFT;
    const [endX, endY] = GenderSelectionScreen.BOX_BOTTOM_RIGHT;
    const widthTiles = endX - xTiles + 1;
    const heightTiles = endY - yTiles + 1;
    this.textboxRenderer.drawWindow(ctx, xTiles, yTiles, widthTiles, heightTiles);

    const optionX = (xTiles + 1) * TILE_SIZE;
    const optionY = (yTiles + 1) * TILE_SIZE;
    const spacing = TILE_SIZE * 2;

    for (let idx = 0; idx < this.optionLabels.length; idx++) {
      const label = this.optionLabels[idx];
      const isSelected = idx === this.selectedIndex;
      const pointer = isSelected ? "\u25b6" : " ";
      this.textboxRenderer.drawText(ctx, pointer, optionX, optionY + idx * spacing);
      this.textboxRenderer.drawText(ctx, label, optionX + TILE_SIZE, optionY + idx * spacing);
    }
  }

  private loadBackgroundColor(): [number, number, number] {
    const palettePath = getAssetPath("gfx", "new_game", "gender_screen.pal");
    const colors: [number, number, number][] = [];
    const lines = fs.readFileSync(palettePath, "utf-8").split("\n");
    for (const raw of lines) {
      const line = raw.trim();
      if (!line.startsWith("RGB")) {
        continue;
      }
      const [rStr, gStr, bStr] = line.split("RGB")[1].trim().split(",");
      const r = Number.parseInt(rStr, 10);
      const g = Number.parseInt(gStr, 10);
      const b = Number.parseInt(bStr, 10);
      colors.push([gbc5To8(r), gbc5To8(g), gbc5To8(b)]);
    }
    const background = colors[1];
    if (!background) {
      throw new Error(`Gender selection palette ${palettePath} is missing color index 1.`);
    }
    return background;
  }

  getTextSnapshot(): TextSnapshotPayload {
    const menuLines = this.optionLabels.map((label, index) =>
      `${index === this.selectedIndex ? "▶" : " "} ${label}`
    );
    return {
      viewportLines: ["GENDER SELECT", GenderSelectionScreen.QUESTION_TEXT],
      infoLines: [
        "STATE: gender",
        `SELECTION: ${this.optionLabels[this.selectedIndex].toLowerCase()}`,
        `CONFIRMED: ${this.confirmed ? "yes" : "no"}`,
        `FADE FRAME: ${this.fadeCounter}/${GenderSelectionScreen.FADE_IN_FRAMES}`,
        ...buildGenderSelectionControlLines(this.confirmed),
      ],
      viewportTitle: "Gender",
      infoTitle: "Gender",
      menuLines,
      promptLines: null,
      dialogueLines: null,
    };
  }
}
