// Mirrors the frontpic animation command stream used by the original engine.
import { readJsonAsset, readJsonAssetSync } from "@pokecrystal/core/core/asset-reader";
import { getDataDir } from "@pokecrystal/core/core/paths";

export type FrontpicAnimCommand =
  | { kind: "frame"; frame: number; duration: number }
  | { kind: "setrepeat"; count: number }
  | { kind: "dorepeat"; target: number }
  | { kind: "endanim" };

export type FrontpicAnimProgram = {
  commands: FrontpicAnimCommand[];
};

const parse_number = (token: string): number => {
  const cleaned = token.trim();
  if (!cleaned) {
    return 0;
  }
  if (cleaned.startsWith("$")) {
    return parseInt(cleaned.slice(1), 16);
  }
  return Number.parseInt(cleaned, 10);
};

export const parse_frontpic_anim_script = (source: string): FrontpicAnimProgram => {
  const commands: FrontpicAnimCommand[] = [];
  const lines = source.split(/\r?\n/);
  for (const rawLine of lines) {
    const line = rawLine.split(";", 1)[0].trim();
    if (!line) {
      continue;
    }
    const parts = line.split(/[\s,]+/).filter(Boolean);
    if (!parts.length) {
      continue;
    }
    const opcode = parts[0].toLowerCase();
    if (opcode === "frame" && parts.length >= 3) {
      commands.push({
        kind: "frame",
        frame: parse_number(parts[1]),
        duration: parse_number(parts[2]),
      });
      continue;
    }
    if (opcode === "setrepeat" && parts.length >= 2) {
      commands.push({ kind: "setrepeat", count: parse_number(parts[1]) });
      continue;
    }
    if (opcode === "dorepeat" && parts.length >= 2) {
      commands.push({ kind: "dorepeat", target: parse_number(parts[1]) });
      continue;
    }
    if (opcode === "endanim") {
      commands.push({ kind: "endanim" });
      continue;
    }
  }
  return { commands };
};

const program_cache = new Map<string, FrontpicAnimProgram>();
const pending_program_loads = new Map<string, Promise<FrontpicAnimProgram | null>>();
const FRONTPIC_ANIM_BUNDLE_PATH = `${getDataDir()}/pokemon_frontpic_anim.json`;
let bundle_loaded = false;

export const register_frontpic_anim_scripts = (entries: Record<string, FrontpicAnimProgram>): void => {
  program_cache.clear();
  pending_program_loads.clear();
  bundle_loaded = true;
  for (const [name, program] of Object.entries(entries)) {
    program_cache.set(name.toLowerCase(), program);
  }
};

const load_frontpic_anim_program_browser = (normalized: string): Promise<FrontpicAnimProgram | null> => {
  const pending = pending_program_loads.get(normalized);
  if (pending) {
    return pending;
  }
  const request = (async () => {
    try {
      const bundle = await readJsonAsset<Record<string, FrontpicAnimProgram>>(FRONTPIC_ANIM_BUNDLE_PATH);
      for (const [name, program] of Object.entries(bundle ?? {})) {
        program_cache.set(name.toLowerCase(), program);
      }
      bundle_loaded = true;
      return program_cache.get(normalized) ?? null;
    } catch {
      return null;
    } finally {
      pending_program_loads.delete(normalized);
    }
  })();
  pending_program_loads.set(normalized, request);
  return request;
};

const load_frontpic_anim_bundle_sync = (): void => {
  if (bundle_loaded) {
    return;
  }
  try {
    const bundle = readJsonAssetSync<Record<string, FrontpicAnimProgram>>(FRONTPIC_ANIM_BUNDLE_PATH);
    for (const [name, program] of Object.entries(bundle ?? {})) {
      program_cache.set(name.toLowerCase(), program);
    }
  } catch {
    // Missing generated data disables frontpic animation without blocking battles.
  } finally {
    bundle_loaded = true;
  }
};

export const ensure_frontpic_anim_program = (speciesId: string): boolean => {
  const normalized = String(speciesId || "").trim().toLowerCase();
  if (!normalized) {
    return false;
  }
  if (program_cache.has(normalized)) {
    return true;
  }
  if (typeof window === "undefined") {
    load_frontpic_anim_bundle_sync();
    return program_cache.has(normalized);
  }
  void load_frontpic_anim_program_browser(normalized);
  return false;
};

export const is_frontpic_anim_program_pending = (speciesId: string): boolean => {
  const normalized = String(speciesId || "").trim().toLowerCase();
  if (!normalized) {
    return false;
  }
  return pending_program_loads.has(normalized);
};

export const resolve_frontpic_anim_program = (speciesId: string): FrontpicAnimProgram | null => {
  const normalized = String(speciesId || "").trim().toLowerCase();
  if (!normalized) {
    return null;
  }
  const cached = program_cache.get(normalized);
  if (cached) {
    return cached;
  }
  if (typeof window !== "undefined") {
    void load_frontpic_anim_program_browser(normalized);
    return null;
  }
  load_frontpic_anim_bundle_sync();
  return program_cache.get(normalized) ?? null;
};

const duration_with_speed = (duration: number, speed: number): number => {
  const base = Math.max(0, Math.trunc(duration));
  const scaled = Math.floor((base * Math.max(0, Math.trunc(speed))) / 16);
  return base + scaled;
};

export class FrontpicAnimator {
  private pointer = 0;
  private repeat = 0;
  private wait = 0;
  private currentFrame = 0;
  public complete = false;

  constructor(private readonly program: FrontpicAnimProgram, private readonly speed: number) {}

  reset(): void {
    this.pointer = 0;
    this.repeat = 0;
    this.wait = 0;
    this.currentFrame = 0;
    this.complete = false;
  }

  step(): { frame: number; complete: boolean } {
    if (this.complete) {
      return { frame: this.currentFrame, complete: true };
    }
    if (this.wait > 0) {
      this.wait = Math.max(0, this.wait - 1);
      return { frame: this.currentFrame, complete: this.complete };
    }
    const commands = this.program.commands;
    let guard = 0;
    while (guard < commands.length + 4) {
      const command = commands[this.pointer];
      this.pointer += 1;
      guard += 1;
      if (!command) {
        this.complete = true;
        return { frame: this.currentFrame, complete: true };
      }
      switch (command.kind) {
        case "setrepeat":
          this.repeat = Math.max(0, command.count);
          continue;
        case "dorepeat":
          if (this.repeat > 0) {
            this.repeat -= 1;
            if (this.repeat > 0) {
              this.pointer = Math.max(0, command.target);
            }
          }
          continue;
        case "endanim":
          this.complete = true;
          return { frame: this.currentFrame, complete: true };
        case "frame":
          this.currentFrame = command.frame;
          this.wait = duration_with_speed(command.duration, this.speed);
          return { frame: this.currentFrame, complete: this.complete };
      }
    }
    throw new Error("Frontpic animation exceeded command guard.");
  }
}
