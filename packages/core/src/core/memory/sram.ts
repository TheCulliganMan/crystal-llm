
import { z } from 'zod';
import {
  BadgesSchema,
  BoxSchema,
  DateSchema,
  DayCareSchema,
  HallOfFameEntrySchema,
  LinkBattleRecordSchema,
  LinkBattleStatsSchema,
  OptionsSchema,
  PartySchema,
  TimeSchema,
} from '../models';
import { MailMessageSchema } from '../mail';
import { BATTLETOWER_STREAK_LENGTH, MAX_MONEY, NUM_LINK_BATTLE_RECORDS, NUM_NON_TROPHY_DECOS, MOBILE_LOGIN_PASSWORD_LENGTH, NUM_POKEMON } from '../constants';
import { PlayerGender } from '../enums';
import { ScriptMemorySchema } from './script-memory';

export const MAX_MYSTERY_GIFT_PARTNERS = 5;
export const BATTLE_TOWER_TRAINER_SLOT_SENTINEL = 0xff;

const defaultLinkBattleRecords = () =>
  Array(NUM_LINK_BATTLE_RECORDS)
    .fill(null)
    .map(() => LinkBattleRecordSchema.parse({}));

const defaultTrainerHistory = () =>
  Array(BATTLETOWER_STREAK_LENGTH).fill(BATTLE_TOWER_TRAINER_SLOT_SENTINEL);

export const PhotoSnapshotSchema = z.object({
  species: z.string(),
  nickname: z.string(),
  palette: z.number(),
  level: z.number(),
  trainer_id: z.number(),
  party_index: z.number(),
  frame: z.number(),
});

export type PhotoSnapshot = z.infer<typeof PhotoSnapshotSchema>;

export const MysteryGiftSaveDataSchema = z.object({
  stored_item: z.string().nullable().default(null),
  backup_item: z.string().nullable().default(null),
  timer: z.number().default(0).transform((val) => Math.max(0, Math.min(val, 0xffff))),
  daily_partner_ids: z.array(z.number()).default([]).transform((val) => {
    const sanitized = [];
    for (const partnerId of val || []) {
      const normalized = Math.max(0, Math.min(Math.trunc(partnerId), 0xffff));
      sanitized.push(normalized);
      if (sanitized.length >= MAX_MYSTERY_GIFT_PARTNERS) {
        break;
      }
    }
    return sanitized;
  }),
  decorations_received: z.array(z.number()).default([]).transform((val) => {
    const cleaned = new Set<number>();
    for (const decoration of val || new Set()) {
      const decorationValue = Math.max(0, Math.min(Math.trunc(decoration), NUM_NON_TROPHY_DECOS - 1));
      cleaned.add(decorationValue);
    }
    return Array.from(cleaned);
  }),
  trainer_house_flag: z.boolean().default(false),
  partner_name: z.string().default(''),
  partner_trainer_data: z.record(z.string(), z.unknown()).default({}),
  recent_partner_id: z.number().default(0),
});

export type MysteryGiftSaveData = z.infer<typeof MysteryGiftSaveDataSchema>;

export const BattleTowerSaveDataSchema = z.object({
  challenge_state: z.number().default(0),
  save_file_flags: z.number().default(0),
  level_group: z.number().default(0),
  reward_item: z.string().default('POTION'),
  reward_given: z.boolean().default(false),
  gs_ball_flag: z.boolean().default(false),
  explanation_read: z.boolean().default(false),
  quick_saved: z.boolean().default(false),
  beaten_trainers: z.number().default(0),
  trainer_history: z.array(z.number()).default(defaultTrainerHistory),
  record_streaks: z.array(z.number()).default([]),
  record_outcomes: z.array(z.boolean()).default([]),
  record_days: z.array(z.number()).default([]),
  record_state: z.number().default(0),
  record_last_day: z.number().default(-1),
  record_reset_counter: z.number().default(0),
  leaderboard_acknowledged: z.boolean().default(false),
});

export type BattleTowerSaveData = z.infer<typeof BattleTowerSaveDataSchema>;

export const MobileAdapterSaveDataSchema = z.object({
  adapter_status: z.number().default(0),
  adapter_secondary_status: z.number().default(0),
  battle_timer: z.tuple([z.number(), z.number(), z.number()]).default([0, 0, 0]),
  login_password: z.string().default('').transform((val) => val.slice(0, MOBILE_LOGIN_PASSWORD_LENGTH)),
  stadium_flag: z.boolean().default(false),
  last_activity: z.coerce.date().nullable().default(null),
});

export type MobileAdapterSaveData = z.infer<typeof MobileAdapterSaveDataSchema>;

export const PCItemSlotSchema = z.object({
  item: z.string().default(''),
  quantity: z.number().default(0),
});

export type PCItemSlot = z.infer<typeof PCItemSlotSchema>;

export const SRAMSchema = z.object({
  gs_healings: z.number().default(0),
  player_id: z.number().default(0),
  secret_id: z.number().default(0),
  player_name: z.string().default(''),
  player_gender: z.nativeEnum(PlayerGender).default(PlayerGender.MALE),
  player_personality: z.number().default(0),
  rival_name: z.string().default(''),
  money: z.number().default(0),
  coins: z.number().default(0),
  lucky_id_number: z.number().default(0),
  lucky_number_day: z.number().default(-1),
  options: OptionsSchema.default(() => OptionsSchema.parse({})),
  textbox_frame: z.number().default(0),
  start_time: TimeSchema.default(() => TimeSchema.parse({})),
  game_time_hours: z.number().default(0),
  game_time_minutes: z.number().default(0),
  game_time_seconds: z.number().default(0),
  day_of_week: z.number().default(0),
  party: PartySchema.default(() => PartySchema.parse({})),
  pc_boxes: z.array(BoxSchema).default([]),
  current_pc_box: z.number().default(0),
  items: z.record(z.string(), z.number()).default({}),
  key_items: z.record(z.string(), z.number()).default({}),
  balls: z.record(z.string(), z.number()).default({}),
  tm_hm: z.array(z.number()).default([]),
  pc_items: z.array(PCItemSlotSchema).default([]),
  mailbox: z.array(MailMessageSchema).default([]),
  hall_of_fame: z.array(HallOfFameEntrySchema).default([]),
  link_battle_stats: LinkBattleStatsSchema.default(() => LinkBattleStatsSchema.parse({})),
  link_battle_records: z.array(LinkBattleRecordSchema).default(defaultLinkBattleRecords),
  badges: BadgesSchema.default(() => BadgesSchema.parse({})),
  pokedex_seen: z.array(z.number()).default(Array(Math.ceil(NUM_POKEMON / 8)).fill(0)),
  pokedex_owned: z.array(z.number()).default(Array(Math.ceil(NUM_POKEMON / 8)).fill(0)),
  pokedex_caught: z.union([z.set(z.number()), z.array(z.number())]).transform((val) => (val instanceof Set ? val : new Set(val))).default(() => new Set<number>()),
  johto_pokedex: z.boolean().default(false),
  unown_dex: z.boolean().default(false),
  last_spawn_map_group: z.number().default(0),
  last_spawn_map_number: z.number().default(0),
  last_pokecenter_map_name: z.string().default(''),
  last_pokecenter_map_group: z.number().default(0),
  last_pokecenter_map_number: z.number().default(0),
  last_pokecenter_player_x: z.number().default(0),
  last_pokecenter_player_y: z.number().default(0),
  last_pokecenter_backup_map_group: z.number().default(0),
  last_pokecenter_backup_map_number: z.number().default(0),
  last_pokecenter_coordinate_units: z.string().default('block'),
  phone_numbers: z.array(z.string()).default([]),
  dst: z.boolean().default(false),
  mystery_gift_unlocked: z.boolean().default(false),
  moms_money: z.number().default(0),
  mom_saving_active: z.boolean().default(false),
  mom_saving_some_money: z.boolean().default(false),
  current_date: DateSchema.optional(),
  mystery_gift: MysteryGiftSaveDataSchema.default(() => MysteryGiftSaveDataSchema.parse({})),
  rtc_anchor: DateSchema.optional(),
  rtc_status_flags: z.number().default(0),
  mobile_adapter: MobileAdapterSaveDataSchema.default(() => MobileAdapterSaveDataSchema.parse({})),
  battle_tower: BattleTowerSaveDataSchema.default(() => BattleTowerSaveDataSchema.parse({})),
  day_care: DayCareSchema.default(() => DayCareSchema.parse({})),
  event_flags: z.record(z.string(), z.boolean()).default({}),
  script_memory: ScriptMemorySchema,
  photo_album: z.array(PhotoSnapshotSchema).default([]),
}).transform((data) => {
    const clampMoney = (value: unknown) => {
        const amount = Number(value) || 0;
        return Math.max(0, Math.min(MAX_MONEY, amount));
    };
    return {
        ...data,
        money: clampMoney(data.money),
        moms_money: clampMoney(data.moms_money),
    };
});

export type SRAM = z.infer<typeof SRAMSchema>;
