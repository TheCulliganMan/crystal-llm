import { calculateExperience } from "@pokecrystal/core/engine/experience";
import { GrowthRate } from "@pokecrystal/core/core/enums";
import type { LearnedMove, Pokemon } from "@pokecrystal/core/core/models";
import { MoveName, Stat } from "@pokecrystal/core/core/enums";
import { BattleMenu } from "./overlays/_battle-menu";
import { waiting_flag as battleWaitingFlag } from "./overlays/battle-dialogue";
import { loadMoveMetadata } from "./overlays/battle-experience";
import { menu_header_for_battle } from "./overlays/battle-ui-draw";
import { BattleUIState, MoveLearningPhase } from "./overlays/battle-ui-state";
import { BattleContext, BattleStateEnum, Weather } from "@pokecrystal/core/engine/battle/battle/battle-context";
import { BagMenu } from "./menus/bag-menu";
import { Menu } from "./menus/menu";
import { MoveReorderMenu } from "./menus/move-reorder-menu";
import { OptionsMenu } from "./menus/options-menu";
import { PokemonMenu } from "./menus/pokemon-menu";
import { TextUI } from "./text-ui";
import {
  buildContinueScreenControlLines,
  buildGenderSelectionControlLines,
  buildIntroSequenceControlLines,
  buildOakIntroControlLines,
  buildTitleScreenControlLines,
} from "./control-lines";

const GrowthRate_VALUES = new Set(Object.values(GrowthRate));

type BattleContextDebugFields = BattleContext &
  Partial<{
    turn_count: number;
    turn_counter: number;
    turn: number;
    weather: Weather;
    weatherTurns: number;
    playerSpikesLayers: number;
    enemySpikesLayers: number;
    playerReflectTurns: number;
    enemyReflectTurns: number;
    playerLightScreenTurns: number;
    enemyLightScreenTurns: number;
    playerSafeguardTurns: number;
    enemySafeguardTurns: number;
  }>;

const resolveDebugContext = (
  context: BattleUIState["context"]
): BattleContextDebugFields | null => {
  return context ? (context as BattleContextDebugFields) : null;
};

const formatBarrierLine = (
  label: string,
  playerValue: number | undefined,
  enemyValue: number | undefined
): string | null => {
  if (playerValue === undefined && enemyValue === undefined) {
    return null;
  }
  return `${label}: player=${Number(playerValue ?? 0)} enemy=${Number(enemyValue ?? 0)}`;
};

export type TextSnapshotPayload = {
  viewportLines: string[];
  infoLines: string[];
  viewportTitle: string;
  infoTitle: string;
  menuLines?: string[] | null;
  promptLines?: string[] | null;
  dialogueLines?: string[] | null;
};

type TextSnapshotTarget = {
  renderSnapshot: TextUI["renderSnapshot"];
  getSnapshot?: TextUI["getSnapshot"];
};

export const resolveTextSnapshotTarget = (candidate: unknown): TextSnapshotTarget | null => {
  if (!candidate) {
    return null;
  }
  const record = candidate as {
    renderSnapshot?: unknown;
    getChildren?: () => unknown[];
  };
  if (typeof record.renderSnapshot === "function") {
    return candidate as TextSnapshotTarget;
  }
  if (typeof record.getChildren === "function") {
    for (const child of record.getChildren()) {
      const resolved = resolveTextSnapshotTarget(child);
      if (resolved) {
        return resolved;
      }
    }
  }
  return null;
};

export const renderTextSnapshot = (ui: unknown, payload: TextSnapshotPayload): void => {
  const target = resolveTextSnapshotTarget(ui);
  if (!target) {
    return;
  }
  target.renderSnapshot(
    payload.viewportLines,
    payload.infoLines,
    payload.viewportTitle,
    payload.infoTitle,
    payload.menuLines ?? null,
    payload.promptLines ?? null,
    payload.dialogueLines ?? null
  );
};

export function renderBagTextOverlay(bagMenu: BagMenu): void {
  const ui = bagMenu.getUi();
  const target = resolveTextSnapshotTarget(ui);
  if (!target) {
    return;
  }
  const gameState = bagMenu.getGameState();
  const money = gameState.sram?.money ?? 0;
  const viewportLines = ["BAG", `MONEY: ¥${Number(money).toLocaleString("en-US")}`];
  const infoLines = buildBagControlLines(bagMenu);
  const menuLines = bagMenuLines(bagMenu);
  target.renderSnapshot(viewportLines, infoLines, "Bag", "Legend", menuLines, null, null);
}

const bagMenuLines = (bagMenu: BagMenu): string[] => {
  const lines = [`POCKET: ${bagMenu.getCurrentPocketLabel()}`];
  const visibleItems = bagMenu.getVisibleItems();
  const allItems = bagMenu.getCurrentItems();
  const scrollMarkers = scrollMarkerLines(
    bagMenu.getScrollOffset(),
    visibleItems.length,
    allItems.length,
  );
  lines.push(...scrollMarkers.above);
  for (let idx = 0; idx < visibleItems.length; idx += 1) {
    const [name, qty] = visibleItems[idx];
    const absoluteIdx = bagMenu.getScrollOffset() + idx;
    const cursor = absoluteIdx === bagMenu.getListIndex();
    const qtyStr = name === "CANCEL" ? "" : `×${String(qty).padStart(2, "0")}`;
    lines.push(cursorLine(`${name} ${qtyStr}`.trim(), cursor));
  }
  lines.push(...scrollMarkers.below);
  if (bagMenu.getMode() === "actions") {
    lines.push("ACTIONS:");
    const options = bagMenu.getActionOptions();
    for (let idx = 0; idx < options.length; idx += 1) {
      lines.push(cursorLine(options[idx], idx === bagMenu.getActionIndex()));
    }
  }
  return lines;
};

export const buildBagMenuLines = bagMenuLines;

const legendLines = (firstLine: string, ...rest: string[]): string[] => {
  return [firstLine, ...rest];
};

export const buildStartMenuControlLines = (): string[] =>
  legendLines("D-Pad=Move A/Start=Select B=Close");

export const buildBagControlLines = (bagMenu: Pick<BagMenu, "getMode">): string[] => {
  if (bagMenu.getMode() === "actions") {
    return legendLines("D-Pad=Move A=Confirm B=Back");
  }
  return legendLines("D-Pad=Move L/R=Pocket A=Select Select=Register B=Close");
};

export const buildPokemonMenuControlLines = (menu: Pick<PokemonMenu, "getMode">): string[] => {
  const mode = menu.getMode();
  if (mode === "submenu") {
    return legendLines("D-Pad=Move A=Confirm B=Back");
  }
  if (mode === "switch") {
    return legendLines("D-Pad=Move A=Swap B=Cancel");
  }
  if (mode === "give_take") {
    return legendLines("Up/Down=Toggle A=Confirm B=Back");
  }
  return legendLines("D-Pad=Move A=Select B=Back");
};

export const buildPokemonStatsControlLines = (pokemon: Pokemon | null): string[] => {
  if (!pokemon) {
    return legendLines("B=Back");
  }
  const speciesId = String(pokemon.species?.id ?? "").toUpperCase();
  if (speciesId === "EGG") {
    return legendLines("A/B=Back");
  }
  return legendLines("L/R/A=Page", "Up/Down=Pokemon B=Back");
};

export const buildMoveMenuControlLines = (menu: Pick<MoveReorderMenu, "getSwapOrigin">): string[] => {
  if (menu.getSwapOrigin() !== null) {
    return legendLines("D-Pad=Move A=Swap B=Cancel");
  }
  return legendLines("D-Pad=Move L/R=Pokemon A=Pick B=Back");
};

export const buildOptionsMenuControlLines = (): string[] =>
  legendLines("D-Pad=Move L/R=Change A=Exit B=Back");

export const buildTrainerCardControlLines = (): string[] =>
  legendLines("L/R=Page A=Toggle B/Start=Exit");

export const buildActiveOptionMenuControlLines = (): string[] =>
  legendLines("D-Pad=Move A=Confirm B=Back");

export const buildDialogueControlLines = (): string[] =>
  legendLines("A=Advance B=Close");

export const buildPromptControlLines = (): string[] =>
  legendLines("Up/Down=Choose A=OK B=Cancel");

export {
  buildContinueScreenControlLines,
  buildGenderSelectionControlLines,
  buildIntroSequenceControlLines,
  buildOakIntroControlLines,
  buildTitleScreenControlLines,
};

export const buildActiveOptionMenuLines = (menu: Menu): string[] => {
  const options = menu.getOptions();
  const selected = menu.getSelectedOption();
  const cursorVisible = menu.cursorVisible;
  return options.map((option, idx) => cursorLine(option, idx === selected && cursorVisible));
};

export const buildMoveMenuLines = (menu: MoveReorderMenu): string[] => {
  const moves = menu.getMoveNames();
  const selected = menu.getSelectionIndex();
  if (!moves.length) {
    return [cursorLine("No moves", true)];
  }
  return moves.map((move, idx) => cursorLine(move, idx === selected));
};

export const buildOptionsMenuLines = (menu: OptionsMenu): string[] => {
  return menu.getTextMenuLines();
};

export const buildPokemonMenuLines = (menu: PokemonMenu): string[] => {
  const entries = menu.getPartyEntries();
  const cursorIndex = menu.getCursorIndex();
  const lines: string[] = [];
  for (let idx = 0; idx < entries.length; idx += 1) {
    const entry = entries[idx];
    const mon = entry.pokemon;
    const hp = Number(mon.hp ?? 0);
    const maxHp = Number((mon as Pokemon & { max_hp?: number }).max_hp ?? hp);
    const status = statusLabel(mon);
    const label = `${labelFor(mon)} L${mon.level} HP ${hp}/${maxHp} ${status}`;
    lines.push(cursorLine(label.trim(), idx === cursorIndex));
  }
  const cancelActive = cursorIndex >= entries.length;
  lines.push(cursorLine("CANCEL", cancelActive));
  if (menu.getMode() === "submenu") {
    lines.push("SUBMENU:");
    const choices = menu.getSubmenuChoices();
    const submenuIndex = menu.getSubmenuIndex();
    for (let idx = 0; idx < choices.length; idx += 1) {
      lines.push(cursorLine(choices[idx].label, idx === submenuIndex));
    }
  }
  return lines;
};

export const render_battle_text_overlay = renderBattleTextOverlay;

const scrollMarkerLines = (
  scrollOffset: number,
  visibleCount: number,
  totalCount: number
): { above: string[]; below: string[] } => ({
  above: scrollOffset > 0 ? ["▲ more above"] : [],
  below: scrollOffset + visibleCount < totalCount ? ["▼ more below"] : [],
});

export const buildBattleControlLines = (options: { hasPrompt: boolean; hasDialogue: boolean }): string[] => {
  if (options.hasPrompt) {
    return buildPromptControlLines();
  }
  if (options.hasDialogue) {
    return buildDialogueControlLines();
  }
  return legendLines("D-Pad=Move A=Confirm B=Back");
};

export const buildBattleSnapshot = (state: BattleUIState): TextSnapshotPayload | null => {
  const context = state.context;
  const player = context?.playerPokemon ?? null;
  const enemy = context?.enemyPokemon ?? null;
  if (!player || !enemy) {
    return null;
  }
  const viewportLines = battleViewportLines(player, enemy);
  const dialogueLinesList = dialogueLines(state);
  const promptLinesList = promptLines(state);
  const hasPrompt = Boolean(promptLinesList && promptLinesList.length);
  const hasDialogue = Boolean(dialogueLinesList.length);
  const infoLines = buildBattleControlLines({
    hasPrompt,
    hasDialogue,
  });
  const moveForget = moveForgetLines(state);
  const menuLinesList = hasPrompt || hasDialogue ? moveForget ?? null : menuLines(state, player);
  return {
    viewportLines,
    infoLines,
    viewportTitle: "Battle",
    infoTitle: "Legend",
    menuLines: menuLinesList,
    promptLines: promptLinesList,
    dialogueLines: dialogueLinesList,
  };
};

export function renderBattleTextOverlay(state: BattleUIState): void {
  const snapshot = buildBattleSnapshot(state);
  if (!snapshot) {
    return;
  }
  renderTextSnapshot(state.ui, snapshot);
}

const battleViewportLines = (player: Pokemon, enemy: Pokemon): string[] => {
  const enemyLine = formatMonLine(enemy, "ENEMY");
  const playerLine = formatMonLine(player, "ALLY ");
  const expToNextValue = expToNext(player);
  const expLine = expToNextValue === null
    ? `ALLY EXP ${player.experience}`
    : `ALLY EXP ${player.experience} NEXT ${expToNextValue}`;
  return [enemyLine, playerLine, expLine];
};

const dialogueLines = (state: BattleUIState): string[] => {
  const window = state.dialogue?.dialogue as { current_page_text?: unknown; visible_text?: unknown } | undefined;
  const text =
    typeof window?.current_page_text === "string"
      ? window.current_page_text
      : typeof window?.visible_text === "string"
        ? window.visible_text
        : "";
  const lines = String(text)
    .split("\n")
    .map((line) => line.trimEnd())
    .filter((line) => line.length > 0);
  if (!lines.length && battleWaitingFlag(state.dialogue)) {
    return ["Waiting for input..."];
  }
  return lines;
};

const promptLines = (state: BattleUIState): string[] | null => {
  const prompt = state.yes_no_prompt;
  if (!prompt?.active || !prompt.prompt) {
    return null;
  }
  const selection = prompt.prompt.selection;
  return ["YES", "NO"].map((label, idx) =>
    cursorLine(label, idx === selection)
  );
};

const menuLines = (state: BattleUIState, player: Pokemon): string[] | null => {
  const moveForget = moveForgetLines(state);
  if (moveForget) {
    return moveForget;
  }
  if (state.active_move_learn) {
    return null;
  }
  if (state.context?.currentState !== BattleStateEnum.PLAYER_ACTION_SELECT) {
    return null;
  }
  const current = state.wram.current_menu;
  if (current === BattleMenu.MAIN) {
    const header = menu_header_for_battle(state);
    const labels = (header.labels?.length ? header.labels : ["FIGHT", "PKMN", "PACK", "RUN"]).map((label) =>
      label.trim()
    );
    const cursor = header.clamp_cursor(state.wram.wBattleMenuCursorPosition);
    const cellWidth = Math.max(...labels.map((label) => label.length)) + 2;
    const rows: string[] = [];
    for (let row = 0; row < header.rows; row += 1) {
      const cells: string[] = [];
      for (let col = 0; col < header.cols; col += 1) {
        const idx = row * header.cols + col;
        const label = labels[idx];
        if (label === undefined) {
          continue;
        }
        const cell = cursorLine(label, idx === cursor);
        cells.push(col === header.cols - 1 ? cell : cell.padEnd(cellWidth, " "));
      }
      if (cells.length) {
        rows.push(cells.join(" ").trimEnd());
      }
    }
    return rows;
  }
  if (current === BattleMenu.FIGHT) {
    return moveLines(player, state.wram.wMoveMenuCursorPosition);
  }
  if (current === BattleMenu.POKEMON) {
    return partyLines(state, player);
  }
  if (current === BattleMenu.PACK) {
    return packLines(state);
  }
  return null;
};

const moveLines = (pokemon: Pokemon, cursor: number): string[] => {
  const moves = Array.isArray(pokemon.moves) ? pokemon.moves : [];
  const lines: string[] = [];
  for (let idx = 0; idx < moves.length; idx += 1) {
    const move = moves[idx];
    const maxPp = maxPpForMove(move);
    const label = moveDisplayName(move);
    const currentPp = moveCurrentPp(move);
    const disabled =
      move &&
      pokemon.disable_turns > 0 &&
      pokemon.disabled_move === move.name;
    lines.push(cursorLine(label, idx === cursor, disabled ? "DISABLED" : `PP ${currentPp}/${maxPp}`));
  }
  lines.push(cursorLine("CANCEL", moves.length === cursor));
  return lines;
};

const partyLines = (state: BattleUIState, player: Pokemon): string[] => {
  const members = (state.context?.playerParty ?? []) as Pokemon[];
  const lines: string[] = [];
  for (let idx = 0; idx < members.length; idx += 1) {
    const mon = members[idx];
    if (!mon) {
      continue;
    }
    const hp = Number(mon.hp ?? 0);
    const maxHp = Number((mon as Pokemon & { max_hp?: number }).max_hp ?? hp);
    const label = `${labelFor(mon)} L${mon.level} HP ${hp}/${maxHp}`;
    lines.push(cursorLine(label, idx === state.wram.wPartyMenuCursorPosition));
  }
  if (!lines.length) {
    lines.push(cursorLine(labelFor(player), true));
  }
  return lines;
};

const packLines = (state: BattleUIState): string[] => {
  const items = state.wram.last_item_names ?? [];
  if (!items.length) {
    return [cursorLine("Empty bag", true)];
  }
  return items.map((name, idx) =>
    cursorLine(name, idx === state.wram.wPackMenuCursorPosition)
  );
};

const cursorLine = (label: string, active: boolean, suffix?: string | null): string => {
  const prefix = active ? "▶" : "  ";
  if (suffix) {
    return `${prefix} ${label} (${suffix})`;
  }
  return `${prefix} ${label}`;
};

const STAT_ABBREVIATIONS: Record<Stat, string> = {
  [Stat.HP]: "HP",
  [Stat.ATTACK]: "ATK",
  [Stat.DEFENSE]: "DEF",
  [Stat.SPEED]: "SPE",
  [Stat.SPECIAL_ATTACK]: "SPA",
  [Stat.SPECIAL_DEFENSE]: "SPD",
  [Stat.ACCURACY]: "ACC",
  [Stat.EVASION]: "EVA",
};

const statStageLines = (player: Pokemon, enemy: Pokemon): string[] => {
  const lines: string[] = [];
  const stats: Stat[] = [
    Stat.ATTACK,
    Stat.DEFENSE,
    Stat.SPEED,
    Stat.SPECIAL_ATTACK,
    Stat.SPECIAL_DEFENSE,
    Stat.ACCURACY,
    Stat.EVASION,
  ];
  const entries: Array<[string, Pokemon]> = [
    ["ALLY", player],
    ["ENEMY", enemy],
  ];
  for (const [label, mon] of entries) {
    const boosts = (mon as Pokemon & { stat_boosts?: Record<string, number> }).stat_boosts ?? {};
    const parts = stats.map((stat) => {
      const value = Number(boosts[stat] ?? 0);
      const prefix = value >= 0 ? "+" : "";
      return `${STAT_ABBREVIATIONS[stat]} ${prefix}${value}`;
    });
    lines.push(`STAT STAGES ${label}: ${parts.join(" ")}`);
  }
  return lines;
};

const turnAndWeatherLines = (context: BattleUIState["context"]): string[] => {
  const lines: string[] = [];
  const ctx = resolveDebugContext(context);
  if (!ctx) {
    return lines;
  }
  const candidateTurn = ctx.turn_count ?? ctx.turn_counter ?? ctx.turn ?? null;
  if (candidateTurn !== null && candidateTurn !== undefined) {
    lines.push(`TURN: ${Number(candidateTurn)}`);
  }
  const weather = ctx.weather;
  if (weather !== undefined && weather !== null) {
    const turns = ctx.weatherTurns;
    const suffix = turns !== undefined && turns !== null ? ` (${Number(turns)})` : "";
    lines.push(`WEATHER: ${String(weather)}${suffix}`);
  }
  return lines;
};

const fieldEffectLines = (context: BattleUIState["context"]): string[] => {
  const lines: string[] = [];
  const ctx = resolveDebugContext(context);
  if (!ctx) {
    return lines;
  }
  const spikesPlayer = ctx.playerSpikesLayers;
  const spikesEnemy = ctx.enemySpikesLayers;
  if (spikesPlayer !== undefined || spikesEnemy !== undefined) {
    lines.push(`SPIKES: player=${Number(spikesPlayer ?? 0)} enemy=${Number(spikesEnemy ?? 0)}`);
  }
  const reflectLine = formatBarrierLine("REFLECT", ctx.playerReflectTurns, ctx.enemyReflectTurns);
  if (reflectLine) {
    lines.push(reflectLine);
  }
  const lightLine = formatBarrierLine(
    "LIGHT_SCREEN",
    ctx.playerLightScreenTurns,
    ctx.enemyLightScreenTurns
  );
  if (lightLine) {
    lines.push(lightLine);
  }
  const safeguardLine = formatBarrierLine(
    "SAFEGUARD",
    ctx.playerSafeguardTurns,
    ctx.enemySafeguardTurns
  );
  if (safeguardLine) {
    lines.push(safeguardLine);
  }
  return lines;
};

const labelFor = (mon: Pokemon): string => {
  const nickname = (mon as Pokemon & { nickname?: string }).nickname ?? "";
  const speciesId = (mon as Pokemon & { species?: { id?: string } }).species?.id ?? "";
  if (nickname) {
    return nickname;
  }
  if (speciesId) {
    return String(speciesId);
  }
  return "UNKNOWN";
};

const formatMonLine = (mon: Pokemon, prefix: string): string => {
  const status = statusLabel(mon);
  const item = (mon as Pokemon & { item?: string | null }).item ?? "NONE";
  const hp = Number(mon.hp ?? 0);
  const maxHp = Number((mon as Pokemon & { max_hp?: number }).max_hp ?? hp);
  return `${prefix} ${labelFor(mon)} L${mon.level} HP ${hp}/${maxHp} STATUS ${status} ITEM ${item}`;
};

const statusLabel = (mon: Pokemon): string => {
  const status = (mon as Pokemon & { status?: string | { name?: string } | null }).status;
  if (!status) {
    return "OK";
  }
  if (typeof status === "string") {
    return status || "OK";
  }
  const name = (status as { name?: string }).name;
  if (name) {
    return String(name);
  }
  return String(status);
};

const playerStatusLine = (mon: Pokemon): string => {
  const status = statusLabel(mon);
  const item = (mon as Pokemon & { item?: string | null }).item ?? "NONE";
  const expTo = expToNext(mon);
  const expText = expTo === null ? "EXP TO NEXT MAX" : `EXP TO NEXT ${expTo}`;
  return `PLAYER STATUS: ${status} ITEM ${item} ${expText}`;
};

const expToNext = (mon: Pokemon): number | null => {
  const growth = (mon as Pokemon & { species?: { growth_rate?: unknown } }).species?.growth_rate;
  const currentExp = (mon as Pokemon & { experience?: number }).experience;
  const level = Number(mon.level ?? 0);
  if (!growth || currentExp === undefined || currentExp === null) {
    return null;
  }
  if (level >= 100) {
    return 0;
  }
  try {
    if (!GrowthRate_VALUES.has(growth as GrowthRate)) {
      return null;
    }
    const target = calculateExperience(growth as GrowthRate, level + 1);
    return Math.max(0, target - Number(currentExp));
  } catch {
    return null;
  }
};

const trainerInfo = (context: BattleUIState["context"], state: BattleUIState): string | null => {
  const trainer = (context as { enemyTrainer?: { trainer_class?: string; name?: string; base_reward?: number } } | null)
    ?.enemyTrainer;
  if (trainer) {
    const parts: string[] = [];
    if (trainer.trainer_class) {
      parts.push(String(trainer.trainer_class));
    }
    if (trainer.name) {
      parts.push(String(trainer.name));
    }
    if (trainer.base_reward) {
      parts.push(`reward=${trainer.base_reward}`);
    }
    const label = parts.length ? parts.join(" ") : "unknown";
    return `TRAINER: ${label}`;
  }
  const wram = state.game_state?.wram;
  const otherClass = wram?.other_trainer_class ?? "";
  const otherId = wram?.other_trainer_id ?? "";
  if (otherClass || otherId) {
    const label = otherId ? `${otherClass || "trainer"} (${otherId})` : otherClass || "trainer";
    return `TRAINER: ${label}`;
  }
  if (context?.trainerBattle) {
    return "TRAINER: unknown";
  }
  return null;
};

const pendingExpLine = (state: BattleUIState): string | null => {
  const expAnim = state.exp_animation as { pendingLevels?: unknown[]; targetExp?: number } | null | undefined;
  if (!expAnim || expAnim.targetExp === undefined || expAnim.targetExp === null) {
    return null;
  }
  const pendingLevels = expAnim.pendingLevels ?? [];
  const pieces = [`PENDING EXP → ${expAnim.targetExp}`];
  if (pendingLevels.length) {
    pieces.push(`+${pendingLevels.length} level(s)`);
  }
  return pieces.join(" ");
};

const moveForgetLines = (state: BattleUIState): string[] | null => {
  const process = state.active_move_learn;
  const menu = state.move_forget_menu;
  if (!process || !menu) {
    return null;
  }
  if (process.stage !== MoveLearningPhase.FORGET_MENU) {
    return null;
  }
  const moves = Array.isArray(process.pokemon.moves) ? process.pokemon.moves : [];
  const labels: string[] = [];
  for (let idx = 0; idx < moves.length; idx += 1) {
    const move = moves[idx];
    if (!move) {
      continue;
    }
    const maxPp = maxPpForMove(move);
    const displayName = moveDisplayName(move);
    const currentPp = moveCurrentPp(move);
    labels.push(
      cursorLine(displayName, idx === menu.selection, `PP ${currentPp}/${maxPp}`)
    );
  }
  labels.push(cursorLine("CANCEL", menu.selection === labels.length));
  return labels;
};

type LegacyMoveRecord = {
  name?: string | { value?: string; name?: string };
  move?: string;
  current_pp?: number;
  pp?: number;
};

const hasLegacyPp = (value: unknown): value is LegacyMoveRecord & { pp: number } =>
  typeof value === "object" && value !== null && typeof (value as LegacyMoveRecord).pp === "number";

type MoveLike = LearnedMove | LegacyMoveRecord | string | null | undefined;

const extractMoveName = (move: LegacyMoveRecord): string | null => {
  const nameValue = move.name;
  if (typeof nameValue === "string") {
    return nameValue;
  }
  if (nameValue && typeof nameValue === "object") {
    if (typeof nameValue.value === "string") {
      return nameValue.value;
    }
    if (typeof nameValue.name === "string") {
      return nameValue.name;
    }
  }
  const fallback = move.move;
  if (typeof fallback === "string") {
    return fallback;
  }
  return null;
};

const moveDisplayName = (move: MoveLike): string => {
  if (!move) {
    return "UNKNOWN";
  }
  if (typeof move === "string") {
    return move;
  }
  const name = extractMoveName(move);
  if (name) {
    return name;
  }
  return "UNKNOWN";
};

const moveCurrentPp = (move: MoveLike): number => {
  if (!move || typeof move !== "object") {
    return 0;
  }
  const current = move.current_pp;
  if (typeof current === "number") {
    return current;
  }
  if (hasLegacyPp(move)) {
    return move.pp;
  }
  return 0;
};

const MOVE_NAME_VALUES = new Set<string>(Object.values(MoveName));

const maxPpForMove = (move: MoveLike): number => {
  const metadata = loadMoveMetadata();
  const name = moveDisplayName(move);
  const normalized = name.toUpperCase();
  const key = MOVE_NAME_VALUES.has(normalized)
    ? (normalized as MoveName)
    : null;
  if (key) {
    const entry = metadata.get(key);
    if (entry) {
      return entry.pp;
    }
  }
  return moveCurrentPp(move);
};

const bagSummaryLines = (sram: { items?: Record<string, number>; balls?: Record<string, number>; key_items?: Record<string, number>; tm_hm?: number[] } | null, maxItemsPerPocket = 4): string[] => {
  if (!sram) {
    return [];
  }
  const pockets: Array<[string, Record<string, number> | number[] | undefined]> = [
    ["ITEMS", sram.items],
    ["BALL", sram.balls],
    ["KEY", sram.key_items],
    ["TM/HM", sram.tm_hm],
  ];
  const lines: string[] = [];
  for (const [label, storage] of pockets) {
    if (storage && !Array.isArray(storage)) {
      const entries = Object.entries(storage).filter(([, qty]) => qty > 0);
      entries.sort((a, b) => a[0].localeCompare(b[0]));
      let preview = entries
        .slice(0, maxItemsPerPocket)
        .map(([name, qty]) => `${name}×${String(qty).padStart(2, "0")}`)
        .join(", ");
      if (entries.length > maxItemsPerPocket) {
        preview = `${preview} (+${entries.length - maxItemsPerPocket})`;
      }
      lines.push(`BAG[${label}]: ${preview || "empty"}`);
    } else if (Array.isArray(storage)) {
      const owned = storage.filter((value) => Boolean(value)).length;
      lines.push(`BAG[${label}]: ${owned} owned`);
    }
  }
  return lines;
};

const badgeSummaryLines = (badges: { johto?: boolean[]; kanto?: boolean[] } | null): string[] => {
  if (!badges) {
    return [];
  }
  const johto = (badges.johto ?? []).filter(Boolean).length;
  const kanto = (badges.kanto ?? []).filter(Boolean).length;
  const total = johto + kanto;
  return [`BADGES: ${total}/16 (JOHTO ${johto}/8 KANTO ${kanto}/8)`];
};

export const buildOverworldMetadata = (
  gameState: { wram?: Record<string, unknown>; sram?: Record<string, unknown> },
  _mapName: string | null,
  playerCoords?: { x: number; y: number } | null
): string[] => {
  const lines = ["D-Pad=Move A=Talk Start=Menu Select=Item B=Back"];
  if (
    playerCoords &&
    Number.isFinite(playerCoords.x) &&
    Number.isFinite(playerCoords.y)
  ) {
    lines.push(`Pos: (${playerCoords.x},${playerCoords.y})`);
    return lines;
  }
  const wram = gameState.wram ?? null;
  if (wram) {
    const x = Number(
      (wram as { wXCoord?: number; player_x?: number }).wXCoord ??
        (wram as { player_x?: number }).player_x ??
        0
    );
    const y = Number(
      (wram as { wYCoord?: number; player_y?: number }).wYCoord ??
        (wram as { player_y?: number }).player_y ??
        0
    );
    lines.push(`Pos: (${x},${y})`);
  }
  return lines;
};

export const formatOverworldPartyLines = (members: Iterable<Pokemon>): string[] => {
  const lines: string[] = [];
  for (const mon of members) {
    if (!mon) {
      continue;
    }
    let totalCurrent = 0;
    let totalMax = 0;
    const moveLabels: string[] = [];
    const moves = Array.isArray(mon.moves) ? mon.moves : [];
    for (const move of moves) {
      if (!move) {
        continue;
      }
      const currentPp = moveCurrentPp(move);
      const maxPp = maxPpForMove(move);
      totalCurrent += currentPp;
      totalMax += maxPp;
      const name = moveDisplayName(move);
      moveLabels.push(`${name} ${currentPp}/${maxPp}`);
    }
    const ppSuffix = ` PP ${totalCurrent}/${totalMax}`;
    const hp = Number(mon.hp ?? 0);
    const maxHp = Number((mon as Pokemon & { max_hp?: number }).max_hp ?? hp);
    const item = (mon as Pokemon & { item?: string | null }).item ?? "NONE";
    lines.push(
      `PARTY: ${labelFor(mon)} L${mon.level} HP ${hp}/${maxHp} STATUS ${statusLabel(mon)} ITEM ${item}${ppSuffix}`
    );
    if (moveLabels.length) {
      lines.push(`  MOVES: ${moveLabels.join(", ")}`);
    }
  }
  return lines;
};
