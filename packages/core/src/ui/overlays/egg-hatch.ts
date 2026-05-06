// Overlay that replays the Game Boy's egg hatch vignette.
// ASM reference: engine/overworld/egg_hatching.asm (HatchEggs timeline).

import { Surface } from '@pokecrystal/core/ui/game-engine';
import { AudioEngine } from '@pokecrystal/core/engine/systems/audio';

export interface UI {
  tileSize: number;
  loadSprite(speciesId: string, spriteType?: string): void;
  _getPokemonFrameSurface(speciesId: string, frame: number): Surface | null;
}

class ShellFragment {
  constructor(
    public x: number,
    public y: number,
    public vx: number,
    public vy: number,
    public lifetime: number
  ) {}

  update(): void {
    this.x += this.vx;
    this.y += this.vy;
    this.vy += 0.28;
    this.vx *= 0.98;
    this.lifetime -= 1;
  }
}

function withContext(
  surface: Surface
): CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D {
  const ctx = surface.canvas.getContext('2d');
  if (!ctx) {
    throw new Error('Failed to acquire 2D context for egg hatch render.');
  }
  return ctx as CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D;
}

function drawEllipse(
  surface: Surface,
  rect: { x: number; y: number; width: number; height: number },
  colour: [number, number, number],
  lineWidth?: number
): void {
  const ctx = withContext(surface);
  const centerX = rect.x + rect.width / 2;
  const centerY = rect.y + rect.height / 2;
  ctx.save();
  ctx.beginPath();
  ctx.ellipse(
    centerX,
    centerY,
    rect.width / 2,
    rect.height / 2,
    0,
    0,
    Math.PI * 2
  );
  ctx.strokeStyle = `rgb(${colour[0]}, ${colour[1]}, ${colour[2]})`;
  ctx.fillStyle = ctx.strokeStyle;
  if (lineWidth && lineWidth > 0) {
    ctx.lineWidth = lineWidth;
    ctx.stroke();
  } else {
    ctx.fill();
  }
  ctx.restore();
}

function drawRect(
  surface: Surface,
  rect: { x: number; y: number; width: number; height: number },
  colour: [number, number, number]
): void {
  const ctx = withContext(surface);
  ctx.fillStyle = `rgb(${colour[0]}, ${colour[1]}, ${colour[2]})`;
  ctx.fillRect(rect.x, rect.y, rect.width, rect.height);
}

type EggHatchRenderContext = CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D;

function applyAlpha(source: Surface, alpha: number): Surface {
  const result = new Surface(source.get_width(), source.get_height());
  const ctx = result.canvas.getContext("2d") as EggHatchRenderContext | null;
  if (!ctx) {
    throw new Error('Failed to acquire 2D context for sprite alpha.');
  }
  const sourceImage = source.getImageData();
  const output = ctx.createImageData(sourceImage.width, sourceImage.height);
  const alphaScale = Math.max(0, Math.min(1, alpha / 255));
  for (let offset = 0; offset < sourceImage.data.length; offset += 4) {
    output.data[offset] = sourceImage.data[offset] ?? 0;
    output.data[offset + 1] = sourceImage.data[offset + 1] ?? 0;
    output.data[offset + 2] = sourceImage.data[offset + 2] ?? 0;
    output.data[offset + 3] = Math.round((sourceImage.data[offset + 3] ?? 0) * alphaScale);
  }
  ctx.putImageData(output, 0, 0);
  return result;
}

export class EggHatchAnimation {
  static readonly INTRO_FRAMES = 16;
  static readonly SHAKE_FRAMES = 56;
  static readonly CRACK_FRAMES = 34;
  static readonly REVEAL_FRAMES = 40;
  static readonly HOLD_FRAMES = 40;
  static readonly EGG_WIDTH = 32;
  static readonly EGG_HEIGHT = 44;
  static readonly FRAGMENT_VELOCITY: Array<[number, number]> = [
    [-2.2, -2.6],
    [2.2, -2.2],
    [-1.8, -1.6],
    [1.8, -1.2],
    [-1.2, -0.6],
    [1.2, -0.8],
  ];

  private readonly ui: UI;
  private readonly audioEngine: AudioEngine | null;
  private readonly speciesId: string;
  private phase: 'intro' | 'shake' | 'crack' | 'reveal' | 'hold' | 'complete' = 'intro';
  private timer = EggHatchAnimation.INTRO_FRAMES;
  private shakeAngle = 0;
  private fragments: ShellFragment[] = [];
  private revealAlpha = 0;
  private spriteSurface: Surface;
  private musicStarted = false;

  constructor(
    ui: UI,
    options: { audioEngine?: AudioEngine | null; speciesId: string }
  ) {
    this.ui = ui;
    this.audioEngine = options.audioEngine ?? null;
    this.speciesId = options.speciesId;
    this.spriteSurface = this.loadSprite();
    this.startEvolutionMusic();
  }

  private loadSprite(): Surface {
    this.ui.loadSprite(this.speciesId, 'pokemon_front');
    const surface = this.ui._getPokemonFrameSurface(this.speciesId, 0);
    if (!surface) {
      throw new Error(
        `Unable to load front sprite for '${this.speciesId}' during hatch.`
      );
    }
    return surface;
  }

  advance(): void {
    if (this.phase === 'intro') {
      this.timer -= 1;
      if (this.timer <= 0) {
        this.enterShakePhase();
      }
      return;
    }
    if (this.phase === 'shake') {
      this.shakeAngle += Math.PI / 16;
      this.timer -= 1;
      if (this.timer <= 0) {
        this.enterCrackPhase();
      }
      return;
    }
    if (this.phase === 'crack') {
      this.timer -= 1;
      this.updateFragments();
      if (this.timer <= 0) {
        this.enterRevealPhase();
      }
      return;
    }
    if (this.phase === 'reveal') {
      this.timer -= 1;
      this.revealAlpha = Math.min(
        255,
        this.revealAlpha + Math.floor(255 / Math.max(1, EggHatchAnimation.REVEAL_FRAMES))
      );
      if (this.timer <= 0) {
        this.enterHoldPhase();
      }
      return;
    }
    if (this.phase === 'hold') {
      this.timer -= 1;
      if (this.timer <= 0) {
        this.phase = 'complete';
      }
    }
  }

  draw(surface: Surface): void {
    if (this.phase === 'complete') {
      return;
    }
    surface.fill([255, 255, 255, 255]);
    const centerX = Math.floor(surface.get_width() / 2);
    const centerY = Math.floor(surface.get_height() / 2);
    const offset = this.computeOffset();
    if (['intro', 'shake', 'crack'].includes(this.phase)) {
      this.drawEgg(surface, centerX + offset, centerY);
    }
    if (this.fragments.length) {
      this.drawFragments(surface, centerX, centerY);
    }
    if (['reveal', 'hold', 'complete'].includes(this.phase)) {
      this.drawSprite(surface, centerX, centerY);
    }
  }

  isFinished(): boolean {
    return this.phase === 'complete';
  }

  private startEvolutionMusic(): void {
    if (!this.audioEngine || this.musicStarted) {
      return;
    }
    try {
      this.audioEngine.playMusic("MUSIC_EVOLUTION", "evolution");
      this.musicStarted = true;
    } catch (error) {
      this.musicStarted = true;
    }
  }

  private playSound(sound: string): void {
    if (!this.audioEngine) {
      return;
    }
    try {
      this.audioEngine.playSound(sound);
    } catch (error) {
      // ignore missing sound
    }
  }

  private enterShakePhase(): void {
    this.phase = 'shake';
    this.timer = EggHatchAnimation.SHAKE_FRAMES;
    this.playSound('SFX_EGG_HATCH');
  }

  private enterCrackPhase(): void {
    this.phase = 'crack';
    this.timer = EggHatchAnimation.CRACK_FRAMES;
    this.spawnFragments();
    this.playSound('SFX_EGG_CRACK');
  }

  private enterRevealPhase(): void {
    this.phase = 'reveal';
    this.timer = EggHatchAnimation.REVEAL_FRAMES;
    this.revealAlpha = 0;
    this.playSound('SFX_EGG_HATCH');
  }

  private enterHoldPhase(): void {
    this.phase = 'hold';
    this.timer = EggHatchAnimation.HOLD_FRAMES;
    this.fragments = [];
  }

  private computeOffset(): number {
    if (this.phase !== 'shake') {
      return 0;
    }
    return Math.floor(Math.sin(this.shakeAngle) * 6);
  }

  private drawEgg(surface: Surface, x: number, y: number): void {
    const eggRect = {
      x: x - EggHatchAnimation.EGG_WIDTH / 2,
      y: y - EggHatchAnimation.EGG_HEIGHT / 2,
      width: EggHatchAnimation.EGG_WIDTH,
      height: EggHatchAnimation.EGG_HEIGHT,
    };
    drawEllipse(surface, eggRect, [228, 228, 224]);
    const highlight = {
      x: eggRect.x + 4,
      y: eggRect.y + 4 - 6,
      width: eggRect.width - 8,
      height: eggRect.height - 8,
    };
    drawEllipse(surface, highlight, [255, 255, 255]);
    const outline = {
      x: eggRect.x + 2,
      y: eggRect.y + 2,
      width: eggRect.width - 4,
      height: eggRect.height - 4,
    };
    drawEllipse(surface, outline, [200, 200, 200], 1);
  }

  private drawFragments(surface: Surface, centerX: number, centerY: number): void {
    for (const fragment of this.fragments) {
      if (fragment.lifetime <= 0) {
        continue;
      }
      drawRect(surface, {
        x: Math.floor(centerX + fragment.x),
        y: Math.floor(centerY + fragment.y),
        width: 6,
        height: 2,
      }, [196, 196, 196]);
    }
  }

  private drawSprite(surface: Surface, centerX: number, centerY: number): void {
    const sprite = applyAlpha(this.spriteSurface, this.revealAlpha);
    const spriteX = Math.floor(centerX - sprite.get_width() / 2);
    const spriteY = Math.floor(centerY - sprite.get_height() / 2 + 8);
    surface.blit(sprite, [spriteX, spriteY]);
  }

  private spawnFragments(): void {
    const magnitude = 1.5;
    const baseY = EggHatchAnimation.EGG_HEIGHT / 2;
    for (const [vx, vy] of EggHatchAnimation.FRAGMENT_VELOCITY) {
      this.fragments.push(
        new ShellFragment(
          0,
          baseY,
          vx * magnitude,
          vy * magnitude,
          36
        )
      );
    }
  }

  private updateFragments(): void {
    for (const fragment of this.fragments) {
      fragment.update();
    }
    this.fragments = this.fragments.filter((fragment) => fragment.lifetime > 0);
  }
}
