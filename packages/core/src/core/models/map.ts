import { z } from 'zod';

export const MapConnection = z.object({
  direction: z.string(),
  target_map: z.string(),
  offset: z.number(),
});
export type MapConnection = z.infer<typeof MapConnection>;

export const MapAttributes = z.object({
  tileset_name: z.string(),
  border_block: z.number(),
  width: z.number(),
  height: z.number(),
  connections: z.array(MapConnection).default([]),
  time_of_day: z.string().nullable().default(null),
  phone_service: z.number().default(0),
  phone_flag: z.boolean().default(false),
  environment: z.string().nullable().default(null),
  location: z.string().nullable().default(null),
  music: z.string().nullable().default(null),
  palette: z.string().nullable().default(null),
  fishing_group: z.string().nullable().default(null),
  map_constant: z.string().nullable().default(null),
  map_group_constant: z.string().nullable().default(null),
  blocks_label: z.string().nullable().default(null),
  map_scripts_label: z.string().nullable().default(null),
  map_events_label: z.string().nullable().default(null),
  connection_flags: z.string().nullable().default(null),
});
export type MapAttributes = z.infer<typeof MapAttributes>;

export const WarpEvent = z.object({
  index: z.number(),
  x: z.number(),
  y: z.number(),
  target_map_constant: z.string(),
  target_map: z.string(),
  target_warp_id: z.number(),
});
export type WarpEvent = z.infer<typeof WarpEvent>;

export const CoordEvent = z.object({
  x: z.number(),
  y: z.number(),
  scene_id: z.string(),
  script_name: z.string(),
});
export type CoordEvent = z.infer<typeof CoordEvent>;

export const BackgroundEvent = z.object({
  x: z.number(),
  y: z.number(),
  event_type: z.string(),
  script: z.string(),
});
export type BackgroundEvent = z.infer<typeof BackgroundEvent>;

export const MapEvents = z.object({
  warps: z.array(WarpEvent).default([]),
  coord_events: z.array(CoordEvent).default([]),
  bg_events: z.array(BackgroundEvent).default([]),
});
export type MapEvents = z.infer<typeof MapEvents>;

export const RoamingPokemon = z.object({
  species: z.string().default(''),
  level: z.number().default(0),
  map_group: z.number().default(-1),
  map_number: z.number().default(-1),
  hp: z.number().default(0),
  dvs: z.number().default(0),
});
export type RoamingPokemon = z.infer<typeof RoamingPokemon>;

export const ObjectEvent = z.object({
  sprite: z.string(),
  x: z.number(),
  y: z.number(),
  spritemovedata: z.string(),
  move_range_x: z.number(),
  move_range_y: z.number(),
  hram_x: z.number(),
  hram_y: z.number(),
  pal: z.number(),
  object_type: z.string(),
  radius: z.number(),
  script: z.string(),
  label: z.string().optional(),
  event_flag: z.string(),
  object_identifier: z.string().nullable().default(null),
  sightline_direction_override: z.string().nullable().default(null),
});
export type ObjectEvent = z.infer<typeof ObjectEvent>;
