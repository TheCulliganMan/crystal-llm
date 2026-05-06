import { GameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { Command, OverworldContext } from "./base";

type ScriptNumericValue = string | number | boolean | null | undefined;

interface MemoryRegion extends Record<string, unknown> {
  script_memory?: Record<string, number>;
}

interface MemoryResolveResult {
  region: MemoryRegion;
  store: Record<string, number>;
  attr: string;
}

type MemoryRunnerState = {
  variables?: Record<string, number>;
  last_value?: number | null;
};

const cleanToken = (token: string): string => {
  return token.split(";", 1)[0].trim();
};

const parseNumeric = (value: ScriptNumericValue): number => {
  if (typeof value === "number" && Number.isFinite(value)) {
    return Math.trunc(value);
  }
  if (typeof value === "boolean") {
    return value ? 1 : 0;
  }
  if (value === null || value === undefined) {
    return 0;
  }
  if (typeof value === "string") {
    let cleaned = cleanToken(value);
    if (!cleaned) {
      return 0;
    }
    let sign = 1;
    if (cleaned.startsWith("+") || cleaned.startsWith("-")) {
      if (cleaned.startsWith("-")) {
        sign = -1;
      }
      cleaned = cleaned.slice(1).trim();
    }
    let base = 10;
    if (cleaned.startsWith("$")) {
      base = 16;
      cleaned = cleaned.slice(1);
    } else if (cleaned.toLowerCase().startsWith("0x")) {
      base = 16;
      cleaned = cleaned.slice(2);
    }
    if (!cleaned) {
      return 0;
    }
    const parsed = Number.parseInt(cleaned, base);
    return Number.isNaN(parsed) ? 0 : parsed * sign;
  }
  return 0;
};

class MemoryAccessor {
  private readonly gameState: GameState;
  private readonly regionPrefixes: Record<string, string> = {
    s: "sram",
    h: "hram",
    v: "vram",
  };

  constructor(gameState: GameState) {
    this.gameState = gameState;
  }

  public read(address: string): number {
    const { region, store, attr } = this.resolve(address);
    if (attr in region) {
      return parseNumeric(region[attr] as ScriptNumericValue);
    }
    return store[attr] ?? 0;
  }

  public write(address: string, value: ScriptNumericValue): void {
    const { region, store, attr } = this.resolve(address);
    const numeric = parseNumeric(value);
    if (attr in region) {
      region[attr] = numeric;
      return;
    }
    store[attr] = numeric;
  }

  private resolve(address: string): MemoryResolveResult {
    const token = cleanToken(address);
    if (!token) {
      throw new Error("Memory address cannot be empty.");
    }
    const regionName = this.regionPrefixes[token[0].toLowerCase()] ?? "wram";
    const scopedState = this.gameState as GameState & Record<string, unknown>;
    const region = scopedState[regionName] as MemoryRegion | undefined;
    if (!region) {
      throw new Error(`Unknown memory region for address ${address}`);
    }
    let store = region.script_memory;
    if (!store) {
      store = {};
      region.script_memory = store;
    }
    return { region, store, attr: token };
  }
}

export class ReadMemCommand extends Command {
  constructor(private address: string) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const runner = this.runner as MemoryRunnerState | undefined;
    if (!runner) {
      return;
    }
    const accessor = new MemoryAccessor(gameState);
    const value = accessor.read(this.address);
    runner.last_value = value;
    if (!runner.variables) {
      runner.variables = {};
    }
    runner.variables["_value"] = value;
  }
}

export class WriteMemCommand extends Command {
  constructor(private address: string) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const runner = this.runner as MemoryRunnerState | undefined;
    if (!runner) {
      return;
    }
    const value = runner.variables?._value ?? runner.last_value;
    const accessor = new MemoryAccessor(gameState);
    accessor.write(this.address, value);
    runner.last_value = parseNumeric(value);
  }
}

export class LoadMemCommand extends Command {
  constructor(private address: string, private value: string) {
    super();
  }

  public execute(gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const runner = this.runner as MemoryRunnerState | undefined;
    const accessor = new MemoryAccessor(gameState);
    const numeric = parseNumeric(this.value);
    accessor.write(this.address, numeric);
    if (runner) {
      runner.last_value = numeric;
      if (!runner.variables) {
        runner.variables = {};
      }
      runner.variables["_value"] = numeric;
    }
  }
}

export class AddValCommand extends Command {
  constructor(private value: string) {
    super();
  }

  public execute(_gameState: GameState, _eventManager: EventManager, _overworld: OverworldContext): void {
    const runner = this.runner as MemoryRunnerState | undefined;
    if (!runner) {
      return;
    }
    const base = runner.variables?._value ?? runner.last_value;
    const total = parseNumeric(base) + parseNumeric(this.value);
    if (!runner.variables) {
      runner.variables = {};
    }
    runner.variables["_value"] = total;
    runner.last_value = total;
  }
}
