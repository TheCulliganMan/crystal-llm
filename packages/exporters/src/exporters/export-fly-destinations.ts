import { _FLY_DESTINATIONS } from "@pokecrystal/core/engine/world/overworld/constants";

export type ExportedFlyDestination = {
  flypoint_flag: string;
  destination_spawn_identifier: number;
  label: string;
};

export type ExportedFlyDestinationTable = Record<string, ExportedFlyDestination>;

export function exportFlyDestinations(): ExportedFlyDestinationTable {
  const destinations: ExportedFlyDestinationTable = {};
  for (const [flypointFlag, landmark, spawn] of _FLY_DESTINATIONS) {
    if (destinations[flypointFlag]) {
      throw new Error(`Duplicate Fly destination flag ${flypointFlag}`);
    }
    destinations[flypointFlag] = {
      flypoint_flag: flypointFlag,
      destination_spawn_identifier: spawn,
      label: landmark,
    };
  }
  return destinations;
}
