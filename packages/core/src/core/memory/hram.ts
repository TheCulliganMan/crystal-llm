import { z } from 'zod';
import { ScriptMemorySchema } from './script-memory';

export const JoypadStateSchema = z.object({
  hJoypadReleased: z.number().default(0),
  hJoypadPressed: z.number().default(0),
  hJoypadDown: z.number().default(0),
  hJoypadSum: z.number().default(0),
  hJoyReleased: z.number().default(0),
  hJoyPressed: z.number().default(0),
  hJoyDown: z.number().default(0),
  hJoyLast: z.number().default(0),
});

export type JoypadState = z.infer<typeof JoypadStateSchema>;

export const createJoypadState = (): JoypadState => JoypadStateSchema.parse({});

export const HRAMSchema = z.object({
  joypad: JoypadStateSchema.default(() => JoypadStateSchema.parse({})),
  hInMenu: z.number().default(0),
  hMapEntryMethod: z.number().default(0),
  hRandomAdd: z.number().default(0),
  hRandomSub: z.number().default(0),
  hardware_divider: z.number().default(0),

  // RTC registers (from GetClock)
  hRTCDayHi: z.number().default(0),
  hRTCDayLo: z.number().default(0),
  hRTCHours: z.number().default(0),
  hRTCMinutes: z.number().default(0),
  hRTCSeconds: z.number().default(0),

  // Current time registers (from FixTime)
  hHours: z.number().default(0),
  hMinutes: z.number().default(0),
  hSeconds: z.number().default(0),
  hSCX: z.number().default(0),
  hSCY: z.number().default(0),
  hWX: z.number().default(0),
  hWY: z.number().default(0),
  script_memory: ScriptMemorySchema,
});

export type HRAM = z.infer<typeof HRAMSchema>;
