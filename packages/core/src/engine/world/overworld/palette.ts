// Bundled NPC palette table asset: apps/web/assets/gfx/overworld/npc_sprites.pal.
import { readTextAssetSync } from "@pokecrystal/core/core/asset-reader";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { gbc5To8 } from "@pokecrystal/core/core/gbc-colors";

const TIME_OF_DAY_SECTIONS: Record<string, string> = {
  morn: "morn",
  morning: "morn",
  day: "day",
  nite: "nite",
  night: "nite",
  dark: "dark",
};

type Palette = [number, number, number][];

type PaletteGroups = Record<string, Palette[]>;

const NPC_SPRITES_PALETTE_PATH = getAssetPath("gfx", "overworld", "npc_sprites.pal");

const parsePaletteChannel = (value: string, context: string): number => {
  const parsed = Number.parseInt(value, 10);
  if (!Number.isFinite(parsed)) {
    throw new Error(`Invalid overworld palette channel '${value}' in ${context}.`);
  }
  return gbc5To8(parsed, context);
};

function parsePaletteFile(palettePath: string): PaletteGroups {
  const sections: PaletteGroups = {
    morn: [],
    day: [],
    nite: [],
    dark: [],
  };

  let current: keyof PaletteGroups | null = null;
  let content: string;
  try {
    content = readTextAssetSync(palettePath);
  } catch (error) {
    const reason = error instanceof Error ? error.message : String(error);
    throw new Error(`Missing overworld palette asset at ${palettePath}: ${reason}`);
  }
  for (const rawLine of content.split("\n")) {
    const line = rawLine.trim();
    if (!line) {
      continue;
    }
    if (line.startsWith(";")) {
      const header = line.slice(1).trim().toLowerCase();
      if (header in sections) {
        current = header as keyof PaletteGroups;
      }
      continue;
    }
    if (!line.toUpperCase().startsWith("RGB") || current === null) {
      continue;
    }
    const payload = line.split(";", 1)[0];
    const values = payload
      .slice(3)
      .split(",")
      .map((token) => token.trim())
      .filter(Boolean);
    if (values.length !== 12) {
      throw new Error(
        `Palette line '${line}' in ${palettePath} produced ${values.length} channels; expected 12.`
      );
    }
    const colours: Palette = [];
    for (let index = 0; index < 12; index += 3) {
      const r = parsePaletteChannel(values[index], `${palettePath} r@${index}`);
      const g = parsePaletteChannel(values[index + 1], `${palettePath} g@${index + 1}`);
      const b = parsePaletteChannel(values[index + 2], `${palettePath} b@${index + 2}`);
      colours.push([r, g, b]);
    }
    sections[current].push(colours);
  }

  for (const [key, palettes] of Object.entries(sections)) {
    if (palettes.length !== 8) {
      throw new Error(
        `Palette section '${key}' in ${palettePath} produced ${palettes.length} entries; expected 8.`
      );
    }
  }

  return sections;
}

export class NpcPaletteManager {
  private readonly palettes: PaletteGroups;

  constructor() {
    this.palettes = parsePaletteFile(NPC_SPRITES_PALETTE_PATH);
  }

  private static normaliseTimeOfDay(label?: string | null): string {
    if (!label) {
      return "day";
    }
    const key = label.trim().toLowerCase();
    return TIME_OF_DAY_SECTIONS[key] ?? "day";
  }

  public normalise_time_of_day(label?: string | null): string {
    return NpcPaletteManager.normaliseTimeOfDay(label);
  }

  public normaliseTimeOfDay(label?: string | null): string {
    return NpcPaletteManager.normaliseTimeOfDay(label);
  }

  public palette(paletteId: number, timeOfDay?: string | null): Palette {
    const group = this.palettes[NpcPaletteManager.normaliseTimeOfDay(timeOfDay)];
    const index = Math.max(0, Math.min(group.length - 1, paletteId & 0x7));
    return group[index];
  }

  public apply(
    surface: InstanceType<typeof gameEngine.Surface>,
    paletteId: number,
    timeOfDay?: string | null
  ): InstanceType<typeof gameEngine.Surface> {
    if (typeof surface.get_width !== "function" || typeof surface.get_height !== "function") {
      return surface;
    }
    const palette = this.palette(paletteId, timeOfDay);
    const width = surface.get_width();
    const height = surface.get_height();
    const tinted = new gameEngine.Surface(width, height);

    for (let y = 0; y < height; y += 1) {
      for (let x = 0; x < width; x += 1) {
        const [r, g, b, a] = surface.get_at([x, y]);
        if (a === 0) {
          continue;
        }
        const intensity = r;
        let colour: [number, number, number] | null = null;
        let alpha = 255;
        if (intensity === 0xff) {
          colour = palette[0];
          alpha = 0;
        } else if (intensity === 0xaa) {
          colour = palette[1];
        } else if (intensity === 0x55) {
          colour = palette[2];
        } else if (intensity === 0x00) {
          colour = palette[3];
        }
        if (!colour) {
          continue;
        }
        tinted.set_at([x, y], [colour[0], colour[1], colour[2], alpha]);
      }
    }
    return tinted;
  }

  public apply_many(
    frames: Iterable<InstanceType<typeof gameEngine.Surface>>,
    paletteId: number,
    timeOfDay?: string | null
  ): InstanceType<typeof gameEngine.Surface>[] {
    const output: InstanceType<typeof gameEngine.Surface>[] = [];
    for (const frame of frames) {
      output.push(this.apply(frame, paletteId, timeOfDay));
    }
    return output;
  }
}
