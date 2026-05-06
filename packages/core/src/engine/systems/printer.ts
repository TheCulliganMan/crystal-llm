import fs from "fs";
import path from "path";
import logger from "@pokecrystal/core/core/logger";
import { PrintOption } from "@pokecrystal/core/core/enums/ui-enums";
import type { PokedexEntryData } from "@pokecrystal/core/ui/menus/pokedex-entry-loader";

export interface Printer {
  printDexEntry(
    speciesId: string,
    pokedexNumber: number,
    entryData: PokedexEntryData,
    options: PrintOption
  ): void;
}

const formatPrintOption = (option: PrintOption): string => {
  return PrintOption[option] ?? String(option);
};

const normalizePage = (page: unknown): string => {
  if (typeof page === "string") {
    return page.replace(/@/g, "\n");
  }
  return String(page);
};

export class DefaultPrinter implements Printer {
  printDexEntry(
    speciesId: string,
    pokedexNumber: number,
    entryData: PokedexEntryData,
    options: PrintOption
  ): void {
    const outputDir = path.join("logs", "printer");
    fs.mkdirSync(outputDir, { recursive: true });
    const filename = `${String(pokedexNumber).padStart(3, "0")}_${speciesId.toLowerCase()}.txt`;
    const outputPath = path.join(outputDir, filename);

    const header = `${speciesId}  #${String(pokedexNumber).padStart(3, "0")}`;
    const metrics = `${entryData.classification} | HT:${entryData.heightDigits} WT:${entryData.weightDigits} | MODE:${formatPrintOption(options)}`;
    const body = entryData.pages.map(normalizePage).join("\n\n");
    fs.writeFileSync(outputPath, `${header}\n${metrics}\n\n${body}\n`, "utf-8");
    logger.info(`Saved Pokédex entry ${speciesId} to ${outputPath}`);
  }
}
