import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "../paths";

export type FieldPosition = Readonly<{
  name: string;
  line: number;
}>;

class AsmFieldScanner {
  private readonly lines: string[];
  private readonly cache = new Map<string, number | null>();

  constructor(private readonly asmPath: string) {
    this.lines = fs.readFileSync(asmPath, "utf-8").split(/\r?\n/);
  }

  find(fieldName: string): number | null {
    if (this.cache.has(fieldName)) {
      return this.cache.get(fieldName) ?? null;
    }

    const needle = `${fieldName}::`;
    for (let index = 0; index < this.lines.length; index += 1) {
      const body = this.lines[index].split(";", 1)[0].trim();
      if (!body) {
        continue;
      }
      if (body.startsWith(needle)) {
        const lineNumber = index + 1;
        this.cache.set(fieldName, lineNumber);
        return lineNumber;
      }
    }

    this.cache.set(fieldName, null);
    return null;
  }
}

const scanners = new Map<string, AsmFieldScanner>();

function getScanner(asmPath: string): AsmFieldScanner {
  const cached = scanners.get(asmPath);
  if (cached) {
    return cached;
  }
  const scanner = new AsmFieldScanner(asmPath);
  scanners.set(asmPath, scanner);
  return scanner;
}

export function assertFieldOrderMatchesAsm(
  fieldNames: Iterable<string>,
  prefix: string,
  asmPath: string
): FieldPosition[] {
  const scanner = getScanner(asmPath);
  const matches: FieldPosition[] = [];

  for (const name of fieldNames) {
    if (!name.startsWith(prefix)) {
      continue;
    }
    const line = scanner.find(name);
    if (line === null) {
      const relativePath = path.relative(getDisassemblyRoot(), asmPath);
      throw new Error(`Field ${JSON.stringify(name)} is missing from ${relativePath}`);
    }
    matches.push({ name, line });
  }

  for (let index = 1; index < matches.length; index += 1) {
    const first = matches[index - 1];
    const second = matches[index];
    if (second.line < first.line) {
      const relativePath = path.relative(getDisassemblyRoot(), asmPath);
      throw new Error(
        `${JSON.stringify(second.name)} appears before ${JSON.stringify(
          first.name
        )} in ${relativePath}`
      );
    }
  }

  return matches;
}

export function fieldsPresentInAsm(
  fieldNames: Iterable<string>,
  asmPath: string
): string[] {
  const scanner = getScanner(asmPath);
  const present: string[] = [];
  for (const name of fieldNames) {
    if (scanner.find(name) !== null) {
      present.push(name);
    }
  }
  return present;
}

const SECTION_PATTERN = /^\s*SECTION\s+"([^"]+)"/i;

const sectionCache = new Map<string, string[]>();

function collectSections(asmPath: string): string[] {
  const cached = sectionCache.get(asmPath);
  if (cached) {
    return cached;
  }
  const lines = fs.readFileSync(asmPath, "utf-8").split(/\r?\n/);
  const sections: string[] = [];
  for (const rawLine of lines) {
    const match = SECTION_PATTERN.exec(rawLine);
    if (match) {
      sections.push(match[1]);
    }
  }
  sectionCache.set(asmPath, sections);
  return sections;
}

export function assertSectionsMatchAsm(
  sections: Iterable<string>,
  asmPath: string
): string[] {
  const expectedSections = Array.from(sections);
  if (expectedSections.length === 0) {
    return [];
  }

  const actual = collectSections(asmPath);
  const ordered: string[] = [];
  let lastIndex = -1;

  for (const expected of expectedSections) {
    const currentIndex = actual.indexOf(expected);
    if (currentIndex === -1) {
      const relativePath = path.relative(getDisassemblyRoot(), asmPath);
      throw new Error(
        `ASM layout ${relativePath} does not contain section ${JSON.stringify(
          expected
        )}`
      );
    }
    if (currentIndex <= lastIndex) {
      const relativePath = path.relative(getDisassemblyRoot(), asmPath);
      throw new Error(
        `ASM section ${JSON.stringify(expected)} is out of order in ${relativePath}`
      );
    }
    ordered.push(expected);
    lastIndex = currentIndex;
  }

  return ordered;
}
