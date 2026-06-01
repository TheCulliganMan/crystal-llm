import { DataLoader } from '../../core/data-loader';
import { LearnedMove, Pokemon } from '../../core/models';
import { MoveName } from '../../core/enums';
import { BattleContext, BattleStateEnum } from '../../engine/battle/battle/battle-context';
import { calculateExperience } from '../../engine/experience';
import { Evolution } from '../../engine/systems/evolution';
import * as battle_dialogue from './battle-dialogue';
import { BattleEvolutionCutscene } from './battle-evolution';
import {
  ExpBarAnimationState,
  LevelUpInfo,
  buildLevelQueue,
} from './battle-experience';
import { formatMoveName } from './battle-move-name';
import { clear_yes_no_prompt, show_yes_no_prompt } from './battle-ui-core';
import {
  ActiveEvolutionState,
  ActiveMoveLearnState,
  BattleUIState,
  MoveForgetMenuState,
  MoveLearningPhase,
  PendingEvolutionRequest,
  PendingMoveLearn,
} from './battle-ui-state';

export const HM_MOVE_NAMES = new Set<MoveName>([
  MoveName.CUT,
  MoveName.FLY,
  MoveName.SURF,
  MoveName.STRENGTH,
  MoveName.FLASH,
  MoveName.WHIRLPOOL,
  MoveName.WATERFALL,
]);

type ExpGainAnimationRequest = {
  pokemon: Pokemon;
  expGain: number;
};

const exp_animation_queue = (state: BattleUIState): ExpGainAnimationRequest[] => {
  if (!Array.isArray(state.exp_animation_queue)) {
    state.exp_animation_queue = [];
  }
  return state.exp_animation_queue as ExpGainAnimationRequest[];
};

const build_exp_animation = (
  pokemon: Pokemon,
  exp_gain: number
): ExpBarAnimationState | null => {
  if (exp_gain <= 0 || pokemon.level >= 100) {
    return null;
  }
  const growth = pokemon.species.growth_rate;
  if (!growth) {
    pokemon.experience += exp_gain;
    return null;
  }
  const maxExp = calculateExperience(growth, 100);
  const startExp = pokemon.experience;
  const target = Math.min(maxExp, startExp + exp_gain);
  const pending = buildLevelQueue(pokemon, target);
  const span = Math.max(1, target - startExp);
  const speed = Math.max(1, Math.floor(span / 48));
  return {
    pokemon,
    targetExp: target,
    pendingLevels: pending,
    speed,
  } as ExpBarAnimationState;
};

const start_next_exp_animation = (state: BattleUIState): boolean => {
  if (battle_dialogue.waiting_flag(state.dialogue)) {
    return false;
  }
  const queue = exp_animation_queue(state);
  while (queue.length) {
    const request = queue.shift();
    if (!request) {
      continue;
    }
    const anim = build_exp_animation(request.pokemon, request.expGain);
    if (anim) {
      state.exp_animation = anim;
      return true;
    }
  }
  return false;
};

export const enqueue_exp_gain = (
  state: BattleUIState,
  pokemon: Pokemon,
  exp_gain: number
): void => {
  if (state.exp_animation || exp_animation_queue(state).length > 0) {
    exp_animation_queue(state).push({ pokemon, expGain: exp_gain });
    return;
  }
  state.exp_animation = build_exp_animation(pokemon, exp_gain);
};

export const update_exp_animation = (state: BattleUIState): void => {
  const anim = state.exp_animation as ExpBarAnimationState | null | undefined;
  if (!anim) {
    start_next_exp_animation(state);
    return;
  }
  const pokemon = anim.pokemon;
  if (pokemon.experience >= anim.targetExp) {
    pokemon.experience = anim.targetExp;
    if (!anim.pendingLevels.length) {
      state.exp_animation = null;
    }
    return;
  }
  const step = Math.max(1, anim.speed);
  pokemon.experience = Math.min(anim.targetExp, pokemon.experience + step);
  while (anim.pendingLevels.length && pokemon.experience >= anim.pendingLevels[0].expThreshold) {
    const info = anim.pendingLevels.shift();
    if (!info) {
      break;
    }
    apply_level_up(state, pokemon, info);
  }
  if (pokemon.experience >= anim.targetExp && !anim.pendingLevels.length) {
    state.exp_animation = null;
  }
};

export const process_move_learning = (state: BattleUIState): void => {
  let process = state.active_move_learn ?? null;
  if (!process && state.pending_move_learns.length) {
    const request = state.pending_move_learns.shift();
    if (request) {
      state.active_move_learn = {
        pokemon: request.pokemon,
        move: { ...request.move },
        stage: MoveLearningPhase.ANNOUNCE,
        replace_index: null,
        pending_selection: null,
        forget_move_name: null,
      };
      process = state.active_move_learn;
    }
  }
  state.block_on_move_learning = Boolean(process || state.pending_move_learns.length);
  if (!process) {
    if (state.move_forget_menu) {
      close_move_forget_menu(state);
    }
    return;
  }

  const stage = process.stage;
  const pokemon = process.pokemon;
  const moveName = process.move.name;

  if (stage === MoveLearningPhase.ANNOUNCE) {
    const intro = `${display_name(pokemon)} is trying to learn ${formatMoveName(moveName)}!`;
    queue_battle_text(state, intro);
    process.stage = MoveLearningPhase.DECIDE;
    return;
  }
  if (stage === MoveLearningPhase.FORGET_PROMPT_RESULT) {
    const result = state.yes_no_prompt.result;
    if (result === null) {
      return;
    }
    clear_yes_no_prompt(state);
    process.stage = result ? MoveLearningPhase.FORGET_MENU_TEXT : MoveLearningPhase.STOP_PROMPT;
    return;
  }
  if (stage === MoveLearningPhase.STOP_PROMPT_RESULT) {
    const result = state.yes_no_prompt.result;
    if (result === null) {
      return;
    }
    clear_yes_no_prompt(state);
    process.stage = result ? MoveLearningPhase.DID_NOT_LEARN : MoveLearningPhase.ASK_FORGET;
    return;
  }
  if (stage === MoveLearningPhase.FORGET_MENU) {
    return;
  }
  if (battle_dialogue.waiting_flag(state.dialogue)) {
    return;
  }
  if (stage === MoveLearningPhase.HM_WARNING) {
    process.stage = MoveLearningPhase.FORGET_MENU_TEXT;
    return;
  }
  if (stage === MoveLearningPhase.DECIDE) {
    const moves = (pokemon.moves ?? []).filter(Boolean) as LearnedMove[];
    if (moves.length < 4) {
      process.stage = MoveLearningPhase.LEARN_NEW_MOVE;
    } else {
      process.stage = MoveLearningPhase.ASK_FORGET;
    }
    return;
  }
  if (stage === MoveLearningPhase.ASK_FORGET) {
    queue_battle_text(state, forget_prompt_text(pokemon, moveName));
    process.stage = MoveLearningPhase.WAIT_FORGET_PROMPT;
    return;
  }
  if (stage === MoveLearningPhase.WAIT_FORGET_PROMPT) {
    show_yes_no_prompt(state);
    process.stage = MoveLearningPhase.FORGET_PROMPT_RESULT;
    return;
  }
  if (stage === MoveLearningPhase.FORGET_MENU_TEXT) {
    queue_battle_text(state, 'Which move should be forgotten?');
    process.stage = MoveLearningPhase.PREPARE_FORGET_MENU;
    return;
  }
  if (stage === MoveLearningPhase.PREPARE_FORGET_MENU) {
    open_move_forget_menu(state, process);
    process.stage = MoveLearningPhase.FORGET_MENU;
    return;
  }
  if (stage === MoveLearningPhase.HANDLE_MENU_SELECTION) {
    const selection = process.pending_selection;
    process.pending_selection = null;
    const moves = (pokemon.moves ?? []).filter((m: LearnedMove | null): m is LearnedMove => m !== null);
    if (selection === null || selection === undefined || selection >= moves.length) {
      process.stage = MoveLearningPhase.STOP_PROMPT;
      return;
    }
    const selectedMove = moves[selection];
    if (is_hm_move(selectedMove.name)) {
      queue_battle_text(state, "HM moves can't be forgotten now.");
      process.stage = MoveLearningPhase.HM_WARNING;
      return;
    }
    process.replace_index = selection;
    process.forget_move_name = selectedMove.name;
    process.stage = MoveLearningPhase.FORGET_ANIMATION;
    return;
  }
  if (stage === MoveLearningPhase.STOP_PROMPT) {
    queue_battle_text(state, stop_learning_text(pokemon, moveName));
    process.stage = MoveLearningPhase.WAIT_STOP_PROMPT;
    return;
  }
  if (stage === MoveLearningPhase.WAIT_STOP_PROMPT) {
    show_yes_no_prompt(state);
    process.stage = MoveLearningPhase.STOP_PROMPT_RESULT;
    return;
  }
  if (stage === MoveLearningPhase.DID_NOT_LEARN) {
    queue_battle_text(state, did_not_learn_text(pokemon, moveName));
    process.stage = MoveLearningPhase.FINAL;
    return;
  }
  if (stage === MoveLearningPhase.FORGET_ANIMATION) {
    queue_battle_text(state, '1, 2 and...');
    if (process.forget_move_name) {
      queue_battle_text(state, forget_animation_text(pokemon, process.forget_move_name));
    }
    process.stage = MoveLearningPhase.WAIT_FORGET_ANIMATION;
    return;
  }
  if (stage === MoveLearningPhase.WAIT_FORGET_ANIMATION) {
    process.stage = MoveLearningPhase.LEARN_NEW_MOVE;
    return;
  }
  if (stage === MoveLearningPhase.LEARN_NEW_MOVE) {
    teach_move(state, pokemon, process.move, process.replace_index);
    process.stage = MoveLearningPhase.FINAL;
    return;
  }
  if (stage === MoveLearningPhase.FINAL) {
    if (state.move_forget_menu) {
      close_move_forget_menu(state);
    }
    state.active_move_learn = null;
    state.block_on_move_learning = Boolean(state.pending_move_learns.length);
  }
};

export const maybe_start_pending_evolution = (
  state: BattleUIState,
  battle_context: BattleContext
): void => {
  if (battle_context.currentState !== BattleStateEnum.BATTLE_END) {
    state.block_on_pending_evolution = false;
    return;
  }
  if (state.evolution_animation || state.active_evolution) {
    state.block_on_pending_evolution = true;
    return;
  }
  if (state.block_on_move_learning) {
    state.block_on_pending_evolution = true;
    return;
  }
  const loader = resolve_evolution_data_loader(state);
  if (!loader) {
    state.pending_evolutions = [];
    state.block_on_pending_evolution = false;
    return;
  }
  while (state.pending_evolutions.length) {
    const request = state.pending_evolutions.shift();
    if (!request) {
      continue;
    }
    const pokemon = request.pokemon;
    const evolution = new Evolution(pokemon, {
      data_loader: loader,
      time_of_day: (state.game_state.wram as { time_of_day?: string }).time_of_day,
      current_item: pokemon.item,
    });
    const candidate = evolution.check_for_evolution();
    if (!candidate) {
      continue;
    }
    const targetSpeciesId = candidate.species;
    const previousSpeciesId = pokemon.species.id;
    state.active_evolution = {
      pokemon,
      evolution,
      previous_species_id: previousSpeciesId,
      target_species_id: targetSpeciesId,
    } as ActiveEvolutionState;
    const text = evolution_text_for_id('EvolvingText', pokemon);
    if (text) {
      queue_battle_text(state, text);
    }
    start_evolution_animation(state, pokemon, previousSpeciesId, targetSpeciesId);
    state.block_on_pending_evolution = true;
    return;
  }
  state.block_on_pending_evolution = false;
};

export const complete_active_evolution = (state: BattleUIState, canceled: boolean): void => {
  const active = state.active_evolution;
  if (!active) {
    return;
  }
  const pokemon = active.pokemon;
  if (canceled) {
    const text = evolution_text_for_id('StoppedEvolvingText', pokemon);
    if (text) {
      queue_battle_text(state, text);
    }
  } else {
    const evolution = active.evolution;
    evolution.evolve(false);
    handle_evolution_events(state, pokemon, evolution.events);
  }
  state.active_evolution = null;
};

export const maybe_cancel_active_evolution = (state: BattleUIState, cancel_requested: boolean): boolean => {
  if (!cancel_requested) {
    return false;
  }
  if (!state.active_evolution) {
    return false;
  }
  state.evolution_animation = null;
  complete_active_evolution(state, true);
  return true;
};

const queue_battle_text = (state: BattleUIState, text: string): void => {
  if (battle_dialogue.enqueue_text(state.dialogue, text)) {
    battle_dialogue.push_wait(state.dialogue);
  }
};

const open_move_forget_menu = (state: BattleUIState, process: ActiveMoveLearnState): void => {
  const moves = (process.pokemon.moves ?? []).filter(Boolean) as LearnedMove[];
  const optionCount = Math.max(1, moves.length + 1);
  state.move_forget_menu = {
    selection: 0,
    option_count: optionCount,
  } as MoveForgetMenuState;
  state.manual_wait_override = true;
};

const close_move_forget_menu = (state: BattleUIState): void => {
  state.move_forget_menu = null;
  state.manual_wait_override = false;
};

const teach_move = (
  state: BattleUIState,
  pokemon: Pokemon,
  learned: LearnedMove,
  replace_index?: number | null
): void => {
  const clone: LearnedMove = { ...learned };
  const moveData = state._move_metadata.get(clone.name);
  if (moveData) {
    clone.current_pp = moveData.pp;
  }
  const moves = (pokemon.moves ?? []).filter((m: LearnedMove | null): m is LearnedMove => m !== null);
  if (replace_index !== undefined && replace_index !== null && replace_index >= 0 && replace_index < moves.length) {
    moves[replace_index] = clone;
  } else {
    if (moves.length >= 4) {
      moves.splice(4);
    }
    moves.push(clone);
  }
  pokemon.moves = moves;
  queue_battle_text(state, `${display_name(pokemon)} learned ${formatMoveName(clone.name)}!`);
};

const is_hm_move = (move_name?: MoveName | null): boolean => {
  if (!move_name) {
    return false;
  }
  return HM_MOVE_NAMES.has(move_name);
};

const apply_level_up = (state: BattleUIState, pokemon: Pokemon, info: LevelUpInfo): void => {
  pokemon.level = info.level;
  const oldHp = pokemon.hp;
  pokemon.max_hp = info.stats.max_hp;
  pokemon.attack = info.stats.attack;
  pokemon.defense = info.stats.defense;
  pokemon.speed = info.stats.speed;
  pokemon.special_attack = info.stats.special_attack;
  pokemon.special_defense = info.stats.special_defense;
  pokemon.hp = Math.min(pokemon.max_hp, Math.max(1, oldHp + info.hpDelta));
  queue_battle_text(state, `${pokemon.nickname} grew to level ${info.level}!`);
  for (const learned of info.learnedMoves) {
    if (pokemon.moves?.some((move: LearnedMove | null) => move?.name === learned.name)) {
      continue;
    }
    schedule_move_learning(state, pokemon, learned);
  }

  schedule_pending_evolution(state, pokemon);
};

const schedule_move_learning = (state: BattleUIState, pokemon: Pokemon, learned: LearnedMove): void => {
  const clone: LearnedMove = { ...learned };
  state.pending_move_learns.push({ pokemon, move: clone } as PendingMoveLearn);
};

const schedule_pending_evolution = (state: BattleUIState, pokemon: Pokemon): void => {
  const loader = resolve_evolution_data_loader(state);
  if (!loader) {
    return;
  }
  const evolution = new Evolution(pokemon, {
    data_loader: loader,
    time_of_day: (state.game_state.wram as { time_of_day?: string }).time_of_day,
    current_item: pokemon.item,
  });
  if (!evolution.check_for_evolution()) {
    return;
  }
  if (state.pending_evolutions.some((request) => request.pokemon === pokemon)) {
    return;
  }
  state.pending_evolutions.push({ pokemon } as PendingEvolutionRequest);
};

const resolve_evolution_data_loader = (state: BattleUIState): DataLoader | null => {
  let loader = state.data_loader as DataLoader | null | undefined;
  if (!loader) {
    loader = new DataLoader();
    state.data_loader = loader;
  }
  if (typeof (loader as DataLoader).ensure_battle_data === 'function') {
    (loader as DataLoader).ensure_battle_data();
  }
  return loader;
};

const handle_evolution_events = (
  state: BattleUIState,
  pokemon: Pokemon,
  events: Array<Record<string, unknown>>,
  previous_species_id?: string | null
): void => {
  for (const event of events) {
    const eventType = event.type;
    if (eventType === 'text') {
      const text = evolution_text_for_id(event.id, pokemon);
      if (text) {
        queue_battle_text(state, text);
      }
    } else if (eventType === 'move') {
      const moveName = event.id as MoveName | undefined;
      if (moveName) {
        const moveText = formatMoveName(moveName);
        queue_battle_text(state, `${display_name(pokemon)} learned ${moveText}!`);
      }
    } else if (eventType === 'item') {
      queue_battle_text(state, 'The held item disappeared!');
    } else if (eventType === 'animation') {
      console.debug(`Unhandled evolution animation event ${event.id}`);
    } else {
      console.debug('Unhandled evolution event', event);
    }
  }
};

const start_evolution_animation = (
  state: BattleUIState,
  pokemon: Pokemon,
  previous_species_id: string | null,
  target_species_id: string
): void => {
  const previous = previous_species_id ?? pokemon.species.id;
  try {
    const cutscene = new BattleEvolutionCutscene(
      state.ui,
      state.audio_engine ?? null,
      previous,
      target_species_id,
      pokemon
    );
    state.evolution_animation = cutscene;
  } catch (error) {
    throw new Error('Failed to initialise evolution animation');
  }
};

const forget_prompt_text = (pokemon: Pokemon, move_name: MoveName): string => {
  const name = display_name(pokemon);
  const moveText = formatMoveName(move_name);
  return (
    `But ${name} can't learn more than four moves.\n\n` +
    `Delete an older move to make room for ${moveText}?`
  );
};

const stop_learning_text = (pokemon: Pokemon, move_name: MoveName): string => {
  const moveText = formatMoveName(move_name);
  return `Stop learning ${moveText}?`;
};

const did_not_learn_text = (pokemon: Pokemon, move_name: MoveName): string => {
  const name = display_name(pokemon);
  const moveText = formatMoveName(move_name);
  return `${name} did not learn ${moveText}.`;
};

const forget_animation_text = (pokemon: Pokemon, move_name: MoveName): string => {
  const name = display_name(pokemon);
  const moveText = formatMoveName(move_name);
  return `Poof! ${name} forgot ${moveText}!\n\nAnd...`;
};

const evolution_text_for_id = (text_id: unknown, pokemon: Pokemon): string | null => {
  if (typeof text_id !== 'string') {
    return null;
  }
  const name = display_name(pokemon);
  if (text_id === 'EvolvingText') {
    return `What? ${name} is evolving!`;
  }
  if (text_id === 'EvolvedIntoText') {
    const species = format_species_display(pokemon.species.id);
    return `Congratulations! ${name} evolved into ${species}!`;
  }
  if (text_id === 'StoppedEvolvingText') {
    return `Huh? ${name} stopped evolving!`;
  }
  return null;
};

const display_name = (pokemon: Pokemon): string => {
  const nickname = (pokemon.nickname ?? '').trim();
  if (nickname) {
    return nickname;
  }
  return format_species_display(pokemon.species.id);
};

const format_species_display = (species_id: string): string => {
  let normalized = species_id.replace(/__/g, ' ');
  normalized = normalized.replace(/_/g, ' ');
  return normalized.split(/\s+/).filter(Boolean).join(' ').trim();
};

export const _apply_level_up = apply_level_up;
export const _process_move_learning = process_move_learning;
export const _schedule_pending_evolution = schedule_pending_evolution;
export const _close_move_forget_menu = close_move_forget_menu;
export const _queue_battle_text = queue_battle_text;
