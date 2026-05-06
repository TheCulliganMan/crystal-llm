// ASM mapping: pokecrystal_disassembly/engine/overworld/overworld.asm (CheckFacingTile / CollisionCheck).
import { FacingDirection, PlayerState } from "@pokecrystal/core/core/enums/overworld";
import { describeCollision, Terrain, CollisionAttributes } from "./collision-data";
import { isPermissionPassable } from "./collision-rules";
import { OverworldMap } from "./overworld-map";
import { Tileset } from "@pokecrystal/core/core/tileset-data";
import { METATILE_SIZE } from "@pokecrystal/core/engine/world/tile";

type PixelRect = [number, number, number, number];

export class CollisionCell {
  public readonly x: number;
  public readonly y: number;
  public readonly metatile_id: number;
  public readonly permissions: [number, number];
  public readonly terrains: [Terrain, Terrain];
  public readonly raw_expressions: [string, string];
  public readonly talk: boolean;
  public readonly passable: boolean;
  public readonly quadrants: [CellQuadrant, CellQuadrant, CellQuadrant, CellQuadrant];

  constructor(options: {
    x: number;
    y: number;
    metatile_id: number;
    permissions: [number, number];
    terrains: [Terrain, Terrain];
    raw_expressions: [string, string];
    talk: boolean;
    passable: boolean;
    quadrants: [CellQuadrant, CellQuadrant, CellQuadrant, CellQuadrant];
  }) {
    this.x = options.x;
    this.y = options.y;
    this.metatile_id = options.metatile_id;
    this.permissions = options.permissions;
    this.terrains = options.terrains;
    this.raw_expressions = options.raw_expressions;
    this.talk = options.talk;
    this.passable = options.passable;
    this.quadrants = options.quadrants;
  }

  get pixel_rect(): PixelRect {
    return [this.x * METATILE_SIZE, this.y * METATILE_SIZE, METATILE_SIZE, METATILE_SIZE];
  }

  get permission_summary(): string {
    return this.permissions.map((value) => value.toString(16).padStart(2, "0").toUpperCase()).join("/");
  }

  get terrain_summary(): string {
    return this.terrains.map((terrain) => terrain).join("/");
  }

  get expression_summary(): string {
    return this.raw_expressions.join(" | ");
  }
}

export class CellQuadrant {
  public readonly index: number;
  public readonly attributes: CollisionAttributes;

  constructor(index: number, attributes: CollisionAttributes) {
    this.index = index;
    this.attributes = attributes;
  }

  get permission(): number {
    return this.attributes.value;
  }

  get terrain(): Terrain {
    return this.attributes.terrain;
  }

  get talk(): boolean {
    return this.attributes.talk;
  }

  get raw_expression(): string {
    return this.attributes.raw_expr;
  }

  public is_passable_for_facing(player_state: PlayerState, facing: FacingDirection): boolean {
    return isPermissionPassable(this.permission, facing, player_state);
  }
}

function* iter_metatiles(map_data: OverworldMap): IterableIterator<[number, number, number]> {
  for (let y = 0; y < map_data.height; y += 1) {
    for (let x = 0; x < map_data.width; x += 1) {
      yield [x, y, map_data.getMetatileAt(x, y)];
    }
  }
}

export function build_collision_grid(
  map_data: OverworldMap,
  tileset: Tileset,
  facing: FacingDirection,
  player_state: PlayerState
): CollisionCell[] {
  const cells: CollisionCell[] = [];

  for (const [x, y, metatile_id] of iter_metatiles(map_data)) {
    if (metatile_id < 0 || metatile_id >= tileset.metatiles.length) {
      throw new Error(
        `Metatile id ${metatile_id} at (${x}, ${y}) exceeds tileset bounds for '${tileset.tilesetName}'.`
      );
    }
    const metatile = tileset.metatiles[metatile_id];
    const quadrants: CellQuadrant[] = [];
    for (let index = 0; index < 4; index += 1) {
      const value = metatile.collision[index];
      const quadrant_attrs = describeCollision(value);
      quadrants.push(new CellQuadrant(index, quadrant_attrs));
    }

    const active_indices = FacingDirection.quadrantIndices(facing);
    const permissions: [number, number] = [
      metatile.collision[active_indices[0]],
      metatile.collision[active_indices[1]],
    ];
    const terrains: [Terrain, Terrain] = [
      quadrants[active_indices[0]].terrain,
      quadrants[active_indices[1]].terrain,
    ];
    const raw_expressions: [string, string] = [
      quadrants[active_indices[0]].raw_expression,
      quadrants[active_indices[1]].raw_expression,
    ];
    const talk = active_indices.some((index: number) => quadrants[index].talk);
    const passable = active_indices.every((index: number) =>
      quadrants[index].is_passable_for_facing(player_state, facing)
    );

    cells.push(
      new CollisionCell({
        x,
        y,
        metatile_id,
        permissions,
        terrains,
        raw_expressions,
        talk,
        passable,
        quadrants: quadrants as [CellQuadrant, CellQuadrant, CellQuadrant, CellQuadrant],
      })
    );
  }

  return cells;
}
