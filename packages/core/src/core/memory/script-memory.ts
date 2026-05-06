import { z } from 'zod';

export const ScriptMemoryValueSchema = z.union([
  z.number(),
  z.string(),
  z.boolean(),
  z.null(),
  z.array(z.unknown()),
  z.record(z.string(), z.unknown()),
]);

export const ScriptMemorySchema = z.record(z.string(), ScriptMemoryValueSchema).default({});

export type ScriptMemoryValue = z.infer<typeof ScriptMemoryValueSchema>;
export type ScriptMemory = z.infer<typeof ScriptMemorySchema>;
