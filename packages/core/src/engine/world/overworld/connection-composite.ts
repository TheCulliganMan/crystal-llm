// ASM mapping: pokecrystal_disassembly/engine/overworld/map_objects.asm (LoadConnectionBlocks).
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { MapAttributes } from "@pokecrystal/core/core/models/map";
import { GameState } from "@pokecrystal/core/core/state";
import type { OverworldTilesetLike } from "@pokecrystal/core/engine/world/overworld/tileset-types";
import { METATILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import { OverworldMap } from "./overworld-map";
import { build_overworld_map, create_map_surface, create_priority_surface } from "./map-geometry";

type Surface = InstanceType<typeof gameEngine.Surface>;

export type CompositeResult = {
  surface: Surface;
  priority_surface: Surface;
  origin: [number, number];
  segments: CompositeSegment[];
};

export type CompositeSegment = {
  name: string;
  surface: Surface;
  priority_surface: Surface;
  dest: [number, number];
  map: OverworldMap;
  tileset: OverworldTilesetLike;
};

type SegmentPayload = {
  name: string;
  surface: Surface;
  priority_surface: Surface;
  position: [number, number];
  map: OverworldMap;
  tileset: OverworldTilesetLike;
};

type TilesetResolver = (
  mapName: string,
  attributes: MapAttributes
) => OverworldTilesetLike | Promise<OverworldTilesetLike>;

function offset_pixels(offset_blocks: number): number {
  return offset_blocks * METATILE_SIZE;
}

function segment_position(
  direction: string,
  offset_px: number,
  base_size: [number, number],
  segment_size: [number, number]
): [number, number] {
  const [base_w, base_h] = base_size;
  const [seg_w, seg_h] = segment_size;
  switch (direction) {
    case "north":
      return [offset_px, -seg_h];
    case "south":
      return [offset_px, base_h];
    case "west":
      return [-seg_w, offset_px];
    case "east":
      return [base_w, offset_px];
    default:
      throw new Error(`Unsupported connection direction '${direction}'`);
  }
}

function outline_bounds(
  base_rect: InstanceType<typeof gameEngine.Rect>,
  segments: Array<[Surface, [number, number]]>
): [number, number, number, number] {
  let min_x = base_rect.left;
  let min_y = base_rect.top;
  let max_x = base_rect.right;
  let max_y = base_rect.bottom;

  for (const [surface, [pos_x, pos_y]] of segments) {
    const [width, height] = surface.get_size();
    min_x = Math.min(min_x, pos_x);
    min_y = Math.min(min_y, pos_y);
    max_x = Math.max(max_x, pos_x + width);
    max_y = Math.max(max_y, pos_y + height);
  }

  return [min_x, min_y, max_x, max_y];
}

function fill_border(
  surface: Surface,
  border_block: number,
  tileset: OverworldTilesetLike,
  { priority_surface = null }: { priority_surface?: Surface | null } = {}
): void {
  const render = tileset.renderMetatile;
  const width_metatiles = Math.trunc(surface.get_width() / METATILE_SIZE);
  const height_metatiles = Math.trunc(surface.get_height() / METATILE_SIZE);
  const metatile_id = border_block & 0xff;
  for (let y = 0; y < height_metatiles; y += 1) {
    for (let x = 0; x < width_metatiles; x += 1) {
      render(metatile_id, surface, x * METATILE_SIZE, y * METATILE_SIZE, { priority_surface });
    }
  }
}

async function load_connection_surface(
  target_map: string,
  {
    attributes,
    data_loader,
    resolve_tileset,
  }: {
    attributes: MapAttributes;
    data_loader: DataLoader;
    resolve_tileset?: TilesetResolver | null;
  }
): Promise<[OverworldMap, OverworldTilesetLike, Surface, Surface]> {
  let tileset: OverworldTilesetLike | null = null;
  if (resolve_tileset) {
    tileset = await Promise.resolve(resolve_tileset(target_map, attributes));
  }
  if (!tileset) {
    throw new Error(`Missing tileset resolver for connection target '${target_map}'.`);
  }
  const overworld_map = build_overworld_map(target_map, attributes, { data_loader });
  const surface = create_map_surface(overworld_map, tileset);
  const priority_surface = create_priority_surface(overworld_map, tileset);
  return [overworld_map, tileset, surface, priority_surface];
}

export async function build_connection_composite({
  map_name,
  map_attributes,
  base_surface,
  base_priority_surface,
  base_tileset,
  data_loader,
  game_state = null,
  resolve_tileset,
}: {
  map_name: string;
  map_attributes: MapAttributes;
  base_surface: Surface;
  base_priority_surface: Surface | null;
  base_tileset: OverworldTilesetLike;
  data_loader: DataLoader;
  game_state?: GameState | null;
  resolve_tileset?: TilesetResolver | null;
}): Promise<CompositeResult> {
  void game_state;

  const [base_width, base_height] = base_surface.get_size();
  const segments: Array<[Surface, [number, number]]> = [];
  const segment_payloads: SegmentPayload[] = [];

  for (const connection of map_attributes.connections) {
    const direction = connection.direction.toLowerCase();
    const target_name = connection.target_map;
    const target_attributes = data_loader.map_attributes.get(target_name);
    if (!target_attributes) {
      throw new Error(
        `Missing map attributes for connection target '${target_name}' referenced by ${map_name}`
      );
    }
    const [ow_map, seg_tileset, surface, priority_surface] = await load_connection_surface(
      target_name,
      {
        attributes: target_attributes,
        data_loader,
        resolve_tileset,
      }
    );
    const offset_px = offset_pixels(connection.offset);
    const position = segment_position(direction, offset_px, [base_width, base_height], surface.get_size());
    segments.push([surface, position]);
    segment_payloads.push({
      surface,
      priority_surface,
      position,
      map: ow_map,
      tileset: seg_tileset,
      name: target_name,
    });
  }

  const base_rect = new gameEngine.Rect(0, 0, base_width, base_height);
  const [min_x, min_y, max_x, max_y] = outline_bounds(base_rect, segments);
  const width = max_x - min_x;
  const height = max_y - min_y;
  if (width <= 0 || height <= 0) {
    throw new Error(
      `Computed invalid composite bounds for ${map_name}: ${width}x${height} (min=${min_x},${min_y} max=${max_x},${max_y})`
    );
  }

  const composite = new gameEngine.Surface(width, height);
  const priority_composite = new gameEngine.Surface(width, height);
  fill_border(composite, map_attributes.border_block, base_tileset, { priority_surface: priority_composite });

  const origin_x = -min_x;
  const origin_y = -min_y;
  const base_dest: [number, number] = [origin_x, origin_y];
  composite.blit(base_surface, base_dest);
  if (base_priority_surface) {
    priority_composite.blit(base_priority_surface, base_dest);
  }

  const segment_meta: CompositeSegment[] = [];
  for (const segment of segment_payloads) {
    const dest: [number, number] = [segment.position[0] - min_x, segment.position[1] - min_y];
    composite.blit(segment.surface, dest);
    priority_composite.blit(segment.priority_surface, dest);
    segment_meta.push({
      name: segment.name,
      surface: segment.surface,
      priority_surface: segment.priority_surface,
      dest,
      map: segment.map,
      tileset: segment.tileset,
    });
  }

  return {
    surface: composite,
    priority_surface: priority_composite,
    origin: [origin_x, origin_y],
    segments: segment_meta,
  };
}
