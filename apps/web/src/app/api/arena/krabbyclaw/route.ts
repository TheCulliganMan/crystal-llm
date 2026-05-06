import { NextResponse } from "next/server";
import { hasValidIdentityToken } from "@/app/api/[transport]/tools/identity";
import { isRequestAuthorized, SESSION_ID_REGEX, verifySessionSecret } from "@/app/mcp/session-guards";
import { applyEloRating, DEFAULT_ARENA_ELO, type EloOutcome } from "@/arena/elo";
import { slugifyAgentName } from "@/arena/utils";
import { MoveName } from "@pokecrystal/core/core/enums/move";
import {
  loadMergedEggMovesSync,
  loadMergedLearnsetsSync,
  loadMergedPokemonDataSync,
} from "@pokecrystal/core/core/content-packs";
import { createSupabaseServiceRoleClient } from "@/lib/supabase/server";
import type { Json, Tables, TablesInsert, TablesUpdate } from "@/lib/supabase/types";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const noStoreHeaders = {
  "Cache-Control": "no-store, max-age=0, must-revalidate",
};

const UUID_LIKE = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const ACTIONS = new Set(["start", "finish", "report", "queue", "leave", "team"]);
const OUTCOMES = new Set(["challenger", "opponent", "draw", "cancelled"]);

const MOVE_SEPARATOR_PATTERN = /[\s-_]+/g;

const MOVE_NAME_LOOKUP = new Map<string, MoveName>();
for (const move of Object.values(MoveName)) {
  MOVE_NAME_LOOKUP.set(move.replace(MOVE_SEPARATOR_PATTERN, ""), move);
}

const DEFAULT_QUEUE = "krabbyclaw-arena";
const DEFAULT_RUNTIME = "mcp-http";
const DEFAULT_K_FACTOR = 32;
const MIN_K_FACTOR = 4;
const MAX_K_FACTOR = 64;
const MAX_TEAM_SIZE = 6;
const TEAM_LEVEL_REGEX = /\b(?:lv|level)\s*\.?\s*(\d{1,3})\b/i;
const readString = (value: unknown): string => {
  if (typeof value !== "string") {
    return "";
  }
  return value.trim();
};

const readOptionalString = (value: unknown): string | null => {
  const trimmed = readString(value);
  return trimmed ? trimmed : null;
};

const readNonNegativeInt = (value: unknown): number | null => {
  if (value === null || value === undefined || value === "") {
    return null;
  }
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) {
    throw new Error("Numeric values must be finite.");
  }
  return Math.max(0, Math.trunc(numeric));
};

const parseSessionId = (value: unknown, label: string): string | null => {
  const sessionId = readOptionalString(value);
  if (!sessionId) {
    return null;
  }
  if (!SESSION_ID_REGEX.test(sessionId)) {
    throw new Error(`${label} must match the MCP session id format.`);
  }
  return sessionId;
};

const parseMetadata = (value: unknown): Record<string, Json> => {
  if (value === undefined || value === null) {
    return {};
  }
  if (typeof value !== "object" || Array.isArray(value)) {
    throw new Error("metadata must be a JSON object.");
  }
  return value as Record<string, Json>;
};

const metadataToRecord = (value: unknown): Record<string, Json> => {
  if (value === null || value === undefined) {
    return {};
  }
  if (typeof value !== "object" || Array.isArray(value)) {
    return {};
  }
  return value as Record<string, Json>;
};

type BattleAction = "start" | "finish" | "report" | "queue" | "leave" | "team";
type BattleOutcome = "challenger" | "opponent" | "draw" | "cancelled";

type TeamMember = {
  species: string;
  level: number | null;
  item: string | null;
  moves: string[];
  nickname: string | null;
};

type ArenaSidePayload = {
  name: string;
  sessionId: string | null;
  runtime: string;
  repoUrl: string | null;
  modelUrl: string | null;
  team: TeamMember[];
};

type ArenaRequestPayload = {
  action: BattleAction;
  matchId: string | null;
  controllerSessionId: string | null;
  queue: string;
  challenger: ArenaSidePayload | null;
  opponent: ArenaSidePayload | null;
  agent: ArenaSidePayload | null;
  outcome: BattleOutcome | null;
  challengerScore: number | null;
  opponentScore: number | null;
  notes: string | null;
  metadata: Record<string, Json>;
  kFactor: number;
};

type ArenaAgentRow = Pick<Tables<"arena_agents">, "id" | "name" | "slug" | "runtime">;
type ArenaRatingRow = Tables<"krabbyclaw_arena_ratings">;
type ArenaMatchRow = Tables<"krabbyclaw_arena_matches">;

type LearnsetEntry = [number, MoveName];
type LearnsetsBySpecies = Record<string, LearnsetEntry[]>;
type EggMovesBySpecies = Record<string, MoveName[]>;
type PokemonDataEntry = { id: string; tmhm_learnset?: MoveName[] };

type ArenaMoveData = {
  learnsets: LearnsetsBySpecies;
  eggMoves: EggMovesBySpecies;
  tmhmBySpecies: Map<string, MoveName[]>;
  speciesLookup: Map<string, string>;
};

let arenaMoveData: ArenaMoveData | null = null;

const normalizeLabel = (value: string): string => {
  const normalized = value.trim().replace(/[_-]+/g, " ").toLowerCase();
  if (!normalized) {
    return "";
  }
  return normalized.replace(/\b\w/g, (char) => char.toUpperCase());
};

const parseNicknameAndSpecies = (value: string): { nickname: string | null; remaining: string } => {
  const trimmed = value.trim();
  if (!trimmed) {
    return { nickname: null, remaining: "" };
  }
  const parenStart = trimmed.indexOf("(");
  const parenEnd = trimmed.indexOf(")", parenStart + 1);
  if (parenStart > 0 && parenEnd > parenStart) {
    const nickname = trimmed.slice(0, parenStart).trim();
    const species = trimmed.slice(parenStart + 1, parenEnd).trim();
    const rest = trimmed.slice(parenEnd + 1).trim();
    return {
      nickname: nickname || null,
      remaining: `${species}${rest ? ` ${rest}` : ""}`.trim(),
    };
  }
  if (trimmed.includes(":")) {
    const [left, right] = trimmed.split(":", 2);
    if (left.trim() && right.trim()) {
      return {
        nickname: left.trim(),
        remaining: right.trim(),
      };
    }
  }
  return { nickname: null, remaining: trimmed };
};

const normalizeSpeciesKey = (value: string): string => value.toUpperCase().replace(/[^A-Z0-9]/g, "");

const loadArenaMoveData = (): ArenaMoveData => {
  if (arenaMoveData) {
    return arenaMoveData;
  }
  const learnsets = loadMergedLearnsetsSync() as LearnsetsBySpecies;
  const eggMoves = loadMergedEggMovesSync() as EggMovesBySpecies;
  const pokemon = Object.values(loadMergedPokemonDataSync()) as PokemonDataEntry[];

  const speciesLookup = new Map<string, string>();
  const tmhmBySpecies = new Map<string, MoveName[]>();
  for (const entry of pokemon) {
    if (!entry?.id) {
      continue;
    }
    const speciesId = String(entry.id).toUpperCase();
    speciesLookup.set(normalizeSpeciesKey(speciesId), speciesId);
    tmhmBySpecies.set(speciesId, (entry.tmhm_learnset ?? []) as MoveName[]);
  }

  arenaMoveData = {
    learnsets,
    eggMoves,
    tmhmBySpecies,
    speciesLookup,
  };
  return arenaMoveData;
};

const resolveSpeciesId = (speciesLabel: string): string => {
  const data = loadArenaMoveData();
  const key = normalizeSpeciesKey(speciesLabel);
  const resolved = data.speciesLookup.get(key);
  if (!resolved) {
    throw new Error(`Unknown species '${speciesLabel}'.`);
  }
  return resolved;
};

const allowedMovesForSpeciesLevel = (speciesId: string, level: number): Set<MoveName> => {
  const data = loadArenaMoveData();
  const allowed = new Set<MoveName>();
  const learnset = data.learnsets[speciesId] ?? [];
  for (const [learnLevel, move] of learnset) {
    if (learnLevel <= level) {
      allowed.add(move);
    }
  }
  for (const move of data.tmhmBySpecies.get(speciesId) ?? []) {
    allowed.add(move);
  }
  for (const move of data.eggMoves[speciesId] ?? []) {
    allowed.add(move);
  }
  return allowed;
};

const validateTeamMoves = (team: TeamMember[], label: string): void => {
  for (const member of team) {
    if (member.level === null) {
      throw new Error(`${label} entries must include a level.`);
    }
    const speciesId = resolveSpeciesId(member.species);
    const allowed = allowedMovesForSpeciesLevel(speciesId, member.level);
    for (const move of member.moves) {
      if (!allowed.has(move as MoveName)) {
        throw new Error(`${member.species} cannot learn ${move} at level ${member.level}.`);
      }
    }
  }
};

const resolveMoveToken = (value: string): MoveName => {
  const trimmed = value.trim();
  if (!trimmed) {
    throw new Error("Move entries cannot be empty.");
  }
  const upper = trimmed.toUpperCase();
  const stripped = upper.replace(MOVE_SEPARATOR_PATTERN, "");

  const resolved = MOVE_NAME_LOOKUP.get(stripped);
  if (resolved) {
    return resolved;
  }

  throw new Error(`Unknown move '${value}'.`);
};

const parseTeamEntryFromString = (value: string): TeamMember | null => {
  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }
  const [leftRaw, movesRaw] = trimmed.split("|", 2).map((part) => part.trim());
  let left = leftRaw;
  let item: string | null = null;
  if (left.includes("@")) {
    const [namePart, itemPart] = left.split("@", 2);
    left = namePart.trim();
    const normalizedItem = normalizeLabel(itemPart.trim());
    item = normalizedItem || null;
  }

  const nickParsed = parseNicknameAndSpecies(left);
  const match = TEAM_LEVEL_REGEX.exec(nickParsed.remaining);
  const level = match ? Math.max(1, Math.min(100, Number(match[1]))) : null;
  const speciesRaw = match ? nickParsed.remaining.slice(0, match.index).trim() : nickParsed.remaining;
  const species = normalizeLabel(speciesRaw);
  if (!species) {
    return null;
  }

  const moves = movesRaw
    ? movesRaw
        .split(/[\\/,+]+/)
        .map((move) => move.trim())
        .filter((move) => move.length > 0)
        .map((move) => resolveMoveToken(move))
        .slice(0, 4)
    : [];

  return {
    species,
    level: Number.isFinite(level) ? level : null,
    item,
    moves,
    nickname: nickParsed.nickname,
  };
};

const parseTeamEntryFromObject = (value: Record<string, unknown>): TeamMember | null => {
  const speciesRaw = readString(value.species);
  if (!speciesRaw) {
    return null;
  }
  const species = normalizeLabel(speciesRaw);
  if (!species) {
    return null;
  }
  const levelRaw = value.level;
  const numeric = typeof levelRaw === "number" ? levelRaw : Number(levelRaw);
  const level = Number.isFinite(numeric) ? Math.max(1, Math.min(100, Math.trunc(numeric))) : null;
  const item = readOptionalString(value.item);
  const nickname = readOptionalString(value.nickname ?? value.nick);
  const moves: string[] = Array.isArray(value.moves)
    ? value.moves
        .map((move) => readString(move))
        .filter((move) => move.length > 0)
        .map((move) => resolveMoveToken(move))
        .slice(0, 4)
    : [];

  return {
    species,
    level,
    item,
    moves,
    nickname,
  };
};

const parseTeam = (value: unknown, label: string): TeamMember[] => {
  if (value === undefined || value === null || value === "") {
    return [];
  }
  const members: TeamMember[] = [];
  const pushMember = (member: TeamMember | null) => {
    if (!member) {
      return;
    }
    if (members.length < MAX_TEAM_SIZE) {
      members.push(member);
    }
  };

  if (typeof value === "string") {
    const parts = value.split(/[\n,]+/);
    for (const part of parts) {
      const member = parseTeamEntryFromString(part);
      if (member && member.level === null) {
        throw new Error(`${label} entries must include a level.`);
      }
      pushMember(member);
    }
    return members;
  }

  if (Array.isArray(value)) {
    for (const entry of value) {
      if (typeof entry === "string") {
        const member = parseTeamEntryFromString(entry);
        if (member && member.level === null) {
          throw new Error(`${label} entries must include a level.`);
        }
        pushMember(member);
        continue;
      }
      if (entry && typeof entry === "object" && !Array.isArray(entry)) {
        const member = parseTeamEntryFromObject(entry as Record<string, unknown>);
        if (member && member.level === null) {
          throw new Error(`${label} entries must include a level.`);
        }
        pushMember(member);
      }
    }
    return members;
  }

  throw new Error(`${label} must be an array or comma-separated string.`);
};

const parseSide = (value: unknown, label: string): ArenaSidePayload => {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${label} must be a JSON object.`);
  }
  const body = value as Record<string, unknown>;
  const name = readOptionalString(body.name);
  if (!name) {
    throw new Error(`${label}.name is required.`);
  }

  const team = parseTeam(body.team ?? body.party ?? body.pokemon, `${label}.team`);
  if (!team.length) {
    throw new Error(`${label}.team is required.`);
  }
  validateTeamMoves(team, label);

  return {
    name,
    sessionId: parseSessionId(body.sessionId ?? body.session_id, `${label}.sessionId`),
    runtime: readString(body.runtime) || DEFAULT_RUNTIME,
    repoUrl: readOptionalString(body.repoUrl ?? body.repo_url),
    modelUrl: readOptionalString(body.modelUrl ?? body.model_url),
    team,
  };
};

const buildLegacySide = (body: Record<string, unknown>, prefix: "challenger" | "opponent"): Record<string, unknown> | null => {
  const name = body[`${prefix}Name`] ?? body[`${prefix}_name`];
  if (!name) {
    return null;
  }
  return {
    name,
    sessionId: body[`${prefix}SessionId`] ?? body[`${prefix}_session_id`],
    runtime: body[`${prefix}Runtime`] ?? body[`${prefix}_runtime`],
    repoUrl: body[`${prefix}RepoUrl`] ?? body[`${prefix}_repo_url`],
    modelUrl: body[`${prefix}ModelUrl`] ?? body[`${prefix}_model_url`],
    team: body[`${prefix}Team`] ?? body[`${prefix}_team`],
  };
};

const parseBody = (raw: unknown): ArenaRequestPayload => {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
    throw new Error("Request body must be a JSON object.");
  }
  const body = raw as Record<string, unknown>;
  const actionRaw = readString(body.action).toLowerCase() || "report";
  if (!ACTIONS.has(actionRaw)) {
    throw new Error("action must be start, finish, report, queue, leave, or team.");
  }

  const queue = readString(body.queue) || DEFAULT_QUEUE;
  const outcomeRaw = readString(body.outcome).toLowerCase();
  const outcome = outcomeRaw
    ? (OUTCOMES.has(outcomeRaw) ? (outcomeRaw as BattleOutcome) : null)
    : null;
  if (outcomeRaw && !outcome) {
    throw new Error("outcome must be challenger, opponent, draw, or cancelled.");
  }

  const kFactorRaw = body.kFactor ?? body.k_factor;
  const kFactorNumeric = kFactorRaw === undefined ? DEFAULT_K_FACTOR : Number(kFactorRaw);
  if (!Number.isFinite(kFactorNumeric)) {
    throw new Error("kFactor must be a finite number.");
  }
  const kFactor = Math.max(MIN_K_FACTOR, Math.min(MAX_K_FACTOR, Math.trunc(kFactorNumeric)));

  const challengerRaw = body.challenger ?? buildLegacySide(body, "challenger");
  const opponentRaw = body.opponent ?? buildLegacySide(body, "opponent");
  const agentRaw = body.agent ?? body.player ?? challengerRaw ?? null;
  const challenger = challengerRaw ? parseSide(challengerRaw, "challenger") : null;
  const opponent = opponentRaw ? parseSide(opponentRaw, "opponent") : null;
  const agent = agentRaw ? parseSide(agentRaw, "agent") : null;

  const payload: ArenaRequestPayload = {
    action: actionRaw as BattleAction,
    matchId: readOptionalString(body.matchId ?? body.match_id),
    controllerSessionId: parseSessionId(body.controllerSessionId ?? body.controller_session_id, "controllerSessionId"),
    queue,
    challenger,
    opponent,
    agent,
    outcome,
    challengerScore: readNonNegativeInt(body.challengerScore ?? body.challenger_score),
    opponentScore: readNonNegativeInt(body.opponentScore ?? body.opponent_score),
    notes: readOptionalString(body.notes),
    metadata: parseMetadata(body.metadata),
    kFactor,
  };

  if ((payload.action === "start" || payload.action === "report") && (!payload.challenger || !payload.opponent)) {
    throw new Error("challenger and opponent payloads are required for start/report.");
  }
  if ((payload.action === "queue" || payload.action === "leave" || payload.action === "team") && !payload.agent) {
    throw new Error("agent payload is required for queue/leave/team.");
  }
  if (payload.action === "finish" && !payload.matchId) {
    throw new Error("matchId is required for finish.");
  }
  return payload;
};

const resolveSystemOwnerId = async (): Promise<string | null> => {
  const configured = (process.env.POKECRYSTAL_MCP_SYSTEM_USER_ID ?? "").trim();
  if (UUID_LIKE.test(configured)) {
    return configured;
  }

  const supabase = createSupabaseServiceRoleClient();
  if (!supabase) {
    return null;
  }

  const { data, error } = await supabase
    .from("arena_agents")
    .select("owner_id")
    .limit(1)
    .maybeSingle();
  if (error) {
    return null;
  }
  const ownerId = data?.owner_id ?? null;
  return ownerId && UUID_LIKE.test(ownerId) ? ownerId : null;
};

const toAgentSummary = (agent: ArenaAgentRow) => ({
  id: agent.id,
  name: agent.name,
  slug: agent.slug,
  runtime: agent.runtime,
});

const upsertAgent = async (ownerId: string, side: ArenaSidePayload): Promise<ArenaAgentRow> => {
  const supabase = createSupabaseServiceRoleClient();
  if (!supabase) {
    throw new Error("Supabase service role is not configured.");
  }

  const now = new Date().toISOString();
  const insert: TablesInsert<"arena_agents"> = {
    owner_id: ownerId,
    name: side.name,
    slug: slugifyAgentName(side.name),
    runtime: side.runtime || DEFAULT_RUNTIME,
    visibility: "public",
    repo_url: side.repoUrl,
    config: {
      huggingfaceModel: side.modelUrl,
      krabbyclaw_arena: true,
      krabbyclaw_team: side.team,
    },
    updated_at: now,
  };

  const { data, error } = await supabase
    .from("arena_agents")
    .upsert(insert, { onConflict: "owner_id,name" })
    .select("id,name,slug,runtime")
    .maybeSingle();
  if (error || !data?.id) {
    throw new Error(`Failed to upsert agent ${side.name}: ${error?.message ?? "Unknown error"}`);
  }
  return data as ArenaAgentRow;
};

const defaultRatingInsert = (agentId: string): TablesInsert<"krabbyclaw_arena_ratings"> => ({
  agent_id: agentId,
  rating: DEFAULT_ARENA_ELO,
  games_played: 0,
  wins: 0,
  losses: 0,
  draws: 0,
  updated_at: new Date().toISOString(),
});

const ensureRating = async (agentId: string): Promise<ArenaRatingRow> => {
  const supabase = createSupabaseServiceRoleClient();
  if (!supabase) {
    throw new Error("Supabase service role is not configured.");
  }

  const { data: existing, error: existingError } = await supabase
    .from("krabbyclaw_arena_ratings")
    .select("*")
    .eq("agent_id", agentId)
    .maybeSingle();

  if (!existingError && existing) {
    return existing as ArenaRatingRow;
  }

  const { data, error } = await supabase
    .from("krabbyclaw_arena_ratings")
    .upsert(defaultRatingInsert(agentId), { onConflict: "agent_id" })
    .select("*")
    .maybeSingle();
  if (error || !data) {
    throw new Error(`Failed to initialize ELO for agent ${agentId}: ${error?.message ?? "Unknown error"}`);
  }
  return data as ArenaRatingRow;
};

const writeRating = async (
  rating: ArenaRatingRow,
  updates: Partial<Pick<ArenaRatingRow, "rating" | "games_played" | "wins" | "losses" | "draws" | "last_match_at">>,
): Promise<ArenaRatingRow> => {
  const supabase = createSupabaseServiceRoleClient();
  if (!supabase) {
    throw new Error("Supabase service role is not configured.");
  }

  const now = new Date().toISOString();
  const payload: TablesUpdate<"krabbyclaw_arena_ratings"> = {
    ...updates,
    updated_at: now,
  };
  const { error } = await supabase
    .from("krabbyclaw_arena_ratings")
    .update(payload)
    .eq("agent_id", rating.agent_id);
  if (error) {
    throw new Error(`Failed to update ELO for agent ${rating.agent_id}: ${error.message}`);
  }

  return {
    ...rating,
    ...updates,
    updated_at: now,
  } as ArenaRatingRow;
};

const toEloOutcome = (outcome: Exclude<BattleOutcome, "cancelled">): EloOutcome => {
  if (outcome === "challenger") {
    return "a";
  }
  if (outcome === "opponent") {
    return "b";
  }
  return "draw";
};

const applyRatings = async (
  challengerAgentId: string,
  opponentAgentId: string,
  outcome: Exclude<BattleOutcome, "cancelled">,
  kFactor: number,
) => {
  const now = new Date().toISOString();
  const challengerRating = await ensureRating(challengerAgentId);
  const opponentRating = await ensureRating(opponentAgentId);

  const snapshot = applyEloRating(
    challengerRating.rating,
    opponentRating.rating,
    toEloOutcome(outcome),
    kFactor,
  );

  const challengerGames = challengerRating.games_played + 1;
  const opponentGames = opponentRating.games_played + 1;

  const challengerUpdates = {
    rating: snapshot.nextRatingA,
    games_played: challengerGames,
    wins: challengerRating.wins + (outcome === "challenger" ? 1 : 0),
    losses: challengerRating.losses + (outcome === "opponent" ? 1 : 0),
    draws: challengerRating.draws + (outcome === "draw" ? 1 : 0),
    last_match_at: now,
  };

  const opponentUpdates = {
    rating: snapshot.nextRatingB,
    games_played: opponentGames,
    wins: opponentRating.wins + (outcome === "opponent" ? 1 : 0),
    losses: opponentRating.losses + (outcome === "challenger" ? 1 : 0),
    draws: opponentRating.draws + (outcome === "draw" ? 1 : 0),
    last_match_at: now,
  };

  const nextChallengerRating = await writeRating(challengerRating, challengerUpdates);
  const nextOpponentRating = await writeRating(opponentRating, opponentUpdates);

  return {
    challenger: nextChallengerRating,
    opponent: nextOpponentRating,
    snapshot,
  };
};

const loadMatchById = async (matchId: string): Promise<ArenaMatchRow | null> => {
  const supabase = createSupabaseServiceRoleClient();
  if (!supabase) {
    throw new Error("Supabase service role is not configured.");
  }

  const { data, error } = await supabase
    .from("krabbyclaw_arena_matches")
    .select("*")
    .eq("id", matchId)
    .maybeSingle();

  if (error) {
    throw new Error(`Failed to load arena match: ${error.message}`);
  }
  return (data ?? null) as ArenaMatchRow | null;
};

const saveMatch = async (payload: TablesInsert<"krabbyclaw_arena_matches">): Promise<ArenaMatchRow> => {
  const supabase = createSupabaseServiceRoleClient();
  if (!supabase) {
    throw new Error("Supabase service role is not configured.");
  }

  const { data, error } = await supabase
    .from("krabbyclaw_arena_matches")
    .insert(payload)
    .select("*")
    .maybeSingle();
  if (error || !data) {
    throw new Error(`Failed to create arena match: ${error?.message ?? "Unknown error"}`);
  }

  return data as ArenaMatchRow;
};

const updateMatch = async (
  matchId: string,
  payload: TablesUpdate<"krabbyclaw_arena_matches">,
): Promise<void> => {
  const supabase = createSupabaseServiceRoleClient();
  if (!supabase) {
    throw new Error("Supabase service role is not configured.");
  }

  const { error } = await supabase
    .from("krabbyclaw_arena_matches")
    .update(payload)
    .eq("id", matchId);

  if (error) {
    throw new Error(`Failed to update arena match: ${error.message}`);
  }
};

const fetchArenaView = async (limit: number) => {
  const supabase = createSupabaseServiceRoleClient();
  if (!supabase) {
    return {
      leaderboard: [],
      queue: [],
      activeMatches: [],
      recentMatches: [],
      agents: {},
      warning: "Supabase service role is not configured.",
    };
  }

  const [leaderboardResult, matchesResult] = await Promise.all([
    supabase
      .from("krabbyclaw_arena_leaderboard")
      .select("*")
      .order("rating", { ascending: false })
      .limit(limit),
    supabase
      .from("krabbyclaw_arena_matches")
      .select("*")
      .order("created_at", { ascending: false })
      .limit(Math.max(limit * 3, 30)),
  ]);

  const leaderboard = (leaderboardResult.data ?? []) as Array<Record<string, Json>>;
  const matches = (matchesResult.data ?? []) as ArenaMatchRow[];

  const ids = new Set<string>();
  for (const match of matches) {
    ids.add(match.challenger_agent_id);
    ids.add(match.opponent_agent_id);
  }

  const agentsLookup: Record<string, ArenaAgentRow> = {};
  if (ids.size > 0) {
    const { data: agents } = await supabase
      .from("arena_agents")
      .select("id,name,slug,runtime")
      .in("id", Array.from(ids));
    for (const agent of (agents ?? []) as ArenaAgentRow[]) {
      agentsLookup[agent.id] = agent;
    }
  }

  const queue = matches.filter(
    (match) => match.status === "pending" && match.challenger_agent_id === match.opponent_agent_id,
  );

  return {
    leaderboard,
    queue: queue.slice(0, limit),
    activeMatches: matches.filter((match) => match.status === "running" || match.status === "pending").slice(0, limit),
    recentMatches: matches.filter((match) => match.status === "completed" || match.status === "cancelled").slice(0, limit),
    agents: Object.fromEntries(
      Object.entries(agentsLookup).map(([agentId, agent]) => [agentId, toAgentSummary(agent)]),
    ),
    warning: leaderboardResult.error?.message ?? matchesResult.error?.message,
  };
};

const validatePostAuth = (request: Request, payload: ArenaRequestPayload): NextResponse | null => {
  const configuredToken = (
    process.env.POKECRYSTAL_ARENA_PROGRESS_TOKEN ??
    process.env.POKECRYSTAL_ARENA_SNAPSHOT_TOKEN ??
    process.env.POKECRYSTAL_MCP_TOKEN ??
    ""
  ).trim();

  const tokenProtected = configuredToken.length > 0;
  const staticAuthorized = tokenProtected ? isRequestAuthorized(request, configuredToken) : false;
  const identityAuthorized = hasValidIdentityToken(request.headers);

  if (tokenProtected && !staticAuthorized && !identityAuthorized) {
    return NextResponse.json({ ok: false, error: "Unauthorized" }, { status: 401, headers: noStoreHeaders });
  }

  if (!staticAuthorized) {
    const sessionForAuth = payload.controllerSessionId ?? payload.agent?.sessionId ?? payload.challenger?.sessionId ?? null;
    const secretCheck = verifySessionSecret(request, sessionForAuth);
    if (!secretCheck.ok) {
      return NextResponse.json(
        { ok: false, error: secretCheck.message ?? "Unauthorized" },
        { status: secretCheck.status, headers: noStoreHeaders },
      );
    }
  }

  return null;
};

const handleTeamUpdate = async (payload: ArenaRequestPayload) => {
  if (!payload.agent) {
    throw new Error("agent payload is required.");
  }
  const ownerId = await resolveSystemOwnerId();
  if (!ownerId) {
    throw new Error("No system owner is configured for KrabbyClawArena.");
  }
  const agent = await upsertAgent(ownerId, payload.agent);
  return {
    agent,
    team: payload.agent.team,
  };
};

const handleQueue = async (payload: ArenaRequestPayload) => {
  if (!payload.agent) {
    throw new Error("agent payload is required.");
  }
  if (!payload.agent.sessionId) {
    throw new Error("agent.sessionId is required for queueing.");
  }
  const ownerId = await resolveSystemOwnerId();
  if (!ownerId) {
    throw new Error("No system owner is configured for KrabbyClawArena.");
  }

  const agent = await upsertAgent(ownerId, payload.agent);

  const supabase = createSupabaseServiceRoleClient();
  if (!supabase) {
    throw new Error("Supabase service role is not configured.");
  }

  const pendingResult = await supabase
    .from("krabbyclaw_arena_matches")
    .select("*")
    .eq("status", "pending")
    .order("created_at", { ascending: true })
    .limit(25);

  const pending = (pendingResult.data ?? []) as ArenaMatchRow[];
  const pendingMatch = pending.find(
    (match) => match.challenger_agent_id !== agent.id && match.challenger_agent_id === match.opponent_agent_id,
  );

  const now = new Date().toISOString();

  if (pendingMatch) {
    const metadata = metadataToRecord(pendingMatch.metadata);
    const teams = metadataToRecord(metadata.teams);
    await updateMatch(pendingMatch.id, {
      opponent_agent_id: agent.id,
      opponent_session_id: payload.agent.sessionId,
      status: "running",
      updated_at: now,
      started_at: pendingMatch.started_at ?? now,
      metadata: {
        ...metadata,
        teams: {
          ...teams,
          opponent: payload.agent.team,
        },
      },
    });

    return {
      matched: true,
      matchId: pendingMatch.id,
      opponent: toAgentSummary(agent),
    };
  }

  const match = await saveMatch({
    challenger_agent_id: agent.id,
    opponent_agent_id: agent.id,
    created_by: ownerId,
    queue: payload.queue,
    status: "pending",
    outcome: null,
    winner_agent_id: null,
    challenger_session_id: payload.agent.sessionId,
    opponent_session_id: payload.agent.sessionId,
    challenger_score: null,
    opponent_score: null,
    notes: payload.notes,
    metadata: {
      ...payload.metadata,
      team_format: "random-battle",
      teams: {
        challenger: payload.agent.team,
      },
    },
    started_at: now,
    updated_at: now,
  });

  return {
    matched: false,
    matchId: match.id,
  };
};

  const handleLeaveQueue = async (payload: ArenaRequestPayload) => {
  if (!payload.agent) {
    throw new Error("agent payload is required.");
  }
  const ownerId = await resolveSystemOwnerId();
  if (!ownerId) {
    throw new Error("No system owner is configured for KrabbyClawArena.");
  }

  const agent = await upsertAgent(ownerId, payload.agent);

  const supabase = createSupabaseServiceRoleClient();
  if (!supabase) {
    throw new Error("Supabase service role is not configured.");
  }

  const pendingResult = await supabase
    .from("krabbyclaw_arena_matches")
    .select("*")
    .eq("status", "pending")
    .order("created_at", { ascending: false })
    .limit(25);

  const pending = (pendingResult.data ?? []) as ArenaMatchRow[];
  const queued = pending.find(
    (match) => match.challenger_agent_id === agent.id && match.challenger_agent_id === match.opponent_agent_id,
  );

  if (!queued) {
    return { removed: false };
  }

  const now = new Date().toISOString();
  await updateMatch(queued.id, {
    status: "cancelled",
    outcome: "cancelled",
    finished_at: now,
    updated_at: now,
  });

    return {
      removed: true,
      matchId: queued.id,
    };
  };

const handleStart = async (payload: ArenaRequestPayload) => {
  if (!payload.challenger || !payload.opponent) {
    throw new Error("challenger and opponent payloads are required.");
  }
  if (!payload.challenger.sessionId || !payload.opponent.sessionId) {
    throw new Error("Both challenger and opponent session ids are required.");
  }
  const ownerId = await resolveSystemOwnerId();
  if (!ownerId) {
    throw new Error("No system owner is configured for KrabbyClawArena.");
  }

  const [challenger, opponent] = await Promise.all([
    upsertAgent(ownerId, payload.challenger),
    upsertAgent(ownerId, payload.opponent),
  ]);

  if (challenger.id === opponent.id) {
    throw new Error("Challenger and opponent must be different agents.");
  }

  const now = new Date().toISOString();
  const match = await saveMatch({
    challenger_agent_id: challenger.id,
    opponent_agent_id: opponent.id,
    created_by: ownerId,
    queue: payload.queue,
    status: "running",
    outcome: null,
    winner_agent_id: null,
    challenger_session_id: payload.challenger.sessionId,
    opponent_session_id: payload.opponent.sessionId,
    challenger_score: payload.challengerScore,
    opponent_score: payload.opponentScore,
    notes: payload.notes,
    metadata: {
      ...payload.metadata,
      team_format: "random-battle",
      teams: {
        challenger: payload.challenger.team,
        opponent: payload.opponent.team,
      },
    },
    started_at: now,
    updated_at: now,
  });

  return {
    match,
    challenger,
    opponent,
  };
};

const resolveOutcome = (payload: ArenaRequestPayload): { outcome: BattleOutcome; source: string } => {
  if (payload.outcome) {
    return {
      outcome: payload.outcome,
      source: "explicit",
    };
  }
  throw new Error("outcome is required for finish/report.");
};

const handleFinish = async (payload: ArenaRequestPayload) => {
  if (!payload.matchId) {
    throw new Error("matchId is required.");
  }
  const existingMatch = await loadMatchById(payload.matchId);
  if (!existingMatch) {
    throw new Error("Match not found.");
  }

  const resolution = resolveOutcome(payload);
  const outcome = resolution.outcome;
  const now = new Date().toISOString();

  let winnerAgentId: string | null = null;
  if (outcome === "challenger") {
    winnerAgentId = existingMatch.challenger_agent_id;
  } else if (outcome === "opponent") {
    winnerAgentId = existingMatch.opponent_agent_id;
  }

  const status: ArenaMatchRow["status"] = outcome === "cancelled" ? "cancelled" : "completed";

  await updateMatch(existingMatch.id, {
    status,
    outcome,
    winner_agent_id: winnerAgentId,
    challenger_score: payload.challengerScore ?? existingMatch.challenger_score,
    opponent_score: payload.opponentScore ?? existingMatch.opponent_score,
    finished_at: now,
    updated_at: now,
    notes: payload.notes ?? existingMatch.notes,
    metadata: {
      ...(existingMatch.metadata && typeof existingMatch.metadata === "object" && !Array.isArray(existingMatch.metadata)
        ? (existingMatch.metadata as Record<string, Json>)
        : {}),
      ...payload.metadata,
      resolution_source: resolution.source,
    },
  });

  let ratings: Awaited<ReturnType<typeof applyRatings>> | null = null;
  if (outcome !== "cancelled") {
    ratings = await applyRatings(
      existingMatch.challenger_agent_id,
      existingMatch.opponent_agent_id,
      outcome,
      payload.kFactor,
    );
  }

  return {
    matchId: existingMatch.id,
    status,
    outcome,
    winnerAgentId,
    resolutionSource: resolution.source,
    ratings,
  };
};

export async function GET(request: Request) {
  const url = new URL(request.url);
  const limitRaw = Number(url.searchParams.get("limit") ?? "20");
  const limit = Number.isFinite(limitRaw) ? Math.max(1, Math.min(50, Math.trunc(limitRaw))) : 20;

  try {
    const snapshot = await fetchArenaView(limit);
    return NextResponse.json({ ok: true, ...snapshot }, { headers: noStoreHeaders });
  } catch (error) {
    const message = error instanceof Error ? error.message : "Failed to load KrabbyClawArena.";
    return NextResponse.json({ ok: false, error: message }, { status: 500, headers: noStoreHeaders });
  }
}

export async function POST(request: Request) {
  let payload: ArenaRequestPayload;
  try {
    payload = parseBody(await request.json());
  } catch (error) {
    return NextResponse.json(
      { ok: false, error: error instanceof Error ? error.message : "Invalid request body." },
      { status: 400, headers: noStoreHeaders },
    );
  }

  const authFailure = validatePostAuth(request, payload);
  if (authFailure) {
    return authFailure;
  }

  try {
    if (payload.action === "team") {
      const updated = await handleTeamUpdate(payload);
      return NextResponse.json(
        {
          ok: true,
          action: payload.action,
          agent: toAgentSummary(updated.agent),
          team: updated.team,
        },
        { headers: noStoreHeaders },
      );
    }

    if (payload.action === "queue") {
      const queued = await handleQueue(payload);
      return NextResponse.json(
        {
          ok: true,
          action: payload.action,
          ...queued,
        },
        { headers: noStoreHeaders },
      );
    }

    if (payload.action === "leave") {
      const removed = await handleLeaveQueue(payload);
      return NextResponse.json(
        {
          ok: true,
          action: payload.action,
          ...removed,
        },
        { headers: noStoreHeaders },
      );
    }

    if (payload.action === "start") {
      const started = await handleStart(payload);
      return NextResponse.json(
        {
          ok: true,
          action: payload.action,
          matchId: started.match.id,
          match: started.match,
          challenger: toAgentSummary(started.challenger),
          opponent: toAgentSummary(started.opponent),
        },
        { headers: noStoreHeaders },
      );
    }

    if (payload.action === "report") {
      const started = await handleStart(payload);
      const finished = await handleFinish({ ...payload, action: "finish", matchId: started.match.id });
      return NextResponse.json(
        {
          ok: true,
          action: payload.action,
          startedMatch: started.match,
          challenger: toAgentSummary(started.challenger),
          opponent: toAgentSummary(started.opponent),
          ...finished,
        },
        { headers: noStoreHeaders },
      );
    }

    const finished = await handleFinish(payload);
    return NextResponse.json(
      {
        ok: true,
        action: payload.action,
        ...finished,
      },
      { headers: noStoreHeaders },
    );
  } catch (error) {
    const message = error instanceof Error ? error.message : "Failed to process KrabbyClawArena request.";
    return NextResponse.json({ ok: false, error: message }, { status: 500, headers: noStoreHeaders });
  }
}
