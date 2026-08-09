import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { removeMatchingOutputs, writeJsonToTargets } from "./asm-utils";
import {
  parseAsmFile,
  type StoryCommand,
  type StoryScripts,
} from "./export-story-events";

const PHONE_TIME_CPU_LABELS = [
  "InitCallReceiveDelay",
  "NextCallReceiveDelay",
  ".okay@NextCallReceiveDelay",
  ".ReceiveCallDelays@NextCallReceiveDelay",
  "RestartReceiveCallDelay",
  "CopyDayHourMinToHL",
] as const;

const PHONE_RANKINGS_NOOP_LABEL = "StubbedTrainerRankings_PhoneCalls";

export const PHONE_CALLASM_ENTRYPOINTS = [
  ".LoadBillScript@Script_SpecialBillCall",
  ".LoadElmScript@Script_SpecialElmCall",
  "HangUp",
  "InitCallReceiveDelay",
  "RingTwice_StartCall",
] as const;

const cloneCommands = (commands: readonly StoryCommand[]): StoryCommand[] =>
  commands.map((command) => ({
    command: command.command,
    args: Array.isArray(command.args) ? [...command.args] : command.args,
  }));

const requireSource = (sourcePath: string, kind: "script" | "CPU"): void => {
  if (!fs.existsSync(sourcePath)) {
    throw new Error(`Required phone ${kind} source is missing: ${sourcePath}`);
  }
};

const resolveLocalLabel = (sourceLabel: string, targetLabel: string): string => {
  if (!targetLabel.startsWith(".") || targetLabel.includes("@")) {
    return targetLabel;
  }
  const parent = sourceLabel.includes("@")
    ? sourceLabel.slice(sourceLabel.indexOf("@") + 1)
    : sourceLabel;
  return `${targetLabel}@${parent}`;
};

const phoneCallasmEntrypoints = (scripts: StoryScripts): string[] =>
  Object.entries(scripts).flatMap(([sourceLabel, commands]) =>
    commands.flatMap((command) => {
      if (command.command !== "callasm" || !Array.isArray(command.args)) {
        return [];
      }
      if (command.args.length !== 1) {
        throw new Error(
          `Phone callasm ${sourceLabel} has ${command.args.length} targets instead of exactly one`,
        );
      }
      return [resolveLocalLabel(sourceLabel, command.args[0])];
    }),
  );

const mergeRequiredCpuLabels = (
  scripts: StoryScripts,
  owners: Map<string, string>,
  sourcePath: string,
  labels: readonly string[],
): void => {
  const shared = parseAsmFile(sourcePath);
  for (const label of labels) {
    const commands = shared[label];
    if (!commands?.length) {
      throw new Error(
        `Required phone CPU label ${label} is missing from ${sourcePath}`,
      );
    }
    const previousSource = owners.get(label);
    if (previousSource) {
      throw new Error(
        `Shared phone CPU label ${label} from ${sourcePath} duplicates ${previousSource}`,
      );
    }
    scripts[label] = cloneCommands(commands);
    owners.set(label, sourcePath);
  }
};

const mergeRankingsNoop = (
  scripts: StoryScripts,
  owners: Map<string, string>,
  sourcePath: string,
): void => {
  const shared = parseAsmFile(sourcePath);
  const commands = shared[PHONE_RANKINGS_NOOP_LABEL];
  if (!commands?.length) {
    throw new Error(
      `Required phone CPU label ${PHONE_RANKINGS_NOOP_LABEL} is missing from ${sourcePath}`,
    );
  }
  const first = commands[0];
  if (
    first.command !== "ret" ||
    !Array.isArray(first.args) ||
    first.args.length !== 0
  ) {
    throw new Error(
      `Phone rankings routine ${PHONE_RANKINGS_NOOP_LABEL} in ${sourcePath} must begin with an unconditional ret`,
    );
  }
  const previousSource = owners.get(PHONE_RANKINGS_NOOP_LABEL);
  if (previousSource) {
    throw new Error(
      `Shared phone CPU label ${PHONE_RANKINGS_NOOP_LABEL} from ${sourcePath} duplicates ${previousSource}`,
    );
  }
  // The first instruction returns unconditionally. Instructions after it are
  // unreachable mobile rankings code, so the exact reachable body is one ret.
  scripts[PHONE_RANKINGS_NOOP_LABEL] = cloneCommands([first]);
  owners.set(PHONE_RANKINGS_NOOP_LABEL, sourcePath);
};

export function parsePhoneScriptCatalog(
  phoneSourcePath: string,
  timeSourcePath: string,
  rankingsSourcePath: string,
): StoryScripts {
  requireSource(phoneSourcePath, "script");
  requireSource(timeSourcePath, "CPU");
  requireSource(rankingsSourcePath, "CPU");

  const normalizedSources = [
    path.resolve(phoneSourcePath),
    path.resolve(timeSourcePath),
    path.resolve(rankingsSourcePath),
  ];
  const repeatedSource = normalizedSources.find(
    (source, index) => normalizedSources.indexOf(source) !== index,
  );
  if (repeatedSource) {
    throw new Error(`Phone CPU source is repeated: ${repeatedSource}`);
  }

  const scripts = parseAsmFile(phoneSourcePath);
  const owners = new Map(
    Object.keys(scripts).map((label) => [label, phoneSourcePath] as const),
  );
  mergeRequiredCpuLabels(
    scripts,
    owners,
    timeSourcePath,
    PHONE_TIME_CPU_LABELS,
  );
  mergeRankingsNoop(scripts, owners, rankingsSourcePath);

  const actualEntrypoints = phoneCallasmEntrypoints(scripts);
  const expectedEntrypoints = [...PHONE_CALLASM_ENTRYPOINTS];
  const actualEntrypointSet = new Set(actualEntrypoints);
  if (
    actualEntrypoints.length !== expectedEntrypoints.length ||
    actualEntrypointSet.size !== expectedEntrypoints.length ||
    expectedEntrypoints.some(
      (entrypoint) => !actualEntrypointSet.has(entrypoint),
    )
  ) {
    throw new Error(
      `Phone callasm entrypoints ${JSON.stringify(actualEntrypoints)} do not match ${JSON.stringify(expectedEntrypoints)}`,
    );
  }
  for (const entrypoint of expectedEntrypoints) {
    if (!scripts[entrypoint]?.length) {
      throw new Error(`Phone callasm entrypoint ${entrypoint} has no CPU body`);
    }
  }
  return scripts;
}

export function exportPhoneScripts(): void {
  removeMatchingOutputs("phone_scripts");

  const root = getDisassemblyRoot();
  const phoneDir = path.join(root, "engine", "phone");
  const scriptDir = path.join(phoneDir, "scripts");
  const phoneSourcePath = path.join(phoneDir, "phone.asm");
  const scripts = parsePhoneScriptCatalog(
    phoneSourcePath,
    path.join(root, "engine", "overworld", "time.asm"),
    path.join(root, "mobile", "mobile_41.asm"),
  );
  writeJsonToTargets(path.join("phone_scripts", "phone.json"), scripts, {
    indent: 2,
  });

  for (const asmPath of fs
    .readdirSync(scriptDir)
    .filter((entry) => entry.endsWith(".asm"))
    .sort()
    .map((entry) => path.join(scriptDir, entry))) {
    const phoneScripts = parseAsmFile(asmPath);
    if (Object.keys(phoneScripts).length === 0) {
      continue;
    }
    writeJsonToTargets(
      path.join("phone_scripts", `${path.basename(asmPath, ".asm")}.json`),
      phoneScripts,
      { indent: 2 },
    );
  }
}
