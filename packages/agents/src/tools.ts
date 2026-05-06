import type { KrabbyClawSession } from "./session.js";

export type PlayerToolOptions = {
  compact?: boolean;
};

const compactPlayerToolNames = new Set([
  "observe",
  "map_info",
  "move",
  "press",
  "status",
]);

const normalizeToolName = (name: string): string =>
  name.startsWith("krabbyclaw_") ? name.slice("krabbyclaw_".length) : name;

export async function createPlayerTools(session: KrabbyClawSession, options: PlayerToolOptions = {}) {
  const tools = await session.listPlayerTools();
  if (!options.compact) {
    return tools;
  }

  // Local llama.cpp models are context-constrained. Never recover with runner-chosen
  // gameplay actions; shrink only the model-visible tool schema and force another
  // agent choice when the model fails.
  return Object.fromEntries(
    Object.entries(tools).filter(([name]) => compactPlayerToolNames.has(normalizeToolName(name))),
  );
}
