import type { GameState } from "@pokecrystal/core/core/state";
import {
  CONTROL_CODE_REPLACEMENTS,
  POKE_GLYPH,
  applyTextReplacements,
} from "@pokecrystal/core/ui/text/constants";
import { resolveScriptConstantExpression } from "./script-constants";

const replaceAllText = (source: string, search: string, replacement: string): string => {
  return source.split(search).join(replacement);
};

export class TextFormatter {
  public gameState: GameState;
  public stringBuffers: Record<string, string>;
  private readonly getMapName?: () => string | null | undefined;

  constructor(gameState: GameState, options: { getMapName?: () => string | null | undefined } = {}) {
    this.gameState = gameState;
    this.stringBuffers = {};
    this.getMapName = options.getMapName;
  }

  formatText(text: string): string {
    let formatted = text;
    formatted = formatted.replace(/<STRING_BUFFER_(\d+)>/g, (_match, index) => {
      const key = `STRING_BUFFER_${index}`;
      return this.stringBuffers?.[key] ?? "";
    });
    const bufferEntries = Object.entries(this.stringBuffers ?? {});
    const indexedBuffers = bufferEntries
      .map(([key, value]) => {
        const match = /^STRING_BUFFER_(\d+)$/.exec(key);
        if (!match) {
          return null;
        }
        return { index: Number(match[1]), value };
      })
      .filter((entry): entry is { index: number; value: string } => Boolean(entry && Number.isFinite(entry.index)))
      .sort((a, b) => a.index - b.index)
      .map((entry) => entry.value);
    const replacementValues = indexedBuffers.length ? indexedBuffers : Object.values(this.stringBuffers);
    if (replacementValues.length) {
      let index = 0;
      formatted = formatted.replace(/@/g, () => {
        if (index >= replacementValues.length) {
          return "";
        }
        return replacementValues[index++];
      });
    }

    let normalized = this.substituteDayTokens(formatted);
    normalized = this.substituteNameTokens(normalized);
    normalized = this.substituteDecimalTokens(normalized);
    normalized = applyTextReplacements(normalized, CONTROL_CODE_REPLACEMENTS);
    return normalized.split(POKE_GLYPH).join("POK\u00e9");
  }

  private substituteDecimalTokens(text: string): string {
    if (!text.includes("{d:")) {
      return text;
    }
    const mapName = this.getMapName?.() ?? null;
    return text.replace(/\{d:([^}]+)\}/g, (_match, expression) => {
      const value = resolveScriptConstantExpression(String(expression), mapName);
      return String(value);
    });
  }

  private substituteNameTokens(text: string): string {
    const playerName = this.gameState.sram.player_name.trim() || "PLAYER";
    const rivalName = this.gameState.sram.rival_name.trim() || "RIVAL";
    const replacements: Record<string, string> = {
      "<PLAYER>": playerName,
      "<PLAY_G>": playerName,
      "<RIVAL>": rivalName,
    };

    let normalized = text;
    for (const [token, replacement] of Object.entries(replacements)) {
      normalized = replaceAllText(normalized, token, replacement);
    }
    return normalized;
  }

  private substituteDayTokens(text: string): string {
    const dayIndex = Number(this.gameState.sram.day_of_week);
    if (!Number.isFinite(dayIndex)) {
      return text;
    }
    const dayNames = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"];
    const replacement = dayNames[((Math.trunc(dayIndex) % 7) + 7) % 7];
    return replaceAllText(text, "<TODAY>", replacement);
  }
}
