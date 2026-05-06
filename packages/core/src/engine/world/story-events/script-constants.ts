import path from "path";
import { readJsonAssetSync } from "@pokecrystal/core/core/asset-reader";
import { getDataDir } from "@pokecrystal/core/core/paths";

const STORY_EVENT_SCRIPT_CONSTANTS_PATH = path.join(getDataDir(), "story_event_script_constants.json");

const TOKEN_RE = /\s*(<<|>>|[()+\-*/|&^]|\$[0-9A-Fa-f]+|%[01]+|0x[0-9A-Fa-f]+|0b[01]+|\d+|[A-Za-z_][A-Za-z0-9_]*)/y;

type Token = string | number;

export type StoryEventScriptConstants = {
  global: Record<string, number>;
  maps: Record<string, Record<string, number>>;
};

let cachedGlobalConstants: Record<string, number> | null = null;
const cachedMapConstants = new Map<string, Record<string, number>>();
let cachedStoryEventScriptConstants: StoryEventScriptConstants | null = null;

const tokenizeExpression = (expr: string): string[] => {
  const tokens: string[] = [];
  const trimmed = expr.trim();
  let index = 0;
  while (index < trimmed.length) {
    TOKEN_RE.lastIndex = index;
    const match = TOKEN_RE.exec(trimmed);
    if (!match) {
      throw new Error(`Unsupported token in expression: ${expr}`);
    }
    const token = match[1];
    index = TOKEN_RE.lastIndex;
    if (token) {
      tokens.push(token);
    }
  }
  return tokens;
};

export const resolveExpression = (expr: string, constants: Record<string, number>): number => {
  const tokens = tokenizeExpression(expr);
  if (!tokens.length) {
    throw new Error("Expression cannot be empty.");
  }
  const normalized: Token[] = [];
  for (const token of tokens) {
    if (["<<", ">>", "+", "-", "*", "/", "%", "|", "&", "^", "(", ")"].includes(token)) {
      normalized.push(token);
      continue;
    }
    if (token.startsWith("$")) {
      normalized.push(parseInt(token.slice(1), 16));
      continue;
    }
    if (token.startsWith("%")) {
      normalized.push(parseInt(token.slice(1), 2));
      continue;
    }
    if (token.toLowerCase().startsWith("0x")) {
      normalized.push(parseInt(token, 16));
      continue;
    }
    if (token.toLowerCase().startsWith("0b")) {
      normalized.push(parseInt(token, 2));
      continue;
    }
    if (/^\d+$/.test(token)) {
      normalized.push(parseInt(token, 10));
      continue;
    }
    if (token in constants) {
      normalized.push(constants[token]);
      continue;
    }
    throw new Error(`Unknown constant '${token}' in expression '${expr}'.`);
  }

  let index = 0;
  const peek = (): Token | undefined => normalized[index];
  const consume = (): Token => normalized[index++]!;

  const parsePrimary = (): number => {
    const token = consume();
    if (token === "(") {
      const value = parseExpressionValue();
      const closing = consume();
      if (closing !== ")") {
        throw new Error(`Expected ')' in expression '${expr}'.`);
      }
      return value;
    }
    if (typeof token === "number") {
      return token;
    }
    throw new Error(`Unexpected token '${token}' in expression '${expr}'.`);
  };

  const parseUnary = (): number => {
    const token = peek();
    if (token === "+" || token === "-") {
      consume();
      const value = parseUnary();
      return token === "-" ? -value : value;
    }
    return parsePrimary();
  };

  const parseMul = (): number => {
    let value = parseUnary();
    while (true) {
      const token = peek();
      if (token === "*" || token === "/" || token === "%") {
        consume();
        const rhs = parseUnary();
        if (token === "*") {
          value *= rhs;
        } else if (token === "/") {
          value = Math.floor(value / rhs);
        } else {
          value -= Math.floor(value / rhs) * rhs;
        }
      } else {
        break;
      }
    }
    return value;
  };

  const parseAdd = (): number => {
    let value = parseMul();
    while (true) {
      const token = peek();
      if (token === "+" || token === "-") {
        consume();
        const rhs = parseMul();
        value = token === "+" ? value + rhs : value - rhs;
      } else {
        break;
      }
    }
    return value;
  };

  const parseShift = (): number => {
    let value = parseAdd();
    while (true) {
      const token = peek();
      if (token === "<<" || token === ">>") {
        consume();
        const rhs = parseAdd();
        value = token === "<<" ? value << rhs : value >> rhs;
      } else {
        break;
      }
    }
    return value;
  };

  const parseAnd = (): number => {
    let value = parseShift();
    while (peek() === "&") {
      consume();
      value &= parseShift();
    }
    return value;
  };

  const parseXor = (): number => {
    let value = parseAnd();
    while (peek() === "^") {
      consume();
      value ^= parseAnd();
    }
    return value;
  };

  const parseExpressionValue = (): number => {
    let value = parseXor();
    while (peek() === "|") {
      consume();
      value |= parseXor();
    }
    return value;
  };

  const result = parseExpressionValue();
  if (index < normalized.length) {
    throw new Error(`Unexpected token '${normalized[index]}' in expression '${expr}'.`);
  }
  return result;
};

export const readRequiredScriptConstants = (): StoryEventScriptConstants => {
  if (cachedStoryEventScriptConstants) {
    return cachedStoryEventScriptConstants;
  }
  let bundled: unknown;
  try {
    bundled = readJsonAssetSync<unknown>(STORY_EVENT_SCRIPT_CONSTANTS_PATH);
  } catch {
    throw new Error(
      `Story event script constants are required for the asset-only runtime: missing or invalid ${STORY_EVENT_SCRIPT_CONSTANTS_PATH}.`
    );
  }
  if (!bundled || typeof bundled !== "object" || Array.isArray(bundled)) {
    throw new Error(
      `Story event script constants are required for the asset-only runtime: missing or invalid ${STORY_EVENT_SCRIPT_CONSTANTS_PATH}.`
    );
  }
  const root = bundled as {
    global?: Record<string, unknown>;
    maps?: Record<string, Record<string, unknown>>;
  };
  const global: Record<string, number> = {};
  for (const [name, value] of Object.entries(root.global ?? {})) {
    if (typeof value === "number" && Number.isFinite(value)) {
      global[name] = value;
    }
  }
  const maps: Record<string, Record<string, number>> = {};
  for (const [mapName, entry] of Object.entries(root.maps ?? {})) {
    if (!entry || typeof entry !== "object" || Array.isArray(entry)) {
      continue;
    }
    const constants: Record<string, number> = {};
    for (const [name, value] of Object.entries(entry)) {
      if (typeof value === "number" && Number.isFinite(value)) {
        constants[name] = value;
      }
    }
    maps[mapName] = constants;
  }
  if (!Object.keys(global).length) {
    throw new Error(
      `Story event script constants are required for the asset-only runtime: missing or invalid ${STORY_EVENT_SCRIPT_CONSTANTS_PATH}.`
    );
  }
  cachedStoryEventScriptConstants = { global, maps };
  return cachedStoryEventScriptConstants;
};

export const loadGlobalConstants = (): Record<string, number> => {
  if (cachedGlobalConstants) {
    return cachedGlobalConstants;
  }
  cachedGlobalConstants = readRequiredScriptConstants().global;
  return cachedGlobalConstants;
};

export const loadMapConstants = (mapName: string): Record<string, number> => {
  if (!mapName) {
    return {};
  }
  if (cachedMapConstants.has(mapName)) {
    return cachedMapConstants.get(mapName)!;
  }
  const constants = readRequiredScriptConstants().maps[mapName] ?? {};
  cachedMapConstants.set(mapName, constants);
  return constants;
};

export const resolveScriptConstantExpression = (expr: string, mapName?: string | null): number => {
  const constants: Record<string, number> = { ...loadGlobalConstants() };
  if (mapName) {
    Object.assign(constants, loadMapConstants(mapName));
  }
  return resolveExpression(expr, constants);
};
