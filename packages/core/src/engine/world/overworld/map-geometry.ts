// ASM mapping: pokecrystal_disassembly/engine/overworld/map_objects.asm (LoadMapAttributes / FillMapConnections).
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { MapAttributes } from "@pokecrystal/core/core/models/map";
import { getMapMetadataByName } from "@pokecrystal/core/engine/world/maps";
import { OverworldMap } from "./overworld-map";
import { normalizeMapKey } from "./overworld-base";
import { METATILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import type {
  OverworldTilesetLike,
  RenderMetatileOptions,
} from "@pokecrystal/core/engine/world/overworld/tileset-types";

export function build_overworld_map(
  map_name: string,
  attributes: MapAttributes,
  { data_loader }: { data_loader: DataLoader }
): OverworldMap {
  const provisional_map = new OverworldMap(
    map_name,
    attributes.width,
    attributes.height,
    attributes.blocks_label ?? null
  );
  const metatiles = Array.from(provisional_map.metatileIds);
  const total_blocks = metatiles.length;

  const [width, height] = deduce_dimensions({
    map_name,
    attributes,
    data_loader,
    total_blocks,
  });

  provisional_map.setDimensions(width, height);

  const total_expected = width * height;
  if (metatiles.length < total_expected) {
    const border_block = (attributes.border_block ?? 0) & 0xff;
    while (metatiles.length < total_expected) {
      metatiles.push(border_block);
    }
  } else if (metatiles.length > total_expected) {
    metatiles.length = total_expected;
  }

  provisional_map.metatileIds = metatiles;
  return provisional_map;
}

export function create_map_surface(
  map_obj: OverworldMap,
  tileset: OverworldTilesetLike,
  options: Pick<RenderMetatileOptions, "vram"> = {},
): InstanceType<typeof gameEngine.Surface> {
  const map_width_px = map_obj.width * METATILE_SIZE;
  const map_height_px = map_obj.height * METATILE_SIZE;
  const surface = new gameEngine.Surface(map_width_px, map_height_px);
  render_map_onto_surface(map_obj, tileset, surface, options);
  return surface;
}

export function create_priority_surface(
  map_obj: OverworldMap,
  tileset: OverworldTilesetLike
): InstanceType<typeof gameEngine.Surface> {
  const map_width_px = map_obj.width * METATILE_SIZE;
  const map_height_px = map_obj.height * METATILE_SIZE;
  const surface = new gameEngine.Surface(map_width_px, map_height_px);
  render_priority_onto_surface(map_obj, tileset, surface);
  return surface;
}

export function render_map_onto_surface(
  map_obj: OverworldMap,
  tileset: OverworldTilesetLike,
  surface: InstanceType<typeof gameEngine.Surface>,
  options: RenderMetatileOptions = {},
): void {
  const render = tileset.renderMetatile;
  const { vram = null } = options;
  let index = 0;
  for (let y = 0; y < map_obj.height; y += 1) {
    for (let x = 0; x < map_obj.width; x += 1) {
      const metatile_id = map_obj.metatileIds[index];
      render(metatile_id, surface, x * METATILE_SIZE, y * METATILE_SIZE, { vram });
      index += 1;
    }
  }
}

export function render_priority_onto_surface(
  map_obj: OverworldMap,
  tileset: OverworldTilesetLike,
  surface: InstanceType<typeof gameEngine.Surface>
): void {
  const render = tileset.renderPriorityMetatile;
  let index = 0;
  for (let y = 0; y < map_obj.height; y += 1) {
    for (let x = 0; x < map_obj.width; x += 1) {
      const metatile_id = map_obj.metatileIds[index];
      render(metatile_id, surface, x * METATILE_SIZE, y * METATILE_SIZE);
      index += 1;
    }
  }
}

function deduce_dimensions({
  map_name,
  attributes,
  data_loader,
  total_blocks,
}: {
  map_name: string;
  attributes: MapAttributes;
  data_loader: DataLoader;
  total_blocks: number;
}): [number, number] {
  const base_width = attributes.width;
  const base_height = attributes.height;
  if (base_width <= 0 || base_height <= 0) {
    throw new Error(`Invalid map dimensions for ${map_name}: ${base_width}x${base_height}`);
  }

  try {
    const metadata = getMapMetadataByName(map_name);
    if (metadata && metadata.width > 0 && metadata.height > 0 && metadata.width * metadata.height === total_blocks) {
      return [metadata.width, metadata.height];
    }
  } catch {
    // Fall back to attribute/dimension heuristics when metadata is unavailable.
  }

  const normalised_key = normalizeMapKey(map_name);
  const mapDimensions = data_loader.map_dimensions;
  const dimensions = mapDimensions.get(normalised_key) ?? null;
  if (dimensions) {
    const dim_w = Number(dimensions.width ?? 0);
    const dim_h = Number(dimensions.height ?? 0);
    if (dim_w > 0 && dim_h > 0 && dim_w * dim_h === total_blocks) {
      return [dim_w, dim_h];
    }
  }

  if (total_blocks % base_width === 0) {
    return [base_width, Math.max(1, Math.trunc(total_blocks / base_width))];
  }

  const divisors: number[] = [];
  for (let candidate = 1; candidate <= total_blocks; candidate += 1) {
    if (total_blocks % candidate === 0) {
      divisors.push(candidate);
    }
  }
  if (divisors.length === 0) {
    throw new Error(`Unable to compute map dimensions for ${map_name}; blocks=${total_blocks}`);
  }
  let best = divisors[0];
  let best_delta = Math.abs(best - base_width);
  for (const candidate of divisors) {
    const delta = Math.abs(candidate - base_width);
    if (delta < best_delta) {
      best = candidate;
      best_delta = delta;
    }
  }
  return [best, Math.trunc(total_blocks / best)];
}
