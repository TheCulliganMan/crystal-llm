import type { BattleUIRoot } from './battle-ui-state';
import type { MenuUI, FontRenderer } from '../menus/types';
import type { PokemonMenuUI } from '../menus/pokemon-menu';

const buildFontRenderer = (font?: BattleUIRoot['font']): FontRenderer => {
  if (!font) {
    throw new Error('Battle UI is missing its font renderer.');
  }
  const renderFn = font.renderText ?? font.render_text;
  if (!renderFn) {
    throw new Error('Battle UI font renderer has no renderText implementation.');
  }
  return {
    renderText: (text, x, y, surface, options) => {
      renderFn.call(font, text, x, y, surface, options);
    },
    paletteVariants: font.paletteVariants,
    fontTiles: font.fontTiles ?? font.font_tiles,
    font_tiles: font.font_tiles,
  };
};

export const createBagMenuUI = (ui: BattleUIRoot): MenuUI => ({
  screen: ui.screen,
  tileSize: ui.tileSize,
  font: buildFontRenderer(ui.font),
  drawWindow: ui.drawWindow.bind(ui),
  eventQueue: ui.eventQueue,
  get_context_palette: ui.get_context_palette?.bind(ui),
  getContextPalette: ui.getContextPalette?.bind(ui),
  update: ui.update.bind(ui),
  playCry: ui.playCry?.bind(ui),
});

const buildPokemonMenuFont = (font?: BattleUIRoot['font']): PokemonMenuUI["font"] => {
  if (!font) {
    return {};
  }
  return {
    paletteVariants: font.paletteVariants,
    fontTiles: font.fontTiles ?? font.font_tiles,
    font_tiles: font.font_tiles,
  };
};

export const createPokemonMenuUI = (ui: BattleUIRoot): PokemonMenuUI => ({
  screen: ui.screen,
  font: buildPokemonMenuFont(ui.font),
});
