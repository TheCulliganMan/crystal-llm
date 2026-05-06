import { promises as fs } from 'fs';
import os from 'os';
import path from 'path';
import {
  saveGame,
  saveGameWithHistory,
  hasSaveGame,
  loadGame,
  deleteSaveGame,
  SaveFileNotFoundError,
  SaveGameValidationError,
  normalizeSaveSnapshot,
} from '../save';
import { createInitialGameState, GameState, useStore } from '../state';
import type { PokemonSpecies } from '../models';
import { createPokemon } from '@pokecrystal/core/engine/systems/pokemon';
import { Ability, EggGroup, GenderRatio, GrowthRate, PokemonType } from '@pokecrystal/core/core/enums';
import { calculateExperience } from '@pokecrystal/core/engine/experience';
import { MAX_COINS } from '@pokecrystal/core/core/constants';
import { MAX_ITEM_STACK } from '@pokecrystal/core/engine/systems/items';
import { MANUAL_SAVE_HISTORY_SLOTS, MANUAL_SAVE_SLOT } from '@pokecrystal/core/core/save-slots';
import {
  guestSessionKey,
  guestSessionMetadataKey,
  readGuestSessionSlot,
} from '../guest-session-storage';
import {
  createBrowserStorage,
  createSerializedSnapshot,
  installBrowserStorageWindow,
  writeGuestSessionSnapshot,
} from './save-test-harness';

const DEFAULT_BASE_STATS = {
  hp: 20,
  attack: 10,
  defense: 10,
  speed: 10,
  special_attack: 10,
  special_defense: 10,
};

const ensureSpecies = (id: string): PokemonSpecies => {
  return {
    id,
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
};

describe('saveGame and loadGame', () => {
  const saveFilePath = 'test-save.sav';
  let gameState: GameState;

  beforeEach(() => {
    // Use the default state from the store, which is already valid
    gameState = useStore.getState();
    // Modify the state for the test
    useStore.setState({
      ...gameState,
      sram: {
        ...gameState.sram,
        player_name: 'Jules',
        money: 5000,
      },
    });
    gameState = useStore.getState();
  });

  afterEach(async () => {
    await deleteSaveGame(saveFilePath);
  });

  it('should save and load a game state', async () => {
    await saveGame(gameState, saveFilePath);

    const loadedGameState = await loadGame(saveFilePath);
    expect(loadedGameState.sram.player_name).toBe('Jules');
    expect(loadedGameState.sram.money).toBe(5000);
  });

  it('should throw an error when loading a non-existent file', async () => {
    await expect(loadGame('non-existent-file.sav')).rejects.toThrow(SaveFileNotFoundError);
  });
});

describe('saveGame serverless mirror path', () => {
  const saveFilePath = 'test-save-serverless.sav';
  const savePath = path.resolve(os.tmpdir(), saveFilePath);
  const backupPath = `${savePath}.bak`;
  const originalVercel = process.env.VERCEL;

  afterEach(async () => {
    process.env.VERCEL = originalVercel;
    await fs.unlink(savePath).catch(() => undefined);
    await fs.unlink(backupPath).catch(() => undefined);
  });

  it('writes relative save paths to tmpdir on Vercel', async () => {
    process.env.VERCEL = '1';
    const gameState = createInitialGameState();
    gameState.sram.player_name = 'Serverless';

    await saveGame(gameState, saveFilePath);

    await expect(fs.readFile(savePath, 'utf-8')).resolves.toContain('"player_name": "Serverless"');
    await expect(fs.readFile(backupPath, 'utf-8')).rejects.toThrow();
  });
});

describe('saveGame node-only persistence', () => {
  const saveFilePath = 'test-save-node-guest-session.sav';

  afterEach(async () => {
    jest.restoreAllMocks();
    await deleteSaveGame(saveFilePath);
  });

  it('writes the canonical filesystem save when running in Node', async () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = 'NodeOnly';

    await expect(saveGame(gameState, saveFilePath)).resolves.toBe(true);
  });

  it('keeps the primary save readable when metadata persistence fails', async () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = 'MetadataOptional';
    const primaryPath = path.resolve(process.cwd(), saveFilePath);
    const metadataPath = `${primaryPath}.meta.json`;
    const originalWriteFile = fs.writeFile.bind(fs);
    const writeSpy = jest.spyOn(fs, 'writeFile').mockImplementation(async (...args) => {
      if (String(args[0]).startsWith(`${metadataPath}.`) && String(args[0]).endsWith('.tmp')) {
        throw new Error('metadata disk full');
      }
      return originalWriteFile(...args);
    });

    await expect(saveGame(gameState, saveFilePath)).resolves.toBe(true);
    await expect(loadGame(saveFilePath)).resolves.toMatchObject({
      sram: expect.objectContaining({
        player_name: 'MetadataOptional',
      }),
    });
    await expect(fs.readFile(metadataPath, 'utf-8')).rejects.toThrow();

    writeSpy.mockRestore();
  });
});

describe('saveGame deterministic surface matrix', () => {
  const browserSlot = 'matrix-browser.sav';
  const filesystemSlot = 'matrix-filesystem.sav';
  let restoreWindow: (() => void) | null = null;

  afterEach(async () => {
    jest.useRealTimers();
    jest.restoreAllMocks();
    await deleteSaveGame(browserSlot).catch(() => undefined);
    await deleteSaveGame(filesystemSlot).catch(() => undefined);
    restoreWindow?.();
    restoreWindow = null;
  });

  it.each([
    {
      name: 'filesystem',
      slot: filesystemSlot,
      setup: () => undefined,
    },
    {
      name: 'browser-guest-session',
      slot: browserSlot,
      setup: () => {
        restoreWindow = installBrowserStorageWindow();
      },
    },
  ])('keeps save/load/has/delete aligned for %s', async ({ slot, setup }) => {
    setup();
    const gameState = createInitialGameState();
    gameState.sram.player_name = `Matrix-${slot}`;

    await expect(hasSaveGame(slot)).resolves.toBe(false);
    await expect(saveGame(gameState, slot)).resolves.toBe(true);
    await expect(hasSaveGame(slot)).resolves.toBe(true);
    await expect(loadGame(slot)).resolves.toMatchObject({
      sram: expect.objectContaining({
        player_name: `Matrix-${slot}`,
      }),
    });
    await expect(deleteSaveGame(slot)).resolves.toBe(true);
    await expect(hasSaveGame(slot)).resolves.toBe(false);
  });
});

describe('saveGame browser guest-session fallback', () => {
  const saveFilePath = 'test-save-browser-guest-session.sav';
  let restoreWindow: (() => void) | null = null;

  beforeEach(() => {
    restoreWindow = installBrowserStorageWindow({
      localStorage: createBrowserStorage(),
      sessionStorage: createBrowserStorage(),
    });
  });

  afterEach(async () => {
    jest.restoreAllMocks();
    await deleteSaveGame(saveFilePath);
    restoreWindow?.();
    restoreWindow = null;
  });

  it('writes and loads guest-session saves in the browser without touching the filesystem', async () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = 'BrowserGuest';
    const writeSpy = jest.spyOn(fs, 'writeFile');

    await expect(saveGame(gameState, saveFilePath)).resolves.toBe(true);
    await expect(loadGame(saveFilePath)).resolves.toMatchObject({
      sram: expect.objectContaining({
        player_name: 'BrowserGuest',
      }),
    });

    expect(readGuestSessionSlot(saveFilePath)).toEqual(expect.any(String));
    expect(writeSpy).not.toHaveBeenCalled();
  });

  it('does not try to read browser filesystem saves when no guest save exists', async () => {
    const accessSpy = jest.spyOn(fs, 'access').mockRejectedValue(
      Object.assign(new Error('ENOENT: no such file or directory'), { code: 'ENOENT' })
    );
    const readFileSpy = jest.spyOn(fs, 'readFile');

    await expect(loadGame(saveFilePath)).rejects.toThrow(SaveFileNotFoundError);

    expect(accessSpy).not.toHaveBeenCalled();
    expect(readFileSpy).not.toHaveBeenCalled();
  });

  it('deletes guest-session saves without filesystem cleanup', async () => {
    const gameState = createInitialGameState();
    const unlinkSpy = jest.spyOn(fs, 'unlink');

    await saveGame(gameState, saveFilePath);
    await expect(hasSaveGame(saveFilePath)).resolves.toBe(true);
    await expect(deleteSaveGame(saveFilePath)).resolves.toBe(true);
    await expect(hasSaveGame(saveFilePath)).resolves.toBe(false);

    expect(readGuestSessionSlot(saveFilePath)).toBeNull();
    expect(unlinkSpy).not.toHaveBeenCalled();
  });

  it('detects and loads legacy browser fs saves via the unified probe path', async () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = 'LegacyBrowser';
    const snapshot = normalizeSaveSnapshot(gameState, 'legacy-browser');

    window.localStorage.setItem('fs:/legacy/savegame.sav', JSON.stringify(snapshot));

    await expect(hasSaveGame('savegame.sav')).resolves.toBe(true);
    await expect(loadGame('savegame.sav')).resolves.toMatchObject({
      sram: expect.objectContaining({
        player_name: 'LegacyBrowser',
      }),
    });
  });

  it('keeps hasSaveGame and loadGame aligned for readable browser saves', async () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = 'ProbeAligned';

    await expect(saveGame(gameState, saveFilePath)).resolves.toBe(true);
    await expect(hasSaveGame(saveFilePath)).resolves.toBe(true);
    await expect(loadGame(saveFilePath)).resolves.toMatchObject({
      sram: expect.objectContaining({
        player_name: 'ProbeAligned',
      }),
    });
  });

  it('falls back to a valid guest-session alias when the canonical .sav entry is invalid', async () => {
    const savedAt = '2026-04-20T00:00:00.000Z';

    window.localStorage.setItem(guestSessionKey('savegame.sav'), JSON.stringify({ nope: true }));
    writeGuestSessionSnapshot('savegame.sav', 'AliasFallback', {
      aliasSlot: 'savegame',
      savedAt,
    });

    await expect(hasSaveGame('savegame.sav')).resolves.toBe(true);
    await expect(loadGame('savegame.sav')).resolves.toMatchObject({
      sram: expect.objectContaining({
        player_name: 'AliasFallback',
      }),
    });
  });

  it('deletes both canonical and alias guest-session entries for the same slot', async () => {
    writeGuestSessionSnapshot('savegame.sav', 'AliasDeletePrimary', {
      savedAt: '2026-04-20T00:00:00.000Z',
    });
    writeGuestSessionSnapshot('savegame.sav', 'AliasDeleteAlias', {
      aliasSlot: 'savegame',
      savedAt: '2026-04-20T00:00:01.000Z',
    });

    await expect(deleteSaveGame('savegame.sav')).resolves.toBe(true);
    await expect(hasSaveGame('savegame.sav')).resolves.toBe(false);
    expect(window.localStorage.getItem(guestSessionKey('savegame.sav'))).toBeNull();
    expect(window.localStorage.getItem(guestSessionKey('savegame'))).toBeNull();
    expect(window.localStorage.getItem(guestSessionMetadataKey('savegame.sav'))).toBeNull();
    expect(window.localStorage.getItem(guestSessionMetadataKey('savegame'))).toBeNull();
  });

  it('falls back to sessionStorage when localStorage rejects guest-session writes', async () => {
    restoreWindow?.();
    restoreWindow = installBrowserStorageWindow({
      localStorage: createBrowserStorage({ failSetItem: true }),
      sessionStorage: createBrowserStorage(),
    });
    const gameState = createInitialGameState();
    gameState.sram.player_name = 'SessionFallback';

    await expect(saveGame(gameState, saveFilePath)).resolves.toBe(true);
    await expect(loadGame(saveFilePath)).resolves.toMatchObject({
      sram: expect.objectContaining({
        player_name: 'SessionFallback',
      }),
    });
    expect(window.sessionStorage.getItem(guestSessionKey(saveFilePath))).toEqual(expect.any(String));
  });

  it('does not rotate manual-save history slots in browser mode', async () => {
    const firstState = createInitialGameState();
    firstState.sram.player_name = 'BrowserHistoryOne';
    const secondState = createInitialGameState();
    secondState.sram.player_name = 'BrowserHistoryTwo';

    await expect(saveGame(firstState, MANUAL_SAVE_SLOT)).resolves.toBe(true);
    await expect(
      saveGameWithHistory(secondState, MANUAL_SAVE_SLOT, MANUAL_SAVE_HISTORY_SLOTS)
    ).resolves.toBe(true);

    await expect(loadGame(MANUAL_SAVE_SLOT)).resolves.toMatchObject({
      sram: expect.objectContaining({
        player_name: 'BrowserHistoryTwo',
      }),
    });
    await expect(loadGame(MANUAL_SAVE_HISTORY_SLOTS[0])).rejects.toThrow(SaveFileNotFoundError);
  });
});

describe('saveGame filesystem backup and atomic writes', () => {
  const saveFilePath = 'test-save-primary-only.sav';
  const savePath = path.resolve(process.cwd(), saveFilePath);
  const backupPath = path.resolve(process.cwd(), `${saveFilePath}.bak`);

  afterEach(async () => {
    jest.restoreAllMocks();
    await deleteSaveGame(saveFilePath);
  });

  it('does not create a backup until a valid primary save is replaced', async () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = 'PrimaryOnly';

    await expect(saveGame(gameState, saveFilePath)).resolves.toBe(true);
    await expect(loadGame(saveFilePath)).resolves.toMatchObject({
      sram: expect.objectContaining({
        player_name: 'PrimaryOnly',
      }),
    });
    await expect(fs.readFile(backupPath, 'utf-8')).rejects.toThrow();
  });

  it('keeps the previous valid save as a backup when replacing it', async () => {
    const firstState = createInitialGameState();
    firstState.sram.player_name = 'FirstSave';
    const secondState = createInitialGameState();
    secondState.sram.player_name = 'SecondSave';

    await expect(saveGame(firstState, saveFilePath)).resolves.toBe(true);
    await expect(saveGame(secondState, saveFilePath)).resolves.toBe(true);

    await expect(loadGame(saveFilePath)).resolves.toMatchObject({
      sram: expect.objectContaining({
        player_name: 'SecondSave',
      }),
    });

    const backup = JSON.parse(await fs.readFile(backupPath, 'utf-8')) as GameState;
    expect(backup.sram.player_name).toBe('FirstSave');
  });

  it('keeps the primary save readable when a replacement write fails', async () => {
    const firstState = createInitialGameState();
    firstState.sram.player_name = 'StableSave';
    const secondState = createInitialGameState();
    secondState.sram.player_name = 'FailedReplacement';

    await expect(saveGame(firstState, saveFilePath)).resolves.toBe(true);

    const originalWriteFile = fs.writeFile.bind(fs);
    jest.spyOn(fs, 'writeFile').mockImplementation(async (...args) => {
      if (String(args[0]).startsWith(`${savePath}.`) && String(args[0]).endsWith('.tmp')) {
        throw new Error('disk write interrupted');
      }
      return originalWriteFile(...args);
    });

    await expect(saveGame(secondState, saveFilePath)).rejects.toThrow('Failed to save game');
    await expect(loadGame(saveFilePath)).resolves.toMatchObject({
      sram: expect.objectContaining({
        player_name: 'StableSave',
      }),
    });
  });
});

describe('loadGame without backup recovery', () => {
  const saveFilePath = 'test-save-backup-recovery.sav';
  const savePath = path.resolve(process.cwd(), saveFilePath);

  afterEach(async () => {
    await deleteSaveGame(saveFilePath);
  });

  it('throws when the primary save is corrupted', async () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = 'NoRecovery';

    await saveGame(gameState, saveFilePath);
    await fs.writeFile(savePath, '{not valid json', 'utf-8');

    await expect(loadGame(saveFilePath)).rejects.toThrow(SaveGameValidationError);
  });

  it('throws when the primary save is missing', async () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = 'MissingPrimary';

    await saveGame(gameState, saveFilePath);
    await fs.unlink(savePath);

    await expect(loadGame(saveFilePath)).rejects.toThrow(SaveFileNotFoundError);
  });
});

describe('saveGame integrity validation', () => {
  const saveFilePath = 'test-save-invalid.sav';

  afterEach(async () => {
    await deleteSaveGame(saveFilePath);
  });

  it('rejects tampered DVs', async () => {
    const gameState = createInitialGameState();
    const pokemon = createPokemon(gameState, ensureSpecies('CYNDAQUIL'), 5);
    pokemon.dvs.attack = 2;
    pokemon.dvs.defense = 2;
    pokemon.dvs.speed = 2;
    pokemon.dvs.special = 2;
    pokemon.dvs.hp = 15;
    gameState.sram.party.pokemon = [pokemon, null, null, null, null, null];

    await expect(saveGame(gameState, saveFilePath)).rejects.toThrow(SaveGameValidationError);
  });

  it('rejects experience that exceeds the next level threshold', async () => {
    const gameState = createInitialGameState();
    const pokemon = createPokemon(gameState, ensureSpecies('TOTODILE'), 5);
    pokemon.experience = calculateExperience(
      pokemon.species.growth_rate,
      pokemon.level + 1
    );
    gameState.sram.party.pokemon = [pokemon, null, null, null, null, null];

    await expect(saveGame(gameState, saveFilePath)).rejects.toThrow(SaveGameValidationError);
  });

  it('rejects coin totals above the ASM cap', async () => {
    const gameState = createInitialGameState();
    gameState.sram.coins = MAX_COINS + 1;
    await expect(saveGame(gameState, saveFilePath)).rejects.toThrow(SaveGameValidationError);
  });

  it('rejects item stacks above the stack limit', async () => {
    const gameState = createInitialGameState();
    gameState.sram.items = { POTION: MAX_ITEM_STACK + 1 };
    await expect(saveGame(gameState, saveFilePath)).rejects.toThrow(SaveGameValidationError);
  });

  it('rejects key items with quantity greater than one', async () => {
    const gameState = createInitialGameState();
    gameState.sram.key_items = { BICYCLE: 2 };
    await expect(saveGame(gameState, saveFilePath)).rejects.toThrow(SaveGameValidationError);
  });
});

describe('saveGameWithHistory', () => {
  const primarySlot = MANUAL_SAVE_SLOT;
  const historySlots = MANUAL_SAVE_HISTORY_SLOTS;

  const cleanupHistory = async () => {
    await deleteSaveGame(primarySlot);
    for (const slot of historySlots) {
      await deleteSaveGame(slot);
    }
  };

  afterEach(async () => {
    await cleanupHistory();
  });

  it('rotates the previous manual save into history', async () => {
    const firstState = createInitialGameState();
    firstState.sram.player_name = 'First';
    await saveGame(firstState, primarySlot);

    const secondState = createInitialGameState();
    secondState.sram.player_name = 'Second';
    await saveGameWithHistory(secondState, primarySlot, historySlots);

    const loadedPrimary = await loadGame(primarySlot);
    const loadedHistory = await loadGame(historySlots[0]);
    expect(loadedPrimary.sram.player_name).toBe('Second');
    expect(loadedHistory.sram.player_name).toBe('First');
  });

  it('shifts older manual saves down the history list', async () => {
    const firstState = createInitialGameState();
    firstState.sram.player_name = 'One';
    await saveGame(firstState, primarySlot);

    const secondState = createInitialGameState();
    secondState.sram.player_name = 'Two';
    await saveGameWithHistory(secondState, primarySlot, historySlots);

    const thirdState = createInitialGameState();
    thirdState.sram.player_name = 'Three';
    await saveGameWithHistory(thirdState, primarySlot, historySlots);

    const loadedPrimary = await loadGame(primarySlot);
    const loadedRecent = await loadGame(historySlots[0]);
    const loadedOlder = await loadGame(historySlots[1]);
    expect(loadedPrimary.sram.player_name).toBe('Three');
    expect(loadedRecent.sram.player_name).toBe('Two');
    expect(loadedOlder.sram.player_name).toBe('One');
  });
});

describe('normalizeSaveSnapshot', () => {
  it('preserves pokedex_seen bitfield byte order', () => {
    const gameState = createInitialGameState();
    gameState.sram.pokedex_seen = [0x80, 0x01, 0x40];
    gameState.sram.pokedex_caught = new Set([25, 1, 7]);

    const normalized = normalizeSaveSnapshot(gameState, 'unit-test');
    const sram = normalized.sram as Record<string, unknown>;

    expect(sram.pokedex_seen).toEqual([0x80, 0x01, 0x40]);
    expect(sram.pokedex_caught).toEqual([1, 7, 25]);
  });

  it('rejects invalid textbox frame values instead of clamping them into range', () => {
    const gameState = createInitialGameState();
    gameState.sram.options.frame = 99 as never;

    expect(() => normalizeSaveSnapshot(gameState, 'unit-test')).toThrow(SaveGameValidationError);
  });
});
