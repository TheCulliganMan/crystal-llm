import fs from "fs";
import os from "os";
import path from "path";
import {
  PHONE_CALLASM_ENTRYPOINTS,
  parsePhoneScriptCatalog,
} from "./export-phone-scripts";
import type { StoryCommand, StoryScripts } from "./export-story-events";

const resolveLocalLabel = (sourceLabel: string, targetLabel: string): string => {
  if (!targetLabel.startsWith(".") || targetLabel.includes("@")) {
    return targetLabel;
  }
  const parent = sourceLabel.includes("@")
    ? sourceLabel.slice(sourceLabel.indexOf("@") + 1)
    : sourceLabel;
  return `${targetLabel}@${parent}`;
};

const callasmTarget = (sourceLabel: string, targetLabel: string): string =>
  resolveLocalLabel(sourceLabel, targetLabel);

const cpuControlTarget = (
  sourceLabel: string,
  command: StoryCommand,
): string | null => {
  if (!Array.isArray(command.args)) {
    return null;
  }
  if (
    !["call", "farcall", "jp", "jr"].includes(command.command) ||
    (command.args.length !== 1 && command.args.length !== 2)
  ) {
    return null;
  }
  return resolveLocalLabel(sourceLabel, command.args.at(-1)!);
};

const reachableCpuInventory = (
  scripts: StoryScripts,
  roots: readonly string[],
): { definitions: Set<string>; externalTargets: Set<string> } => {
  const definitions = new Set<string>();
  const externalTargets = new Set<string>();
  const pending = [...roots];
  while (pending.length > 0) {
    const sourceLabel = pending.shift()!;
    if (definitions.has(sourceLabel)) {
      continue;
    }
    const commands = scripts[sourceLabel];
    if (!commands) {
      externalTargets.add(sourceLabel);
      continue;
    }
    definitions.add(sourceLabel);
    for (const command of commands) {
      const target = cpuControlTarget(sourceLabel, command);
      if (!target) {
        continue;
      }
      if (scripts[target]) {
        pending.push(target);
      } else {
        externalTargets.add(target);
      }
    }
  }
  return { definitions, externalTargets };
};

describe("export-phone-scripts", () => {
  it("closes all five phone callasm CPU entrypoints over exact canonical shared sources", () => {
    const repositoryRoot = path.resolve(__dirname, "../../../..");
    const scripts = parsePhoneScriptCatalog(
      path.join(repositoryRoot, "vendor/pokecrystal/engine/phone/phone.asm"),
      path.join(repositoryRoot, "vendor/pokecrystal/engine/overworld/time.asm"),
      path.join(repositoryRoot, "vendor/pokecrystal/mobile/mobile_41.asm"),
    );

    const callasms = Object.entries(scripts).flatMap(([sourceLabel, commands]) =>
      commands.flatMap((command) =>
        command.command === "callasm" && Array.isArray(command.args)
          ? [callasmTarget(sourceLabel, command.args[0])]
          : [],
      ),
    );
    expect(new Set(callasms)).toEqual(new Set(PHONE_CALLASM_ENTRYPOINTS));
    for (const entrypoint of PHONE_CALLASM_ENTRYPOINTS) {
      expect(scripts[entrypoint]).toBeDefined();
    }

    expect(scripts.InitCallReceiveDelay).toEqual([
      { command: "xor", args: ["a"] },
      { command: "ld", args: ["[wTimeCyclesSinceLastCall]", "a"] },
      { command: "jp", args: ["NextCallReceiveDelay"] },
    ]);
    expect(scripts.NextCallReceiveDelay).toEqual(
      expect.arrayContaining([
        { command: "jr", args: ["c", ".okay"] },
        { command: "ld", args: ["hl", ".ReceiveCallDelays"] },
        { command: "jp", args: ["RestartReceiveCallDelay"] },
      ]),
    );
    expect(scripts[".ReceiveCallDelays@NextCallReceiveDelay"]).toEqual([
      { command: "db", args: ["20", "10", "5", "3"] },
    ]);
    expect(scripts.RestartReceiveCallDelay).toEqual([
      { command: "ld", args: ["hl", "wReceiveCallDelay_MinsRemaining"] },
      { command: "ld", args: ["[hl]", "a"] },
      { command: "call", args: ["UpdateTime"] },
      { command: "ld", args: ["hl", "wReceiveCallDelay_StartTime"] },
      { command: "call", args: ["CopyDayHourMinToHL"] },
      { command: "ret", args: [] },
    ]);
    expect(scripts.CopyDayHourMinToHL).toEqual([
      { command: "ld", args: ["a", "[wCurDay]"] },
      { command: "ld", args: ["[hli]", "a"] },
      { command: "ldh", args: ["a", "[hHours]"] },
      { command: "ld", args: ["[hli]", "a"] },
      { command: "ldh", args: ["a", "[hMinutes]"] },
      { command: "ld", args: ["[hli]", "a"] },
      { command: "ret", args: [] },
    ]);
    expect(scripts.StubbedTrainerRankings_PhoneCalls).toEqual([
      { command: "ret", args: [] },
    ]);

    const inventory = reachableCpuInventory(scripts, PHONE_CALLASM_ENTRYPOINTS);
    for (const required of [
      "RingTwice_StartCall",
      ".Ring@RingTwice_StartCall",
      "HangUp",
      "Phone_CallEnd",
      ".LoadBillScript@Script_SpecialBillCall",
      ".LoadElmScript@Script_SpecialElmCall",
      "LoadCallerScript",
      "InitCallReceiveDelay",
      "NextCallReceiveDelay",
      ".okay@NextCallReceiveDelay",
      "RestartReceiveCallDelay",
      "CopyDayHourMinToHL",
      "StubbedTrainerRankings_PhoneCalls",
    ]) {
      expect(inventory.definitions).toContain(required);
    }
    expect(inventory.externalTargets).not.toContain("InitCallReceiveDelay");
    expect(inventory.externalTargets).not.toContain(
      "StubbedTrainerRankings_PhoneCalls",
    );
    expect(inventory.externalTargets).toEqual(
      new Set([
        "AddNTimes",
        "DelayFrames",
        "FarCopyBytes",
        "GetTrainerClassName",
        "GetTrainerName",
        "PhoneRing_CopyTilemapAtOnce",
        "PlaceString",
        "PlaySFX",
        "PrintText",
        "SpeechTextbox",
        "Textbox",
        "UpdateSprites",
        "UpdateTime",
        "WaitSFX",
      ]),
    );
  });

  it("rejects missing and duplicate required phone CPU sources and labels", () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "phone-cpu-sources-"));
    const phoneSource = path.join(tmpDir, "phone.asm");
    const timeSource = path.join(tmpDir, "time.asm");
    const rankingsSource = path.join(tmpDir, "mobile_41.asm");
    const missingSource = path.join(tmpDir, "missing.asm");
    fs.writeFileSync(
      phoneSource,
      `Script_ReceivePhoneCall:
\tcallasm RingTwice_StartCall
\tcallasm HangUp
\tcallasm InitCallReceiveDelay
\tend

Script_SpecialBillCall:
\tcallasm .LoadBillScript
\tend
.LoadBillScript:
\tret

Script_SpecialElmCall:
\tcallasm .LoadElmScript
\tend
.LoadElmScript:
\tret

RingTwice_StartCall:
\tret

HangUp:
\tret
`,
      "utf8",
    );
    fs.writeFileSync(
      timeSource,
      `InitCallReceiveDelay:
\txor a

NextCallReceiveDelay:
\tjr c, .okay
.okay
\tld hl, .ReceiveCallDelays
\tjp RestartReceiveCallDelay
.ReceiveCallDelays:
\tdb 20, 10, 5, 3

RestartReceiveCallDelay:
\tcall UpdateTime
\tcall CopyDayHourMinToHL
\tret

CopyDayHourMinToHL:
\tret
`,
      "utf8",
    );
    fs.writeFileSync(
      rankingsSource,
      `StubbedTrainerRankings_PhoneCalls:
\tret
\tld hl, sTrainerRankingPhoneCalls
`,
      "utf8",
    );

    expect(() =>
      parsePhoneScriptCatalog(phoneSource, timeSource, missingSource),
    ).toThrow(`Required phone CPU source is missing: ${missingSource}`);
    expect(() =>
      parsePhoneScriptCatalog(phoneSource, timeSource, timeSource),
    ).toThrow(`Phone CPU source is repeated: ${timeSource}`);

    fs.writeFileSync(
      rankingsSource,
      `StubbedTrainerRankings_PhoneCalls:
\tnop
\tret
`,
      "utf8",
    );
    expect(() =>
      parsePhoneScriptCatalog(phoneSource, timeSource, rankingsSource),
    ).toThrow(
      `Phone rankings routine StubbedTrainerRankings_PhoneCalls in ${rankingsSource} must begin with an unconditional ret`,
    );
    fs.writeFileSync(
      rankingsSource,
      `StubbedTrainerRankings_PhoneCalls:
\tret
\tld hl, sTrainerRankingPhoneCalls
`,
      "utf8",
    );

    fs.appendFileSync(
      phoneSource,
      `
InitCallReceiveDelay:
\tret
`,
      "utf8",
    );
    expect(() =>
      parsePhoneScriptCatalog(phoneSource, timeSource, rankingsSource),
    ).toThrow(
      `Shared phone CPU label InitCallReceiveDelay from ${timeSource} duplicates ${phoneSource}`,
    );
  });
});
