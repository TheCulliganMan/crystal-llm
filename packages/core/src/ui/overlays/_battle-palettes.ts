export const DMG_BGP_DEFAULT = 0xe4;
export const DMG_OBP0_ASSIGNED = 0xe0;
export const DMG_OBP_DEFAULT = 0xe4;

export const assign_dmg_palettes = (paletteState: Record<string, number | null>): void => {
  paletteState.bgp = DMG_BGP_DEFAULT;
  paletteState.obp0 = DMG_OBP0_ASSIGNED;
  paletteState.obp1 = DMG_OBP_DEFAULT;
};

export const revert_to_dmg_defaults = (paletteState: Record<string, number | null>): void => {
  paletteState.bgp = DMG_BGP_DEFAULT;
  paletteState.obp0 = DMG_OBP_DEFAULT;
  paletteState.obp1 = DMG_OBP_DEFAULT;
};

export const is_dmg_baseline_palette_state = (paletteState: Record<string, number | null>): boolean => {
  const bgp = paletteState.bgp;
  const obp0 = paletteState.obp0;
  const obp1 = paletteState.obp1;
  if (bgp !== DMG_BGP_DEFAULT || obp1 !== DMG_OBP_DEFAULT) {
    return false;
  }
  return obp0 === DMG_OBP0_ASSIGNED || obp0 === DMG_OBP_DEFAULT;
};
