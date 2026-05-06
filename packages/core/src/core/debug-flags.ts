export type DebugFlagSource = {
  tokens: Set<string>;
  console: boolean;
};

declare global {
  interface Window {
    __POKE_DEBUG__?: string | {
      tokens?: string | string[];
      console?: boolean;
    };
  }
}

const normalizeTokenList = (value: unknown): Set<string> => {
  if (typeof value !== "string") {
    return new Set();
  }
  const tokens = value
    .split(/[,\s]+/g)
    .map((token) => token.trim().toLowerCase())
    .filter(Boolean);
  return new Set(tokens);
};

const mergeTokens = (...sets: Array<Set<string>>): Set<string> => {
  const merged = new Set<string>();
  for (const set of sets) {
    for (const token of set) {
      merged.add(token);
    }
  }
  return merged;
};

const DEFAULT_DEBUG_TOKENS = new Set<string>([
  "overworld:interaction",
  "interaction",
]);

const parseEnv = (): DebugFlagSource => {
  const tokens = mergeTokens(
    normalizeTokenList(process.env.NEXT_PUBLIC_POKE_DEBUG),
    normalizeTokenList(process.env.POKE_DEBUG),
    DEFAULT_DEBUG_TOKENS,
  );
  const consoleEnabled =
    process.env.NEXT_PUBLIC_POKE_DEBUG_CONSOLE === "1" ||
    process.env.POKE_DEBUG_CONSOLE === "1";
  return { tokens, console: consoleEnabled };
};

const parseBrowser = (): DebugFlagSource => {
  if (typeof window === "undefined") {
    return parseEnv();
  }

  let queryTokens = new Set<string>();
  let queryConsole = false;
  try {
    const params = new URLSearchParams(window.location?.search ?? "");
    queryTokens = normalizeTokenList(params.get("poke_debug") ?? params.get("debug"));
    const consoleValue = params.get("poke_debug_console") ?? params.get("debug_console");
    queryConsole = consoleValue === "1" || consoleValue === "true";
  } catch {
    // Ignore query parsing failures.
  }

  let storageTokens = new Set<string>();
  let storageConsole = false;
  try {
    storageTokens = normalizeTokenList(window.localStorage?.getItem("poke_debug"));
    const value = window.localStorage?.getItem("poke_debug_console");
    storageConsole = value === "1" || value === "true";
  } catch {
    // Ignore localStorage issues (e.g., blocked).
  }

  let windowTokens = new Set<string>();
  let windowConsole = false;
  const maybeDebug = window.__POKE_DEBUG__;
  if (typeof maybeDebug === "string") {
    windowTokens = normalizeTokenList(maybeDebug);
  } else if (maybeDebug && typeof maybeDebug === "object") {
    if (Array.isArray(maybeDebug.tokens)) {
      windowTokens = normalizeTokenList(maybeDebug.tokens.join(","));
    } else if (typeof maybeDebug.tokens === "string") {
      windowTokens = normalizeTokenList(maybeDebug.tokens);
    }
    windowConsole = Boolean(maybeDebug.console);
  }

  const env = parseEnv();
  return {
    tokens: mergeTokens(env.tokens, queryTokens, storageTokens, windowTokens),
    console: env.console || queryConsole || storageConsole || windowConsole,
  };
};

const tokenMatchesChannel = (token: string, channel: string): boolean => {
  if (!token || !channel) {
    return false;
  }
  if (token === "all" || token === "*") {
    return true;
  }
  if (token === channel) {
    return true;
  }
  if (channel.startsWith(`${token}:`)) {
    return true;
  }
  if (token.endsWith(":*")) {
    const prefix = token.slice(0, -"*".length);
    return channel.startsWith(prefix);
  }
  return false;
};

export const getDebugFlags = (): DebugFlagSource => {
  return parseBrowser();
};

export const isDebugEnabled = (channel: string): boolean => {
  const { tokens } = getDebugFlags();
  const normalized = channel.trim().toLowerCase();
  for (const token of tokens) {
    if (tokenMatchesChannel(token, normalized)) {
      return true;
    }
  }
  return false;
};

export const isDebugConsoleEnabled = (): boolean => {
  return getDebugFlags().console;
};
