import { z } from "zod";

import { DEFAULT_AGENT_MODEL } from "./defaults.js";

export const buttonSchema = z.enum(["A", "B", "Start", "Select"]);
export const directionSchema = z.enum(["up", "down", "left", "right"]);
const coordsSchema = z.tuple([z.number(), z.number()]);

const statusTargetSchema = z.object({
  kind: z.string().optional(),
  coords: coordsSchema.optional(),
  label: z.string().optional(),
  token: z.string().optional(),
  hotspotType: z.string().optional(),
  script: z.string().optional(),
});

const recommendedApproachSchema = z.object({
  coords: coordsSchema,
  facing: z.string(),
  setupFrom: coordsSchema,
});

const localMovementEntrySchema = z.object({
  direction: directionSchema,
  tile: z.string(),
});

const surfaceStatusSchema = z.object({
  kind: z.string(),
  title: z.string(),
  state: z.string().optional(),
  phase: z.string().optional(),
  waiting: z.boolean().optional(),
  menuOpen: z.boolean().optional(),
  promptOpen: z.boolean().optional(),
  dialogueOpen: z.boolean().optional(),
  selected: z.string().optional(),
  controls: z.array(z.string()).optional(),
  primaryText: z.string().optional(),
});

export const sessionConfigSchema = z.object({
  baseUrl: z.url().default("http://127.0.0.1:3000"),
  mcpUrl: z.url().optional(),
  sessionId: z.string().min(1),
  agentId: z.string().min(1).optional(),
  identityName: z.string().min(1).optional(),
});

export const sessionAuthSchema = z.object({
  baseUrl: z.url(),
  mcpUrl: z.url().optional(),
  sessionId: z.string().min(1),
  agentId: z.string().min(1),
  identityName: z.string().min(1),
  token: z.string().min(1),
  sessionSecret: z.string().min(1),
});

export const statusSchema = z.object({
  mode: z.string(),
  surface: surfaceStatusSchema.optional(),
  map: z.string(),
  location: z.string().optional(),
  mapId: z.string(),
  coords: coordsSchema.optional(),
  interactionTile: coordsSchema.optional(),
  interactionTarget: statusTargetSchema.optional(),
  currentHotspot: z
    .object({
      coords: coordsSchema,
      label: z.string(),
      token: z.string(),
      hotspotType: z.string(),
    })
    .optional(),
  interactionSetup: z
    .object({
      hotspot: z.object({
        coords: coordsSchema,
        label: z.string(),
        token: z.string(),
        hotspotType: z.string(),
      }),
      recommendedApproach: recommendedApproachSchema.optional(),
    })
    .optional(),
  interactionLane: z
    .object({
      hotspot: z
        .object({
          coords: coordsSchema,
          label: z.string(),
          token: z.string(),
          hotspotType: z.string(),
        })
        .optional(),
      lane: z
        .object({
          coords: coordsSchema,
          facing: z.string(),
          facingAligned: z.boolean(),
          facingMoveLeavesLane: z.boolean().optional(),
          targetConfirmed: z.boolean().optional(),
        })
        .optional(),
    })
    .optional(),
  localFocus: z
    .object({
      source: z.string(),
      target: statusTargetSchema,
      recommendedApproach: recommendedApproachSchema.optional(),
    })
    .optional(),
  scene: z
    .object({
      activeScript: z.string().optional(),
      owner: statusTargetSchema.optional(),
    })
    .optional(),
  facing: z.string().optional(),
  badges: z.number().int().nonnegative(),
  inMenu: z.boolean().optional(),
  inBattle: z.boolean().optional(),
  inDialog: z.boolean().optional(),
  textBoxOpen: z.boolean().optional(),
  textAdvancePending: z.boolean().optional(),
  promptPending: z.boolean().optional(),
  movementLocked: z.boolean().optional(),
  scriptBusy: z.boolean().optional(),
  canMove: z.boolean(),
  blockedReason: z.string().optional(),
  localMovement: z
    .object({
      openDirections: z.array(localMovementEntrySchema),
      blockedDirections: z.array(localMovementEntrySchema),
    })
    .optional(),
  partyCount: z.number().int().nonnegative(),
  flowSummary: z.string(),
  flowNextGoal: z.string(),
  flowCompletionTarget: z.string(),
});

export const observeSnapshotSchema = z.object({
  flow_state: z
    .object({
      sum: z.string(),
      done: z.number().int().nonnegative(),
      total: z.number().int().nonnegative(),
      next: z.string(),
      target: z.string(),
    })
    .optional(),
  view: z
    .object({
      focus: z.string(),
      pos: z.tuple([z.number(), z.number()]).optional(),
      viewport: z.array(z.string()).optional(),
      ahead: z.string().optional(),
    })
    .optional(),
})
  .passthrough();

export const observationSchema = z.object({
  summaryText: z.string(),
  snapshot: observeSnapshotSchema.optional(),
  image: z
    .object({
      data: z.string().min(1),
      mimeType: z.string().min(1),
    })
    .optional(),
  rawTexts: z.array(z.string()).min(1),
});

export const taskmasterBatchSchema = z.object({
  summary: z.string(),
  immediateGoalStatus: z.enum(["done", "in_progress", "blocked"]),
  nextImmediateGoal: z.string(),
  shouldContinue: z.boolean(),
  evidence: z.array(z.string()).min(1),
});

export const runnerStateSchema = z.object({
  session: sessionAuthSchema.optional(),
  batchesCompleted: z.number().int().nonnegative().default(0),
  lastCheckpoint: z
    .object({
      summary: z.string(),
      nextImmediateGoal: z.string(),
      immediateGoalStatus: z.enum(["done", "in_progress", "blocked"]),
      shouldContinue: z.boolean(),
    })
    .optional(),
});

export const runnerInputSchema = z.object({
  session: sessionConfigSchema,
  overallGoal: z
    .string()
    .default("Beat Pokemon Crystal by reaching and clearing Mt. Silver."),
  immediateGoal: z.string().min(1),
  taskmasterModel: z.string().min(1).default(DEFAULT_AGENT_MODEL),
  playerModel: z.string().min(1).default(DEFAULT_AGENT_MODEL),
  supervisorMaxSteps: z.number().int().positive().max(20).default(8),
  playerMaxSteps: z.number().int().positive().max(12).default(5),
  autoSuspend: z.boolean().default(true),
  includeObservationCheckpoint: z.boolean().default(true),
});

export const resumeRunSchema = z.object({
  immediateGoal: z.string().min(1).optional(),
  baseUrl: z.url().optional(),
});

export const checkpointSchema = z.object({
  overallGoal: z.string(),
  immediateGoal: z.string(),
  taskmaster: taskmasterBatchSchema,
  beforeStatus: statusSchema,
  afterStatus: statusSchema,
  observation: observationSchema.optional(),
});

export const suspendPayloadSchema = z.object({
  reason: z.string(),
  session: sessionAuthSchema,
  checkpoint: checkpointSchema,
});

export const runnerOutputSchema = z.object({
  session: sessionAuthSchema,
  checkpoint: checkpointSchema,
});

export type SessionConfig = z.infer<typeof sessionConfigSchema>;
export type SessionAuth = z.infer<typeof sessionAuthSchema>;
export type Status = z.infer<typeof statusSchema>;
export type Observation = z.infer<typeof observationSchema>;
export type TaskmasterBatch = z.infer<typeof taskmasterBatchSchema>;
export type RunnerState = z.infer<typeof runnerStateSchema>;
export type RunnerInput = z.infer<typeof runnerInputSchema>;
export type ResumeRunInput = z.infer<typeof resumeRunSchema>;
export type RunnerCheckpoint = z.infer<typeof checkpointSchema>;
export type RunnerOutput = z.infer<typeof runnerOutputSchema>;

export interface CliRunOptions {
  command: "run" | "resume" | "help";
  sessionId: string;
  model: string;
  baseUrl: string;
  mcpBaseUrl: string;
  mcpUrl?: string;
  maxSteps: number;
  graphCycleSteps: number;
  identityName: string;
  requestDelayMs: number;
  terminalUi: boolean;
  recordTraining: boolean;
  goal: string;
  ollamaBaseUrl?: string;
  openaiBaseUrl?: string;
  openaiApiKey?: string;
  anthropicBaseUrl?: string;
  anthropicApiKey?: string;
  googleBaseUrl?: string;
  googleApiKey?: string;
}
