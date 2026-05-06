export type CliCommand = "mcp" | "play" | "play-recorded" | "register" | "skill" | "help";

export type ToolContent = {
  type: string;
  text?: string;
  data?: string;
  mimeType?: string;
};

export type ToolResult = {
  content?: ToolContent[];
  isError?: boolean;
};

export type ToolEnvelope = {
  ok?: boolean;
  tool?: string;
  error?: string;
  result?: ToolResult;
};

export type CliOptions = {
  command: CliCommand;
  transport: "local" | "http";
  baseUrl: string;
  toolsUrl?: string;
  sessionId: string;
  sessionMode?: "automation" | "interactive";
  token?: string;
  sessionSecret?: string;
  agentId?: string;
  identityName?: string;
  printSkill?: boolean;
  sessionLogEnabled?: boolean;
  sessionLogDir?: string;
  sessionLogFile?: string;
  recordTraining?: boolean;
  trainingDir?: string;
  agent?: boolean;
  agentCommand?: "run" | "resume";
  agentModel?: string;
  agentGoal?: string;
  agentMaxSteps?: number;
  agentGraphCycleSteps?: number;
  agentRequestDelayMs?: number;
  agentIdentityName?: string;
};

export type KeyAction =
  | { type: "direction"; direction: "up" | "down" | "left" | "right" }
  | { type: "move"; direction: "up" | "down" | "left" | "right" }
  | { type: "press"; button: "a" | "b" | "start" | "select" | "up" | "down" | "left" | "right" }
  | { type: "text"; text: string }
  | { type: "wait"; frames: number }
  | { type: "refresh" }
  | { type: "quit" }
  | { type: "noop" };

export type CliPlayTrainingTurn = {
  session_id: string;
  recorded_at: string;
  step_index: number;
  raw_key: string;
  action: KeyAction;
  tool_name: "move" | "press" | "observe";
  tool_input: Record<string, unknown>;
  before_snapshot: string;
  action_result_snapshot: string;
  after_snapshot: string;
  status_snapshot: string;
  recent_events_snapshot: string;
  result_flags: {
    ok?: boolean;
    changed?: boolean;
    effect?: string;
    reason?: string;
  };
  tags: string[];
  response_meta: {
    action_result: Array<Record<string, any>>;
    observe: Array<Record<string, any>>;
    status: Array<Record<string, any>>;
    recent_events: Array<Record<string, any>>;
  };
  transport: "local" | "http";
};

export type CliPlayTrainingManifest = {
  schema_version: number;
  session_id: string;
  created_at: string;
  updated_at: string;
  transport: "local" | "http";
  base_url: string;
  training_dir: string;
  episode_path: string;
  examples_path: string;
  agent_events_path: string;
  total_turns: number;
  skipped_turns: number;
  example_turns: number;
  total_agent_events: number;
};
