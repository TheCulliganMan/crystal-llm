import * as z from "zod";
import { renderRouteRenderSurface } from "@pokecrystal/core/engine/world/overworld/route-render";
import {
  McpToolExtra,
  McpToolResponse,
  loadSession,
  resolveSessionId,
  withRequestIdentity,
} from "./common";
import {
  PayloadFormatSchema,
  serializeStructuredPayload,
} from "./serialization";
import { encodeSurfaceToPng } from "@/app/mcp/image-encoding";

const coerceOptionalInt = (min: number, max: number) =>
  z.preprocess((value) => {
    if (typeof value === "string") {
      const trimmed = value.trim();
      if (/^\d+$/.test(trimmed)) {
        return Number(trimmed);
      }
    }
    return value;
  }, z.number().int().min(min).max(max).optional());

export const RouteRenderSchema = z.object({
  include_image: z.boolean().optional(),
  image_scale: coerceOptionalInt(1, 8),
  cell_size: coerceOptionalInt(4, 16),
  detail: z.enum(["compact", "full"]).optional(),
  image_style: z.enum(["schematic", "tiles"]).optional(),
  format: PayloadFormatSchema,
});

export const routeRenderHandler = async (
  input: z.infer<typeof RouteRenderSchema>,
  extra?: McpToolExtra
): Promise<McpToolResponse> => {
  return withRequestIdentity(extra, async () => {
    const session = await loadSession(resolveSessionId(extra), extra);
    const snapshot = await session.routeRender({ detail: input.detail ?? "compact" });
    const serialized = await serializeStructuredPayload(snapshot);
    const content: McpToolResponse["content"] = [
      { type: "text", text: serialized.text, mimeType: serialized.mimeType },
    ];

    if (input.include_image && snapshot.available) {
      const surface = input.image_style === "tiles"
        ? await session.routeRenderImage(snapshot, { cellSize: input.cell_size })
        : renderRouteRenderSurface(snapshot, { cellSize: input.cell_size });
      const image = encodeSurfaceToPng(surface, { scale: input.image_scale });
      content.push({
        type: "image",
        data: image.data,
        mimeType: "image/png",
      });
    }

    return { content, snapshot };
  });
};
