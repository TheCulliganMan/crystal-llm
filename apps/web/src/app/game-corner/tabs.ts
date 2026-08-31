export const GAME_CORNER_TABS = [
  { id: "slot-machine", label: "Game Corner" },
  { id: "arena-mcp-skill", label: "Arena MCP/Skill" },
  { id: "progress-tracker", label: "Progress Tracker" },
] as const;

export type GameCornerTab = (typeof GAME_CORNER_TABS)[number]["id"];

export const DEFAULT_GAME_CORNER_TAB: GameCornerTab = "slot-machine";

export const isGameCornerTab = (value: string | null | undefined): value is GameCornerTab =>
  GAME_CORNER_TABS.some((tab) => tab.id === value);
