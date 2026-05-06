import type { TextSnapshot } from "@pokecrystal/core/ui/text-ui";

export type SnapshotLineKind = "normal" | "heading" | "selected" | "hint";
export type SnapshotLine = { text: string; kind: SnapshotLineKind };

export const MAX_TEXT_RENDER_CHARS = 240;
const SNAPSHOT_LINE_CACHE_LIMIT = 512;

const ANSI_ESCAPE_REGEX = /\u001b\[[0-9;]*m/g;
const SELECTION_ARROW_CODE = 62;
const SELECTION_TRIANGLE_CODE = 9658;
const textSnapshotLayoutCache = new WeakMap<TextSnapshot, SnapshotLine[]>();
const formattedSnapshotLineCache = new Map<string, SnapshotLine>();
const sectionLabelCache = new Map<string, string>();

const pruneSnapshotCache = <T,>(cache: Map<string, T>): void => {
  if (cache.size < SNAPSHOT_LINE_CACHE_LIMIT) {
    return;
  }
  const oldestKey = cache.keys().next().value;
  if (oldestKey !== undefined) {
    cache.delete(oldestKey);
  }
};

const hasSelectionPrefix = (value: string): boolean => {
  if (!value) {
    return false;
  }
  const firstCode = value.charCodeAt(0);
  return firstCode === SELECTION_ARROW_CODE || firstCode === SELECTION_TRIANGLE_CODE;
};

const stripAnsiCodes = (value: string): string => value.replace(ANSI_ESCAPE_REGEX, "").trim();
const formatSectionLabel = (value: string): string => {
  const cached = sectionLabelCache.get(value);
  if (cached !== undefined) {
    return cached;
  }
  const formatted = stripAnsiCodes(value).toUpperCase();
  pruneSnapshotCache(sectionLabelCache);
  sectionLabelCache.set(value, formatted);
  return formatted;
};

const selectedLineLabel = (line: string): string | undefined => {
  const cleaned = stripAnsiCodes(line).trimStart();
  return hasSelectionPrefix(cleaned) ? cleaned.slice(1).trimStart() : undefined;
};

const findSelectedLineLabel = (lines?: readonly string[] | null): string | undefined => {
  if (!lines?.length) {
    return undefined;
  }
  for (let i = 0; i < lines.length; i += 1) {
    const label = selectedLineLabel(lines[i]);
    if (label !== undefined) {
      return label;
    }
  }
  return undefined;
};

const getFormattedSnapshotLine = (
  line: string,
  kind: SnapshotLineKind = "normal",
): SnapshotLine => {
  const cacheKey = `${kind}:${line}`;
  const cached = formattedSnapshotLineCache.get(cacheKey);
  if (cached) {
    return cached;
  }
  const cleaned = stripAnsiCodes(line).trimStart();
  const selected = hasSelectionPrefix(cleaned);
  const formatted: SnapshotLine = {
    text: selected ? `>> ${cleaned.slice(1).trimStart()}` : cleaned,
    kind: selected ? "selected" : kind,
  };
  pruneSnapshotCache(formattedSnapshotLineCache);
  formattedSnapshotLineCache.set(cacheKey, formatted);
  return formatted;
};

export const buildTextSnapshotLayout = (snapshot: TextSnapshot | null): SnapshotLine[] => {
  if (!snapshot) {
    return [{ text: "(waiting for text snapshot...)", kind: "normal" }];
  }
  const cachedLayout = textSnapshotLayoutCache.get(snapshot);
  if (cachedLayout) {
    return cachedLayout;
  }
  const lines: SnapshotLine[] = [];

  const pushLine = (text: string, kind: SnapshotLineKind = "normal") => {
    lines.push({ text, kind });
  };

  const pushFormattedLine = (line: string, kind: SnapshotLineKind = "normal"): void => {
    lines.push(getFormattedSnapshotLine(line, kind));
  };

  const appendViewport = (): void => {
    const viewportLines = snapshot.viewportLines ?? [];
    const infoLines = snapshot.infoLines ?? [];
    const title = snapshot.viewportTitle?.trim() ?? "";
    const normalizedTitle = stripAnsiCodes(title);
    const normalizedTitleLower = normalizedTitle.toLowerCase();
    const inlineLegend = (snapshot.infoTitle ?? "").trim() === "Legend";
    if (title) {
      const firstLine = viewportLines[0] ? stripAnsiCodes(viewportLines[0]).trim().toLowerCase() : "";
      if (!firstLine || firstLine !== normalizedTitleLower) {
        pushLine(formatSectionLabel(title), "heading");
      }
    }
    if (viewportLines.length) {
      for (const line of viewportLines) {
        pushFormattedLine(line);
      }
      if (inlineLegend && infoLines.length) {
        for (const line of infoLines) {
          pushFormattedLine(line, "hint");
        }
      }
      return;
    }
    pushLine(title ? `${normalizedTitle} (waiting for data...)` : "(viewport waiting for data...)");
    if (inlineLegend && infoLines.length) {
      for (const line of infoLines) {
        pushFormattedLine(line, "hint");
      }
    }
  };

  const pushSection = (label: string, entries: readonly string[] | null | undefined, kind: SnapshotLineKind = "normal"): void => {
    if (!entries || !entries.length) {
      return;
    }
    if (lines.length) {
      pushLine("", "normal");
    }
    pushLine(formatSectionLabel(label), "heading");
    for (const entry of entries) {
      pushFormattedLine(entry, kind);
    }
  };

  appendViewport();
  if ((snapshot.infoTitle ?? "").trim() !== "Legend") {
    pushSection(snapshot.infoTitle ?? "Info", snapshot.infoLines);
  }
  pushSection("Action Log", snapshot.actionLog ?? null, "hint");
  if (snapshot.marker) {
    const [x, y, char] = snapshot.marker;
    pushSection("Marker", [`(${x}, ${y}) ${char}`]);
  }
  pushSection("Dialogue", snapshot.dialogueLines);
  pushSection("Menu", snapshot.menuLines);
  pushSection("Prompt", snapshot.promptLines);

  const showHints = snapshot.infoTitle !== "Legend";
  if (showHints) {
    const selectedMenu = findSelectedLineLabel(snapshot.menuLines);
    const selectedPrompt = findSelectedLineLabel(snapshot.promptLines);
    const selectionSummary: string[] = [];
    if (selectedMenu !== undefined) {
      selectionSummary.push(`MENU: ${selectedMenu}`);
    }
    if (selectedPrompt !== undefined) {
      selectionSummary.push(`PROMPT: ${selectedPrompt}`);
    }
    if (selectionSummary.length) {
      pushSection("Selection", selectionSummary, "hint");
    }

    const hints: string[] = [];
    const hasPrompt = Boolean(snapshot.promptLines?.length);
    const hasDialogue = Boolean(snapshot.dialogueLines?.length);
    const hasMenu = Boolean(snapshot.menuLines?.length);
    const hasPromptSelection = selectedPrompt !== undefined;
    const hasMenuSelection = selectedMenu !== undefined;

    if (hasPrompt) {
      hints.push("UP/DOWN: Choose");
      hints.push(hasPromptSelection ? "A: Confirm selection" : "A: Confirm");
      hints.push("B: Cancel");
    } else if (hasMenu && hasMenuSelection) {
      hints.push("UP/DOWN: Choose");
      hints.push("A: Select");
      hints.push("B: Back");
    }

    if (!hasPrompt && hasDialogue) {
      hints.push("A: Advance dialogue");
    }

    if (hints.length) {
      pushSection("Active Input", hints, "hint");
    }
  }

  textSnapshotLayoutCache.set(snapshot, lines);
  return lines;
};

export const buildTextSnapshotLines = (snapshot: TextSnapshot | null): string[] =>
  buildTextSnapshotLayout(snapshot).map((line) => line.text);

export const wrapLinesToWidth = (lines: string[], maxChars: number): string[] => {
  const safeMaxChars = Math.max(1, Math.min(maxChars, MAX_TEXT_RENDER_CHARS));
  const wrapped: string[] = [];

  const wrapLine = (line: string): void => {
    if (line.length === 0) {
      wrapped.push("");
      return;
    }
    if (line.length <= safeMaxChars) {
      wrapped.push(line);
      return;
    }
    const words = line.split(" ");
    let current = "";
    const flush = () => {
      if (current.length) {
        wrapped.push(current);
        current = "";
      }
    };
    for (const word of words) {
      const next = current ? `${current} ${word}` : word;
      if (next.length <= safeMaxChars) {
        current = next;
        continue;
      }
      flush();
      if (word.length <= safeMaxChars) {
        current = word;
        continue;
      }
      let chunk = "";
      for (const char of word) {
        if ((chunk + char).length > safeMaxChars) {
          if (chunk.length) {
            wrapped.push(chunk);
          }
          chunk = char;
        } else {
          chunk += char;
        }
      }
      if (chunk.length) {
        current = chunk;
      }
    }
    flush();
  };

  for (const line of lines) {
    wrapLine(line);
  }

  return wrapped.length ? wrapped : [""];
};
