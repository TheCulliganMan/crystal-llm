import { z, type ZodTypeAny } from "zod";
import { MAX_ADVANCE_FRAMES, type McpToolHandler } from "./common";
import { ObserveSchema, observeHandler } from "./observe";
import { MapInfoSchema, mapInfoHandler } from "./map_info";
import { RouteRenderSchema, routeRenderHandler } from "./route_render";
import { FlowStateSchema, flowStateHandler } from "./flow_state";
import {
  ExecuteMacroSchema,
  executeMacroHandler,
  HoldButtonSchema,
  holdButtonHandler,
  JournalSchema,
  journalHandler,
  MoveSchema,
  moveHandler,
  PressSchema,
  pressHandler,
  RecentEventsSchema,
  recentEventsHandler,
  StatusSchema,
  statusHandler,
  TypeTextSchema,
  typeTextHandler,
} from "./input";
import {
  RegisterIdentitySchema,
  registerIdentityHandler,
  WhoAmISchema,
  whoAmIHandler,
} from "./identity";

export type McpToolDefinition = {
  name: string;
  title: string;
  description: string;
  inputSchema: ZodTypeAny;
  handler: McpToolHandler<any>;
};

const TrainingMetadataSchema = z.object({
  reasoning: z.string().trim().min(1).max(800).optional(),
  goal: z.string().trim().min(1).max(240).optional(),
});

const TrainingMetadataShape = TrainingMetadataSchema.shape;

const withTrainingMetadata = (schema: ZodTypeAny): ZodTypeAny =>
  schema instanceof z.ZodObject
    ? schema.extend(TrainingMetadataShape)
    : schema.and(TrainingMetadataSchema);

const macroLimit = MAX_ADVANCE_FRAMES;

export const MCP_TOOL_DEFINITIONS: McpToolDefinition[] = [
  {
    name: "register_identity",
    title: "Register identity",
    description: "Register an anonymous player identity and return { playerId, token }.",
    inputSchema: withTrainingMetadata(RegisterIdentitySchema),
    handler: registerIdentityHandler,
  },
  {
    name: "whoami",
    title: "Who am I",
    description: "Return current identity and save-slot summary for the current token.",
    inputSchema: withTrainingMetadata(WhoAmISchema),
    handler: whoAmIHandler,
  },
  {
    name: "observe",
    title: "Observe",
    description:
      "Observe the current snapshot. Pass include_image=true to return an image/png content block of the current game view. Coordinates use screen/map convention x+ right, x- left, y+ down, y- up. Compact JSON includes dir plus warp guidance: ow.w[].stand is where to stand, ow.w[].move/go is the direction to enter.",
    inputSchema: withTrainingMetadata(ObserveSchema),
    handler: observeHandler,
  },
  {
    name: "map_info",
    title: "Map info",
    description:
      "Return structured current-map info, including player position, stable warps, and hotspot metadata for the current map. Use this when status is not enough for route planning.",
    inputSchema: withTrainingMetadata(MapInfoSchema),
    handler: mapInfoHandler,
  },
  {
    name: "route_render",
    title: "Route render",
    description:
      "Render the full current overworld route/city/interior as an agent-readable schematic. Returns JSON grid rows by default; pass include_image=true for an annotated image/png, or image_style=tiles for high-fidelity metatile art. Does not inspect arbitrary maps or choose routes.",
    inputSchema: withTrainingMetadata(RouteRenderSchema),
    handler: routeRenderHandler,
  },
  {
    name: "flow_state",
    title: "Flow state",
    description:
      "Return spoiler-safe game progression state toward the Mt. Silver clear, masking surprise story beats as ???. Use this to confirm the next honest story goal.",
    inputSchema: withTrainingMetadata(FlowStateSchema),
    handler: flowStateHandler,
  },
  {
    name: "move",
    title: "Move",
    description:
      "Send a directional input. Direction means x/y movement in the observe convention: left x-1, right x+1, up y-1, down y+1. In menus, name entry, and time entry it moves the cursor or adjusts the selected value. Prefer one small move, then check status again.",
    inputSchema: withTrainingMetadata(MoveSchema),
    handler: moveHandler,
  },
  {
    name: "press",
    title: "Press",
    description:
      "Press a button with hardware-accurate press + release. A selects/confirms, B cancels/deletes, Start accepts END on name entry, and Select sends the Game Boy Select button.",
    inputSchema: withTrainingMetadata(PressSchema),
    handler: pressHandler,
  },
  {
    name: "type_text",
    title: "Type text",
    description:
      "Send literal text input to text-entry surfaces such as the naming screen. On name entry, pass clear:true to delete the current name first and submit:true to choose END after typing; preserves typed case and supports spaces.",
    inputSchema: withTrainingMetadata(TypeTextSchema),
    handler: typeTextHandler,
  },
  {
    name: "hold_button",
    title: "Hold button",
    description: "Hold a button for a specific number of frames when timing matters and a simple press is not enough.",
    inputSchema: withTrainingMetadata(HoldButtonSchema),
    handler: holdButtonHandler,
  },
  {
    name: "execute_macro",
    title: "Execute macro",
    description:
      `Execute either explicit movement/button actions or a bounded built-in macro. Schema: { actions?: [{ type: "move", value: "up|down|left|right", times?: 1-${macroLimit}, hold_frames?: 1-${macroLimit}, delay_frames?: 0-${macroLimit} } | { type: "button", value: "a|b|start|select", times?: 1-${macroLimit}, hold_frames?: 1-${macroLimit}, delay_frames?: 0-${macroLimit} }], macro?: "advance_dialog"|"mash_a"|"interact"|"approach_target", target_token?: string, max_presses?: 1-${macroLimit}, max_steps?: 1-${macroLimit}, max_observes?: 1-${macroLimit}, max_tries?: 1-${macroLimit}, press_a?: boolean, settle_frames?: 0-${macroLimit}, delay_frames?: 0-${macroLimit}, stop_on_event?: boolean }`,
    inputSchema: withTrainingMetadata(ExecuteMacroSchema),
    handler: executeMacroHandler,
  },
  {
    name: "status",
    title: "Status",
    description:
      "Return compact structured session state and last action result, including localFocus, interactionSetup, interactionLane, localMovement, menu/dialog flags, and blockedReason. Use this first before observe.",
    inputSchema: withTrainingMetadata(StatusSchema),
    handler: statusHandler,
  },
  {
    name: "recent_events",
    title: "Recent events",
    description: "Return the last N action events plus a short recap string so the agent can confirm what changed after an action.",
    inputSchema: withTrainingMetadata(RecentEventsSchema),
    handler: recentEventsHandler,
  },
  {
    name: "journal",
    title: "Journal",
    description: "Alias for recent_events.",
    inputSchema: withTrainingMetadata(JournalSchema),
    handler: journalHandler,
  },
];

const TOOL_MAP = new Map(MCP_TOOL_DEFINITIONS.map((definition) => [definition.name, definition]));

export const getMcpToolDefinition = (name: string): McpToolDefinition | undefined =>
  TOOL_MAP.get(name);
