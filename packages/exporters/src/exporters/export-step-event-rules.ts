import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { parseAsmNumber, writeJsonToTargets } from "./asm-utils";

export type ExportedStepEventRules = {
  poison_step_interval: number;
  egg_step_trigger: number;
  hatched_egg_happiness: number;
  poison_status: string;
  egg_nickname: string;
  happiness_step_counter_mask: number;
  happiness_step_counter_target: number;
};

const readAsm = (relativePath: string): string =>
  fs.readFileSync(path.join(getDisassemblyRoot(), relativePath), "utf8");

const labelSlice = (content: string, label: string, bytes: number): string => {
  const match = new RegExp(`^\\s*${label.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}:{0,2}\\s*$`, "m").exec(content);
  if (!match || match.index === undefined) {
    throw new Error(`Unable to find ${label} label.`);
  }
  return content.slice(match.index, match.index + bytes);
};

const parseEggTrigger = (events: string): number => {
  const slice = labelSlice(events, ".skip_happiness", 600);
  const match = slice.match(/ld\s+a,\s*\[wStepCount\][\s\S]*?cp\s+(\$[0-9a-fA-F]+|\d+)/);
  if (!match) {
    throw new Error("Unable to find egg step trigger in overworld events.");
  }
  return parseAsmNumber(match[1]);
};

const parsePoisonStepInterval = (events: string): number => {
  const slice = labelSlice(events, ".skip_egg", 600);
  const match = slice.match(/ld\s+hl,\s*wPoisonStepCount[\s\S]*?ld\s+a,\s*\[hl\][\s\S]*?cp\s+(\$[0-9a-fA-F]+|\d+)/);
  if (!match) {
    throw new Error("Unable to find poison step interval in overworld events.");
  }
  return parseAsmNumber(match[1]);
};

const parseHatchedEggHappiness = (breeding: string): number => {
  const slice = labelSlice(breeding, "HatchEggs", 900);
  const match = slice.match(/cp\s+EGG[\s\S]*?ld\s+\[hl\],\s*(\$[0-9a-fA-F]+|\d+)/);
  if (!match) {
    throw new Error("Unable to find hatched egg happiness in HatchEggs.");
  }
  return parseAsmNumber(match[1]);
};

const parseEggNickname = (daycare: string): string => {
  const match = daycare.match(/\.String_EGG:\s*\r?\n\s*db\s+"([^"@]+)@"/);
  if (!match) {
    throw new Error("Unable to find daycare egg nickname string.");
  }
  return match[1];
};

const assertEggStepPattern = (breeding: string): void => {
  const slice = labelSlice(breeding, "DoEggStep", 650);
  for (const expected of ["cp EGG", "dec [hl]", "jr nz, .next"]) {
    if (!slice.includes(expected)) {
      throw new Error(`DoEggStep does not contain expected instruction '${expected}'.`);
    }
  }
};

const parseHappinessStepCounter = (happiness: string): Pick<
  ExportedStepEventRules,
  "happiness_step_counter_mask" | "happiness_step_counter_target"
> => {
  const slice = labelSlice(happiness, "StepHappiness", 450);
  const match = slice.match(/inc\s+a\s*\r?\n\s*and\s+(\$[0-9a-fA-F]+|\d+)\s*\r?\n\s*ld\s+\[hl\],\s*a\s*\r?\n\s*ret\s+nz/);
  if (!match) {
    throw new Error("Unable to find happiness step counter mask in StepHappiness.");
  }
  return {
    happiness_step_counter_mask: parseAsmNumber(match[1]),
    happiness_step_counter_target: 0,
  };
};

const assertPoisonStepPattern = (poisonStep: string): void => {
  const slice = labelSlice(poisonStep, ".DamageMonIfPoisoned", 900);
  for (const expected of ["and 1 << PSN", "dec bc", "ld [hl], 0", "ld c, %10", "ld c, %01"]) {
    if (!slice.includes(expected)) {
      throw new Error(`DoPoisonStep does not contain expected instruction '${expected}'.`);
    }
  }
};

export function exportStepEventRules(): ExportedStepEventRules {
  const events = readAsm("engine/overworld/events.asm");
  const breeding = readAsm("engine/pokemon/breeding.asm");
  const happiness = readAsm("engine/events/happiness_egg.asm");
  const poisonStep = readAsm("engine/events/poisonstep.asm");
  const daycare = readAsm("engine/events/daycare.asm");
  assertEggStepPattern(breeding);
  assertPoisonStepPattern(poisonStep);
  const payload: ExportedStepEventRules = {
    poison_step_interval: parsePoisonStepInterval(events),
    egg_step_trigger: parseEggTrigger(events),
    hatched_egg_happiness: parseHatchedEggHappiness(breeding),
    poison_status: "POISON",
    egg_nickname: parseEggNickname(daycare),
    ...parseHappinessStepCounter(happiness),
  };
  writeJsonToTargets("step_event_rules.json", payload, { indent: 2 });
  return payload;
}
