import { Mastra } from "@mastra/core";
import type { MastraCompositeStore } from "@mastra/core/storage";
import { createStep, createWorkflow } from "@mastra/core/workflows";

import { runTaskmasterBatch } from "./agents.js";
import { createDefaultMastraStorage, createTaskmasterMemory } from "./memory.js";
import { bootstrapSession, createDirectMcpSessionAuth, KrabbyClawSession } from "./session.js";
import {
  checkpointSchema,
  resumeRunSchema,
  runnerInputSchema,
  runnerOutputSchema,
  runnerStateSchema,
  sessionAuthSchema,
  suspendPayloadSchema,
  type ResumeRunInput,
  type RunnerInput,
  type RunnerOutput,
} from "./types.js";

export function createPokemonCrystalWorkflow() {
  const playBatchStep = createStep({
    id: "play-batch",
    description:
      "Bootstraps the KrabbyClaw session if needed, runs one bounded taskmaster batch, and suspends when more play remains.",
    inputSchema: runnerInputSchema,
    outputSchema: runnerOutputSchema,
    stateSchema: runnerStateSchema,
    suspendSchema: suspendPayloadSchema,
    resumeSchema: resumeRunSchema,
    execute: async ({ inputData, state, setState, resumeData, suspend, mastra }) => {
      const parsedInput = runnerInputSchema.parse(inputData);
      const effectiveInput: RunnerInput = {
        ...parsedInput,
        immediateGoal: resumeData?.immediateGoal ?? parsedInput.immediateGoal,
      };
      const resumeBaseUrl = resumeData?.baseUrl?.trim();
      const priorSession = state?.session;
      const directMcpUrl = priorSession?.mcpUrl ?? effectiveInput.session.mcpUrl;
      const auth =
        directMcpUrl
          ? createDirectMcpSessionAuth({
              baseUrl: resumeBaseUrl ?? effectiveInput.session.baseUrl,
              mcpUrl: directMcpUrl,
              sessionId: priorSession?.sessionId ?? effectiveInput.session.sessionId,
              agentId: priorSession?.agentId ?? effectiveInput.session.agentId,
              identityName: priorSession?.identityName ?? effectiveInput.session.identityName,
            })
          : priorSession && (!resumeBaseUrl || resumeBaseUrl === priorSession.baseUrl)
          ? priorSession
          : sessionAuthSchema.parse(
              await bootstrapSession({
                baseUrl: resumeBaseUrl ?? effectiveInput.session.baseUrl,
                sessionId: priorSession?.sessionId ?? effectiveInput.session.sessionId,
                agentId: priorSession?.agentId ?? effectiveInput.session.agentId,
                identityName: priorSession?.identityName ?? effectiveInput.session.identityName,
              }),
            );
      const liveSession = new KrabbyClawSession(auth);
      const taskmasterMemory = createTaskmasterMemory({
        storage: mastra?.getStorage() ?? createDefaultMastraStorage(),
      });

      try {
        const beforeStatus = await liveSession.status();
        const { taskmaster, afterStatus } = await runTaskmasterBatch({
          input: effectiveInput,
          beforeStatus,
          session: liveSession,
          sessionAuth: auth,
          memory: taskmasterMemory,
        });
        const observation = effectiveInput.includeObservationCheckpoint
          ? await liveSession.observe()
          : undefined;

        const checkpoint = checkpointSchema.parse({
          overallGoal: effectiveInput.overallGoal,
          immediateGoal: effectiveInput.immediateGoal,
          taskmaster,
          beforeStatus,
          afterStatus,
          observation,
        });

        setState({
          session: auth,
          batchesCompleted: (state?.batchesCompleted ?? 0) + 1,
          lastCheckpoint: {
            summary: checkpoint.taskmaster.summary,
            nextImmediateGoal: checkpoint.taskmaster.nextImmediateGoal,
            immediateGoalStatus: checkpoint.taskmaster.immediateGoalStatus,
            shouldContinue: checkpoint.taskmaster.shouldContinue,
          },
        });

        if (effectiveInput.autoSuspend && checkpoint.taskmaster.shouldContinue) {
          return suspend({
            reason: "Resume this workflow to continue the next supervised gameplay batch.",
            session: auth,
            checkpoint,
          });
        }

        return runnerOutputSchema.parse({
          session: auth,
          checkpoint,
        });
      } finally {
        await liveSession.disconnect();
      }
    },
  });

  return createWorkflow({
    id: "pokemon-crystal-taskmaster-workflow",
    description:
      "A suspendable, long-running Mastra workflow that supervises Pokemon Crystal progress through a taskmaster/player agent pair.",
    inputSchema: runnerInputSchema,
    outputSchema: runnerOutputSchema,
    stateSchema: runnerStateSchema,
  })
    .then(playBatchStep)
    .commit();
}

export function createPokemonCrystalMastra(options: {
  storage?: MastraCompositeStore;
}) {
  const pokemonCrystalWorkflow = createPokemonCrystalWorkflow();
  const storage = options.storage ?? createDefaultMastraStorage();

  return new Mastra({
    storage,
    workflows: {
      pokemonCrystalWorkflow,
    },
  });
}

export async function startPokemonCrystalRun(options: {
  mastra?: ReturnType<typeof createPokemonCrystalMastra>;
  input: RunnerInput;
  runId?: string;
  resourceId?: string;
}): Promise<{ runId: string }> {
  const mastra = options.mastra ?? createPokemonCrystalMastra({});
  const workflow = mastra.getWorkflow("pokemonCrystalWorkflow");
  const run = await workflow.createRun({
    runId: options.runId,
    resourceId: options.resourceId ?? options.input.session.sessionId,
  });

  return run.startAsync({
    inputData: options.input,
  });
}

export async function resumePokemonCrystalRun(options: {
  mastra: ReturnType<typeof createPokemonCrystalMastra>;
  runId: string;
  resumeData?: ResumeRunInput;
}) {
  const workflow = options.mastra.getWorkflow("pokemonCrystalWorkflow");
  const run = await workflow.createRun({
    runId: options.runId,
  });

  return run.resume({
    resumeData: options.resumeData,
  });
}
