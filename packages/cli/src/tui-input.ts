import type { KeyAction, ToolResult } from "./types";

export type CommandModeState = {
  buffer: string | null;
  note?: string;
  isError?: boolean;
  viewCycleCommand?: true;
  gameboyRendererToggleCommand?: true;
  soundToggleCommand?: true;
  controlsToggleCommand?: true;
  agentToggleCommand?: true;
  agentSettingCommand?: {
    key: "model" | "goal" | "maxSteps" | "graphCycleSteps" | "requestDelayMs" | "identityName";
    value: string | number | undefined;
  };
  agentMessageCommand?: string;
  quitCommand?: "q!" | "wq" | "wq!" | "x" | "x!";
};

const arrowDirections = {
  A: "up",
  B: "down",
  C: "right",
  D: "left",
} as const;

const arrowSequenceMatch = (input: string): RegExpMatchArray | null =>
  input.match(/^\u001b(?:O([ABCD])|\[(?:1;\d+)?([ABCD]))/);

const deleteKeySequenceMatch = (input: string): RegExpMatchArray | null =>
  input.match(/^\u001b\[3(?:;\d+)?~/);

const endKeySequenceMatch = (input: string): RegExpMatchArray | null =>
  input.match(/^\u001b(?:OF|\[(?:(?:1;\d+)?F|[48](?:;\d+)?~))/);

const knownEscapeSequenceMatch = (input: string): RegExpMatchArray | null =>
  arrowSequenceMatch(input) ?? deleteKeySequenceMatch(input) ?? endKeySequenceMatch(input);

const mapArrowEscapeToDirection = (key: string): KeyAction | null => {
  const match = arrowSequenceMatch(key);
  if (!match || match[0] !== key) {
    return null;
  }
  const arrowCode = (match[1] ?? match[2]) as keyof typeof arrowDirections;
  return { type: "direction", direction: arrowDirections[arrowCode] };
};

const isEnterKey = (key: string): boolean => key === "\r" || key === "\n";

export const mapKeypressToAction = (key: string): KeyAction => {
  const arrowAction = mapArrowEscapeToDirection(key);
  if (arrowAction) {
    return arrowAction;
  }

  switch (key) {
    case "w":
    case "W":
    case "k":
      return { type: "direction", direction: "up" };
    case "s":
    case "S":
      return { type: "direction", direction: "down" };
    case "a":
    case "h":
    case "H":
      return { type: "direction", direction: "left" };
    case "d":
    case "D":
    case "l":
    case "L":
      return { type: "direction", direction: "right" };
    case "j":
    case "J":
    case "z":
    case "Z":
    case "A":
    case " ":
      return { type: "press", button: "a" };
    case "b":
    case "B":
    case "x":
    case "X":
    case "K":
    case "\u001b":
      return { type: "press", button: "b" };
    case "\r":
    case "\n":
      return { type: "press", button: "start" };
    case "\t":
      return { type: "press", button: "select" };
    case ".":
      return { type: "wait", frames: 8 };
    case "r":
    case "R":
      return { type: "refresh" };
    case "\u0003":
      return { type: "quit" };
    default:
      return { type: "noop" };
  }
};

export const splitKeypressChunk = (chunk: string): string[] => {
  const keys: string[] = [];
  for (let index = 0; index < chunk.length; index += 1) {
    const char = chunk[index] ?? "";
    if (char === "\u001b") {
      const maybeSequence = knownEscapeSequenceMatch(chunk.slice(index));
      if (maybeSequence) {
        keys.push(maybeSequence[0]);
        index += maybeSequence[0].length - 1;
        continue;
      }
    }
    keys.push(char);
  }
  return keys;
};

const isPotentialEscapeSequencePrefix = (input: string): boolean => {
  if (!input.startsWith("\u001b")) {
    return false;
  }
  if (input === "\u001b") {
    return true;
  }
  if (input === "\u001bO") {
    return true;
  }
  if (/^\u001bO[ABCDF]?$/.test(input)) {
    return true;
  }
  if (/^\u001b\[(?:[0-9;]*)?$/.test(input)) {
    return true;
  }
  if (/^\u001b\[(?:1;\d+)?[ABCDF]?$/.test(input)) {
    return true;
  }
  if (/^\u001b\[[348](?:;\d+)?~?$/.test(input)) {
    return true;
  }
  return false;
};

export type KeypressChunkParser = {
  push(chunk: string): string[];
  flush(): string[];
};

export const createKeypressChunkParser = (): KeypressChunkParser => {
  let pending = "";
  return {
    push(chunk: string): string[] {
      const input = `${pending}${chunk}`;
      pending = "";
      const keys: string[] = [];
      for (let index = 0; index < input.length; index += 1) {
        const char = input[index] ?? "";
        if (char === "\u001b") {
          const rest = input.slice(index);
          const maybeSequence = knownEscapeSequenceMatch(rest);
          if (maybeSequence) {
            keys.push(maybeSequence[0]);
            index += maybeSequence[0].length - 1;
            continue;
          }
          if (isPotentialEscapeSequencePrefix(rest)) {
            pending = rest;
            break;
          }
        }
        keys.push(char);
      }
      return keys;
    },
    flush(): string[] {
      if (!pending) {
        return [];
      }
      const keys = splitKeypressChunk(pending);
      pending = "";
      return keys;
    },
  };
};

const NAME_ENTRY_ROWS = ["ABCDEFGHI", "JKLMNOPQR", "STUVWXYZ"];
const NAME_ENTRY_SPECIAL_KEYS: Array<{ keys: string[]; row: number; column: number }> = [
  { keys: ["-"], row: 3, column: 0 },
  { keys: ["?"], row: 3, column: 1 },
  { keys: ["!"], row: 3, column: 2 },
  { keys: ["/"], row: 3, column: 3 },
  { keys: ["."], row: 3, column: 4 },
  { keys: [","], row: 3, column: 5 },
];
const NAME_ENTRY_ROW_COUNT = 5;
const NAME_ENTRY_COLUMN_COUNT = 9;
const NAME_ENTRY_BOTTOM_CASE_COLUMN = 0;
const NAME_ENTRY_GAMEPLAY_KEYS = new Set([" "]);
const isNameEntryDeleteKey = (key: string): boolean => key === "\u007f" || key === "\b" || /^\u001b\[3(?:;\d+)?~$/.test(key);
const isNameEntryEndKey = (key: string): boolean => /^\u001b(?:OF|\[(?:(?:1;\d+)?F|[48](?:;\d+)?~))$/.test(key);

const readStructuredSnapshotLines = (result?: ToolResult): { viewport: string[]; info: string[] } => {
  const output = { viewport: [] as string[], info: [] as string[] };
  for (const entry of result?.content ?? []) {
    if (entry.type !== "text" || typeof entry.text !== "string") {
      continue;
    }
    try {
      const parsed = JSON.parse(entry.text) as { view?: { viewport?: unknown; info?: unknown } };
      const view = parsed && typeof parsed === "object" ? parsed.view : undefined;
      if (Array.isArray(view?.viewport)) {
        output.viewport.push(...view.viewport.map(String));
      }
      if (Array.isArray(view?.info)) {
        output.info.push(...view.info.map(String));
      }
    } catch {
      const lines = entry.text.split("\n");
      output.viewport.push(...lines.filter((line) => line.trim().toUpperCase() === "NAME ENTRY"));
      output.info.push(
        ...lines.filter((line) =>
          /^STATE:\s*name_entry/i.test(line.trim()) || /^CURSOR:/i.test(line.trim()) || /^CASE:/i.test(line.trim())
        ),
      );
    }
  }
  return output;
};

const isNameEntrySnapshot = (result?: ToolResult): boolean => {
  const lines = readStructuredSnapshotLines(result);
  return (
    lines.info.some((line) => /^STATE:\s*name_entry/i.test(line.trim())) ||
    lines.viewport.some((line) => line.trim().toUpperCase() === "NAME ENTRY")
  );
};

const readNameEntryCursor = (result?: ToolResult): { row: number; column: number } => {
  const lines = readStructuredSnapshotLines(result);
  const cursorLine = lines.info.find((line) => /^CURSOR:/i.test(line.trim()));
  const match = cursorLine?.match(/row\s+(\d+)\s+col\s+(\d+)/i);
  return {
    row: Math.max(0, Math.min(NAME_ENTRY_ROW_COUNT - 1, Number(match?.[1] ?? 0))),
    column: Math.max(0, Math.min(NAME_ENTRY_COLUMN_COUNT - 1, Number(match?.[2] ?? 0))),
  };
};

const readNameEntryCase = (result?: ToolResult): "upper" | "lower" => {
  const lines = readStructuredSnapshotLines(result);
  const caseLine = lines.info.find((line) => /^CASE:/i.test(line.trim()));
  return /lower/i.test(caseLine ?? "") ? "lower" : "upper";
};

const shortestWrappedDirections = (
  current: number,
  target: number,
  size: number,
  forward: KeyAction,
  backward: KeyAction,
): KeyAction[] => {
  const forwardCount = (target - current + size) % size;
  const backwardCount = (current - target + size) % size;
  return Array(forwardCount <= backwardCount ? forwardCount : backwardCount).fill(
    forwardCount <= backwardCount ? forward : backward,
  );
};

const moveToNameEntryCell = (
  current: { row: number; column: number },
  target: { row: number; column: number },
): KeyAction[] => {
  const actions = [
    ...shortestWrappedDirections(
      current.row,
      target.row,
      NAME_ENTRY_ROW_COUNT,
      { type: "direction", direction: "down" },
      { type: "direction", direction: "up" },
    ),
  ];

  if (target.row === NAME_ENTRY_ROW_COUNT - 1) {
    const currentBottomGroup = Math.floor(current.column / 3);
    const targetBottomGroup = Math.floor(target.column / 3);
    actions.push(
      ...shortestWrappedDirections(
        currentBottomGroup,
        targetBottomGroup,
        3,
        { type: "direction", direction: "right" },
        { type: "direction", direction: "left" },
      ),
    );
  } else {
    actions.push(
      ...shortestWrappedDirections(
        current.column,
        target.column,
        NAME_ENTRY_COLUMN_COUNT,
        { type: "direction", direction: "right" },
        { type: "direction", direction: "left" },
      ),
    );
  }
  current.row = target.row;
  current.column = target.column;
  return actions;
};

const pressNameEntryCell = (
  current: { row: number; column: number },
  target: { row: number; column: number },
): KeyAction[] => [...moveToNameEntryCell(current, target), { type: "press", button: "a" }];

export const resolveNameEntryKeypressActions = (
  key: string,
  snapshotResult?: ToolResult,
): KeyAction[] | null => {
  if (!isNameEntrySnapshot(snapshotResult)) {
    return null;
  }

  const normalized = key.toUpperCase();
  if (isNameEntryDeleteKey(key)) {
    return [{ type: "press", button: "b" }];
  }
  if (isNameEntryEndKey(key)) {
    return [
      { type: "press", button: "start" },
      { type: "press", button: "a" },
    ];
  }
  if (/^[a-z]$/i.test(key)) {
    return [{ type: "text", text: key }];
  }
  if (NAME_ENTRY_GAMEPLAY_KEYS.has(key)) {
    return null;
  }

  const specialTarget = NAME_ENTRY_SPECIAL_KEYS.find((entry) => entry.keys.includes(key));
  const desiredCase = /^[a-z]$/i.test(key)
    ? key === key.toLowerCase()
      ? "lower"
      : "upper"
    : "upper";
  const letterRow = /^[a-z]$/i.test(key)
    ? NAME_ENTRY_ROWS.findIndex((row) => row.includes(normalized))
    : -1;
  const targetRow = specialTarget?.row ?? letterRow;
  if (targetRow < 0) {
    return null;
  }
  const targetColumn = specialTarget?.column ?? NAME_ENTRY_ROWS[targetRow]!.indexOf(normalized);
  const current = readNameEntryCursor(snapshotResult);
  const actions: KeyAction[] = [];
  const currentCase = readNameEntryCase(snapshotResult);
  if (currentCase !== desiredCase) {
    actions.push(
      ...pressNameEntryCell(current, {
        row: 4,
        column: NAME_ENTRY_BOTTOM_CASE_COLUMN,
      }),
    );
  }
  actions.push(...pressNameEntryCell(current, { row: targetRow, column: targetColumn }));
  return actions;
};

export const resolveDirectionalAction = (
  action: KeyAction,
  statusResult?: ToolResult,
  observeResult?: ToolResult,
): KeyAction => {
  if (action.type !== "direction") {
    return action;
  }
  if (isUnownPuzzleSurface(statusResult) || isUnownPuzzleSurface(observeResult)) {
    return { type: "press", button: action.direction };
  }
  const menuLike = isMenuLikeSurface(statusResult) || isMenuLikeSurface(observeResult);
  if (!menuLike && (isDialogueTextSurface(statusResult) || isDialogueTextSurface(observeResult))) {
    return { type: "noop" };
  }
  if (isBattleSurface(statusResult) || isBattleSurface(observeResult)) {
    return { type: "move", direction: action.direction };
  }
  if (menuLike) {
    return { type: "press", button: action.direction };
  }
  return { type: "move", direction: action.direction };
};

const isBattleSurface = (result?: ToolResult): boolean => {
  for (const entry of result?.content ?? []) {
    if (entry.type !== "text" || typeof entry.text !== "string") {
      continue;
    }
    try {
      const parsed = JSON.parse(entry.text) as {
        mode?: unknown;
        in_battle?: unknown;
        inBattle?: unknown;
      };
      if (parsed.mode === "battle" || parsed.in_battle === true || parsed.inBattle === true) {
        return true;
      }
    } catch {
      const normalized = entry.text.toLowerCase();
      if (
        normalized.includes('"mode":"battle"') ||
        normalized.includes('"in_battle":true') ||
        normalized.includes('"inbattle":true') ||
        normalized.includes("m: battle")
      ) {
        return true;
      }
    }
  }
  return false;
};

const isUnownPuzzleSurface = (result?: ToolResult): boolean => {
  for (const entry of result?.content ?? []) {
    if (entry.type !== "text" || typeof entry.text !== "string") {
      continue;
    }
    try {
      const parsed = JSON.parse(entry.text) as {
        unown_puzzle_active?: unknown;
        unownPuzzleActive?: unknown;
        unown_state?: unknown;
        unownState?: unknown;
        surface?: { kind?: unknown };
      };
      const unownState = Number(parsed.unown_state ?? parsed.unownState ?? 0);
      if (
        parsed.unown_puzzle_active === true ||
        parsed.unownPuzzleActive === true ||
        unownState !== 0 ||
        parsed.surface?.kind === "unown_puzzle"
      ) {
        return true;
      }
    } catch {
      const normalized = entry.text.toLowerCase();
      if (
        normalized.includes('"unown_puzzle_active":true') ||
        normalized.includes('"unownpuzzleactive":true') ||
        normalized.includes('"unown_state":1') ||
        normalized.includes('"unownstate":1') ||
        normalized.includes('"kind":"unown_puzzle"')
      ) {
        return true;
      }
    }
  }
  return false;
};

const isDialogueTextSurface = (result?: ToolResult): boolean => {
  if (isUnownPuzzleSurface(result)) {
    return false;
  }
  for (const entry of result?.content ?? []) {
    if (entry.type !== "text" || typeof entry.text !== "string") {
      continue;
    }
    try {
      const parsed = JSON.parse(entry.text) as {
        in_dialog?: unknown;
        inDialog?: unknown;
        text_box_open?: unknown;
        textBoxOpen?: unknown;
        textbox_open?: unknown;
        textboxOpen?: unknown;
        text_advance_pending?: unknown;
        textAdvancePending?: unknown;
        input_blocked_reason?: unknown;
        blockedReason?: unknown;
        surface?: {
          dialogue_open?: unknown;
          dialogueOpen?: unknown;
          waiting?: unknown;
        };
      };
      if (
        parsed.in_dialog === true ||
        parsed.inDialog === true ||
        parsed.text_box_open === true ||
        parsed.textBoxOpen === true ||
        parsed.textbox_open === true ||
        parsed.textboxOpen === true ||
        parsed.text_advance_pending === true ||
        parsed.textAdvancePending === true ||
        parsed.input_blocked_reason === "dialogue" ||
        parsed.blockedReason === "dialogue" ||
        parsed.surface?.dialogue_open === true ||
        parsed.surface?.dialogueOpen === true ||
        parsed.surface?.waiting === true
      ) {
        return true;
      }
    } catch {
      const compact = entry.text.toLowerCase().replace(/[_\s-]+/g, "");
      if (
        compact.includes('"indialog":true') ||
        compact.includes('"textboxopen":true') ||
        compact.includes('"textadvancepending":true') ||
        compact.includes('"inputblockedreason":"dialogue"') ||
        compact.includes('"blockedreason":"dialogue"') ||
        compact.includes('"dialogueopen":true') ||
        compact.includes("textqueue:0(pressatoadvance)")
      ) {
        return true;
      }
    }
  }
  return false;
};

export const isMenuLikeSurface = (result?: ToolResult): boolean => {
  if (isUnownPuzzleSurface(result)) {
    return false;
  }
  for (const entry of result?.content ?? []) {
    if (entry.type !== "text" || typeof entry.text !== "string") {
      continue;
    }
    try {
      const parsed = JSON.parse(entry.text) as {
        mode?: unknown;
        in_menu?: unknown;
        inMenu?: unknown;
        menu?: unknown;
        prompt_pending?: unknown;
        promptPending?: unknown;
        input_blocked_reason?: unknown;
        surface?: {
          menu_open?: unknown;
          prompt_open?: unknown;
          kind?: unknown;
        };
      };
      if (
        parsed.mode === "menu" ||
        parsed.mode === "title" ||
        parsed.mode === "main_menu" ||
        parsed.mode === "continue" ||
        parsed.mode === "delete_save" ||
        parsed.mode === "clock_reset" ||
        parsed.mode === "gender" ||
        parsed.mode === "name_entry" ||
        parsed.in_menu === true ||
        parsed.inMenu === true ||
        parsed.menu === true ||
        parsed.prompt_pending === true ||
        parsed.promptPending === true ||
        parsed.input_blocked_reason === "prompt" ||
        parsed.input_blocked_reason === "menu" ||
        parsed.surface?.menu_open === true ||
        parsed.surface?.prompt_open === true ||
        parsed.surface?.kind === "pokegear" ||
        parsed.surface?.kind === "slot_machine" ||
        parsed.surface?.kind === "fly_to_where"
      ) {
        return true;
      }
    } catch {
      const normalized = entry.text.toLowerCase();
      if (
        normalized.includes('"mode":"menu"') ||
        normalized.includes('"mode":"title"') ||
        normalized.includes('"mode":"main_menu"') ||
        normalized.includes('"mode":"continue"') ||
        normalized.includes('"mode":"delete_save"') ||
        normalized.includes('"mode":"clock_reset"') ||
        normalized.includes('"mode":"gender"') ||
        normalized.includes('"mode":"name_entry"') ||
        normalized.includes('"in_menu":true') ||
        normalized.includes('"menu":true') ||
        normalized.includes('"prompt_pending":true') ||
        normalized.includes('"input_blocked_reason":"prompt"') ||
        normalized.includes('"input_blocked_reason":"menu"') ||
        normalized.includes('"kind":"pokegear"') ||
        normalized.includes('"kind":"slot_machine"') ||
        normalized.includes('"kind":"fly_to_where"')
      ) {
        return true;
      }
    }
  }
  return false;
};

export const resolveLowercaseAAction = (
  key: string,
  statusResult?: ToolResult,
  observeResult?: ToolResult,
): KeyAction | null => {
  if (key !== "a") {
    return null;
  }
  if (isUnownPuzzleSurface(statusResult) || isUnownPuzzleSurface(observeResult)) {
    return { type: "press", button: "a" };
  }
  if (isMenuLikeSurface(statusResult) || isMenuLikeSurface(observeResult)) {
    return { type: "press", button: "a" };
  }
  return null;
};

export const updateCommandMode = (state: CommandModeState, key: string): CommandModeState => {
  if (state.buffer === null) {
    if (key === "\u001b") {
      return { buffer: "", note: "COMMAND>", isError: false };
    }
    if (key === ":") {
      return { buffer: ":", note: "COMMAND> :", isError: false };
    }
    return state;
  }

  if (key === "\u001b") {
    return { buffer: null };
  }
  if (isEnterKey(key)) {
    const command = state.buffer.replace(/^:/, "").trim();
    if (!command) {
      return { buffer: null };
    }
    if (command === "q") {
      return { buffer: null, note: "Use :wq to save+quit or :q! to quit without saving.", isError: true };
    }
    if (command === "q!" || command === "wq" || command === "wq!" || command === "x" || command === "x!") {
      return { buffer: null, quitCommand: command };
    }
    if (command === "v") {
      return { buffer: null, viewCycleCommand: true };
    }
    if (command === "u") {
      return { buffer: null, gameboyRendererToggleCommand: true };
    }
    if (command === "a") {
      return { buffer: null, soundToggleCommand: true };
    }
    if (command === "c") {
      return { buffer: null, controlsToggleCommand: true };
    }
    if (command === "t") {
      return { buffer: null, agentToggleCommand: true };
    }
    const settingMatch = command.match(/^set\s+(model|goal|steps|cycle|delay|identity)\s*(.*)$/i);
    if (settingMatch) {
      const rawKey = settingMatch[1]!.toLowerCase();
      const rawValue = settingMatch[2]?.trim() ?? "";
      const key =
        rawKey === "model"
          ? "model"
          : rawKey === "goal"
          ? "goal"
          : rawKey === "steps"
          ? "maxSteps"
          : rawKey === "cycle"
          ? "graphCycleSteps"
          : rawKey === "delay"
          ? "requestDelayMs"
          : "identityName";
      if (!rawValue || rawValue === "default" || rawValue === "clear" || rawValue === "unset") {
        return { buffer: null, agentSettingCommand: { key, value: undefined } };
      }
      if (key === "maxSteps" || key === "graphCycleSteps" || key === "requestDelayMs") {
        const value = Number.parseInt(rawValue, 10);
        if (!Number.isFinite(value) || value <= 0) {
          return { buffer: null, note: `:set ${rawKey} requires a positive integer`, isError: true };
        }
        return { buffer: null, agentSettingCommand: { key, value } };
      }
      return { buffer: null, agentSettingCommand: { key, value: rawValue } };
    }
    const messageMatch = command.match(/^i\s+(.+)$/i);
    if (messageMatch?.[1]?.trim()) {
      return { buffer: null, agentMessageCommand: messageMatch[1].trim() };
    }
    if (command === "i") {
      return { buffer: null, note: "Usage: :i tell the agent what to do next", isError: true };
    }
    return { buffer: null, note: `Unknown command :${command}`, isError: true };
  }
  if (key === "\u007f" || key === "\b") {
    const next = state.buffer.length > 1 ? state.buffer.slice(0, -1) : null;
    return next ? { buffer: next, note: `COMMAND> ${next}`, isError: false } : { buffer: null };
  }
  if (key.length === 1 && key >= " " && key <= "~") {
    const next = `${state.buffer}${key}`;
    return { buffer: next, note: `COMMAND> ${next}`, isError: false };
  }
  return state;
};
