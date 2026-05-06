const STATUS_PREFIXES = ["Text queue:"] as const;
const STATUS_EXACT = new Set(["Waiting for input..."]);

export const filterPromptContextLines = (lines: string[] | null | undefined): string[] => {
  if (!lines || !lines.length) {
    return [];
  }
  const filtered: string[] = [];
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) {
      continue;
    }
    if (STATUS_EXACT.has(trimmed)) {
      continue;
    }
    if (STATUS_PREFIXES.some((prefix) => trimmed.startsWith(prefix))) {
      continue;
    }
    filtered.push(line);
  }
  return filtered;
};
