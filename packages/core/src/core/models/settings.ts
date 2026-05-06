import { z } from 'zod';
import { BattleScene, BattleStyle } from '../enums/battle';
import {
  FrameType,
  MenuAccount,
  PrintOption,
  Sound,
  TextSpeed,
} from '../enums/ui-enums';

export { PlayerGender } from '../enums/pokemon';


export const OptionsSchema = z.object({
  text_speed: z.nativeEnum(TextSpeed).default(TextSpeed.FAST),
  battle_scene: z.nativeEnum(BattleScene).default(BattleScene.ON),
  battle_style: z.nativeEnum(BattleStyle).default(BattleStyle.SHIFT),
  sound: z.nativeEnum(Sound).default(Sound.STEREO),
  menu_account: z.nativeEnum(MenuAccount).default(MenuAccount.ON),
  print_option: z.nativeEnum(PrintOption).default(PrintOption.NORMAL),
  frame: z.nativeEnum(FrameType).default(FrameType.FRAME_1),
  no_text_scroll: z.boolean().default(false),
});
export type Options = z.infer<typeof OptionsSchema>;

export const BadgesSchema = z.object({
  johto: z.array(z.boolean()).default(Array(8).fill(false)),
  kanto: z.array(z.boolean()).default(Array(8).fill(false)),
});
export type Badges = z.infer<typeof BadgesSchema>;
