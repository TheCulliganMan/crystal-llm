"use server";

import { revalidatePath } from "next/cache";
import { createSupabaseServerClient, createSupabaseServiceRoleClient } from "@/lib/supabase/server";
import type { TablesInsert, TablesUpdate } from "@/lib/supabase/types";
import { extractSessionIdFromUrl, slugifyAgentName } from "./utils";

const asString = (value: FormDataEntryValue | null): string => (value ?? "").toString();

const parseAgentPayload = (
  payload:
    | {
        name: string;
        description?: string;
        repoUrl?: string;
        modelUrl?: string;
        mcpEndpoint?: string;
        runtime?: string;
        visibility?: "public";
      }
    | FormData
) => {
  if (payload instanceof FormData) {
    return {
      name: asString(payload.get("name")),
      description: asString(payload.get("description")),
      repoUrl: asString(payload.get("repoUrl")),
      modelUrl: asString(payload.get("modelUrl")),
      mcpEndpoint: asString(payload.get("mcpEndpoint")),
      runtime: (asString(payload.get("runtime")) as "mcp-http" | "mcp-stdio" | "langgraph-worker") || "mcp-http",
      visibility: "public",
    };
  }
  return payload;
};

const parseProfilePayload = (
  payload:
    | {
        handle: string;
        displayName?: string;
        bio?: string;
        avatarUrl?: string;
      }
    | FormData
) => {
  if (payload instanceof FormData) {
    return {
      handle: asString(payload.get("handle")),
      displayName: asString(payload.get("displayName")),
      bio: asString(payload.get("bio")),
      avatarUrl: asString(payload.get("avatarUrl")),
    };
  }
  return payload;
};

const parseRunPayload = (
  payload:
    | {
        agentId: string;
        queue?: string;
        seed?: string;
        mcpSessionUrl?: string;
        spectatorFrameUrl?: string;
      }
    | FormData
) => {
  if (payload instanceof FormData) {
    return {
      agentId: asString(payload.get("agentId")),
      queue: asString(payload.get("queue")) || "main",
      seed: asString(payload.get("seed")) || undefined,
      mcpSessionUrl: asString(payload.get("mcpSessionUrl")) || undefined,
      spectatorFrameUrl: asString(payload.get("spectatorFrameUrl")) || undefined,
    };
  }
  return payload;
};

const ensureUser = async () => {
  const supabase = createSupabaseServerClient();
  if (!supabase) {
    throw new Error("Supabase is not configured.");
  }
  const {
    data: { user },
    error,
  } = await supabase.auth.getUser();
  if (error || !user) {
    throw new Error("You must be signed in to manage arena data.");
  }
  return { supabase, userId: user.id };
};

export const upsertProfileAction = async (
  payload:
    | {
        handle: string;
        displayName?: string;
        bio?: string;
        avatarUrl?: string;
      }
    | FormData
) => {
  const { supabase, userId } = await ensureUser();
  const normalized = parseProfilePayload(payload);
  const updates: TablesInsert<"arena_profiles"> = {
    id: userId,
    handle: normalized.handle.trim(),
    display_name: normalized.displayName ?? null,
    bio: normalized.bio ?? null,
    avatar_url: normalized.avatarUrl ?? null,
    updated_at: new Date().toISOString(),
  };
  const { error } = await supabase.from("arena_profiles").upsert(updates);
  if (error) {
    throw new Error(`Failed to update profile: ${error.message}`);
  }
  revalidatePath("/arena");
};

export const createAgentAction = async (
  payload:
    | {
        name: string;
        description?: string;
        repoUrl?: string;
        modelUrl?: string;
        mcpEndpoint?: string;
        runtime?: string;
        visibility?: "public";
      }
    | FormData
) => {
  const { supabase, userId } = await ensureUser();
  const normalized = parseAgentPayload(payload);
  const slug = slugifyAgentName(normalized.name);
  const insert: TablesInsert<"arena_agents"> = {
    owner_id: userId,
    name: normalized.name.trim(),
    slug,
    description: normalized.description?.trim() || null,
    repo_url: normalized.repoUrl?.trim() || null,
    mcp_endpoint: normalized.mcpEndpoint?.trim() || null,
    runtime: normalized.runtime ?? "mcp-http",
    visibility: "public",
    config: {
      huggingfaceModel: normalized.modelUrl?.trim() || null,
    },
  };
  const { error } = await supabase.from("arena_agents").insert(insert);
  if (error) {
    throw new Error(`Failed to register agent: ${error.message}`);
  }
  revalidatePath("/arena");
};

export const queueRunAction = async (
  payload:
    | {
        agentId: string;
        queue?: string;
        seed?: string;
        mcpSessionUrl?: string;
        spectatorFrameUrl?: string;
      }
    | FormData
) => {
  const { supabase, userId } = await ensureUser();
  const normalized = parseRunPayload(payload);
  const sessionId = extractSessionIdFromUrl(normalized.mcpSessionUrl);
  const insert: TablesInsert<"arena_runs"> = {
    agent_id: normalized.agentId,
    created_by: userId,
    queue: normalized.queue ?? "main",
    seed: normalized.seed ?? null,
    mcp_session_url:
      normalized.mcpSessionUrl ??
      process.env.NEXT_PUBLIC_MCP_ENTRYPOINT ??
      process.env.POKECRYSTAL_MCP_URL ??
      "/api/mcp",
    spectator_frame_url: normalized.spectatorFrameUrl ?? null,
    metrics: sessionId ? { session_id: sessionId } : undefined,
  };
  const { error } = await supabase.from("arena_runs").insert(insert);
  if (error) {
    throw new Error(`Failed to enqueue run: ${error.message}`);
  }
  revalidatePath("/arena");
};

export const updateRunStatusAction = async (
  payload: TablesUpdate<"arena_runs"> & { id: string }
) => {
  const supabase = createSupabaseServiceRoleClient();
  if (!supabase) {
    throw new Error("Supabase service role is not configured.");
  }
  const { id, ...rest } = payload;
  const updates: TablesUpdate<"arena_runs"> = {
    ...rest,
    updated_at: new Date().toISOString(),
  };
  const { error } = await supabase.from("arena_runs").update(updates).eq("id", id);
  if (error) {
    throw new Error(`Failed to update run: ${error.message}`);
  }
  revalidatePath("/arena");
};
