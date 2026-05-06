import { createInitialGameState } from "@pokecrystal/core/core/state";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { elmsLabScripts } from "@pokecrystal/assets/content/data/scripts";
import { ScriptRunnerImpl } from "./runner";

describe("Elms Lab scripts", () => {
  it("executes scall scripts in order during the aide walk sequence", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "KRIS";
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const scripts: Record<string, Array<{ command: string; args: string[] }>> = {
      AideScript_WalkPotion2: [
        { command: "applymovement", args: ["ELMSLAB_ELMS_AIDE", "AideWalksRight2"] },
        { command: "turnobject", args: ["PLAYER", "DOWN"] },
        { command: "scall", args: ["AideScript_GivePotion"] },
        { command: "applymovement", args: ["ELMSLAB_ELMS_AIDE", "AideWalksLeft2"] },
        { command: "end", args: [] },
      ],
      AideScript_GivePotion: [
        { command: "opentext", args: [] },
        { command: "writetext", args: ["AideText_GiveYouPotion"] },
        { command: "waitbutton", args: [] },
        { command: "closetext", args: [] },
        { command: "end", args: [] },
      ],
    };
    const textMap: Record<string, string> = {
      AideText_GiveYouPotion: "<PLAY_G>, take this potion.",
    };
    dataLoader.get_script = (name: string) => scripts[name] ?? null;
    dataLoader.get_text = (label: string) => textMap[label] ?? label;

    const aide = {
      object_id: "ELMSLAB_ELMS_AIDE",
      x: 0,
      y: 0,
      apply_movement: () => undefined,
    };
    const player = {
      object_id: "PLAYER",
      x: 0,
      y: 0,
      turn: () => undefined,
    };
    const overworld = {
      current_map_name: "ElmsLab",
      get_object_by_id: (id: string) => (id === "ELMSLAB_ELMS_AIDE" ? aide : id === "PLAYER" ? player : null),
      get_movement_data: (label: string) =>
        label === "AideWalksRight2" || label === "AideWalksLeft2" ? ["step"] : null,
    } as unknown as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    const events: string[] = [];

    eventManager.on("open_text", (event) => events.push(event.name));
    eventManager.on("show_text", (event) => events.push(event.name));

    runner.run("AideScript_WalkPotion2");

    expect(events).toContain("open_text");
    expect(events).toContain("show_text");
  });

  it("opens dialogue when Elm's aide gives the potion", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "KRIS";
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const textMap: Record<string, string> = {
      AideText_GiveYouPotion: "<PLAY_G>, I want\nyou to have this\nfor your errand.",
      AideText_AlwaysBusy: "There are only two\nof us, so we're\nalways busy.",
    };
    dataLoader.get_script = (name: string) => elmsLabScripts[name] ?? null;
    dataLoader.get_text = (label: string) => textMap[label] ?? label;
    const overworld = { current_map_name: "ElmsLab" } as unknown as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    const events: Array<{ name: string; data?: Record<string, unknown> }> = [];

    for (const name of ["open_text", "show_text", "wait_for_input"]) {
      eventManager.on(name, (event) => {
        events.push({ name: event.name, data: event.data as Record<string, unknown> });
      });
    }

    runner.run("AideScript_GivePotion");

    expect(events[0]?.name).toBe("open_text");
    expect(events[1]?.name).toBe("show_text");
    expect(String(events[1]?.data?.text ?? "")).toContain("have this");
    expect(events[2]?.name).toBe("wait_for_input");
    expect(runner.stopExecution).toBe(true);
    expect(runner._awaiting_resume).toBe(1);
  });

  it("displays the potion dialogue text after the prompt resumes", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "KRIS";
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const textMap: Record<string, string> = {
      AideText_GiveYouPotion: "<PLAY_G>, I want\nyou to have this\nfor your errand.",
      AideText_AlwaysBusy: "There are only two\nof us, so we're\nalways busy.",
    };
    dataLoader.get_script = (name: string) => elmsLabScripts[name] ?? null;
    dataLoader.get_text = (label: string) => textMap[label] ?? label;
    const overworld = { current_map_name: "ElmsLab" } as unknown as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    const showTexts: string[] = [];

    eventManager.on("show_text", (event) => {
      const text = event.data?.text;
      if (typeof text === "string") {
        showTexts.push(text);
      }
    });

    runner.run("AideScript_GivePotion");
    runner.resume();
    runner.resume();

    expect(showTexts[0]).toBe("KRIS, I want\nyou to have this\nfor your errand.");
    expect(showTexts[1]).toBe("KRIS received\nPOTION.");
    expect(showTexts[2]).toBe("There are only two\nof us, so we're\nalways busy.");
  });

  it("shows Elm dialogue after receiving a pokemon", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "KRIS";
    gameState.wram.event_flags.EVENT_GOT_A_POKEMON_FROM_ELM = true;
    gameState.wram.last_talked = 1;
    const eventManager = new EventManager(gameState);
    const dataLoader = new DataLoader();
    const overworld = {
      current_map_name: "ElmsLab",
      get_object_by_id: (id: number | string) => {
        const numeric = Number(id);
        if (numeric === 0) {
          return { object_id: "PLAYER", x: 0, y: 0 };
        }
        if (numeric === 1) {
          return { object_id: "ELMSLAB_ELM", facePlayer: () => undefined };
        }
        return null;
      },
    } as unknown as OverworldEngine;
    const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
    const showTexts: string[] = [];

    eventManager.on("show_text", (event) => {
      const text = event.data?.text;
      if (typeof text === "string") {
        showTexts.push(text);
      }
    });

    runner.run("ProfElmScript");

    const rawText = dataLoader.get_text("ElmDescribesMrPokemonText");
    expect(showTexts.length).toBeGreaterThan(0);
    if (rawText) {
      expect(showTexts[0]).toBe(runner.formatText(rawText));
    }
  });
});
