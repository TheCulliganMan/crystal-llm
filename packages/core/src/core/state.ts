import { createStore } from "zustand/vanilla";
import { z } from "zod";
import { SRAMSchema, WRAMSchema, HRAMSchema, SRAM, WRAM, HRAM } from "./memory";
import { VRAMSchema, VRAM } from "./memory/vram";
import { BattleScene, BattleStyle } from "./enums/battle";
import { FrameType, MenuAccount, Sound, TextSpeed } from "./enums/ui-enums";

export { SRAMSchema, WRAMSchema, HRAMSchema };

export const GameStateSchema = z.object({
  sram: SRAMSchema.default(() => SRAMSchema.parse({})),
  wram: WRAMSchema.default(() => WRAMSchema.parse({})),
  vram: VRAMSchema.default(() => VRAMSchema.parse({})),
  hram: HRAMSchema.default(() => HRAMSchema.parse({})),
  frame_counter: z.number().default(0),
  has_seen_intro: z.boolean().default(false),
});

export type GameState = z.infer<typeof GameStateSchema>;

export function createInitialGameState(): GameState {
  return {
    sram: SRAMSchema.parse({
      options: {
        text_speed: TextSpeed.FAST,
        battle_scene: BattleScene.ON,
        battle_style: BattleStyle.SHIFT,
        sound: Sound.STEREO,
        menu_account: MenuAccount.ON,
        frame: FrameType.FRAME_1,
      },
      party: {
        pokemon: Array(6).fill(null),
      },
      link_battle_stats: {
        wins: 0,
        losses: 0,
        draws: 0,
      },
      badges: {
        johto: Array(8).fill(false),
        kanto: Array(8).fill(false),
      },
    }),
    wram: WRAMSchema.parse({}),
    vram: VRAMSchema.parse({}),
    hram: HRAMSchema.parse({
      joypad: {
        hJoypadReleased: 0,
        hJoypadPressed: 0,
        hJoypadDown: 0,
        hJoypadSum: 0,
        hJoyReleased: 0,
        hJoyPressed: 0,
        hJoyDown: 0,
      hJoyLast: 0,
    },
  }),
    frame_counter: 0,
    has_seen_intro: false,
  };
}

export const useStore = createStore<GameState>((set) => createInitialGameState());

export function resetGameState() {
  useStore.setState(createInitialGameState());
}
