import { asmTextLoader } from "@pokecrystal/core/core/asm-text-loader";
import { EventManager, openText, closeText } from "@pokecrystal/core/engine/events/events";
import { showText, waitForInput, LOGGER, STANDARD_TEXT_FALLBACKS } from "./common";
import type { OverworldContext as BaseOverworldContext } from "./commands/base";

export type TextLoader = {
  get_text?: (label: string) => string | null;
  getText?: (label: string) => string | null;
} | null;

export type ScriptContext = {
  dataLoader?: TextLoader;
  format_text?: (text: string) => string;
  formatText?: (text: string) => string;
} | null;

export type OverworldContext = BaseOverworldContext | { dataLoader?: TextLoader } | null;

const getTextFromLoader = (
  loader: TextLoader,
  label: string,
): string => {
  if (!loader) {
    return "";
  }
  try {
    return loader.get_text?.(label) ?? loader.getText?.(label) ?? "";
  } catch {
    return "";
  }
};

const formatText = (runner: ScriptContext, text: string): string => {
  if (!runner) {
    return text;
  }
  if (runner.format_text) {
    return runner.format_text(text);
  }
  if (runner.formatText) {
    return runner.formatText(text);
  }
  return text;
};

export const openTextBox = (eventManager: EventManager): void => {
  openText(eventManager);
};

export const closeTextBox = (eventManager: EventManager): void => {
  closeText(eventManager);
};

export const resolveText = (
  runner: ScriptContext,
  overworld: OverworldContext,
  label: string,
): string => {
  if (!label) {
    throw new Error("resolveText requires a non-empty label.");
  }
  let text = "";
  text = getTextFromLoader(runner?.dataLoader ?? null, label);
  if (!text) {
    text = getTextFromLoader(overworld?.dataLoader ?? null, label);
  }
  if (!text) {
    text = asmTextLoader.get(label);
  }
  if (!text && !label.startsWith("_")) {
    // ASM text_far pointers (e.g., CaughtAskNicknameText -> _CaughtAskNicknameText).
    text = asmTextLoader.get(`_${label}`);
  }
  if (!text) {
    text = STANDARD_TEXT_FALLBACKS[label] ?? "";
  }
  let formatted = formatText(runner, text);
  if (!formatted || !String(formatted).trim() || String(formatted).trim() === label) {
    throw new Error(`Missing ASM text for label '${label}'.`);
  }
  return formatted;
};

export const showLabelledText = (
  runner: ScriptContext,
  overworld: OverworldContext,
  eventManager: EventManager,
  label: string,
  {
    wait = true,
    logEvent = true,
    autoCloseAfterWait = false,
  }: {
    wait?: boolean;
    logEvent?: boolean;
    autoCloseAfterWait?: boolean;
  } = {},
): string => {
  const message = resolveText(runner, overworld, label);
  if (logEvent) {
    LOGGER.debug("Displaying text %s -> %s", label, message);
  }
  showText(eventManager, message, { auto_close_after_wait: Boolean(autoCloseAfterWait && wait) });
  if (wait) {
    waitForInput(eventManager, true);
  }
  return message;
};
