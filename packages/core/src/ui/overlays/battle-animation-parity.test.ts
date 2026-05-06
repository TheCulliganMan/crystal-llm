import fs from 'fs';
import path from 'path';
import { update_animation_sprite } from './battle-anim-runtime';
import { AnimationPlayer } from './battle-animation';
import { load_animation_scripts } from './battle-animation-loader';
import { AnimationSpriteSchema } from './battle-animation-state';
import { buildParsedBgEffect, make_effect } from './battle-bg-effects';

const configureDisassemblyRoot = () => {
  const disassemblyRoot = path.resolve(
    __dirname,
    '../../../../../vendor/pokecrystal',
  );
  process.env.POKECRYSTAL_DISASSEMBLY_ROOT = disassemblyRoot;
  expect(fs.existsSync(disassemblyRoot)).toBe(true);
};

describe('battle-animation parity', () => {
  beforeAll(() => {
    configureDisassemblyRoot();
  });

  it('implements every opcode used by loaded battle animation scripts', () => {
    const player = new AnimationPlayer({});
    const handlers = (player as unknown as {
      command_handlers: Record<string, unknown>;
    }).command_handlers;
    const missing = new Set<string>();

    for (const animation of load_animation_scripts().values()) {
      if (!animation.name.startsWith('BattleAnim_')) {
        continue;
      }
      for (const command of animation.script) {
        const opcode = command.command.toLowerCase();
        if (!handlers[opcode]) {
          missing.add(opcode);
        }
      }
    }

    expect(Array.from(missing)).toEqual([]);
  });

  it('resolves every anim_obj/object frameset/oamset/function reference', () => {
    const player = new AnimationPlayer({});
    const data = player.anim_data;
    const scripts = load_animation_scripts();
    const dataAny = data as unknown as {
      oamSets: Map<string, unknown>;
    };
    const oamSets = dataAny.oamSets;

    const missingObjects = new Set<string>();
    for (const animation of scripts.values()) {
      if (!animation.name.startsWith('BattleAnim_')) {
        continue;
      }
      for (const command of animation.script) {
        if (command.command.toLowerCase() !== 'anim_obj') {
          continue;
        }
        const objectId = command.args[0]?.trim().toUpperCase() ?? '';
        if (!objectId || data.object_defs.has(objectId)) {
          continue;
        }
        missingObjects.add(`${animation.name}:${objectId}`);
      }
    }

    const missingFramesets: string[] = [];
    const missingOamSets: string[] = [];
    const missingFunctions: string[] = [];
    const handlerErrors: string[] = [];
    const checkedFunctions = new Set<string>();

    for (const objectDef of data.object_defs.values()) {
      const frames = data.get_frameset_frames(objectDef.frameset) as Array<{
        command: string;
        oam_set: string | null;
      }> | null;
      if (!frames || !frames.length) {
        missingFramesets.push(`${objectDef.object_id}:${objectDef.frameset}`);
        continue;
      }
      for (const entry of frames) {
        if (entry.command !== 'frame' || !entry.oam_set) {
          continue;
        }
        if (!oamSets.has(entry.oam_set)) {
          missingOamSets.push(
            `${objectDef.object_id}:${objectDef.frameset}:${entry.oam_set}`,
          );
        }
      }
      if (!objectDef.function || checkedFunctions.has(objectDef.function)) {
        continue;
      }
      checkedFunctions.add(objectDef.function);
      const sprite = AnimationSpriteSchema.parse({
        object_id: objectDef.object_id,
        function_id: objectDef.function,
        x: 0,
        y: 0,
        param: 0,
      });
      try {
        update_animation_sprite(sprite);
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        if (message.startsWith('Missing animation handler for')) {
          missingFunctions.push(objectDef.function);
          continue;
        }
        handlerErrors.push(`${objectDef.function}: ${message}`);
      }
    }

    expect(Array.from(missingObjects)).toEqual([]);
    expect(missingFramesets).toEqual([]);
    expect(missingOamSets).toEqual([]);
    expect(missingFunctions).toEqual([]);
    expect(handlerErrors).toEqual([]);
  });

  it('keeps healing move scripts ASM-faithful for cadence, cues, and state transitions', () => {
    const scripts = load_animation_scripts();
    const expectScript = (name: string): Array<{ command: string; args: string[] }> => {
      const animation = scripts.get(name);
      expect(animation).toBeDefined();
      return animation?.script ?? [];
    };

    const recover = expectScript('BattleAnim_Recover');
    expect(recover.slice(0, 4).map((command) => command.command)).toEqual([
      'anim_1gfx',
      'anim_call',
      'anim_sound',
      'anim_bgeffect',
    ]);
    expect(recover.filter((command) => command.command === 'anim_obj' && command.args[0] === 'BATTLE_ANIM_OBJ_RECOVER').map((command) => command.args[3]))
      .toEqual(['$30', '$31', '$32', '$33', '$34', '$35', '$36', '$37']);

    const softboiled = expectScript('BattleAnim_Softboiled');
    expect(softboiled.some((command) => command.command === 'anim_sound' && command.args[2] === 'SFX_METRONOME')).toBe(true);
    expect(softboiled.some((command) => command.command === 'anim_loop' && command.args[0] === '8')).toBe(true);

    const milkDrink = expectScript('BattleAnim_MilkDrink');
    expect(milkDrink.some((command) => command.command === 'anim_sound' && command.args[2] === 'SFX_MILK_DRINK')).toBe(true);
    expect(milkDrink.some((command) => command.command === 'anim_loop' && command.args[0] === '8')).toBe(true);

    const healBell = expectScript('BattleAnim_HealBell');
    expect(healBell.some((command) => command.command === 'anim_loop' && command.args[0] === '4')).toBe(true);
    expect(healBell.filter((command) => command.command === 'anim_obj' && command.args[0] === 'BATTLE_ANIM_OBJ_HEAL_BELL_NOTE').map((command) => command.args[3]))
      .toEqual(['$0', '$1', '$2', '$0', '$2']);

    const synthesis = expectScript('BattleAnim_Synthesis');
    expect(synthesis.some((command) => command.command === 'anim_sound' && command.args[2] === 'SFX_OUTRAGE')).toBe(true);
    expect(synthesis.some((command) => command.command === 'anim_wait' && command.args[0] === '72')).toBe(true);

    const moonlight = expectScript('BattleAnim_Moonlight');
    expect(moonlight.filter((command) => command.command === 'anim_obj' && command.args[0] === 'BATTLE_ANIM_OBJ_MOONLIGHT').length).toBe(5);
    expect(moonlight.some((command) => command.command === 'anim_sound' && command.args[2] === 'SFX_MOONLIGHT')).toBe(true);
  });

  it('implements every bg effect referenced by loaded battle animation scripts', () => {
    for (const animation of load_animation_scripts().values()) {
      if (!animation.name.startsWith('BattleAnim_')) {
        continue;
      }
      for (const command of animation.script) {
        if (command.command.toLowerCase() !== 'anim_bgeffect') {
          continue;
        }
        const [name, durationToken = '$0', rawTurn = '$0', paramToken = '$0'] = command.args;
        make_effect(buildParsedBgEffect({
          name,
          duration: parseInt(durationToken.replace(/^\$/, ''), 16) || 0,
          raw_turn: rawTurn,
          param: parseInt(paramToken.replace(/^\$/, ''), 16) || 0,
          is_player_move: true,
          turn_value: null,
        }));
      }
    }
  });

});
