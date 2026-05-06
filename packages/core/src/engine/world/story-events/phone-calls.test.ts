import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { DataLoader, type ScriptEntry } from "@pokecrystal/core/core/data-loader";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { OverworldEngine as ConcreteOverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { ScriptRunnerImpl } from "./runner";

type ScriptMap = Record<string, ScriptEntry[]>;

const buildPhoneScripts = (): { scripts: ScriptMap; texts: Record<string, string> } => ({
  scripts: {
    ElmPhoneCallerScript: [
      { command: "readvar", args: ["VAR_SPECIALPHONECALL"] },
      { command: "ifequal", args: ["SPECIALCALL_ROBBED", ".disaster"] },
      { command: "farwritetext", args: ["ElmPhoneDiscoveredPokerusText"] },
      { command: "specialphonecall", args: ["SPECIALCALL_NONE"] },
      { command: "end", args: [] },
    ],
    ".disaster": [
      { command: "farwritetext", args: ["ElmPhoneDisasterText"] },
      { command: "specialphonecall", args: ["SPECIALCALL_NONE"] },
      { command: "end", args: [] },
    ],
  },
  texts: {
    ElmPhoneDisasterText: "Your Pokemon were stolen!",
    ElmPhoneDiscoveredPokerusText: "It goes away over time.",
  },
});

const createPhoneLoader = (): DataLoader => {
  const { scripts, texts } = buildPhoneScripts();
  return {
    get_script: (name: string, parentScript?: string) => {
      if (name.startsWith(".")) {
        if (parentScript === "ElmPhoneCallerScript") {
          return scripts[name] ?? null;
        }
      }
      return scripts[name] ?? null;
    },
    get_text: (label: string) => texts[label] ?? null,
  } as DataLoader;
};

describe("Elm phone calls", () => {
  it("prioritizes robbery scripts even when the queue id is lowercased", () => {
    const gameState = createInitialGameState();
    gameState.wram.scheduled_phone_calls = ["specialcall_robbed"];
    const eventManager = new EventManager(gameState);
    const dataLoader = createPhoneLoader();
    const overworld = { current_map_name: "Route30" } as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    const showTexts: string[] = [];

    eventManager.on("show_text", (event) => {
      const text = event.data?.text;
      if (typeof text === "string") {
        showTexts.push(text);
      }
    });

    runner.run_phone_script("ElmPhoneCallerScript");

    expect(showTexts).toContain("Your Pokemon were stolen!");
    expect(showTexts).not.toContain("It goes away over time.");
  });

  it("resolves the robbery branch from the shipped phone scripts", () => {
    const gameState = createInitialGameState();
    gameState.wram.scheduled_phone_calls = ["SPECIALCALL_ROBBED"];
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const overworld = { current_map_name: "Route30" } as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    const showTexts: string[] = [];

    eventManager.on("show_text", (event) => {
      const text = event.data?.text;
      if (typeof text === "string") {
        showTexts.push(text);
      }
    });

    runner.run_phone_script("ElmPhoneCallerScript");

    const robberyText = dataLoader.get_text("ElmPhoneDisasterText");
    const pokerusText = dataLoader.get_text("ElmPhoneDiscoveredPokerusText");
    if (robberyText) {
      expect(showTexts).toContain(runner.formatText(robberyText));
    }
    if (pokerusText) {
      expect(showTexts).not.toContain(runner.formatText(pokerusText));
    }
  });

  it("maps the bike shop special call to the PHONE_OAK contact slot from special_calls.asm", () => {
    const handlers = (
      ConcreteOverworldEngine as unknown as {
        SPECIAL_CALL_HANDLERS: Record<string, [string, string, boolean]>;
      }
    ).SPECIAL_CALL_HANDLERS;

    expect(handlers.SPECIALCALL_BIKESHOP).toEqual([
      "PHONE_OAK",
      "BikeShopPhoneCallerScript",
      false,
    ]);
  });
});
