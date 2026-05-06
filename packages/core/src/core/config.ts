
import { z } from "zod";

const MIN_PLAYTEST_SESSION_TTL_SECONDS = 30 * 60;

const AppSettingsSchema = z.object({
  basePath: z.string().optional().default(""),
  saveRoot: z.string().optional().default(""),
  logLevel: z.string().optional(),
  skipAudioPreload: z.boolean().default(false),
  skipStoryEvents: z.boolean().default(false),
  skipSpecialEvents: z.boolean().default(false),
  lazyScripts: z.boolean().default(false),
  debugSightlines: z.boolean().default(false),
  initAudioWhenMuted: z.boolean().default(false),
  masterVolume: z.number().min(0).max(1).optional(),
  superGameBoy: z.boolean().default(false),

  textuiInitPygam: z.boolean().default(false),
  textuiLive: z.boolean().optional(),
  textuiRefreshHz: z.number().min(0).optional(),
  textuiColor: z.string().default("0"),
  textSnapshotImage: z.boolean().default(true),
  textSnapshotAudio: z.boolean().default(true),

  mcpAllowAudio: z.boolean().default(false),
  mcpPlayerName: z.string().default("AI"),
  mcpTransport: z.string().default("stdio"),
  mcpHost: z.string().default("127.0.0.1"),
  mcpPort: z.number().positive().default(8000),
  mcpPath: z.string().default("/mcp"),
  mcpLogLevel: z.string().optional(),
  mcpFrameTimeout: z.number().positive().default(5.0),
  mcpInitialFrameTimeout: z.number().positive().default(15.0),
  mcpSlowSceneTimeout: z.number().positive().default(12.0),
  mcpDualRender: z.boolean().default(true),
  mcpAllowWindow: z.boolean().default(false),
  mcpAutoDialogue: z.boolean().default(true),
  mcpAutoDialogueOnWait: z.boolean().default(true),
  mcpAutoDialogueReserveFinalPress: z.boolean().default(false),
  mcpAutoDialogueTimeout: z.number().positive().optional(),
  mcpAutoDialogueStepTimeout: z.number().positive().default(2.0),
  mcpHoldFrames: z.number().positive().default(6),
  mcpDebugInput: z.boolean().default(false),
  mcpNotesDir: z.string().optional(),
  mcpMaxSessions: z.number().int().positive().default(32),
  mcpSessionTtlSeconds: z.number().int().positive().default(1800),
  mcpMaxActionsPerMinute: z.number().int().positive().default(180),
  mcpMaxActionsPerCall: z.number().int().positive().default(240),
  mcpMaxFramesPerMinute: z.number().int().positive().default(3600),
});

export type AppSettings = z.infer<typeof AppSettingsSchema>;

let cachedSettings: AppSettings | null = null;

export function getSettings(): AppSettings {
  if (cachedSettings) {
    return cachedSettings;
  }

  const settings = AppSettingsSchema.parse({
    basePath: process.env.POKECRYSTAL_BASE_PATH,
    saveRoot: process.env.POKECRYSTAL_SAVE_ROOT,
    logLevel: process.env.POKECRYSTAL_LOG_LEVEL,
    skipAudioPreload: process.env.POKECRYSTAL_SKIP_AUDIO_PRELOAD === "true",
    skipStoryEvents: process.env.POKECRYSTAL_SKIP_STORY_EVENTS === "true",
    skipSpecialEvents: process.env.POKECRYSTAL_SKIP_SPECIAL_EVENTS === "true",
    lazyScripts: process.env.POKECRYSTAL_LAZY_SCRIPTS === "true",
    debugSightlines: process.env.POKECRYSTAL_DEBUG_SIGHTLINES === "true",
    initAudioWhenMuted: process.env.POKECRYSTAL_INIT_AUDIO_WHEN_MUTED === "true",
    masterVolume: process.env.POKECRYSTAL_MASTER_VOLUME
      ? parseFloat(process.env.POKECRYSTAL_MASTER_VOLUME)
      : undefined,
    superGameBoy: process.env.POKECRYSTAL_SUPER_GAME_BOY === "true",

    textuiInitPygam: process.env.POKECRYSTAL_TEXTUI_INIT_PYGAME === "true",
    textuiLive: process.env.POKECRYSTAL_TEXTUI_LIVE
      ? process.env.POKECRYSTAL_TEXTUI_LIVE === "true"
      : undefined,
    textuiRefreshHz: process.env.POKECRYSTAL_TEXTUI_REFRESH_HZ
      ? parseFloat(process.env.POKECRYSTAL_TEXTUI_REFRESH_HZ)
      : undefined,
    textuiColor: process.env.POKECRYSTAL_TEXTUI_COLOR || "0",
    textSnapshotImage: process.env.POKECRYSTAL_TEXT_SNAPSHOT_IMAGE !== "false",
    textSnapshotAudio: process.env.POKECRYSTAL_TEXT_SNAPSHOT_AUDIO !== "false",

    mcpAllowAudio: process.env.POKECRYSTAL_MCP_ALLOW_AUDIO === "true",
    mcpPlayerName: process.env.POKECRYSTAL_MCP_PLAYER_NAME || "AI",
    mcpTransport: process.env.POKECRYSTAL_MCP_TRANSPORT || "stdio",
    mcpHost: process.env.POKECRYSTAL_MCP_HOST || "127.0.0.1",
    mcpPort: process.env.POKECRYSTAL_MCP_PORT
      ? parseInt(process.env.POKECRYSTAL_MCP_PORT)
      : 8000,
    mcpPath: process.env.POKECRYSTAL_MCP_PATH || "/mcp",
    mcpLogLevel: process.env.POKECRYSTAL_MCP_LOG_LEVEL,
    mcpFrameTimeout: process.env.POKECRYSTAL_MCP_FRAME_TIMEOUT
      ? parseFloat(process.env.POKECRYSTAL_MCP_FRAME_TIMEOUT)
      : 5.0,
    mcpInitialFrameTimeout: process.env.POKECRYSTAL_MCP_INITIAL_FRAME_TIMEOUT
      ? parseFloat(process.env.POKECRYSTAL_MCP_INITIAL_FRAME_TIMEOUT)
      : 15.0,
    mcpSlowSceneTimeout: process.env.POKECRYSTAL_MCP_SLOW_SCENE_TIMEOUT
      ? parseFloat(process.env.POKECRYSTAL_MCP_SLOW_SCENE_TIMEOUT)
      : 12.0,
    mcpDualRender: process.env.POKECRYSTAL_MCP_DUAL_RENDER !== "false",
    mcpAllowWindow: process.env.POKECRYSTAL_MCP_ALLOW_WINDOW === "true",
    mcpAutoDialogue: process.env.POKECRYSTAL_MCP_AUTO_DIALOGUE !== "false",
    mcpAutoDialogueOnWait:
      process.env.POKECRYSTAL_MCP_AUTO_DIALOGUE_ON_WAIT !== "false",
    mcpAutoDialogueReserveFinalPress:
      process.env.POKECRYSTAL_MCP_AUTO_DIALOGUE_RESERVE_FINAL_PRESS === "true",
    mcpAutoDialogueTimeout: process.env.POKECRYSTAL_MCP_AUTO_DIALOGUE_TIMEOUT
      ? parseFloat(process.env.POKECRYSTAL_MCP_AUTO_DIALOGUE_TIMEOUT)
      : undefined,
    mcpAutoDialogueStepTimeout: process.env.POKECRYSTAL_MCP_AUTO_DIALOGUE_STEP_TIMEOUT
      ? parseFloat(process.env.POKECRYSTAL_MCP_AUTO_DIALOGUE_STEP_TIMEOUT)
      : 2.0,
    mcpHoldFrames: process.env.POKECRYSTAL_MCP_HOLD_FRAMES
      ? parseInt(process.env.POKECRYSTAL_MCP_HOLD_FRAMES)
      : 6,
    mcpDebugInput: process.env.POKECRYSTAL_MCP_DEBUG_INPUT === "true",
    mcpNotesDir: process.env.POKECRYSTAL_MCP_NOTES_DIR,
    mcpMaxSessions: process.env.POKECRYSTAL_MCP_MAX_SESSIONS
      ? parseInt(process.env.POKECRYSTAL_MCP_MAX_SESSIONS)
      : 32,
    mcpSessionTtlSeconds: process.env.POKECRYSTAL_MCP_SESSION_TTL_SECONDS
      ? Math.max(
          MIN_PLAYTEST_SESSION_TTL_SECONDS,
          parseInt(process.env.POKECRYSTAL_MCP_SESSION_TTL_SECONDS)
        )
      : MIN_PLAYTEST_SESSION_TTL_SECONDS,
    mcpMaxActionsPerMinute: process.env.POKECRYSTAL_MCP_MAX_ACTIONS_PER_MINUTE
      ? parseInt(process.env.POKECRYSTAL_MCP_MAX_ACTIONS_PER_MINUTE)
      : 180,
    mcpMaxActionsPerCall: process.env.POKECRYSTAL_MCP_MAX_ACTIONS_PER_CALL
      ? parseInt(process.env.POKECRYSTAL_MCP_MAX_ACTIONS_PER_CALL)
      : 25,
    mcpMaxFramesPerMinute: process.env.POKECRYSTAL_MCP_MAX_FRAMES_PER_MINUTE
      ? parseInt(process.env.POKECRYSTAL_MCP_MAX_FRAMES_PER_MINUTE)
      : 3600,
  });

  cachedSettings = settings;
  return settings;
}

export function resetSettings(): void {
  cachedSettings = null;
}
