import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { removeMatchingOutputs, writeJsonToTargets } from "./asm-utils";
import { parseAsmFile } from "./export-story-events";

export function exportPhoneScripts(): void {
  removeMatchingOutputs("phone_scripts");

  const root = getDisassemblyRoot();
  const phoneDir = path.join(root, "engine", "phone");
  const scriptDir = path.join(phoneDir, "scripts");
  const asmSources = [path.join(phoneDir, "phone.asm")].concat(
    fs.readdirSync(scriptDir)
      .filter((entry) => entry.endsWith(".asm"))
      .sort()
      .map((entry) => path.join(scriptDir, entry))
  );

  for (const asmPath of asmSources) {
    const scripts = parseAsmFile(asmPath);
    if (Object.keys(scripts).length === 0) {
      continue;
    }
    writeJsonToTargets(path.join("phone_scripts", `${path.basename(asmPath, ".asm")}.json`), scripts, {
      indent: 2,
    });
  }
}

