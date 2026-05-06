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
  serializeStructuredPayload,
} from "./serialization";

const MapInfoFormatSchema = z.literal("json").optional();

export const MapInfoSchema = z.object({
  format: MapInfoFormatSchema,
  detail: PayloadDetailSchema,
});

export const mapInfoHandler = async (
  input: z.infer<typeof MapInfoSchema>,
  extra?: McpToolExtra
): Promise<McpToolResponse> => {
  return withRequestIdentity(extra, async () => {
    const session = await loadSession(resolveSessionId(extra), extra);
    const info = await session.mapInfo();
    const serialized = await serializeStructuredPayload(info);
    return { content: [{ type: "text", text: serialized.text, mimeType: serialized.mimeType }] };
  });
};
