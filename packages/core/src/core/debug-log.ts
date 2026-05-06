import { isDebugConsoleEnabled } from "./debug-flags";

type DebugLogValue = string | number | boolean | null | undefined;
export type DebugLogDetails = Record<string, DebugLogValue | DebugLogValue[]>;
export type DebugLogEntry = {
  id: number;
  timestamp: number;
  message: string;
  details?: DebugLogDetails;
};

type DebugLogListener = (entries: DebugLogEntry[]) => void;

const listeners = new Set<DebugLogListener>();
const maxEntries = 200;
let nextId = 1;
let entries: DebugLogEntry[] = [];

const safeConsole = (method: "debug" | "info" | "warn" | "error", ...args: unknown[]): void => {
  try {
    const fn = console[method] as ((...inner: unknown[]) => void) | undefined;
    if (typeof fn === "function") {
      fn(...args);
      return;
    }
  } catch {
    // Ignore console failures.
  }
  try {
     
    console.log(...args);
  } catch {
    // Ignore.
  }
};

const notify = (): void => {
  for (const listener of listeners) {
    listener(entries);
  }
};

export const pushDebugLog = (message: string, details?: DebugLogDetails): void => {
  const consoleEnabled = isDebugConsoleEnabled();

  const entry: DebugLogEntry = {
    id: nextId++,
    timestamp: Date.now(),
    message,
    details,
  };
  entries = [...entries, entry].slice(-maxEntries);
  notify();

  if (consoleEnabled) {
    if (details && Object.keys(details).length) {
      safeConsole("debug", message, details);
    } else {
      safeConsole("debug", message);
    }
  }
};

export const getDebugLogEntries = (): DebugLogEntry[] => entries;

export const subscribeDebugLog = (listener: DebugLogListener): (() => void) => {
  listeners.add(listener);
  listener(entries);
  return () => {
    listeners.delete(listener);
  };
};
