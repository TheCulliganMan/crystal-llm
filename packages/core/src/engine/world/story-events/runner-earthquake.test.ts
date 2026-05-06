import fs from "fs";
import path from "path";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { getDataDir } from "@pokecrystal/core/core/paths";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { ScriptRunnerImpl } from "./runner";

describe("ScriptRunner earthquake command", () => {
  it("decodes the ASM byte parameter for intensity and duration", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    dataLoader.get_script = (name: string) =>
      name === "TestEarthquake"
        ? [{ command: "earthquake", args: ["$50"] }, { command: "end", args: [] }]
        : null;

    const start_earthquake = jest.fn();
    const overworld = { start_earthquake } as unknown as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);

    runner.run("TestEarthquake");

    expect(start_earthquake).toHaveBeenCalledTimes(1);
    expect(start_earthquake).toHaveBeenCalledWith(2, 16);
  });

  it("throws a descriptive error when the byte parameter is missing", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    dataLoader.get_script = (name: string) =>
      name === "BadEarthquake"
        ? [{ command: "earthquake", args: [] }, { command: "end", args: [] }]
        : null;

    const overworld = {} as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);

    expect(() => runner.run("BadEarthquake")).toThrow("undefined numeric token");
  });

  it("accepts numeric args payloads by coercing them to script tokens", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    dataLoader.get_script = (name: string) =>
      name === "NumericEarthquake"
        ? [{ command: "earthquake", args: 0x50 as unknown as [] }, { command: "end", args: [] }]
        : null;

    const start_earthquake = jest.fn();
    const overworld = { start_earthquake } as unknown as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);

    runner.run("NumericEarthquake");

    expect(start_earthquake).toHaveBeenCalledWith(2, 16);
  });

  it("parses all shipped earthquake scripts from story events data", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const overworld = {} as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);

    const storyEventsPath = path.join(getDataDir(), "story_events.json");
    const storyEvents = JSON.parse(fs.readFileSync(storyEventsPath, "utf8")) as Record<string, unknown>;
    const earthquakeScripts: Array<{ mapName: string; label: string; script: unknown[] }> = [];

    for (const [mapName, payload] of Object.entries(storyEvents)) {
      if (!payload || typeof payload !== "object") {
        continue;
      }
      for (const [label, script] of Object.entries(payload as Record<string, unknown>)) {
        if (!Array.isArray(script)) {
          continue;
        }
        const hasEarthquake = script.some((entry) => {
          if (!entry || typeof entry !== "object") {
            return false;
          }
          return String((entry as { command?: unknown }).command ?? "").trim().toLowerCase() === "earthquake";
        });
        if (hasEarthquake) {
          earthquakeScripts.push({ mapName, label, script });
        }
      }
    }

    expect(earthquakeScripts.length).toBeGreaterThan(0);
    for (const scriptEntry of earthquakeScripts) {
      expect(() => runner.parse(scriptEntry.script as any)).not.toThrow();
    }
  });
});
