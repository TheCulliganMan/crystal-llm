import { Animation, AnimationCommand, AnimationSchema } from './_battle-animation-state';
import { readJsonAssetSync, readTextAssetSync } from '../../core/asset-reader';
import { getAnimationsOutputPath, getDataDir, getDisassemblyRoot } from '../../core/paths';

const FALLTHROUGH_ALIASES: Record<string, string> = {
  BattleAnim_Dummy: 'BattleAnim_MirrorMove',
  BattleAnim_Gust: 'BattleAnim_Sonicboom',
  BattleAnim_Poisonpowder: 'BattleAnim_StunSpore',
  BattleAnim_SleepPowder: 'BattleAnim_StunSpore',
  BattleAnim_Spore: 'BattleAnim_StunSpore',
};

let cachedAnimations: Map<string, Animation> | null = null;
let cachedTable: string[] | null = null;
const BATTLE_ANIMATION_TABLE_JSON_PATH = `${getDataDir()}/battle_animation_table.json`;

const missingAnimationSourcesError = (jsonPath: string): Error =>
  new Error(
    `Generated battle animation scripts are required for the asset-only runtime: missing or invalid ${jsonPath}.`
  );

const missingAnimationTableError = (jsonPath: string): Error =>
  new Error(
    `Generated battle animation table is required for the asset-only runtime: missing or invalid ${jsonPath}.`
  );

export const load_animation_table = (): string[] => {
  if (cachedTable) {
    return cachedTable;
  }
  let parsed: unknown;
  try {
    parsed = readJsonAssetSync<unknown>(BATTLE_ANIMATION_TABLE_JSON_PATH);
  } catch {
    throw missingAnimationTableError(BATTLE_ANIMATION_TABLE_JSON_PATH);
  }
  if (!Array.isArray(parsed)) {
    throw missingAnimationTableError(BATTLE_ANIMATION_TABLE_JSON_PATH);
  }
  cachedTable = parsed.filter(
    (entry): entry is string => typeof entry === 'string' && entry.length > 0
  );
  if (!cachedTable.length) {
    throw missingAnimationTableError(BATTLE_ANIMATION_TABLE_JSON_PATH);
  }
  return cachedTable;
};

export const load_animation_scripts = (): Map<string, Animation> => {
  if (cachedAnimations) {
    return cachedAnimations;
  }
  cachedAnimations = new Map();

  const jsonPath = getAnimationsOutputPath();
  let raw: Record<string, string[]> = {};
  try {
    raw = readJsonAssetSync<Record<string, string[]>>(jsonPath);
  } catch {
    throw missingAnimationSourcesError(jsonPath);
  }
  if (!raw || !Object.keys(raw).length) {
    throw missingAnimationSourcesError(jsonPath);
  }
  let animations = convertJsonScripts(raw);
  const asmSource = readBattleAnimationAsm();
  if (asmSource) {
    const asmAnimations = parseAsmScripts(asmSource);
    if (asmAnimations.size) {
      animations = asmAnimations;
    }
  }
  applyFallthroughAliases(animations);
  injectCoreScripts(animations);
  cachedAnimations = animations;
  return cachedAnimations;
};

export const register_animation_scripts = (entries: Iterable<Animation>): void => {
  const map = new Map<string, Animation>();
  for (const entry of entries) {
    const parsed = AnimationSchema.parse(entry);
    map.set(parsed.name, parsed);
  }
  cachedAnimations = map;
};

export const register_animation_table = (table: string[]): void => {
  cachedTable = [...table];
};

export const reset_animation_loader_cache = (): void => {
  cachedAnimations = null;
  cachedTable = null;
};

const convertJsonScripts = (raw: Record<string, string[]>): Map<string, Animation> => {
  const animations = new Map<string, Animation>();
  for (const [name, scriptLines] of Object.entries(raw)) {
    const commands: AnimationCommand[] = [];
    const labels: Record<string, number> = {};
    for (const line of scriptLines) {
      const trimmed = line.trim();
      if (trimmed.startsWith('.') && !trimmed.includes(' ')) {
        labels[trimmed] = commands.length;
        continue;
      }
      const parts = splitCommandLine(line);
      if (!parts.length) {
        continue;
      }
      const [command, ...args] = parts;
      commands.push({ command, args });
    }
    labels[name] = 0;
    animations.set(name, { name, script: commands, labels });
  }
  return animations;
};

const battleAnimationAsmCandidates = (): string[] => {
  return [`${getDisassemblyRoot()}/data/moves/animations.asm`];
};

const readBattleAnimationAsm = (): string | null => {
  for (const candidate of battleAnimationAsmCandidates()) {
    try {
      return readTextAssetSync(candidate);
    } catch {
      continue;
    }
  }
  return null;
};

const parseAsmScripts = (source: string): Map<string, Animation> => {
  const animations = new Map<string, Animation>();
  type ScriptSegment = Animation & { globalLabels: Record<string, number> };
  let current: ScriptSegment | null = null;
  let ended = true;

  const flushCurrent = (): void => {
    if (!current || !current.script.length) {
      current = null;
      return;
    }
    const root = {
      name: current.name,
      script: current.script,
      labels: current.labels,
    };
    animations.set(current.name, root);
    for (const [label, offset] of Object.entries(current.globalLabels)) {
      if (label === current.name || offset < 0) {
        continue;
      }
      const labels = Object.fromEntries(
        Object.entries(current.labels)
          .filter(([, position]) => position >= offset)
          .map(([name, position]) => [name, position - offset])
      );
      labels[label] = 0;
      animations.set(label, {
        name: label,
        script: current.script.slice(offset),
        labels,
      });
    }
    current = null;
  };

  for (const rawLine of source.split(/\r?\n/)) {
    const line = rawLine.split(';', 1)[0].trim();
    if (!line) {
      continue;
    }
    if (/^[A-Za-z_][A-Za-z0-9_]*:$/.test(line)) {
      const label = line.slice(0, -1);
      if (!label.startsWith('BattleAnim')) {
        flushCurrent();
        ended = true;
        continue;
      }
      if (current && !ended) {
        current.labels[label] = current.script.length;
        current.globalLabels[label] = current.script.length;
        continue;
      }
      flushCurrent();
      current = {
        name: label,
        script: [],
        labels: { [label]: 0 },
        globalLabels: { [label]: 0 },
      };
      ended = false;
      continue;
    }
    if (!current) {
      continue;
    }
    if (/^\.[A-Za-z0-9_]+:?$/.test(line)) {
      current.labels[line.replace(/:$/, '')] = current.script.length;
      continue;
    }
    if (line.startsWith('assert_table_length') || line.startsWith('BattleAnimations::')) {
      flushCurrent();
      ended = true;
      continue;
    }
    const parts = splitCommandLine(line);
    if (parts.length) {
      const [command, ...args] = parts;
      current.script.push({ command, args });
      ended = command.toLowerCase() === 'anim_ret';
    }
  }

  flushCurrent();
  return animations;
};

const applyFallthroughAliases = (animations: Map<string, Animation>): void => {
  for (const [alias, targetName] of Object.entries(FALLTHROUGH_ALIASES)) {
    const target = animations.get(targetName);
    if (!target || !target.script.length) {
      continue;
    }
    const existing = animations.get(alias);
    if (existing && existing.script.length) {
      continue;
    }
    const labels = { ...target.labels };
    if (labels[alias] === undefined) {
      labels[alias] = 0;
    }
    animations.set(alias, {
      name: alias,
      script: [...target.script],
      labels,
    });
  }
};

const injectCoreScripts = (animations: Map<string, Animation>): void => {
  if (animations.has('BattleAnim_FaintMon')) {
    return;
  }
  animations.set('BattleAnim_FaintMon', {
    name: 'BattleAnim_FaintMon',
    script: [
      { command: 'anim_sound', args: ['0', '0', 'SFX_FAINT'] },
      {
        command: 'anim_bgeffect',
        args: ['BATTLE_BG_EFFECT_FAINT_MON', '$14', 'BG_EFFECT_USER', '$04'],
      },
      { command: 'anim_wait', args: ['24'] },
      { command: 'anim_ret', args: [] },
    ],
    labels: { BattleAnim_FaintMon: 0 },
  });
};

const splitCommandLine = (line: string): string[] => {
  const commandText = line.split(';', 1)[0].trim();
  if (!commandText) {
    return [];
  }
  return commandText.split(/\s+/).map((token) => token.replace(/,+$/, ''));
};
