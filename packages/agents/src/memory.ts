import fs from "node:fs";
import path from "node:path";

import type { MastraCompositeStore } from "@mastra/core/storage";
import { LibSQLStore } from "@mastra/libsql";
import { Memory } from "@mastra/memory";
import { z } from "zod";

import {
  DEFAULT_AGENT_MODEL,
  resolveMastraModel,
  resolveMastraProviderOptions,
} from "./defaults.js";
import type { SessionAuth } from "./types.js";

const taskmasterWorkingMemorySchema = z.object({
  runSummary: z
    .object({
      overallGoal: z.string().optional(),
      currentImmediateGoal: z.string().optional(),
      lastCheckpointSummary: z.string().optional(),
      latestMap: z.string().optional(),
      latestCoords: z.string().optional(),
      latestFlowGoal: z.string().optional(),
    })
    .optional(),
  discoveries: z.array(z.string()).optional(),
  blockers: z.array(z.string()).optional(),
  routeNotes: z.array(z.string()).optional(),
  npcGoals: z.array(z.string()).optional(),
  interactableNotes: z.array(z.string()).optional(),
  partyNotes: z.array(z.string()).optional(),
  openQuestions: z.array(z.string()).optional(),
});

export function getDefaultMemoryDbPath(): string {
  return path.resolve(process.cwd(), "packages/agents/.mastra/pokemon-crystal-memory.db");
}

export function createDefaultMastraStorage(): MastraCompositeStore {
  const dbPath = getDefaultMemoryDbPath();
  fs.mkdirSync(path.dirname(dbPath), { recursive: true });

  return new LibSQLStore({
    id: "pokemon-crystal-agents-storage",
    url: `file:${dbPath}`,
  });
}

export function createTaskmasterMemory(params: {
  storage: MastraCompositeStore;
  observationModel?: string;
}): Memory {
  const observationModel = params.observationModel ?? DEFAULT_AGENT_MODEL;
  const providerOptions = resolveMastraProviderOptions(observationModel);

  return new Memory({
    storage: params.storage,
    options: {
      lastMessages: 24,
      workingMemory: {
        enabled: true,
        scope: "resource",
        schema: taskmasterWorkingMemorySchema,
      },
      observationalMemory: {
        model: resolveMastraModel(observationModel),
        ...(providerOptions ? { providerOptions } : {}),
        scope: "thread",
      },
    },
  });
}

export function createTaskmasterMemoryScope(session: SessionAuth): {
  resource: string;
  thread: string;
}
export function createTaskmasterMemoryScope(
  session: SessionAuth,
  options: { batchId?: string },
): {
  resource: string;
  thread: string;
}
export function createTaskmasterMemoryScope(
  session: SessionAuth,
  options?: { batchId?: string },
): {
  resource: string;
  thread: string;
} {
  const batchSuffix = options?.batchId?.trim() ? `-${options.batchId.trim()}` : "";
  return {
    resource: `playthrough:${session.agentId}`,
    thread: `taskmaster:${session.sessionId}${batchSuffix}`,
  };
}
