import { z } from "zod";

export const TMHMType = {
  TM: "TM",
  HM: "HM",
} as const;

export const TMHMData = z.object({
  name: z.string(),
  type: z.nativeEnum(TMHMType),
  move: z.string(),
});

export type TMHMData = z.infer<typeof TMHMData>;
