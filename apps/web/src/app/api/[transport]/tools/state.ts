import * as z from "zod";
import {
  McpToolExtra,
  McpToolResponse,
  loadSession,
  resolveSessionId,
  reportSnapshot,
  withRequestIdentity,
} from "./common";

export const GetGameStateSchema = z.object({});

export const getGameStateHandler = async (
  _input: z.infer<typeof GetGameStateSchema>,
  extra?: McpToolExtra
): Promise<McpToolResponse> => {
  return withRequestIdentity(extra, async () => {
    const resolvedSessionId = resolveSessionId(extra);
    const session = await loadSession(resolvedSessionId, extra);
    const data = await session.getGameStateData();
    const payload = JSON.stringify(data, null, 2);
    await reportSnapshot(resolvedSessionId, session, payload, "get_game_state");
    return {
      content: [{ type: "text", text: payload }],
    };
  });
};
