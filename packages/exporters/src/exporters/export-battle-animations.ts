import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { stripAsmComment, writeJsonToTargets } from "./asm-utils";

const ASM_FALLTHROUGH_ANIMATIONS: Record<string, string> = {
  BattleAnim_Dummy: "BattleAnim_MirrorMove",
  BattleAnim_Gust: "BattleAnim_Sonicboom",
  BattleAnim_Poisonpowder: "BattleAnim_StunSpore",
  BattleAnim_SleepPowder: "BattleAnim_StunSpore",
  BattleAnim_Spore: "BattleAnim_StunSpore",
  BattleAnim_ReturnMon: "BattleAnimSub_Return",
};

export function exportBattleAnimations(): Record<string, string[]> {
  const sourcePath = path.join(getDisassemblyRoot(), "data", "moves", "animations.asm");
  const animations: Record<string, string[]> = {};
  let currentName: string | null = null;
  let currentLines: string[] = [];

  const flush = (): void => {
    if (currentName && currentLines.length) {
      animations[currentName] = currentLines;
    }
    currentName = null;
    currentLines = [];
  };

  for (const rawLine of fs.readFileSync(sourcePath, "utf8").split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    const labelMatch = /^(BattleAnim(?:Sub)?_[A-Za-z0-9_]+):$/.exec(line);
    if (labelMatch) {
      flush();
      currentName = labelMatch[1];
      currentLines = [];
      continue;
    }
    if (!currentName || !line) {
      continue;
    }
    const localLabel = /^(\.[A-Za-z0-9_]+):$/.exec(line);
    currentLines.push(localLabel ? localLabel[1] : line);
  }
  flush();

  if (!Object.keys(animations).length) {
    throw new Error(`Could not parse battle animation scripts from ${sourcePath}`);
  }
  for (const [label, target] of Object.entries(ASM_FALLTHROUGH_ANIMATIONS)) {
    const targetCommands = animations[target];
    if (!targetCommands?.length) {
      throw new Error(`Battle animation '${label}' falls through to missing animation '${target}'.`);
    }
    animations[label] = [...(animations[label] ?? []), ...targetCommands];
  }
  validateBattleAnimationTargets(animations, sourcePath);

  writeJsonToTargets("animations.json", animations);
  return animations;
}

function validateBattleAnimationTargets(
  animations: Record<string, string[]>,
  sourcePath: string,
): void {
  for (const [label, commands] of Object.entries(animations)) {
    const localLabels = new Set(commands.filter((command) => /^\.[A-Za-z0-9_]+$/.test(command)));
    commands.forEach((command, index) => {
      const [opcode, ...rawArgs] = command.split(/\s+/);
      const args = rawArgs.join(" ").split(",").map((arg) => arg.trim()).filter(Boolean);
      let target: string | undefined;
      if (["anim_call", "anim_jump", "anim_jumpuntil"].includes(opcode)) {
        target = args[0];
      } else if (["anim_loop", "anim_if_var_equal", "anim_if_param_equal", "anim_if_param_and"].includes(opcode)) {
        target = args[1];
      }
      if (!target) {
        return;
      }
      const resolved = target.startsWith(".")
        ? localLabels.has(target)
        : Object.prototype.hasOwnProperty.call(animations, target);
      if (!resolved) {
        throw new Error(
          `Battle animation '${label}' command ${index} references missing target '${target}' in ${sourcePath}`,
        );
      }
    });
  }
}
