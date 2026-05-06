import { Surface } from '../surface';
import { BattleHardwareRegisters } from './_battle-vram';
import { TrainerExitAnimation } from './battle-intro';
import {
  INTRO_BOTTOM_LINES,
  INTRO_MID_LINES,
  INTRO_TOP_LINES,
  START_MID_OFFSET,
  START_TOP_OFFSET,
  TOTAL_FRAMES,
  TrainerEntranceAnimation,
} from './trainer-entrance';

const TILE_SIZE = 8;

describe('TrainerEntranceAnimation', () => {
  it('slides trainer sprites from the ASM start offset into place', () => {
    const screen = new Surface(160, 144);
    const player = new Surface(48, 48);
    const enemy = new Surface(56, 56);
    const background = new Surface(160, 144);
    const hardware = new BattleHardwareRegisters();
    const animation = new TrainerEntranceAnimation({
      player_surface: player,
      enemy_surface: enemy,
      background_surface: background,
      hardware,
      strict_player_size: true,
    });
    const blitSpy = jest.spyOn(screen, 'blit');

    animation.draw(screen);

    const playerCall = blitSpy.mock.calls.find((call) => call[0] === player);
    const enemyCall = blitSpy.mock.calls.find((call) => call[0] === enemy);
    expect(playerCall?.[1]).toEqual([2 * TILE_SIZE - START_TOP_OFFSET, 6 * TILE_SIZE]);
    expect(enemyCall?.[1]).toEqual([12 * TILE_SIZE + START_TOP_OFFSET, 0]);

    for (let i = 1; i < TOTAL_FRAMES - 1; i += 1) {
      animation.draw(screen);
    }
    blitSpy.mockClear();
    animation.draw(screen);

    const finalPlayerCall = blitSpy.mock.calls.find((call) => call[0] === player);
    const finalEnemyCall = blitSpy.mock.calls.find((call) => call[0] === enemy);
    expect(finalPlayerCall?.[1]).toEqual([2 * TILE_SIZE, 6 * TILE_SIZE]);
    expect(finalEnemyCall?.[1]).toEqual([12 * TILE_SIZE, 0]);

    blitSpy.mockRestore();
  });

  it('applies the ASM scanline scroll split for the intro background', () => {
    const screen = new Surface(160, 144);
    const player = new Surface(48, 48);
    const enemy = new Surface(56, 56);
    const background = new Surface(512, 144);
    const animation = new TrainerEntranceAnimation({
      player_surface: player,
      enemy_surface: enemy,
      background_surface: background,
      strict_player_size: true,
    });
    const blitSpy = jest.spyOn(screen, 'blit');

    animation.draw(screen);

    const bgCalls = blitSpy.mock.calls.filter((call) => call[0] === background);
    expect(bgCalls).toHaveLength(3);

    const [topCall, midCall, bottomCall] = bgCalls;
    expect(topCall?.[1]).toEqual([0, 0]);
    expect(topCall?.[2]).toMatchObject({
      x: START_TOP_OFFSET,
      y: 0,
      width: 160,
      height: INTRO_TOP_LINES,
    });
    expect(midCall?.[1]).toEqual([0, INTRO_TOP_LINES]);
    expect(midCall?.[2]).toMatchObject({
      x: START_MID_OFFSET,
      y: INTRO_TOP_LINES,
      width: 160,
      height: INTRO_MID_LINES,
    });
    expect(bottomCall?.[1]).toEqual([0, INTRO_TOP_LINES + INTRO_MID_LINES]);
    expect(bottomCall?.[2]).toMatchObject({
      x: 0,
      y: INTRO_TOP_LINES + INTRO_MID_LINES,
      width: 160,
      height: INTRO_BOTTOM_LINES,
    });

    blitSpy.mockRestore();
  });

  it('restores scroll registers after the intro finishes', () => {
    const screen = new Surface(160, 144);
    const player = new Surface(48, 48);
    const enemy = new Surface(56, 56);
    const hardware = new BattleHardwareRegisters();
    const animation = new TrainerEntranceAnimation({
      player_surface: player,
      enemy_surface: enemy,
      hardware,
      strict_player_size: true,
    });

    for (let i = 0; i < TOTAL_FRAMES; i += 1) {
      animation.draw(screen);
    }

    expect(hardware.scx).toBe(0);
    expect(hardware.scy).toBe(0);
  });
});


describe('TrainerExitAnimation', () => {
  const buildExit = (side: 'player' | 'enemy'): TrainerExitAnimation =>
    new TrainerExitAnimation(
      {
        get_sprite_surface: () => null,
        _apply_colorkey_transparency: (surface: Surface) => surface,
        _get_pokemon_frame_surface: () => null,
      },
      { hardware: new BattleHardwareRegisters(), side }
    );

  it('matches SlideBattlePicOut cadence for player side (9 shifts with 2-frame gaps)', () => {
    const animation = buildExit('player');
    const screen = new Surface(160, 144);

    for (let frame = 1; frame <= 27; frame += 1) {
      animation.draw(screen);
      if (frame === 1) expect(animation.x_offset).toBe(-8);
      if (frame === 2) expect(animation.x_offset).toBe(-8);
      if (frame === 3) expect(animation.x_offset).toBe(-8);
      if (frame === 4) expect(animation.x_offset).toBe(-16);
      if (frame === 25) expect(animation.x_offset).toBe(-72);
      if (frame === 27) expect(animation.is_finished).toBe(true);
    }
  });

  it('matches SlideBattlePicOut cadence for enemy side (8 shifts with 2-frame gaps)', () => {
    const animation = buildExit('enemy');
    const screen = new Surface(160, 144);

    for (let frame = 1; frame <= 24; frame += 1) {
      animation.draw(screen);
      if (frame === 1) expect(animation.x_offset).toBe(8);
      if (frame === 4) expect(animation.x_offset).toBe(16);
      if (frame === 22) expect(animation.x_offset).toBe(64);
      if (frame === 24) expect(animation.is_finished).toBe(true);
    }
  });
});
