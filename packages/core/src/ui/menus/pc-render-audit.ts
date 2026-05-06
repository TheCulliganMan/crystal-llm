// ASM audit helper: render representative PC states from pokecrystal_disassembly/engine/events/pokecenter_pc.asm
// and pokecrystal_disassembly/engine/pokemon/bills_pc*.asm.
import { BoxSchema, setSlot } from "../../core/models/box";
import type { PokemonSpecies } from "../../core/models";
import { createInitialGameState } from "../../core/state";
import { Ability, EggGroup, GenderRatio, GrowthRate, PokemonType } from "../../core/enums";
import { createPokemon } from "../../engine/systems/pokemon";
import { toPokemon } from "../../core/models/pokemon";
import { BaseUI, type BaseFontRenderer } from "../base-ui";
import { Rect, Surface } from "../surface";
import { BitmapFont } from "../text/bitmap-font";
import { PCHubMenu } from "./pc-hub-prompt";
import { MailboxMenu, HallOfFameViewer } from "./pc-auxiliary";
import { PlayerPCMenu, type PlayerPCUI } from "./pc-player-menu";
import { PokemonPCMenu, type PCFont, type SupportsPokemonPCUI } from "./pc-components";
import { PokemonCenterPCSession } from "./pc-menu";
import { pcHubEntries } from "../../engine/world/special-events/pc-helpers";
import type { MailMessage } from "../../core/mail";
import type { MenuUI } from "./types";
import { renderBillsPcTopMenu } from "./pc-asm-render";

const SCREEN_WIDTH = 160;
const SCREEN_HEIGHT = 144;

const DEFAULT_BASE_STATS = {
  hp: 20,
  attack: 10,
  defense: 10,
  speed: 10,
  special_attack: 10,
  special_defense: 10,
};

export type PCAuditFrame = {
  slug: string;
  title: string;
  surface: Surface;
};

export type PCAuditSummary = {
  frameCount: number;
  slugs: string[];
};

const speciesCache = new Map<string, PokemonSpecies>();

const ensureSpecies = (id: string): PokemonSpecies => {
  const upperId = id.toUpperCase();
  const cached = speciesCache.get(upperId);
  if (cached) {
    return cached;
  }
  const species: PokemonSpecies = {
    id: upperId,
    int_id: 0,
    base_stats: DEFAULT_BASE_STATS,
    type1: PokemonType.NORMAL,
    type2: PokemonType.NONE,
    catch_rate: 45,
    base_exp: 64,
    item1: undefined,
    item2: undefined,
    gender_ratio: GenderRatio.GENDER_F50,
    unknown1: 0,
    step_cycles_to_hatch: 5120,
    unknown2: 0,
    growth_rate: GrowthRate.GROWTH_MEDIUM_FAST,
    egg_group1: EggGroup.EGG_MONSTER,
    egg_group2: EggGroup.EGG_MONSTER,
    tmhm_learnset: [],
    ability: Ability.NONE,
    pic_size: 0,
    front_pic: 0,
    back_pic: 0,
    evolutions: null,
    weight: 0,
  };
  speciesCache.set(upperId, species);
  return species;
};

class HeadlessPCAuditUI extends BaseUI {
  public readonly auditFont: BitmapFont;
  public readonly font: BaseFontRenderer;

  private constructor(font: BitmapFont) {
    super(SCREEN_WIDTH, SCREEN_HEIGHT, 1);
    this.auditFont = font;
    this.font = {
      renderText: font.renderText.bind(font),
      render_text: (text, x, y, surface, options) => {
        if (typeof options === "boolean") {
          font.renderText(text, x, y, surface, options);
          return;
        }
        font.render_text(text, x, y, surface, options);
      },
      fontTiles: font.fontTiles,
      font_tiles: font.font_tiles,
      paletteVariants: font.paletteVariants.bind(font),
      getCharTile: font.getCharTile.bind(font),
      get_char_tile: font.getCharTile.bind(font),
    };
  }

  static async create(): Promise<HeadlessPCAuditUI> {
    const font = new BitmapFont();
    await font.load();
    const ui = new HeadlessPCAuditUI(font);
    await ui.preloadWindowFrames([ui.defaultFrameId]);
    return ui;
  }

  protected createScreenSurface(): Surface {
    return new Surface(this.screenWidth, this.screenHeight);
  }

  update(): void {
    return;
  }

  drawSprite(speciesId: string, x: number, y: number, _spriteType: string = "pokemon", frame: number = 0): void {
    const sprite = this.getPokemonFrontSurface(speciesId, frame);
    if (!sprite) {
      return;
    }
    this.screen.blit(sprite, [x, y]);
  }

  drawRect(color: [number, number, number], rect: Rect, _width: number = 1): void {
    this.screen.fill([color[0], color[1], color[2], 255], rect);
  }

  asHubUi(): ConstructorParameters<typeof PCHubMenu>[0] {
    return {
      screen: this.screen,
      screenWidth: this.screenWidth,
      screenHeight: this.screenHeight,
      font: this.font,
      drawWindow: (surface, x, y, width, height, options) => {
        this.drawWindow(surface, x, y, width, height, {
          frameId: options?.frameId ?? undefined,
          fill: options?.fill ?? undefined,
          zIndex: options?.zIndex ?? undefined,
          record: options?.record ?? undefined,
        });
      },
      update: this.update.bind(this),
    };
  }

  asPokemonPcUi(): SupportsPokemonPCUI {
    return {
      screen: this.screen,
      font: this.font as PCFont,
      drawWindow: (surface, x, y, width, height, options) => {
        this.drawWindow(surface, x, y, width, height, {
          frameId: options?.frameId ?? undefined,
          fill: options?.fill ?? undefined,
        });
      },
      drawSprite: this.drawSprite.bind(this),
      drawRect: this.drawRect.bind(this),
      update: this.update.bind(this),
    };
  }

  asPlayerPcUi(): PlayerPCUI {
    return {
      screen: this.screen,
      font: this.font as PlayerPCUI["font"],
      drawWindow: (surface, x, y, width, height, options) => {
        this.drawWindow(surface, x, y, width, height, {
          fill: options?.fill ?? undefined,
        });
      },
      update: this.update.bind(this),
    };
  }

  asMenuUi(): MenuUI {
    return {
      screen: this.screen,
      font: this.font as MenuUI["font"],
      tileSize: this.tileSize,
      drawWindow: (surface, x, y, width, height, options) => {
        this.drawWindow(surface, x, y, width, height, {
          frameId: options?.frameId ?? undefined,
          fill: options?.fill ?? undefined,
          zIndex: options?.zIndex ?? undefined,
        });
      },
      update: this.update.bind(this),
      eventQueue: this.eventQueue,
    };
  }
}

const snapshotScreen = (screen: Surface): Surface => {
  const snapshot = new Surface(SCREEN_WIDTH, SCREEN_HEIGHT);
  snapshot.blit(screen, [0, 0]);
  return snapshot;
};

const seedPartyPokemon = (gameState: ReturnType<typeof createInitialGameState>, speciesId: string): void => {
  gameState.sram.party.pokemon = [toPokemon(createPokemon(gameState, ensureSpecies(speciesId), 5)), null, null, null, null, null];
};

const renderHubFrame = async (): Promise<PCAuditFrame> => {
  const ui = await HeadlessPCAuditUI.create();
  const gameState = createInitialGameState();
  gameState.sram.player_name = "KRIS";
  gameState.sram.johto_pokedex = true;
  gameState.sram.hall_of_fame = [{ team: [{ species: "CYNDAQUIL", nickname: "BLAZE" }] } as never];
  ui.clearScreen();
  const menu = new PCHubMenu(
    ui.asHubUi(),
    pcHubEntries(gameState).map((entry) => entry.label),
    null,
    { promptText: "Access whose PC?" },
  );
  menu.draw();
  return { slug: "hub", title: "PC Hub", surface: snapshotScreen(ui.screen) };
};

const renderBillsBrowseFrame = async (): Promise<PCAuditFrame> => {
  const ui = await HeadlessPCAuditUI.create();
  const gameState = createInitialGameState();
  seedPartyPokemon(gameState, "CYNDAQUIL");
  const box = BoxSchema.parse({ name: "BOX 1" });
  setSlot(box, 0, toPokemon(createPokemon(gameState, ensureSpecies("TOTODILE"), 8)));
  setSlot(box, 1, toPokemon(createPokemon(gameState, ensureSpecies("SQUIRTLE"), 10)));
  gameState.sram.pc_boxes = [box];
  ui.clearScreen();
  const menu = new PokemonPCMenu(ui.asPokemonPcUi(), gameState, null);
  menu.draw();
  return { slug: "bills-browse", title: "Bill PC Browse", surface: snapshotScreen(ui.screen) };
};

const renderBillsTopMenuFrame = async (): Promise<PCAuditFrame> => {
  const ui = await HeadlessPCAuditUI.create();
  renderBillsPcTopMenu(ui, PokemonCenterPCSession.BILL_PC_MENU);
  return { slug: "bills-top-menu", title: "Bill PC Top Menu", surface: snapshotScreen(ui.screen) };
};

const renderBillsActionFrame = async (): Promise<PCAuditFrame> => {
  const ui = await HeadlessPCAuditUI.create();
  const gameState = createInitialGameState();
  seedPartyPokemon(gameState, "CYNDAQUIL");
  const box = BoxSchema.parse({ name: "BOX 1" });
  setSlot(box, 0, toPokemon(createPokemon(gameState, ensureSpecies("TOTODILE"), 8)));
  gameState.sram.pc_boxes = [box];
  ui.clearScreen();
  const menu = new PokemonPCMenu(ui.asPokemonPcUi(), gameState, null);
  const internals = menu as unknown as {
    currentSelectionEntry: () => unknown;
    enterActionMode: (entry: unknown) => void;
    advanceJumptableState: () => void;
  };
  internals.enterActionMode(internals.currentSelectionEntry());
  internals.advanceJumptableState();
  menu.draw();
  return { slug: "bills-actions", title: "Bill PC Actions", surface: snapshotScreen(ui.screen) };
};

const renderBillsDepositFrame = async (): Promise<PCAuditFrame> => {
  const ui = await HeadlessPCAuditUI.create();
  const gameState = createInitialGameState();
  gameState.sram.party.pokemon = [
    toPokemon(createPokemon(gameState, ensureSpecies("CYNDAQUIL"), 5)),
    toPokemon(createPokemon(gameState, ensureSpecies("TOGEPI"), 6)),
    null,
    null,
    null,
    null,
  ];
  const box = BoxSchema.parse({ name: "BOX 1" });
  setSlot(box, 0, toPokemon(createPokemon(gameState, ensureSpecies("TOTODILE"), 8)));
  gameState.sram.pc_boxes = [box];
  ui.clearScreen();
  const menu = new PokemonPCMenu(ui.asPokemonPcUi(), gameState, null);
  menu.showBillAction("deposit");
  menu.draw();
  return { slug: "bills-deposit", title: "Bill PC Deposit", surface: snapshotScreen(ui.screen) };
};

const renderBillsMoveSourceFrame = async (): Promise<PCAuditFrame> => {
  const ui = await HeadlessPCAuditUI.create();
  const gameState = createInitialGameState();
  seedPartyPokemon(gameState, "CYNDAQUIL");
  const box = BoxSchema.parse({ name: "BOX 1" });
  setSlot(box, 0, toPokemon(createPokemon(gameState, ensureSpecies("TOTODILE"), 8)));
  setSlot(box, 1, toPokemon(createPokemon(gameState, ensureSpecies("SQUIRTLE"), 10)));
  gameState.sram.pc_boxes = [box];
  ui.clearScreen();
  const menu = new PokemonPCMenu(ui.asPokemonPcUi(), gameState, null);
  menu.showBillAction("move");
  await menu.startMoveSessionAsync({ confirmationProvider: async () => true });
  menu.draw();
  return { slug: "bills-move-source", title: "Bill PC Move Source", surface: snapshotScreen(ui.screen) };
};

const renderBillsMoveFrame = async (): Promise<PCAuditFrame> => {
  const ui = await HeadlessPCAuditUI.create();
  const gameState = createInitialGameState();
  seedPartyPokemon(gameState, "CYNDAQUIL");
  const box = BoxSchema.parse({ name: "BOX 1" });
  setSlot(box, 0, toPokemon(createPokemon(gameState, ensureSpecies("TOTODILE"), 8)));
  setSlot(box, 1, toPokemon(createPokemon(gameState, ensureSpecies("SQUIRTLE"), 10)));
  gameState.sram.pc_boxes = [box];
  ui.clearScreen();
  const menu = new PokemonPCMenu(ui.asPokemonPcUi(), gameState, null);
  menu.showBillAction("move");
  await menu.startMoveSessionAsync({ confirmationProvider: async () => true });
  const internals = menu as unknown as {
    currentSelectionEntry: () => unknown;
    enterActionMode: (entry: unknown) => void;
    advanceJumptableState: () => void;
    beginMoveSelection: () => void;
  };
  internals.enterActionMode(internals.currentSelectionEntry());
  internals.advanceJumptableState();
  internals.beginMoveSelection();
  menu.draw();
  return { slug: "bills-move-insert", title: "Bill PC Move Insert", surface: snapshotScreen(ui.screen) };
};

const renderPlayerPcFrame = async (): Promise<PCAuditFrame> => {
  const ui = await HeadlessPCAuditUI.create();
  const gameState = createInitialGameState();
  gameState.sram.pc_items = [
    { item: "POTION", quantity: 12 },
    { item: "ANTIDOTE", quantity: 5 },
  ] as never;
  ui.clearScreen();
  const menu = new PlayerPCMenu(ui.asPlayerPcUi(), gameState, undefined, null);
  menu.draw();
  return { slug: "player-pc", title: "Player PC", surface: snapshotScreen(ui.screen) };
};

const renderMailboxFrame = async (): Promise<PCAuditFrame> => {
  const ui = await HeadlessPCAuditUI.create();
  const messages: MailMessage[] = [
    {
      message: "RIVAL wrote you!",
      author: "RIVAL",
      nationality: 0,
      author_id: 1,
      species_id: 0,
      mail_type: 0,
    } as never,
    {
      message: "Mom says hi",
      author: "MOM",
      nationality: 0,
      author_id: 2,
      species_id: 0,
      mail_type: 0,
    } as never,
  ];
  ui.clearScreen();
  new MailboxMenu(ui.asMenuUi(), messages).draw();
  return { slug: "mailbox", title: "Mailbox", surface: snapshotScreen(ui.screen) };
};

const renderHallOfFameFrame = async (): Promise<PCAuditFrame> => {
  const ui = await HeadlessPCAuditUI.create();
  ui.clearScreen();
  new HallOfFameViewer(ui.asMenuUi()).draw(["CYNDAQUIL", "PIDGEY", "TOGEPI"], 0);
  return { slug: "hall-of-fame", title: "Hall Of Fame", surface: snapshotScreen(ui.screen) };
};

export const summarizePcAuditFrames = (frames: PCAuditFrame[]): PCAuditSummary => ({
  frameCount: frames.length,
  slugs: frames.map((frame) => frame.slug),
});

export const renderRepresentativePcAuditFrames = async (): Promise<PCAuditFrame[]> => {
  return [
    await renderHubFrame(),
    await renderBillsTopMenuFrame(),
    await renderBillsBrowseFrame(),
    await renderBillsActionFrame(),
    await renderBillsDepositFrame(),
    await renderBillsMoveSourceFrame(),
    await renderBillsMoveFrame(),
    await renderPlayerPcFrame(),
    await renderMailboxFrame(),
    await renderHallOfFameFrame(),
  ];
};
