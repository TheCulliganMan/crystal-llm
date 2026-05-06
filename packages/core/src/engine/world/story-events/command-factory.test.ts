import fs from "fs";
import path from "path";

import { getDataDir } from "@pokecrystal/core/core/paths";
import { CommandFactory } from "./command-factory";
import type { ScriptRunner } from "./runner";

type ScriptEntry = {
  command?: unknown;
  args?: unknown;
};

const DATA_DIRECTIVES = new Set([
  "db",
  "dw",
  "dba",
  "dbw",
  "dbb",
  "dbbw",
  "dn",
  "line",
  "cont",
  "next",
  "para",
  "text",
  "text_start",
  "text_end",
  "done",
  "prompt",
  "text_ram",
  "text_decimal",
  "text_promptbutton",
  "menu_coords",
  "scene_script",
  "scene_const",
  "callback",
  "def_scene_scripts",
  "def_callbacks",
  "def_warp_events",
  "def_coord_events",
  "def_bg_events",
  "def_object_events",
  "warp_event",
  "coord_event",
  "bg_event",
  "object_event",
  "elevfloor",
  "stonetable",
  "cmdqueue",
  "writecmdqueue",
  "push",
  "pop",
  "ld",
  "ldh",
  "ret",
  "big_step",
  "step",
  "step_end",
  "step_sleep",
  "slow_step",
  "jump_step",
  "slow_jump_step",
  "fast_jump_step",
  "turn_head",
  "fix_facing",
  "remove_fixed_facing",
  "set_sliding",
  "remove_sliding",
  "teleport_from",
  "skyfall_top",
  "tree_shake",
  "sound_item",
  "wait",
  "_2dmenu",
]);

const BUILTIN_SCRIPT_ALIASES = new Set([
  "jumptext",
  "jumptextfaceplayer",
  "farjumptext",
]);

const collectCommands = (value: unknown, commands: Set<string>): void => {
  if (Array.isArray(value)) {
    for (const entry of value as ScriptEntry[]) {
      if (entry && typeof entry === "object" && "command" in entry) {
        commands.add(String(entry.command).toLowerCase());
      }
    }
    return;
  }

  if (!value || typeof value !== "object") {
    return;
  }

  for (const child of Object.values(value as Record<string, unknown>)) {
    collectCommands(child, commands);
  }
};

describe("CommandFactory exported command coverage", () => {
  it("implements every executable command in exported story events", () => {
    const storyEventsPath = path.join(getDataDir(), "story_events.json");
    const storyEvents = JSON.parse(fs.readFileSync(storyEventsPath, "utf8")) as unknown;
    const exportedCommands = new Set<string>();
    collectCommands(storyEvents, exportedCommands);

    const factory = new CommandFactory({} as ScriptRunner);
    const missing = [...exportedCommands]
      .filter((command) => !factory.commandMap.has(command))
      .filter((command) => !DATA_DIRECTIVES.has(command))
      .filter((command) => !BUILTIN_SCRIPT_ALIASES.has(command))
      .sort();

    expect(missing).toEqual([]);
  });
});
