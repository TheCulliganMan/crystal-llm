import * as z from "zod";

export const PayloadFormatSchema = z.literal("json").optional();
export const PayloadDetailSchema = z.enum(["full", "compact"]).optional();
export const IncludeSnapshotTextSchema = z.boolean().optional();

export type PayloadFormat = "json";
export type PayloadDetail = "full" | "compact";

export type PayloadSerializationOptions = {
  format?: PayloadFormat;
  detail?: PayloadDetail;
  include_snapshot_text?: boolean;
};

const pruneUndefinedDeep = (value: unknown): unknown => {
  if (Array.isArray(value)) {
    return value
      .map((entry) => pruneUndefinedDeep(entry))
      .filter((entry) => entry !== undefined);
  }
  if (value && typeof value === "object") {
    const prunedEntries = Object.entries(value as Record<string, unknown>)
      .map(([key, entry]) => [key, pruneUndefinedDeep(entry)] as const)
      .filter(([, entry]) => entry !== undefined);
    if (!prunedEntries.length) {
      return undefined;
    }
    return Object.fromEntries(prunedEntries);
  }
  return value === undefined ? undefined : value;
};

export const normalizePayloadOptions = (
  input: PayloadSerializationOptions
): Required<PayloadSerializationOptions> => ({
  format: input.format ?? "json",
  detail: input.detail ?? "compact",
  include_snapshot_text: input.include_snapshot_text ?? false,
});

export const serializeStructuredPayload = async (
  payload: unknown
): Promise<{ text: string; mimeType: string }> => {
  const prunedPayload = pruneUndefinedDeep(payload) ?? {};
  return {
    text: JSON.stringify(prunedPayload),
    mimeType: "application/json",
  };
};
