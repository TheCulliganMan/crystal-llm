import * as z from "zod";
import {
  McpToolExtra,
  McpToolResponse,
  loadSession,
  resolveSessionId,
  withRequestIdentity,
} from "./common";
import {
  PayloadDetailSchema,
  PayloadFormatSchema,
  normalizePayloadOptions,
  serializeStructuredPayload,
} from "./serialization";

export const FlowStateSchema = z.object({
  format: PayloadFormatSchema,
  detail: PayloadDetailSchema,
});

export const flowStateHandler = async (
  input: z.infer<typeof FlowStateSchema>,
  extra?: McpToolExtra
): Promise<McpToolResponse> => {
  return withRequestIdentity(extra, async () => {
    const resolvedSessionId = resolveSessionId(extra);
    const session = await loadSession(resolvedSessionId, extra);
    const flowState = await session.flowState();
    normalizePayloadOptions({
      format: input.format,
      detail: input.detail,
    });
    const serialized = await serializeStructuredPayload(flowState);
    return {
      content: [{ type: "text", text: serialized.text, mimeType: serialized.mimeType }],
    };
  });
};
