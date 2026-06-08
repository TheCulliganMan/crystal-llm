import { createInitialGameState, type GameState } from "@pokecrystal/core/core/state";
import { BoxSchema, formatDefaultBoxName, type Pokemon, type PokemonSpecies } from "@pokecrystal/core/core/models";
import { setSlot } from "@pokecrystal/core/core/models/box";
import { MailMessageSchema } from "@pokecrystal/core/core/mail";
import { Ability, EggGroup, GenderRatio, GrowthRate, PokemonType } from "@pokecrystal/core/core/enums";
import { Surface } from "@pokecrystal/core/ui/surface";
import { BaseUI } from "@pokecrystal/core/ui/base-ui";
import { CompositeUI, type CompositeChild } from "@pokecrystal/core/ui/composite-ui";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { BitmapFont } from "@pokecrystal/core/ui/text/bitmap-font";
import { SelectionPrompt } from "@pokecrystal/core/ui/text/prompts";
import { createPokemon } from "@pokecrystal/core/engine/systems/pokemon";
import { toPokemon } from "@pokecrystal/core/core/models/pokemon";
import { getDebugLogEntries } from "@pokecrystal/core/core/debug-log";
import { MAX_PC_ITEMS, MAX_PC_ITEM_QUANTITY } from "@pokecrystal/core/core/constants";
import { hall_of_fame_terminal } from "@pokecrystal/core/engine/world/special-events/pc";
import { Event } from "@pokecrystal/core/engine/events/events";
import { FieldDialogueManager } from "@pokecrystal/core/ui/text/dialogue";
import { PCHubMenu } from "./pc-hub-prompt";
import { PokemonPCMenu, type PCMenuActionResult, type PCFont, type SupportsPokemonPCUI } from "./pc-components";
import { MailboxMenu } from "./pc-auxiliary";
import { PlayerPCMenu, PlayerPCMenuActionResult } from "./pc-player-menu";
import { PokemonCenterPCSession } from "./pc-menu";

jest.setTimeout(20000);

type BillResult = {
  actions: PCMenuActionResult[];
  party: Array<Record<string, unknown> | null>;
  boxes: Record<string, unknown>[];
};

type PlayerPCResult = {
  actions: PlayerPCMenuActionResult[];
  pc_items: Record<string, unknown>[];
};

type MailboxResult = {
  messages: string[];
  actions: Array<{ action?: string; status?: string; message?: string }>;
};

type SessionResult = {
  bill?: BillResult;
  player_pc?: PlayerPCResult;
  mailbox?: MailboxResult;
  selection_name?: string;
  result_text?: string;
};

const requireBillResult = (result: SessionResult): BillResult => {
  if (!result.bill) {
    throw new Error("Expected bill summary");
  }
  return result.bill;
};

const requirePlayerResult = (result: SessionResult): PlayerPCResult => {
  if (!result.player_pc) {
    throw new Error("Expected player PC summary");
  }
  return result.player_pc;
};

const requireMailboxResult = (result: SessionResult): MailboxResult => {
  if (!result.mailbox) {
    throw new Error("Expected mailbox summary");
  }
  return result.mailbox;
};

const DEFAULT_BASE_STATS = {
  hp: 20,
  attack: 10,
  defense: 10,
  speed: 10,
  special_attack: 10,
  special_defense: 10,
};

const speciesCache = new Map<string, PokemonSpecies>();
let fontProxy: PCFont;

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

const makePokemon = (gameState: GameState, speciesId: string, level = 5): Pokemon =>
  toPokemon(createPokemon(gameState, ensureSpecies(speciesId), level));

const seedParty = (gameState: GameState, lead: Pokemon, extra?: Pokemon | null) => {
  gameState.sram.party.pokemon = [lead, extra ?? null, null, null, null, null];
};

const makeBox = (name = formatDefaultBoxName(0)) => BoxSchema.parse({ name });

const makeMenuUi = (): SupportsPokemonPCUI => {
  const screen = new Surface(160, 144);
  return {
    screen,
    font: fontProxy,
    drawWindow: jest.fn(),
    drawSprite: jest.fn(),
    drawRect: jest.fn(),
    update: jest.fn(),
  };
};

const makeInteractiveMenuUi = () => {
  const ui = makeMenuUi() as SupportsPokemonPCUI & {
    screenWidth: number;
    screenHeight: number;
    eventQueue: ReturnType<typeof gameEngine.event.createQueue>;
    pollEvents: () => ReturnType<typeof gameEngine.event.get>;
  };
  ui.screenWidth = 160;
  ui.screenHeight = 144;
  ui.eventQueue = gameEngine.event.createQueue();
  ui.pollEvents = () => gameEngine.event.get(ui.eventQueue);
  return ui;
};

class TestPromptUI extends BaseUI {
  public update = jest.fn();
  public font: PCFont = {
    paletteVariants: jest.fn(() => ({})),
    renderText: jest.fn(),
  };

  protected createScreenSurface(): Surface {
    return new Surface(160, 144);
  }
}

describe("PokemonCenterPCSession", () => {
  beforeAll(async () => {
    const font = new BitmapFont();
    await font.load();
    fontProxy = {
      paletteVariants: font.paletteVariants.bind(font),
      renderText: font.renderText.bind(font),
    };
  });

  it("keeps Bill's top menu in the Crystal order, including MOVE PKMN W/O MAIL", () => {
    expect(PokemonCenterPCSession.BILL_PC_MENU).toEqual([
      ["WITHDRAW <PK><MN>", "withdraw"],
      ["DEPOSIT <PK><MN>", "deposit"],
      ["CHANGE BOX", "change_box"],
      ["MOVE <PK><MN> W/O MAIL", "move"],
      ["SEE YA!", "see_ya"],
    ]);
  });

  it("moves Bill's PC top menu from withdraw to deposit with a down-arrow key event", () => {
    const ui = makeInteractiveMenuUi();
    gameEngine.event.post({ type: "keydown", key: gameEngine.K_DOWN }, ui.eventQueue);
    gameEngine.event.post({ type: "keydown", button: "a" }, ui.eventQueue);
    const prompt = new SelectionPrompt(ui, PokemonCenterPCSession.BILL_PC_MENU.map((entry) => entry[0]), {
      windowOriginTiles: [0, 0],
      windowMinWidth: 13,
      windowMinHeight: 7,
    });

    const selected = prompt.run();

    expect(PokemonCenterPCSession.BILL_PC_MENU[selected]).toEqual(["DEPOSIT <PK><MN>", "deposit"]);
  });

  it("emits debug logs for PC actions when enabled", () => {
    const originalDebug = process.env.POKE_DEBUG;
    process.env.POKE_DEBUG = "pc";
    const initial = getDebugLogEntries().length;

    const gameState = createInitialGameState();
    seedParty(gameState, toPokemon(makePokemon(gameState, "CYNDAQUIL")));
    const stored = toPokemon(makePokemon(gameState, "TOTODILE", 7));
    const box = makeBox();
    setSlot(box, 0, stored);
    gameState.sram.pc_boxes = [box];

    const session = new PokemonCenterPCSession(gameState);
    session.run({
      selection: 0,
      scriptedActions: { bill: [{ action: "withdraw", box: 0, slot: 0 }] },
    });

    const entries = getDebugLogEntries().slice(initial);
    expect(entries.some((entry) => entry.message.includes("[pc] session start"))).toBe(true);
    expect(entries.some((entry) => entry.message.includes("[pc] bill action"))).toBe(true);

    process.env.POKE_DEBUG = originalDebug;
  });

  it("withdraws a Pokemon from Bill's PC into the party", () => {
    const gameState = createInitialGameState();
    seedParty(gameState, toPokemon(makePokemon(gameState, "CYNDAQUIL")));
    const stored = toPokemon(makePokemon(gameState, "TOTODILE", 7));
    const box = makeBox();
    setSlot(box, 0, stored);
    gameState.sram.pc_boxes = [box];

    const session = new PokemonCenterPCSession(gameState);
    const result = session.run({
      selection: 0,
      scriptedActions: { bill: [{ action: "withdraw", box: 0, slot: 0 }] },
    }) as SessionResult;
    const bill = requireBillResult(result);

    expect(gameState.sram.party.pokemon[1]?.species?.id).toBe(stored.species.id);
    expect(bill.actions[0].status).toBe("ok");
  });

  it("compacts a PC box after withdrawing a Pokemon", () => {
    const gameState = createInitialGameState();
    seedParty(gameState, toPokemon(makePokemon(gameState, "CYNDAQUIL")));
    const stored = toPokemon(makePokemon(gameState, "TOTODILE", 7));
    const remaining = toPokemon(makePokemon(gameState, "CHIKORITA", 8));
    const box = makeBox();
    setSlot(box, 0, stored);
    setSlot(box, 1, remaining);
    gameState.sram.pc_boxes = [box];

    const session = new PokemonCenterPCSession(gameState);
    const result = session.run({
      selection: 0,
      scriptedActions: { bill: [{ action: "withdraw", box: 0, slot: 0 }] },
    }) as SessionResult;
    const bill = requireBillResult(result);

    expect(bill.actions[0].status).toBe("ok");
    expect(gameState.sram.pc_boxes[0].pokemon[0]?.species?.id).toBe(remaining.species.id);
    expect(gameState.sram.pc_boxes[0].pokemon[1]).toBe(null);
  });

  it("deposits and releases a Pokemon from Bill's PC", () => {
    const gameState = createInitialGameState();
    const lead = makePokemon(gameState, "CYNDAQUIL");
    const extra = makePokemon(gameState, "TOTODILE", 7);
    seedParty(gameState, lead, extra);
    gameState.sram.pc_boxes = [makeBox()];

    const session = new PokemonCenterPCSession(gameState);
    const result = session.run({
      selection: 0,
      scriptedActions: {
        bill: [
          { action: "deposit", party_slot: 1, box: 0, slot: 0 },
          { action: "release", box: 0, slot: 0 },
        ],
      },
    }) as SessionResult;
    const bill = requireBillResult(result);

    expect(bill.actions[0].status).toBe("ok");
    expect(bill.actions[1].status).toBe("ok");
    expect(gameState.sram.party.pokemon[1]).toBe(null);
    expect(gameState.sram.pc_boxes[0].pokemon[0]).toBe(null);
  });

  it("blocks deposits when the party would be empty", () => {
    const gameState = createInitialGameState();
    const lead = makePokemon(gameState, "CYNDAQUIL");
    seedParty(gameState, lead);
    gameState.sram.pc_boxes = [makeBox()];

    const session = new PokemonCenterPCSession(gameState);
    const result = session.run({
      selection: 0,
      scriptedActions: { bill: [{ action: "deposit", party_slot: 0, box: 0, slot: 0 }] },
    }) as SessionResult;
    const bill = requireBillResult(result);

    expect(bill.actions[0].status).toBe("party_size");
    expect(gameState.sram.party.pokemon[0]).toBe(lead);
    expect(gameState.sram.pc_boxes[0].pokemon[0]).toBe(null);
  });

  it("blocks deposits of mail-holding Pokemon", () => {
    const gameState = createInitialGameState();
    const lead = makePokemon(gameState, "CYNDAQUIL");
    const extra = makePokemon(gameState, "TOTODILE", 7);
    lead.item = "FLOWER_MAIL";
    seedParty(gameState, lead, extra);
    gameState.sram.pc_boxes = [makeBox()];

    const session = new PokemonCenterPCSession(gameState);
    const result = session.run({
      selection: 0,
      scriptedActions: { bill: [{ action: "deposit", party_slot: 0, box: 0, slot: 0 }] },
    }) as SessionResult;
    const bill = requireBillResult(result);

    expect(bill.actions[0].status).toBe("mail");
    expect(gameState.sram.party.pokemon[0]).toBe(lead);
    expect(gameState.sram.pc_boxes[0].pokemon[0]).toBe(null);
  });

  it("allows deposits of eggs like the ASM deposit path", () => {
    const gameState = createInitialGameState();
    const egg = toPokemon(makePokemon(gameState, "TOGEPI"));
    egg.nickname = "EGG";
    const extra = toPokemon(makePokemon(gameState, "TOTODILE", 7));
    seedParty(gameState, egg, extra);
    gameState.sram.pc_boxes = [makeBox()];

    const session = new PokemonCenterPCSession(gameState);
    const result = session.run({
      selection: 0,
      scriptedActions: { bill: [{ action: "deposit", party_slot: 0, box: 0, slot: 0 }] },
    }) as SessionResult;
    const bill = requireBillResult(result);

    expect(bill.actions[0].status).toBe("ok");
    expect(gameState.sram.party.pokemon[0]?.species?.id).toBe(extra.species.id);
    expect(gameState.sram.pc_boxes[0].pokemon[0]?.nickname).toBe("EGG");
  });

  it("blocks deposits when the current box is full", () => {
    const gameState = createInitialGameState();
    const lead = makePokemon(gameState, "CYNDAQUIL");
    const extra = makePokemon(gameState, "TOTODILE", 7);
    seedParty(gameState, lead, extra);
    const box = makeBox();
    for (let index = 0; index < box.pokemon.length; index += 1) {
      setSlot(box, index, makePokemon(gameState, "PIDGEY", 3));
    }
    gameState.sram.pc_boxes = [box];

    const session = new PokemonCenterPCSession(gameState);
    const result = session.run({
      selection: 0,
      scriptedActions: { bill: [{ action: "deposit", party_slot: 1, box: 0 }] },
    }) as SessionResult;
    const bill = requireBillResult(result);

    expect(bill.actions[0].status).toBe("box_full");
    expect(gameState.sram.party.pokemon[1]?.species?.id).toBe(extra.species.id);
  });

  it("releases a party Pokemon from Bill's deposit flow and compacts the party", () => {
    const gameState = createInitialGameState();
    const lead = makePokemon(gameState, "CYNDAQUIL");
    const extra = makePokemon(gameState, "TOTODILE", 7);
    seedParty(gameState, lead, extra);
    gameState.sram.pc_boxes = [makeBox()];

    const session = new PokemonCenterPCSession(gameState);
    const result = session.run({
      selection: 0,
      scriptedActions: { bill: [{ action: "release", box: null, slot: 0 }] },
    }) as SessionResult;
    const bill = requireBillResult(result);

    expect(bill.actions[0].status).toBe("ok");
    expect(gameState.sram.party.pokemon[0]?.species?.id).toBe(extra.species.id);
    expect(gameState.sram.party.pokemon[1]).toBe(null);
  });

  it("moves a Pokemon between boxes", () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    const boxA = makeBox(formatDefaultBoxName(0));
    const boxB = makeBox(formatDefaultBoxName(1));
    const stored = toPokemon(makePokemon(gameState, "TOTODILE", 7));
    setSlot(boxA, 0, stored);
    gameState.sram.pc_boxes = [boxA, boxB];

    const session = new PokemonCenterPCSession(gameState);
    const result = session.run({
      selection: 0,
      scriptedActions: {
        bill: [{ action: "move", box: 0, slot: 0, target_box: 1, target_slot: 3 }],
      },
    }) as SessionResult;
    const bill = requireBillResult(result);

    expect(bill.actions[0].status).toBe("ok");
    expect(gameState.sram.pc_boxes[0].pokemon[0]).toBe(null);
    expect(gameState.sram.pc_boxes[1].pokemon[3]?.species?.id).toBe(stored.species.id);
  });

  it("moves a Pokemon from Bill's PC into the party during move mode", () => {
    const gameState = createInitialGameState();
    const lead = toPokemon(makePokemon(gameState, "CYNDAQUIL"));
    const extra = toPokemon(makePokemon(gameState, "CHIKORITA", 7));
    seedParty(gameState, lead, extra);
    const boxA = makeBox(formatDefaultBoxName(0));
    const stored = toPokemon(makePokemon(gameState, "TOTODILE", 9));
    setSlot(boxA, 0, stored);
    gameState.sram.pc_boxes = [boxA];

    const session = new PokemonCenterPCSession(gameState);
    const result = session.run({
      selection: 0,
      scriptedActions: {
        bill: [{ action: "move", box: 0, slot: 0, target_box: null, target_slot: 1 }],
      },
    }) as SessionResult;
    const bill = requireBillResult(result);

    expect(bill.actions[0].status).toBe("swapped");
    expect(gameState.sram.party.pokemon[1]?.species?.id).toBe(stored.species.id);
    expect(gameState.sram.pc_boxes[0].pokemon[0]?.species?.id).toBe(extra.species.id);
  });

  it("moves a party Pokemon into Bill's PC and compacts the party", () => {
    const gameState = createInitialGameState();
    const lead = toPokemon(makePokemon(gameState, "CYNDAQUIL"));
    const extra = toPokemon(makePokemon(gameState, "TOTODILE", 7));
    seedParty(gameState, lead, extra);
    const boxA = makeBox(formatDefaultBoxName(0));
    gameState.sram.pc_boxes = [boxA];

    const session = new PokemonCenterPCSession(gameState);
    const result = session.run({
      selection: 0,
      scriptedActions: {
        bill: [{ action: "move", box: null, slot: 0, target_box: 0, target_slot: 0 }],
      },
    }) as SessionResult;
    const bill = requireBillResult(result);

    expect(bill.actions[0].status).toBe("ok");
    expect(gameState.sram.pc_boxes[0].pokemon[0]?.species?.id).toBe(lead.species.id);
    expect(gameState.sram.party.pokemon[0]?.species?.id).toBe(extra.species.id);
    expect(gameState.sram.party.pokemon[1]).toBe(null);
  });

  it("blocks moves of mail-holding Pokemon", () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    const boxA = makeBox(formatDefaultBoxName(0));
    const boxB = makeBox(formatDefaultBoxName(1));
    const stored = toPokemon(makePokemon(gameState, "TOTODILE", 7));
    stored.item = "FLOWER_MAIL";
    setSlot(boxA, 0, stored);
    gameState.sram.pc_boxes = [boxA, boxB];

    const session = new PokemonCenterPCSession(gameState);
    const result = session.run({
      selection: 0,
      scriptedActions: {
        bill: [{ action: "move", box: 0, slot: 0, target_box: 1, target_slot: 0 }],
      },
    }) as SessionResult;
    const bill = requireBillResult(result);

    expect(bill.actions[0].status).toBe("mail");
    expect(gameState.sram.pc_boxes[0].pokemon[0]?.species?.id).toBe(stored.species.id);
    expect(gameState.sram.pc_boxes[1].pokemon[0]).toBe(null);
  });

  it("blocks move-without-mail when any Pokemon is holding mail", () => {
    const gameState = createInitialGameState();
    const lead = makePokemon(gameState, "CYNDAQUIL");
    const mailedPartyMon = makePokemon(gameState, "PIDGEY", 4);
    mailedPartyMon.item = "FLOWER_MAIL";
    seedParty(gameState, lead, mailedPartyMon);
    const boxA = makeBox(formatDefaultBoxName(0));
    const boxB = makeBox(formatDefaultBoxName(1));
    const stored = toPokemon(makePokemon(gameState, "TOTODILE", 7));
    setSlot(boxA, 0, stored);
    gameState.sram.pc_boxes = [boxA, boxB];

    const session = new PokemonCenterPCSession(gameState);
    const result = session.run({
      selection: 0,
      scriptedActions: {
        bill: [{ action: "move", box: 0, slot: 0, target_box: 1, target_slot: 0 }],
      },
    }) as SessionResult;
    const bill = requireBillResult(result);

    expect(bill.actions[0].status).toBe("mail");
    expect(gameState.sram.pc_boxes[0].pokemon[0]?.species?.id).toBe(stored.species.id);
    expect(gameState.sram.pc_boxes[1].pokemon[0]).toBe(null);
  });

  it("handles player PC deposit and withdraw actions", () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    gameState.sram.items.POTION = 5;
    const ui = makeMenuUi();
    const session = new PokemonCenterPCSession(gameState, { ui });
    const result = session.run({
      selection: 1,
      scriptedActions: {
        player: [
          { action: "deposit", item: "POTION", quantity: 3 },
          { action: "withdraw", item: "POTION", quantity: 2 },
        ],
      },
    }) as SessionResult;
    const playerResult = requirePlayerResult(result);

    expect(playerResult.actions[0].status).toBe("ok");
    expect(playerResult.actions[1].status).toBe("ok");
    expect(gameState.sram.items.POTION).toBe(4);
    expect(playerResult.pc_items).toEqual(
      expect.arrayContaining([{ item: "POTION", quantity: 1 }])
    );
  });

  it("moves the player's PC action menu from withdraw to deposit with a down-arrow key event", () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    gameState.sram.items.POTION = 3;
    gameState.sram.pc_items = [];
    const ui = makeInteractiveMenuUi();
    const menu = new PlayerPCMenu(ui, gameState);

    expect(menu.handleInput({ type: "keydown", key: gameEngine.K_DOWN })).toBeNull();
    expect(menu.handleInput({ type: "keydown", button: "a" })).toBeNull();
    expect(menu.handleInput({ type: "keydown", button: "a" })).toBeNull();
    const result = menu.handleInput({ type: "keydown", button: "a" });

    expect(result).toEqual(expect.objectContaining({ action: "deposit", status: "ok", item: "POTION", quantity: 1 }));
    expect(gameState.sram.items.POTION).toBe(2);
    expect(gameState.sram.pc_items).toEqual([{ item: "POTION", quantity: 1 }]);
  });

  it("tosses items from player PC item storage", () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    gameState.sram.items.POTION = 3;
    gameState.sram.pc_items = [{ item: "POTION", quantity: 3 }];
    const ui = makeMenuUi();
    const session = new PokemonCenterPCSession(gameState, { ui });
    const result = session.run({
      selection: 1,
      scriptedActions: { player: [{ action: "toss", item: "POTION", quantity: 2 }] },
    }) as SessionResult;
    const playerResult = requirePlayerResult(result);

    expect(playerResult.actions[0].status).toBe("ok");
    expect(gameState.sram.items.POTION).toBe(3);
    expect(playerResult.pc_items).toEqual([{ item: "POTION", quantity: 1 }]);
  });

  it("preserves PC item limits for deposit and withdraw quantities", () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    gameState.sram.items.POTION = 10;
    gameState.sram.pc_items = [
      { item: "POTION", quantity: MAX_PC_ITEM_QUANTITY - 1 },
      { item: "ANTIDOTE", quantity: MAX_PC_ITEM_QUANTITY - 1 },
    ];
    const ui = makeMenuUi();
    const session = new PokemonCenterPCSession(gameState, { ui });
    const result = session.run({
      selection: 1,
      scriptedActions: {
        player: [
          { action: "deposit", item: "POTION", quantity: 2 },
          { action: "withdraw", item: "ANTIDOTE", quantity: MAX_PC_ITEM_QUANTITY + 10 },
        ],
      },
    }) as SessionResult;
    const playerResult = requirePlayerResult(result);

    expect(playerResult.actions[0].status).toBe("pc_full");
    expect(playerResult.actions[1].status).toBe("ok");
    expect(playerResult.actions[1].quantity).toBe(MAX_PC_ITEM_QUANTITY - 1);
    expect(playerResult.pc_items).toEqual([{ item: "POTION", quantity: MAX_PC_ITEM_QUANTITY - 1 }]);
  });

  it("rejects new PC item deposits when all PC item slots are occupied", () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    gameState.sram.items.POTION = 1;
    gameState.sram.pc_items = Array.from({ length: MAX_PC_ITEMS }, (_unused, index) => ({
      item: `ITEM_${index}`,
      quantity: 1,
    }));
    const ui = makeMenuUi();
    const session = new PokemonCenterPCSession(gameState, { ui });
    const result = session.run({
      selection: 1,
      scriptedActions: { player: [{ action: "deposit", item: "POTION", quantity: 1 }] },
    }) as SessionResult;
    const playerResult = requirePlayerResult(result);

    expect(playerResult.actions[0].status).toBe("pc_full");
    expect(gameState.sram.items.POTION).toBe(1);
    expect(playerResult.pc_items).toHaveLength(MAX_PC_ITEMS);
  });

  it("executes mailbox scripted actions", () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    gameState.sram.mailbox = [
      MailMessageSchema.parse({ message: "RIVAL wrote you!" }),
      MailMessageSchema.parse({ message: "Mom says hi" }),
    ];
    const ui = makeMenuUi();
    const session = new PokemonCenterPCSession(gameState, { ui });
    const result = session.run({
      selection: 1,
      scriptedActions: {
        mail: [
          { action: "read", index: 0 },
          { action: "take", index: 1 },
          { action: "delete", index: 0 },
          { action: "give", message: "GIVE MAIL" },
          { action: "add", message: "Bill left a note." },
        ],
      },
    }) as SessionResult;
    const mailbox = requireMailboxResult(result);
    const actions = mailbox.actions;
    expect(actions[0].message).toContain("RIVAL");
    expect(actions[1].status).toBe("ok");
    expect(actions[2].status).toBe("ok");
    expect(actions[3].status).toBe("ok");
    expect(mailbox.messages).toEqual(
      expect.arrayContaining(["GIVE MAIL", "Bill left a note."])
    );
  });

  it("reports turn off selection with a result message", () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    const session = new PokemonCenterPCSession(gameState);
    const result = session.run({ selection: 3 }) as SessionResult;

    expect(result.selection_name).toBe("TURN OFF");
    expect(result.result_text).toBe("Turning off the PC.");
  });

  it("uses the ASM shutdown text fallback when PC text data is unavailable", () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    const showText = jest.fn();
    const waitForInput = jest.fn();
    const session = new PokemonCenterPCSession(gameState, {
      dataLoader: { getText: () => "" },
      eventManager: { showText, waitForInput },
    });

    session.run({ selection: 3 });

    expect(showText).toHaveBeenCalledWith("Turning off the PC.");
    expect(waitForInput).toHaveBeenCalled();
  });

  it("uses async prompt selection for interactive sessions", async () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    const ui = new TestPromptUI();
    const session = new PokemonCenterPCSession(gameState, { ui });
    const runAsyncSpy = jest.spyOn(PCHubMenu.prototype, "runAsync").mockResolvedValue(3);
    const runSpy = jest.spyOn(SelectionPrompt.prototype, "run");

    const result = await session.runAsync();

    expect(runAsyncSpy).toHaveBeenCalled();
    expect(runSpy).not.toHaveBeenCalled();
    expect(result.selection_name).toBe("TURN OFF");
    runAsyncSpy.mockRestore();
    runSpy.mockRestore();
  });

  it("does not let boot dialogue input swallow the PC hub selection", async () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    const ui = makeInteractiveMenuUi();
    const downEvent = { type: "keydown", key: gameEngine.K_DOWN };
    const dialogue = {
      visible: false,
      waiting_for_input: false,
      pending_waits: 0,
      handle_input: jest.fn((event: { button?: string }) => {
        if (event.button === "a") {
          dialogue.pending_waits = 0;
          dialogue.waiting_for_input = false;
          return true;
        }
        return false;
      }),
    };
    const eventManager = {
      showText: jest.fn(() => {
        dialogue.visible = true;
      }),
      waitForInput: jest.fn(() => {
        dialogue.waiting_for_input = true;
        dialogue.pending_waits += 1;
      }),
    };
    const hubRunAsyncSpy = jest.spyOn(PCHubMenu.prototype, "runAsync").mockImplementation(async function () {
      const provider = (this as unknown as { eventProvider?: () => unknown[] }).eventProvider;
      expect(typeof provider).toBe("function");
      expect(provider?.()).toEqual([downEvent]);
      expect(dialogue.handle_input).not.toHaveBeenCalled();
      return 2;
    });
    ui.pollEvents = jest.fn(() => [downEvent]);
    const session = new PokemonCenterPCSession(gameState, {
      ui,
      eventManager,
      dialogue,
    });

    const result = await session.runAsync();

    expect(result.selection_name).toBe("TURN OFF");
    expect(eventManager.waitForInput).toHaveBeenCalledTimes(2);
    hubRunAsyncSpy.mockRestore();
  });

  it("runs Bill's PC through the synchronous interactive path", () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    const ui = makeInteractiveMenuUi();
    const hubSpy = jest
      .spyOn(PCHubMenu.prototype, "run")
      .mockReturnValueOnce(0)
      .mockReturnValueOnce(4);
    const menuSpy = jest
      .spyOn(SelectionPrompt.prototype, "run")
      .mockReturnValueOnce(0)
      .mockReturnValueOnce(4);
    const billInteractiveSpy = jest.spyOn(PokemonPCMenu.prototype, "runInteractive").mockReturnValue([]);

    const session = new PokemonCenterPCSession(gameState, { ui });
    const result = session.run() as SessionResult & { first_selection_name?: string };

    expect(hubSpy).toHaveBeenCalledTimes(2);
    expect(menuSpy).toHaveBeenCalledTimes(2);
    expect(billInteractiveSpy).toHaveBeenCalledTimes(1);
    expect(result.first_selection_name).toBe("BILL's PC");
    expect(result.selection_name).toBe("TURN OFF");
    hubSpy.mockRestore();
    menuSpy.mockRestore();
    billInteractiveSpy.mockRestore();
  });

  it("deposits the sixth party Pokemon from Bill's PC manual deposit list", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [
      makePokemon(gameState, "CYNDAQUIL"),
      makePokemon(gameState, "GEODUDE", 25),
      makePokemon(gameState, "SANDSHREW", 17),
      makePokemon(gameState, "BELLSPROUT", 18),
      makePokemon(gameState, "BUTTERFREE", 20),
      makePokemon(gameState, "TOGEPI", 5),
    ];
    gameState.sram.pc_boxes = [makeBox()];
    const ui = makeInteractiveMenuUi();
    const menu = new PokemonPCMenu(ui, gameState, null);
    menu.showBillAction("deposit");
    for (let index = 0; index < 5; index += 1) {
      gameEngine.event.post({ type: "keydown", key: gameEngine.K_DOWN }, ui.eventQueue);
    }
    gameEngine.event.post({ type: "keydown", button: "a" }, ui.eventQueue);
    gameEngine.event.post({ type: "keydown", button: "b" }, ui.eventQueue);

    const actions = menu.runInteractive({
      actionHandler: (payload) => payload,
    });

    expect(actions).toEqual([
      expect.objectContaining({
        action: "deposit",
        box: 0,
        party_slot: 5,
        slot: null,
      }),
    ]);
  });

  it("accepts MCP button-confirm events in Bill's PC manual deposit list", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [
      makePokemon(gameState, "CYNDAQUIL"),
      makePokemon(gameState, "GEODUDE", 25),
      makePokemon(gameState, "SANDSHREW", 17),
      makePokemon(gameState, "BELLSPROUT", 18),
      makePokemon(gameState, "BUTTERFREE", 20),
      makePokemon(gameState, "TOGEPI", 5),
    ];
    gameState.sram.pc_boxes = [makeBox()];
    const ui = makeInteractiveMenuUi();
    const menu = new PokemonPCMenu(ui, gameState, null);
    menu.showBillAction("deposit");
    for (let index = 0; index < 5; index += 1) {
      menu.handleInput({ type: "keydown", key: gameEngine.K_DOWN, code: gameEngine.K_DOWN, is_press: true });
    }

    expect(menu.handleInput({ type: "keydown", button: "a", is_press: true })).toEqual([
      "deposit",
      0,
      5,
    ]);
  });

  it("deposits from Bill's PC even if the deposit list is in browse mode", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [
      makePokemon(gameState, "CYNDAQUIL"),
      makePokemon(gameState, "GEODUDE", 25),
      makePokemon(gameState, "SANDSHREW", 17),
      makePokemon(gameState, "BELLSPROUT", 18),
      makePokemon(gameState, "BUTTERFREE", 20),
      makePokemon(gameState, "TOGEPI", 5),
    ];
    gameState.sram.pc_boxes = [makeBox()];
    const ui = makeInteractiveMenuUi();
    const menu = new PokemonPCMenu(ui, gameState, null);
    menu.showBillAction("deposit");
    (menu as unknown as { mode: string }).mode = "browse";
    for (let index = 0; index < 5; index += 1) {
      menu.handleInput({ type: "keydown", key: gameEngine.K_DOWN, code: gameEngine.K_DOWN, is_press: true });
    }

    expect(menu.handleInput({ type: "keydown", button: "a", is_press: true })).toEqual([
      "deposit",
      0,
      5,
    ]);
  });

  it("runs Player's PC through the synchronous interactive path", () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    const ui = makeInteractiveMenuUi();
    gameEngine.event.post({ type: "keydown", button: "b" }, ui.eventQueue);
    const selectionSpy = jest
      .spyOn(PCHubMenu.prototype, "run")
      .mockReturnValueOnce(1)
      .mockReturnValueOnce(4);

    const session = new PokemonCenterPCSession(gameState, { ui });
    const result = session.run() as SessionResult & { first_selection_name?: string };

    expect(selectionSpy).toHaveBeenCalledTimes(2);
    expect(result.first_selection_name).toBe("PLAYER's PC");
    expect(result.selection_name).toBe("TURN OFF");
    selectionSpy.mockRestore();
  });

  it("runs Hall of Fame through the synchronous interactive path", () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    gameState.sram.hall_of_fame = [
      {
        team: [{ species: "CYNDAQUIL", nickname: "TOPPER" }],
      } as never,
    ];
    gameState.wram.wHallOfFameCount = 1;
    const ui = makeInteractiveMenuUi();
    gameEngine.event.post({ type: "keydown", button: "b" }, ui.eventQueue);
    const selectionSpy = jest
      .spyOn(PCHubMenu.prototype, "run")
      .mockReturnValueOnce(3)
      .mockReturnValueOnce(4);

    const session = new PokemonCenterPCSession(gameState, { ui });
    const result = session.run() as SessionResult & { first_selection_name?: string };

    expect(selectionSpy).toHaveBeenCalledTimes(2);
    expect(result.first_selection_name).toBe("HALL OF FAME");
    expect(result.selection_name).toBe("TURN OFF");
    selectionSpy.mockRestore();
  });

  it("displays canonical Hall of Fame team data before legacy pokemon aliases", () => {
    const gameState = createInitialGameState();
    gameState.sram.hall_of_fame = [
      {
        team: [{ species: "CYNDAQUIL", nickname: "TEAMDATA" }],
        pokemon: [{ species: "TOTODILE", nickname: "LEGACY" }],
      } as never,
    ];
    gameState.wram.wHallOfFameCount = 1;
    const session = new PokemonCenterPCSession(gameState, { ui: makeMenuUi() });

    const result = session.runHallOfFame(null) as { selected?: string[]; entries?: string[][] };

    expect(result.entries).toEqual([["TEAMDATA"]]);
    expect(result.selected).toEqual(["TEAMDATA"]);
  });

  it("runs the mailbox menu through the synchronous interactive path", () => {
    const ui = makeInteractiveMenuUi();
    gameEngine.event.post({ type: "keydown", button: "a" }, ui.eventQueue);
    gameEngine.event.post({ type: "keydown", button: "a" }, ui.eventQueue);
    gameEngine.event.post({ type: "keydown", button: "b" }, ui.eventQueue);
    const messages = [
      MailMessageSchema.parse({ message: "RIVAL wrote you!" }),
      MailMessageSchema.parse({ message: "Mom says hi" }),
    ];
    const menu = new MailboxMenu(ui, messages);

    const results = menu.runInteractive({
      handler: (action) => action,
    });

    expect(results).toHaveLength(1);
    expect(results[0].action).toBe("read");
    expect(results[0].message).toContain("RIVAL");
  });

  it("routes hall_of_fame_terminal through the async Hall of Fame call chain", async () => {
    const gameState = createInitialGameState();
    const asyncSpy = jest
      .spyOn(PokemonCenterPCSession.prototype as any, "runHallOfFameInteractiveAsync")
      .mockResolvedValue({ status: "ok" });
    const syncSpy = jest.spyOn(PokemonCenterPCSession.prototype, "runHallOfFameInteractive");

    await hall_of_fame_terminal(gameState, {
      overworld: { ui: null } as never,
      event_manager: {} as never,
    });

    expect(asyncSpy).toHaveBeenCalledTimes(1);
    expect(syncSpy).not.toHaveBeenCalled();
    asyncSpy.mockRestore();
    syncSpy.mockRestore();
  });

  it("prompts for selection with composite UI", async () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    const baseUi = new TestPromptUI();
    const ui = new CompositeUI(baseUi as unknown as CompositeChild) as unknown as BaseUI;
    const session = new PokemonCenterPCSession(gameState, { ui });
    const runAsyncSpy = jest.spyOn(PCHubMenu.prototype, "runAsync").mockResolvedValue(3);

    const result = await session.runAsync();

    expect(runAsyncSpy).toHaveBeenCalled();
    expect(result.selection_name).toBe("TURN OFF");
    runAsyncSpy.mockRestore();
  });

  it("adapts prompt-only UI for Bill's PC interactive sessions", async () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    const ui = new TestPromptUI();
    const session = new PokemonCenterPCSession(gameState, { ui });
    const billMenuSpy = jest
      .spyOn(PokemonPCMenu.prototype, "runInteractiveAsync")
      .mockResolvedValue([]);
    const hubRunAsyncSpy = jest
      .spyOn(PCHubMenu.prototype, "runAsync")
      .mockResolvedValueOnce(0)
      .mockResolvedValueOnce(3);
    const promptRunAsyncSpy = jest
      .spyOn(SelectionPrompt.prototype, "runAsync")
      .mockResolvedValueOnce(0)
      .mockResolvedValueOnce(4);

    const result = await session.runAsync();

    expect(hubRunAsyncSpy).toHaveBeenCalledTimes(2);
    expect(promptRunAsyncSpy).toHaveBeenCalledTimes(2);
    expect(billMenuSpy).toHaveBeenCalledTimes(1);
    expect((result as unknown as { first_selection_name?: string }).first_selection_name).toBe("BILL's PC");
    expect(result.selection_name).toBe("TURN OFF");
    billMenuSpy.mockRestore();
    hubRunAsyncSpy.mockRestore();
    promptRunAsyncSpy.mockRestore();
  });

  it("clears stale dialogue waits before routing input to PC menus", () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    const dialogue = {
      visible: false,
      waiting_for_input: true,
      pending_waits: 1,
      clear_script_waits: jest.fn(() => {
        dialogue.pending_waits = 0;
      }),
      handle_input: jest.fn((event: { button?: string }) => {
        if (event.button === "a") {
          dialogue.pending_waits = 0;
          dialogue.waiting_for_input = false;
          return true;
        }
        return true;
      }),
    };
    const session = new PokemonCenterPCSession(gameState, {
      dialogue,
    });
    const event = { type: "keydown", button: "a" };
    const provider = jest.fn(() => [event]);
    const wrapped = (
      session as unknown as {
        dialogueAwareEventProvider: <T>(provider: () => T[]) => () => T[];
      }
    ).dialogueAwareEventProvider(provider);

    expect(wrapped()).toEqual([event]);
    expect(dialogue.handle_input).not.toHaveBeenCalled();
    expect(dialogue.clear_script_waits).toHaveBeenCalled();
    expect(dialogue.waiting_for_input).toBe(false);
    expect(dialogue.pending_waits).toBe(0);
    expect(wrapped()).toEqual([event]);
  });

  it("does not let stale Bill's PC dialogue swallow the top menu confirm", async () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    const ui = new TestPromptUI();
    const confirmEvent = { type: "keydown", button: "a" };
    ui.eventQueue = gameEngine.event.createQueue();
    gameEngine.event.post(confirmEvent, ui.eventQueue);
    const dialogue = {
      visible: false,
      waiting_for_input: false,
      pending_waits: 0,
      handle_input: jest.fn(() => true),
    };
    const eventManager = {
      showText: jest.fn(() => {
        dialogue.visible = true;
      }),
      waitForInput: jest.fn(() => {
        dialogue.waiting_for_input = true;
        dialogue.pending_waits += 1;
      }),
    };
    const session = new PokemonCenterPCSession(gameState, { ui, dialogue, eventManager });
    const hubRunAsyncSpy = jest
      .spyOn(PCHubMenu.prototype, "runAsync")
      .mockResolvedValueOnce(0)
      .mockResolvedValueOnce(3);
    const promptRunAsyncSpy = jest.spyOn(SelectionPrompt.prototype, "runAsync").mockImplementationOnce(async function (options) {
      const eventProvider = (options as { eventProvider?: () => unknown[] }).eventProvider;
      expect(eventProvider?.()).toEqual([confirmEvent]);
      return 4;
    });

    try {
      const result = await session.runAsync();

      expect(result.first_selection_name).toBe("BILL's PC");
      expect(dialogue.handle_input).not.toHaveBeenCalled();
      expect(eventManager.waitForInput).toHaveBeenCalled();
    } finally {
      hubRunAsyncSpy.mockRestore();
      promptRunAsyncSpy.mockRestore();
    }
  });

  it("clears FieldDialogueManager-style getter-only pending waits before Bill's menu", async () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    const ui = new TestPromptUI();
    let pendingWaits = 0;
    const dialogue = {
      visible: false,
      waiting_for_input: false,
      get pending_waits() {
        return pendingWaits;
      },
      clear_script_waits: jest.fn(() => {
        pendingWaits = 0;
        dialogue.waiting_for_input = false;
      }),
      handle_input: jest.fn(() => true),
    };
    const eventManager = {
      showText: jest.fn(() => {
        dialogue.visible = true;
      }),
      waitForInput: jest.fn(() => {
        dialogue.waiting_for_input = true;
        pendingWaits += 1;
      }),
    };
    const session = new PokemonCenterPCSession(gameState, { ui, dialogue, eventManager });
    const hubRunAsyncSpy = jest
      .spyOn(PCHubMenu.prototype, "runAsync")
      .mockResolvedValueOnce(0)
      .mockResolvedValueOnce(3);
    const promptRunAsyncSpy = jest.spyOn(SelectionPrompt.prototype, "runAsync").mockResolvedValueOnce(4);

    try {
      const result = await session.runAsync();

      expect(result.first_selection_name).toBe("BILL's PC");
      expect(result.selection_name).toBe("TURN OFF");
      expect(dialogue.clear_script_waits).toHaveBeenCalled();
      expect(promptRunAsyncSpy).toHaveBeenCalledTimes(1);
    } finally {
      hubRunAsyncSpy.mockRestore();
      promptRunAsyncSpy.mockRestore();
    }
  });

  it("selects the player PC from the real async hub with Down then A while using FieldDialogueManager", async () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "CHRIS";
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    const ui = makeInteractiveMenuUi();
    const runner = {
      pause: jest.fn(),
      resume: jest.fn(),
      _script_stack: [{}],
      _awaiting_resume: 0,
    };
    const dialogue = new FieldDialogueManager(ui, gameState, runner);
    const eventManager = {
      showText: jest.fn((text: string) => {
        dialogue.handle_event(new Event("open_text"), gameState);
        dialogue.handle_event(new Event("show_text", { text }), gameState);
      }),
      waitForInput: jest.fn(() => {
        dialogue.handle_event(new Event("wait_for_input"), gameState);
      }),
    };
    const session = new PokemonCenterPCSession(gameState, { ui, dialogue, eventManager });
    session.setHubOptions([
      { label: "BILL's PC", action: "bills_pc" },
      { label: "CHRIS's PC", action: "player_pc" },
      { label: "TURN OFF", action: "turn_off" },
    ]);
    const eventBatches = [
      [{ type: "keydown", key: gameEngine.K_DOWN }],
      [{ type: "keydown", button: "a" }],
      [{ type: "keydown", button: "b" }],
      [{ type: "keydown", key: gameEngine.K_DOWN }],
      [{ type: "keydown", key: gameEngine.K_DOWN }],
      [{ type: "keydown", button: "a" }],
    ];
    ui.pollEvents = jest.fn(() => eventBatches.shift() ?? []);

    const result = await session.runAsync();

    expect(result.first_selection_name).toBe("CHRIS's PC");
    expect(result.selection_name).toBe("TURN OFF");
  });

  it("falls back to game engine event queue polling for Bill's PC interactive sessions", async () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    const ui = new TestPromptUI();
    ui.eventQueue = gameEngine.event.createQueue();
    const session = new PokemonCenterPCSession(gameState, { ui });

    const billMenuSpy = jest.spyOn(PokemonPCMenu.prototype, "runInteractiveAsync").mockImplementation(async function () {
      const pollEvents = (this as unknown as { ui: SupportsPokemonPCUI }).ui.pollEvents;
      expect(typeof pollEvents).toBe("function");
      return [];
    });
    const hubRunAsyncSpy = jest
      .spyOn(PCHubMenu.prototype, "runAsync")
      .mockResolvedValueOnce(0)
      .mockResolvedValueOnce(3);
    const promptRunAsyncSpy = jest
      .spyOn(SelectionPrompt.prototype, "runAsync")
      .mockResolvedValueOnce(0)
      .mockResolvedValueOnce(4);

    const result = await session.runAsync();

    expect(billMenuSpy).toHaveBeenCalledTimes(1);
    expect(hubRunAsyncSpy).toHaveBeenCalledTimes(2);
    expect(promptRunAsyncSpy).toHaveBeenCalledTimes(2);
    expect(result.selection_name).toBe("TURN OFF");
    billMenuSpy.mockRestore();
    hubRunAsyncSpy.mockRestore();
    promptRunAsyncSpy.mockRestore();
  });

  it("treats null selections as interactive input in async sessions", async () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    const ui = new TestPromptUI();
    const session = new PokemonCenterPCSession(gameState, { ui });
    const runAsyncSpy = jest.spyOn(PCHubMenu.prototype, "runAsync").mockResolvedValue(3);
    const runSpy = jest.spyOn(PCHubMenu.prototype, "run");

    const result = await session.runAsync({ selection: null });

    expect(runAsyncSpy).toHaveBeenCalled();
    expect(runSpy).not.toHaveBeenCalled();
    expect(result.selection_name).toBe("TURN OFF");
    runAsyncSpy.mockRestore();
    runSpy.mockRestore();
  });

  it("renders the ASM hub prompt in the menu instead of showing loose prompt text", () => {
    const gameState = createInitialGameState();
    seedParty(gameState, makePokemon(gameState, "CYNDAQUIL"));
    const ui = new TestPromptUI();
    const showText = jest.fn();
    const waitForInput = jest.fn();
    const hubRunSpy = jest.spyOn(PCHubMenu.prototype, "run").mockReturnValue(4);
    const session = new PokemonCenterPCSession(gameState, {
      ui,
      eventManager: { showText, waitForInput },
    });

    session.run();

    expect(showText).toHaveBeenCalledWith("PLAYER turned on\nthe PC.");
    expect(showText).not.toHaveBeenCalledWith("Access whose PC?");
    expect(hubRunSpy).toHaveBeenCalled();
    hubRunSpy.mockRestore();
  });

  it("rates Oak PC using owned dex flags instead of the caught set cache", () => {
    const gameState = createInitialGameState();
    gameState.sram.pokedex_owned[0] = 0b0000_0011;
    gameState.sram.pokedex_caught = new Set([1, 7, 25]);
    const session = new PokemonCenterPCSession(gameState);

    const result = (
      session as unknown as {
        runOakRating: () => { status?: string; caught: number; label: string; text: string };
      }
    ).runOakRating();

    expect(result.status).toBe("no_ui");
    expect(result.caught).toBe(2);
    expect(result.label).toBe("OakRating01");
  });
});
