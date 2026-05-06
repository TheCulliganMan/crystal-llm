import type { TuiSnapshot } from "./tui-snapshot";
import {
  renderAgentCurrentLines,
  renderAgentEventLines,
  renderAgentMcpLines,
  renderAgentStreamLines,
  renderAgentTokenLines,
  type AgentStreamState,
} from "./agent-stream";
import {
  buildKittyPlaceholderLines,
  createKittyImageRenderer,
  isKittyGraphicsSupported,
  resolveKittyImageIds,
  type GameboyRendererMode,
  type KittyImageDisplay,
  type KittyImagePlacement,
  type KittyPngFrame,
} from "./tui-kitty";

export type ReactModuleLike = {
  createElement: (type: unknown, props?: Record<string, unknown> | null, ...children: unknown[]) => unknown;
  useEffect: (effect: () => void | (() => void), deps?: unknown[]) => void;
  useState: <T>(initial: T | (() => T)) => [T, (next: T | ((previous: T) => T)) => void];
};

export type InkModuleLike = {
  Box: unknown;
  Text: unknown;
  useStdoutDimensions?: () => [number, number];
  render: (
    tree: unknown,
    options?: {
      stdin?: NodeJS.ReadStream;
      stdout?: NodeJS.WriteStream;
      stderr?: NodeJS.WriteStream;
      exitOnCtrlC?: boolean;
      patchConsole?: boolean;
    },
  ) => { unmount: () => void; waitUntilExit?: () => Promise<void> };
};

export type InkRuntime = {
  React: ReactModuleLike;
  ink: InkModuleLike;
};

export type TuiViewState = {
  title: string;
  endpoint: string;
  sessionId: string;
  startedAtMs: number;
  elapsedMs: number;
  interactionCount: number;
  snapshot: TuiSnapshot;
  agentStream?: AgentStreamState;
  activeView?: "play" | "agent" | "agent-split" | "settings";
  gameboyRenderer?: GameboyRendererMode;
  gameboyImage?: KittyPngFrame;
  kittyImageDisplay?: KittyImageDisplay;
  settings?: TuiSettingsState;
  livePlay?: TuiLivePlayState;
  commandNote?: string;
  commandError?: boolean;
  controlsVisible?: boolean;
};

export type TuiLivePlayState = {
  active: boolean;
  remainingMs: number;
  actionCount: number;
  resuming: boolean;
};

export type TuiSettingsState = {
  agentStatus: "running" | "paused" | "stopped";
  agentPid?: number;
  agentModel?: string;
  agentGoal?: string;
  agentMaxSteps?: number;
  agentGraphCycleSteps?: number;
  agentRequestDelayMs?: number;
  agentIdentityName?: string;
  soundEnabled: boolean;
};

type TerminalDimensions = {
  columns?: number;
  rows?: number;
};

type ResizableTerminal = TerminalDimensions & {
  on?: (event: "resize", listener: () => void) => unknown;
  off?: (event: "resize", listener: () => void) => unknown;
  removeListener?: (event: "resize", listener: () => void) => unknown;
};

const nativeDynamicImport = new Function("specifier", "return import(specifier)") as (
  specifier: string,
) => Promise<Record<string, unknown>>;

type PanelLine = string | {
  text: string;
  color?: string;
};

type DiagnosticPanelSpec = {
  title: string;
  color: string;
  highlight?: boolean;
  buildLines: (maxLines: number) => PanelLine[];
};

export const loadInkRuntime = async (): Promise<InkRuntime> => {
  if (process.env.POKECRYSTAL_CLI_TEST_INK === "1") {
    return {
      React: {
        createElement: (type: unknown, props?: Record<string, unknown> | null, ...children: unknown[]) => {
          if (typeof type === "function") {
            return (type as (props: Record<string, unknown>) => unknown)({ ...(props ?? {}), children });
          }
          return { type, props, children };
        },
        useEffect: (effect) => {
          effect();
        },
        useState: <T,>(initial: T | (() => T)): [T, (next: T | ((previous: T) => T)) => void] => {
          let value = typeof initial === "function" ? (initial as () => T)() : initial;
          return [
            value,
            (next) => {
              value = typeof next === "function" ? (next as (previous: T) => T)(value) : next;
            },
          ];
        },
      },
      ink: {
        Box: "Box",
        Text: "Text",
        render: (tree, options) => {
          options?.stdout?.write(`${JSON.stringify(tree)}\n`);
          return { unmount: () => undefined };
        },
      },
    };
  }
  const [reactModule, inkModule] = await Promise.all([
    nativeDynamicImport("react"),
    nativeDynamicImport("ink"),
  ]);
  return {
    React: (reactModule.default ?? reactModule) as ReactModuleLike,
    ink: inkModule as unknown as InkModuleLike,
  };
};

const limitLines = (lines: string[], max: number): string[] =>
  (lines.length ? lines : [" "]).slice(0, max);

const limitLinesWithOverflow = (lines: string[], max: number): string[] => {
  const source = lines.length ? lines : [" "];
  if (source.length <= max) {
    return source.slice(0, max);
  }
  if (max <= 1) {
    return [`▼ ${source.length} rows hidden`];
  }
  return [
    ...source.slice(0, max - 1),
    `▼ ${source.length - (max - 1)} more rows hidden`,
  ];
};

const limitPanelLines = (lines: PanelLine[], max: number): PanelLine[] =>
  (lines.length ? lines : [" "]).slice(0, max);

const limitGameLines = (lines: string[], max: number): string[] => {
  const source = lines.length ? lines : [" "];
  if (source.length <= max) {
    return source;
  }
  const playerLineIndex = source.findIndex((line) => line.includes("@"));
  if (playerLineIndex < 0 || max <= 1) {
    return source.slice(0, max);
  }
  const header = source[0] ?? "";
  const body = source.slice(1);
  const playerBodyIndex = Math.max(0, playerLineIndex - 1);
  const bodyBudget = Math.max(1, max - 1);
  const start = Math.max(
    0,
    Math.min(playerBodyIndex - Math.floor(bodyBudget / 2), body.length - bodyBudget),
  );
  return [header, ...body.slice(start, start + bodyBudget)];
};

const terminalWidth = (value: number | undefined): number =>
  typeof value === "number" && Number.isFinite(value) && value > 0 ? Math.floor(value) : 100;

const terminalHeight = (value: number | undefined): number =>
  typeof value === "number" && Number.isFinite(value) && value > 0 ? Math.floor(value) : 32;

const readTerminalDimensions = (terminal?: TerminalDimensions): Required<TerminalDimensions> => ({
  columns: terminalWidth(terminal?.columns),
  rows: terminalHeight(terminal?.rows),
});

const wideControlParts = [
  "arrows/WASD/HJKL=d-pad",
  "A=Space/Z/J/Shift+A",
  "B=X/K/B",
  "Enter=Start",
  "Tab=Select",
  "name entry: type letters, Space selects",
  ".=wait",
  "R=refresh",
  "view :v",
  "render :u",
  ":c controls",
  ":a audio",
  ":t agent",
  ":set key value",
  ":i msg",
  "Esc/:=command",
];

const compactControlParts = [
  "arrows/WASD/HJKL=d-pad",
  "A=Sp/Z/J/S+A",
  "B=X/K/B",
  "Ent=St",
  "Tab",
  ".=wait",
  "R=ref",
  ":c controls",
  ":t agent",
  ":u render",
  "view :v",
  ":set key value",
  ":i msg",
  "Esc/:=cmd",
];

const sectionLines = (title: string, lines: string[]): string[] =>
  lines.length ? ["", title, ...lines] : [];

const isNameEntrySnapshot = (snapshot: TuiSnapshot): boolean =>
  snapshot.viewport.some((line) => line.trim().toUpperCase() === "NAME ENTRY");

const buildGameScreenLines = (snapshot: TuiSnapshot): string[] => {
  if (isNameEntrySnapshot(snapshot)) {
    return snapshot.viewport;
  }
  return [
    ...snapshot.viewport,
    ...sectionLines("DIALOGUE", snapshot.dialogue),
    ...sectionLines("PROMPT", snapshot.prompt),
    ...sectionLines("MENU", snapshot.menu),
  ];
};

const formatSettingValue = (value: string | number | undefined): string =>
  value === undefined || value === "" ? "(default)" : String(value);

const formatMaxStepsSettingValue = (value: number | undefined): string =>
  value === undefined || value === Number.POSITIVE_INFINITY ? "infinite" : String(value);

const formatTitle = (state: TuiViewState): string => {
  const livePlay = state.livePlay;
  if (!livePlay?.active) {
    return state.title;
  }
  const suffix = livePlay.resuming
    ? "Professor Culligan live play: resuming"
    : `Professor Culligan live play: resume ${Math.ceil(livePlay.remainingMs / 1000)}s`;
  return `${state.title} | ${suffix}`;
};

export const formatElapsedRunTime = (elapsedMs: number): string => {
  const totalSeconds = Math.max(0, Math.floor(elapsedMs / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  return [hours, minutes, seconds]
    .map((value) => String(value).padStart(2, "0"))
    .join(":");
};

const formatRunCounter = (state: TuiViewState): string =>
  [
    state.livePlay?.active
      ? state.livePlay.resuming
        ? "Resuming"
        : `Resume ${Math.ceil(state.livePlay.remainingMs / 1000)}s`
      : undefined,
    `Played ${formatElapsedRunTime(state.elapsedMs)}`,
    `Interactions ${state.interactionCount}`,
  ].filter(Boolean).join(" | ");

const buildSettingsLines = (state: TuiViewState): string[] => {
  const settings = state.settings;
  if (!settings) {
    return ["No settings available."];
  }
  return [
    `Agent: ${settings.agentStatus}${settings.agentPid ? ` (pid ${settings.agentPid})` : ""}`,
    `Model: ${formatSettingValue(settings.agentModel)}`,
    `Goal: ${formatSettingValue(settings.agentGoal)}`,
    `Max steps: ${formatMaxStepsSettingValue(settings.agentMaxSteps)}`,
    `Graph cycle steps: ${formatSettingValue(settings.agentGraphCycleSteps)}`,
    `Request delay: ${formatSettingValue(settings.agentRequestDelayMs)} ms`,
    `Identity: ${formatSettingValue(settings.agentIdentityName)}`,
    `Sound: ${settings.soundEnabled ? "on" : "off"}`,
    "",
    ":v cycle views",
    ":t start/pause agent",
    ":set model <name>",
    ":set goal <text>",
    ":set steps <n>",
    ":set cycle <n>",
    ":set delay <ms>",
    ":set identity <name>",
    ":i <message> interrupt running agent",
    ":a toggle audio",
    ":u toggle image/text renderer",
  ];
};

export const resolveControlLines = (columns: number, compact: boolean): string[] => {
  const maxLineLength = Math.max(20, columns - 4);
  const parts = compact ? compactControlParts : wideControlParts;
  const lines: string[] = [];
  let current = "Controls:";

  for (const part of parts) {
    const separator = current === "Controls:" ? " " : "  ";
    const next = `${current}${separator}${part}`;
    if (next.length <= maxLineLength) {
      current = next;
      continue;
    }
    lines.push(current);
    current = `  ${part}`;
  }
  lines.push(current);

  return lines;
};

const contentLineBudget = (panelHeight: number): number =>
  Math.max(1, panelHeight - 3);

const allocatePanelHeights = (count: number, totalHeight: number): number[] => {
  if (count <= 0) {
    return [];
  }
  const minimumPanelHeight = 3;
  const safeTotalHeight = Math.max(count * minimumPanelHeight, Math.floor(totalHeight));
  const baseHeight = Math.max(minimumPanelHeight, Math.floor(safeTotalHeight / count));
  const remainder = safeTotalHeight - baseHeight * count;
  return Array.from({ length: count }, (_value, index) =>
    baseHeight + (index < remainder ? 1 : 0)
  );
};

const buildKittyGamePanelLines = (display: KittyImageDisplay): PanelLine[] =>
  buildKittyPlaceholderLines(display.columns, display.rows).map((line) => ({
    text: line,
    color: display.color,
  }));

export const resolveTuiLayout = (dimensions: {
  columns?: number;
  rows?: number;
  controlsVisible?: boolean;
}) => {
  const columns = terminalWidth(dimensions.columns);
  const rows = terminalHeight(dimensions.rows);
  const compactControls = columns < 96;
  const controlLines = resolveControlLines(columns, compactControls);
  const controlsVisible = dimensions.controlsVisible ?? false;
  const narrow = columns < 72;
  const medium = columns >= 72 && columns < 112;
  const wide = columns >= 112;
  const headerHeight = 7;
  const footerHeight = controlsVisible ? controlLines.length + 2 : 0;
  const verticalGaps = controlsVisible ? 2 : 1;
  const mainHeight = Math.max(6, rows - headerHeight - footerHeight - verticalGaps);
  const diagnosticsVisible = !narrow && mainHeight >= 12;
  const gameHeight = diagnosticsVisible && medium
    ? Math.max(10, mainHeight - Math.max(4, Math.floor(mainHeight * 0.24)) - 1)
    : mainHeight;
  const diagnosticsHeight = diagnosticsVisible && medium
    ? Math.max(3, mainHeight - gameHeight - 1)
    : mainHeight;
  const widePanelSlots = 4;
  const sidePanelHeight = wide ? Math.max(3, Math.floor(mainHeight / widePanelSlots)) : diagnosticsHeight;
  const finalSidePanelHeight = wide ? Math.max(3, mainHeight - sidePanelHeight * (widePanelSlots - 1)) : diagnosticsHeight;
  const sideLines = Math.max(1, contentLineBudget(sidePanelHeight));
  const finalSideLines = Math.max(1, contentLineBudget(finalSidePanelHeight));

  return {
    columns,
    rows,
    headerHeight,
    footerHeight,
    mainHeight,
    compactControls,
    controlLines,
    narrow,
    medium,
    wide,
    diagnosticsVisible,
    gameLines: Math.max(3, contentLineBudget(gameHeight)),
    menuLines: sideLines,
    promptLines: sideLines,
    dialogueLines: finalSideLines,
    infoLines: sideLines,
    legendLines: sideLines,
    actionsLines: finalSideLines,
    gameHeight,
    diagnosticsHeight,
    sidePanelHeight,
    finalSidePanelHeight,
    gameWidth: wide ? "66%" : "100%",
    diagnosticsWidth: wide ? "34%" : "100%",
  };
};

export const createInkTuiApp = (
  runtime: InkRuntime,
  props: {
    initialState: TuiViewState;
    subscribe: (listener: (state: TuiViewState) => void) => () => void;
    terminal?: ResizableTerminal;
  },
): unknown => {
  const { React, ink } = runtime;
  const h = React.createElement;
  const { Box, Text } = ink;

  const Panel = ({
    title,
    lines,
    color = "cyan",
    highlight = false,
    width,
    height,
    grow,
  }: {
    title: string;
    lines: PanelLine[];
    color?: string;
    highlight?: boolean;
    width?: string | number;
    height?: string | number;
    grow?: number;
  }) =>
    h(
      Box,
      {
        borderStyle: "round",
        borderColor: highlight ? "yellow" : color,
        paddingX: 1,
        flexDirection: "column",
        width,
        height,
        flexGrow: grow,
        flexShrink: 1,
        minHeight: 3,
        overflow: "hidden",
      },
      h(Text, { bold: true, color: highlight ? "yellow" : color }, title),
      ...lines.map((line, index) => {
        const text = typeof line === "string" ? line : line.text;
        const lineColor = typeof line === "string" ? undefined : line.color;
        return h(Text, { key: `${title}-${index}`, ...(lineColor ? { color: lineColor } : {}) }, text || " ");
      }),
    );

  const App = () => {
    const [state, setState] = React.useState<TuiViewState>(props.initialState);
    React.useEffect(() => props.subscribe(setState), []);
    const [terminalDimensions, setTerminalDimensions] = React.useState(() => readTerminalDimensions(props.terminal));
    React.useEffect(() => {
      const updateDimensions = () => setTerminalDimensions(readTerminalDimensions(props.terminal));
      props.terminal?.on?.("resize", updateDimensions);
      return () => {
        if (props.terminal?.off) {
          props.terminal.off("resize", updateDimensions);
        } else {
          props.terminal?.removeListener?.("resize", updateDimensions);
        }
      };
    }, []);
    const [stdoutColumns, stdoutRows] = ink.useStdoutDimensions?.() ?? [undefined, undefined];
    const agentUiVisible = Boolean(state.agentStream);
    const layout = resolveTuiLayout({
      columns: stdoutColumns ?? terminalDimensions.columns,
      rows: stdoutRows ?? terminalDimensions.rows,
      controlsVisible: state.controlsVisible,
    });
    const snapshot = state.snapshot;
    const requestedActiveView = state.activeView ?? "play";
    const activeView =
      agentUiVisible || (requestedActiveView !== "agent" && requestedActiveView !== "agent-split")
        ? requestedActiveView
        : "play";
    const rawGameLines = buildGameScreenLines(snapshot);
    const note = state.commandNote
      ? [h(Text, { key: "note", color: state.commandError ? "red" : "yellow" }, state.commandNote)]
      : [];
    const diagnosticsLineLength = Math.max(
      24,
      layout.wide ? Math.floor(layout.columns * 0.34) - 6 : layout.columns - 6,
    );
    const kittyGamePanelActive =
      activeView !== "settings" &&
      state.gameboyRenderer === "kitty" &&
      Boolean(state.gameboyImage) &&
      isKittyGraphicsSupported();
    const diagnosticSpecs: DiagnosticPanelSpec[] = [
      ...(snapshot.menu.length && !isNameEntrySnapshot(snapshot)
        ? [{
            title: "MENU",
            color: "green",
            highlight: true,
            buildLines: (maxLines: number) => limitLinesWithOverflow(snapshot.menu, maxLines),
          }]
        : []),
      ...(snapshot.prompt.length
        ? [{
            title: "PROMPT",
            color: "green",
            highlight: true,
            buildLines: (maxLines: number) => limitLinesWithOverflow(snapshot.prompt, maxLines),
          }]
        : []),
      ...(snapshot.dialogue.length && (kittyGamePanelActive || !rawGameLines.includes("DIALOGUE"))
        ? [{
            title: "DIALOGUE",
            color: "green",
            highlight: true,
            buildLines: (maxLines: number) => limitLinesWithOverflow(snapshot.dialogue, maxLines),
          }]
        : []),
      ...(layout.wide && agentUiVisible && state.agentStream
        ? [
            {
              title: "CURRENT",
              color: "magenta",
              highlight: true,
              buildLines: (maxLines: number) =>
                renderAgentCurrentLines(state.agentStream, {
                  maxLines,
                  maxLineLength: diagnosticsLineLength,
                }),
            },
            {
              title: "TOKENS",
              color: "magenta",
              highlight: true,
              buildLines: (maxLines: number) =>
                renderAgentTokenLines(state.agentStream, {
                  maxLines,
                  maxLineLength: diagnosticsLineLength,
                }),
            },
            {
              title: "MCP",
              color: "cyan",
              highlight: true,
              buildLines: (maxLines: number) =>
                renderAgentMcpLines(state.agentStream, {
                  maxLines,
                  maxLineLength: diagnosticsLineLength,
                }),
            },
          ]
        : []),
    ];
    const showDiagnostics = layout.diagnosticsVisible && diagnosticSpecs.length > 0;
    const diagnosticsColumnHeight = layout.wide ? layout.mainHeight : layout.diagnosticsHeight;
    const diagnosticHeights = allocatePanelHeights(diagnosticSpecs.length, diagnosticsColumnHeight);
    const diagnosticPanels = diagnosticSpecs.map((spec, index) => {
      const height = diagnosticHeights[index] ?? 3;
      return h(Panel, {
        title: spec.title,
        lines: spec.buildLines(contentLineBudget(height)),
        height,
        color: spec.color,
        highlight: spec.highlight,
      });
    });
    const gameHeight = showDiagnostics ? layout.gameHeight : layout.mainHeight;
    const gameWidth = showDiagnostics && layout.wide ? layout.gameWidth : "100%";
    const gameLineBudget = contentLineBudget(gameHeight);
    const renderGamePanelLines = (textLines: string[], lineBudget: number): PanelLine[] =>
      kittyGamePanelActive
        ? state.kittyImageDisplay
          ? limitPanelLines(buildKittyGamePanelLines(state.kittyImageDisplay), lineBudget)
          : Array.from({ length: lineBudget }, () => " ")
        : limitGameLines(textLines, lineBudget);
    const detailLineLength = Math.max(40, Math.floor(layout.columns / 2) - 6);
    const agentOutputLineLength = Math.max(40, Math.floor(layout.columns * 0.44) - 6);
    const agentMcpLineLength = Math.max(24, Math.floor(layout.columns * 0.24) - 6);
    const detailsMainHeight = Math.max(6, layout.mainHeight);
    const agentSummaryLines = renderAgentStreamLines(state.agentStream, {
      maxLines: Math.max(4, contentLineBudget(detailsMainHeight)),
      maxLineLength: detailLineLength,
    });
    const agentDetailsPanel = h(Panel, {
      title: "AGENT DETAILS",
      lines: agentSummaryLines,
      width: activeView === "agent-split" ? "50%" : "100%",
      height: detailsMainHeight,
      color: "magenta",
      highlight: true,
    });
    const agentDetailsFull = h(
      Box,
      {
        flexDirection: "row",
        marginTop: 1,
        width: "100%",
        height: layout.mainHeight,
        overflow: "hidden",
      },
      h(Panel, {
        title: "GAME BOY",
        lines: renderGamePanelLines(rawGameLines, contentLineBudget(detailsMainHeight)),
        width: "32%",
        height: detailsMainHeight,
        color: "cyan",
        highlight: isNameEntrySnapshot(snapshot),
      }),
      h(Panel, {
        title: "AGENT OUTPUT",
        lines: renderAgentEventLines(state.agentStream, {
          maxLines: contentLineBudget(detailsMainHeight),
          maxLineLength: agentOutputLineLength,
          types: ["thinking-delta", "text-delta", "status", "tool-call"],
          labelMode: "type",
        }),
        width: "44%",
        height: detailsMainHeight,
        color: "magenta",
        highlight: true,
      }),
      h(Panel, {
        title: "MCP CALLS",
        lines: renderAgentEventLines(state.agentStream, {
          maxLines: contentLineBudget(detailsMainHeight),
          maxLineLength: agentMcpLineLength,
          types: ["mcp-call", "mcp-result"],
          labelMode: "type",
        }),
        width: "24%",
        height: detailsMainHeight,
        color: "cyan",
        highlight: true,
      }),
    );
    const settingsPanel = h(
      Box,
      {
        flexDirection: "row",
        marginTop: 1,
        width: "100%",
        height: layout.mainHeight,
        overflow: "hidden",
      },
      h(Panel, {
        title: "AGENT SETTINGS",
        lines: limitLines(buildSettingsLines(state), contentLineBudget(layout.mainHeight)),
        width: "100%",
        height: layout.mainHeight,
        color: "yellow",
        highlight: true,
      }),
    );

    const diagnosticsColumn = h(
      Box,
      {
        flexDirection: "column",
        marginLeft: layout.wide ? 1 : 0,
        marginTop: layout.wide ? 0 : 1,
        height: layout.wide ? layout.mainHeight : layout.diagnosticsHeight,
        overflow: "hidden",
        width: layout.diagnosticsWidth,
      },
      ...diagnosticPanels,
    );
    const gamePanel = h(Panel, {
      title: "GAME BOY",
      lines: renderGamePanelLines(rawGameLines, gameLineBudget),
      width: activeView === "agent-split" ? "50%" : gameWidth,
      height: gameHeight,
      color: "cyan",
      highlight: isNameEntrySnapshot(snapshot),
    });
    const playMainContent = h(
      Box,
      {
        flexDirection: layout.wide ? "row" : "column",
        marginTop: 1,
        width: "100%",
        height: layout.mainHeight,
        overflow: "hidden",
      },
      gamePanel,
      ...(showDiagnostics ? [diagnosticsColumn] : []),
    );
    const splitMainContent = h(
      Box,
      {
        flexDirection: "row",
        marginTop: 1,
        width: "100%",
        height: layout.mainHeight,
        overflow: "hidden",
      },
      h(Panel, {
        title: "GAME BOY",
        lines: renderGamePanelLines(rawGameLines, contentLineBudget(layout.mainHeight)),
        width: "50%",
        height: layout.mainHeight,
        color: "cyan",
        highlight: isNameEntrySnapshot(snapshot),
      }),
      agentDetailsPanel,
    );
    const mainContent =
      activeView === "agent"
        ? agentDetailsFull
        : activeView === "settings"
        ? settingsPanel
        : activeView === "agent-split"
        ? splitMainContent
        : playMainContent;

    return h(
      Box,
      { flexDirection: "column", width: layout.columns, height: layout.rows, overflow: "hidden" },
      h(
        Box,
        { borderStyle: "round", borderColor: "cyan", paddingX: 1, flexDirection: "column", height: layout.headerHeight, overflow: "hidden" },
        h(
          Box,
          { flexDirection: "row", justifyContent: "space-between", width: "100%" },
          h(Text, { bold: true, color: "cyanBright", flexShrink: 1 }, formatTitle(state)),
          h(Text, { color: "cyanBright", flexShrink: 0 }, formatRunCounter(state)),
        ),
        h(Text, {}, snapshot.statusLine),
        h(Text, {}, `Session: ${state.sessionId}`),
        ...(agentUiVisible ? [h(Text, { key: "mcp" }, `MCP: ${state.endpoint}`)] : []),
        ...note,
      ),
      mainContent,
      ...(state.controlsVisible
        ? [
            h(
              Box,
              {
                marginTop: 1,
                borderStyle: "round",
                borderColor: "yellow",
                paddingX: 1,
                height: layout.footerHeight,
                overflow: "hidden",
                flexDirection: "column",
              },
              ...layout.controlLines.map((line, index) => h(Text, { key: `controls-${index}`, color: "yellow" }, line)),
            ),
          ]
        : []),
    );
  };

  return h(App, {});
};

const hasPlayDiagnostics = (state: TuiViewState): boolean => {
  const gameLines = buildGameScreenLines(state.snapshot);
  return Boolean(
    state.snapshot.menu.length ||
    state.snapshot.prompt.length ||
    (state.snapshot.dialogue.length && !gameLines.includes("DIALOGUE")) ||
    state.agentStream,
  );
};

const GAME_BOY_CELL_ASPECT_COLUMNS = 20;
const GAME_BOY_CELL_ASPECT_ROWS = 9;

export const resolveGameBoyImageCellSize = (
  maxColumns: number,
  maxRows: number,
): { columns: number; rows: number } => {
  const safeMaxColumns = Math.max(1, Math.floor(maxColumns));
  const safeMaxRows = Math.max(1, Math.floor(maxRows));
  const columnsForRows = Math.max(
    1,
    Math.floor((safeMaxRows * GAME_BOY_CELL_ASPECT_COLUMNS) / GAME_BOY_CELL_ASPECT_ROWS),
  );
  if (columnsForRows <= safeMaxColumns) {
    return { columns: columnsForRows, rows: safeMaxRows };
  }
  const rowsForColumns = Math.max(
    1,
    Math.floor((safeMaxColumns * GAME_BOY_CELL_ASPECT_ROWS) / GAME_BOY_CELL_ASPECT_COLUMNS),
  );
  return { columns: safeMaxColumns, rows: rowsForColumns };
};

const resolveKittyImagePlacement = (
  state: TuiViewState,
  terminal?: TerminalDimensions,
): KittyImagePlacement | null => {
  const agentUiVisible = Boolean(state.agentStream);
  const requestedActiveView = state.activeView ?? "play";
  const activeView =
    agentUiVisible || (requestedActiveView !== "agent" && requestedActiveView !== "agent-split")
      ? requestedActiveView
      : "play";
  if (activeView === "settings") {
    return null;
  }
  if (state.gameboyRenderer !== "kitty" || !state.gameboyImage) {
    return null;
  }
  const layout = resolveTuiLayout({
    ...(terminal ?? {}),
    controlsVisible: state.controlsVisible,
  });
  const showDiagnostics = activeView === "play" && layout.diagnosticsVisible && hasPlayDiagnostics(state);
  const gameHeight = activeView === "play" && showDiagnostics ? layout.gameHeight : layout.mainHeight;
  const contentRows = Math.max(1, contentLineBudget(gameHeight));
  const panelColumns =
    activeView === "agent"
      ? Math.max(20, Math.floor(layout.columns * 0.32) - 4)
      : activeView === "agent-split"
      ? Math.max(20, Math.floor(layout.columns * 0.5) - 4)
      : showDiagnostics && layout.wide
      ? Math.max(20, Math.floor(layout.columns * 0.66) - 4)
      : Math.max(20, layout.columns - 4);
  const gameBoySize = resolveGameBoyImageCellSize(panelColumns, contentRows);
  return {
    row: layout.headerHeight + 4,
    column: 3,
    columns: gameBoySize.columns,
    rows: gameBoySize.rows,
  };
};

export type InkTuiRenderer = {
  update: (state: TuiViewState) => void;
  unmount: () => void;
};

const ENTER_ALTERNATE_SCREEN = "\u001b[?1049h\u001b[?25l\u001b[2J\u001b[H";
const EXIT_ALTERNATE_SCREEN = "\u001b[?25h\u001b[?1049l";

const enterFullscreenTerminal = (stdout: NodeJS.WriteStream): (() => void) => {
  if (!stdout.isTTY) {
    return () => undefined;
  }
  stdout.write(ENTER_ALTERNATE_SCREEN);
  let restored = false;
  return () => {
    if (restored) {
      return;
    }
    restored = true;
    stdout.write(EXIT_ALTERNATE_SCREEN);
  };
};

export const renderInkTui = (
  runtime: InkRuntime,
  initialState: TuiViewState,
  io: { stdin: NodeJS.ReadStream; stdout: NodeJS.WriteStream },
): InkTuiRenderer => {
  const listeners = new Set<(state: TuiViewState) => void>();
  const leaveFullscreen = enterFullscreenTerminal(io.stdout);
  const kittyRenderer = createKittyImageRenderer(io.stdout, {
    imageIds: resolveKittyImageIds(initialState.sessionId),
  });
  const prepareKittyState = (state: TuiViewState): TuiViewState => {
    if (!kittyRenderer.usesPlaceholders) {
      return { ...state, kittyImageDisplay: undefined };
    }
    return {
      ...state,
      kittyImageDisplay: kittyRenderer.update(
        state.gameboyRenderer === "kitty" ? state.gameboyImage : null,
        resolveKittyImagePlacement(state, io.stdout),
      ),
    };
  };
  const syncFallbackKitty = (state: TuiViewState): void => {
    if (kittyRenderer.usesPlaceholders) {
      kittyRenderer.commit();
      return;
    }
    kittyRenderer.update(
      state.gameboyRenderer === "kitty" ? state.gameboyImage : null,
      resolveKittyImagePlacement(state, io.stdout),
    );
    kittyRenderer.commit();
  };
  const preparedInitialState = prepareKittyState(initialState);
  const app = runtime.ink.render(
    createInkTuiApp(runtime, {
      initialState: preparedInitialState,
      terminal: io.stdout,
      subscribe: (listener) => {
        listeners.add(listener);
        return () => listeners.delete(listener);
      },
    }),
    {
      stdin: io.stdin,
      stdout: io.stdout,
      exitOnCtrlC: false,
      patchConsole: false,
    },
  );
  setImmediate(() => syncFallbackKitty(preparedInitialState));
  return {
    update: (state) => {
      const preparedState = prepareKittyState(state);
      for (const listener of listeners) {
        listener(preparedState);
      }
      setImmediate(() => syncFallbackKitty(preparedState));
    },
    unmount: () => {
      kittyRenderer.clear();
      app.unmount();
      leaveFullscreen();
    },
  };
};
