import { loadAllMoves } from '@pokecrystal/core/core/data-loader';
import { MoveEffect, MoveName } from '@pokecrystal/core/core/enums';
import type { Move } from '@pokecrystal/core/core/models';
import { animation_label_for_move } from '@pokecrystal/core/ui/overlays/battle-animation-util';

export type BattleSkillCoverageStatus =
  | 'complete'
  | 'delegated_shared'
  | 'mechanically_incomplete'
  | 'animation_incomplete'
  | 'sequencing_incomplete';

export interface BattleSkillAuditEntry {
  move_name: MoveName;
  effect: MoveEffect;
  animation_label: string | null;
  status: BattleSkillCoverageStatus;
  mechanic_owner: 'move-effects' | 'move-execution' | 'shared-system' | 'incomplete';
  notes: string[];
}

const SHARED_EXECUTION_EFFECTS = new Set<MoveEffect>([
  MoveEffect.ALWAYS_HIT,
  MoveEffect.COUNTER,
  MoveEffect.FALSE_SWIPE,
  MoveEffect.FRUSTRATION,
  MoveEffect.FURY_CUTTER,
  MoveEffect.HIDDEN_POWER,
  MoveEffect.LEVEL_DAMAGE,
  MoveEffect.MAGNITUDE,
  MoveEffect.MIRROR_COAT,
  MoveEffect.PAY_DAY,
  MoveEffect.PRESENT,
  MoveEffect.PSYWAVE,
  MoveEffect.RETURN,
  MoveEffect.REVERSAL,
  MoveEffect.ROLLOUT,
  MoveEffect.STATIC_DAMAGE,
  MoveEffect.SUPER_FANG,
  MoveEffect.THUNDER,
]);

const KNOWN_INCOMPLETE_EFFECTS = new Set<MoveEffect>([
  MoveEffect.BATON_PASS,
  MoveEffect.BEAT_UP,
  MoveEffect.CONVERSION,
  MoveEffect.DOUBLE_HIT,
  MoveEffect.EARTHQUAKE,
  MoveEffect.FLAME_WHEEL,
  MoveEffect.FLY,
  MoveEffect.FORCE_SWITCH,
  MoveEffect.GUST,
  MoveEffect.HYPER_BEAM,
  MoveEffect.JUMP_KICK,
  MoveEffect.METRONOME,
  MoveEffect.MIMIC,
  MoveEffect.MIRROR_MOVE,
  MoveEffect.MULTI_HIT,
  MoveEffect.OHKO,
  MoveEffect.PAIN_SPLIT,
  MoveEffect.POISON_MULTI_HIT,
  MoveEffect.PRIORITY_HIT,
  MoveEffect.PURSUIT,
  MoveEffect.RAMPAGE,
  MoveEffect.RAZOR_WIND,
  MoveEffect.SACRED_FIRE,
  MoveEffect.SKETCH,
  MoveEffect.SKULL_BASH,
  MoveEffect.SKY_ATTACK,
  MoveEffect.SLEEP_TALK,
  MoveEffect.SNORE,
  MoveEffect.SOLARBEAM,
  MoveEffect.STOMP,
  MoveEffect.TWISTER,
]);

const DIRECT_MOVE_EFFECTS = new Set<MoveEffect>(
  Object.values(MoveEffect).filter(
    (effect) =>
      !SHARED_EXECUTION_EFFECTS.has(effect) &&
      !KNOWN_INCOMPLETE_EFFECTS.has(effect),
  ),
);

const sort_moves = (entries: Array<[MoveName, Move]>): Array<[MoveName, Move]> =>
  entries.sort(([left], [right]) => String(left).localeCompare(String(right)));

export const build_battle_skill_audit = (): BattleSkillAuditEntry[] => {
  const moves = loadAllMoves();
  return sort_moves(Array.from(moves.entries()) as Array<[MoveName, Move]>).map(([move_name, move]) => {
    let animation_label: string | null = null;
    const notes: string[] = [];
    try {
      animation_label = animation_label_for_move(move_name);
    } catch (error) {
      notes.push(error instanceof Error ? error.message : String(error));
    }

    if (animation_label === null) {
      return {
        move_name,
        effect: move.effect,
        animation_label,
        status: 'animation_incomplete',
        mechanic_owner: 'incomplete',
        notes,
      };
    }

    if (KNOWN_INCOMPLETE_EFFECTS.has(move.effect)) {
      notes.push(`Effect ${move.effect} still depends on incomplete shared battle-skill handling.`);
      return {
        move_name,
        effect: move.effect,
        animation_label,
        status: 'mechanically_incomplete',
        mechanic_owner: 'incomplete',
        notes,
      };
    }

    if (SHARED_EXECUTION_EFFECTS.has(move.effect)) {
      notes.push(`Primary behavior for ${move.effect} is dispatched outside applyMoveEffect.`);
      return {
        move_name,
        effect: move.effect,
        animation_label,
        status: 'delegated_shared',
        mechanic_owner: 'move-execution',
        notes,
      };
    }

    if (DIRECT_MOVE_EFFECTS.has(move.effect)) {
      return {
        move_name,
        effect: move.effect,
        animation_label,
        status: 'complete',
        mechanic_owner: 'move-effects',
        notes,
      };
    }

    notes.push(`Effect ${move.effect} is not classified by the battle skill audit.`);
    return {
      move_name,
      effect: move.effect,
      animation_label,
      status: 'mechanically_incomplete',
      mechanic_owner: 'incomplete',
      notes,
    };
  });
};

export const summarize_battle_skill_audit = (
  entries: BattleSkillAuditEntry[],
): Record<BattleSkillCoverageStatus, number> => {
  return entries.reduce<Record<BattleSkillCoverageStatus, number>>(
    (summary, entry) => {
      summary[entry.status] += 1;
      return summary;
    },
    {
      complete: 0,
      delegated_shared: 0,
      mechanically_incomplete: 0,
      animation_incomplete: 0,
      sequencing_incomplete: 0,
    },
  );
};
