// Public battle UI API that mirrors the original battle ASM drawing pipeline.

import type { BattleUILayout } from './_battle-layout';
import { BattleMenu } from './_battle-menu';
import { HP_BAR_LENGTH_PX, HP_GREEN, HP_RED, HP_YELLOW } from './battle-bars';
import { build_trainer_entrance_animation } from './battle-intro';

import {
  begin_battle,
  clear_yes_no_prompt,
  create_battle_ui,
  end_battle,
  force_party_menu_selection,
  get_yes_no_prompt_result,
  is_text_box_visible,
  is_waiting_for_input,
  release_force_party_menu,
  reset_menu_selection,
  set_audio_engine,
  set_game_state,
  set_text_box_visible,
  set_waiting_for_input,
  show_trainer_sprites,
  should_block_state_advance,
  show_yes_no_prompt,
  start_trainer_intro,
  start_trainer_exit_animation,
  start_trainer_victory_slide,
  trainer_intro_active,
} from './battle-ui-core';
import { menu_header_for_battle, menu_visible, text_window_target as _text_window_target } from './battle-ui-draw';
import { apply_runtime_postprocessing as _apply_runtime_postprocessing, draw_battle_sprites as _draw_battle_sprites } from './battle-ui-sprites';
import {
  handle_event,
  handle_input,
  handle_show_text,
  _dispatch_bag_event,
  forward_pack_menu_inputs,
  get_player_input,
  _resolve_pack_menu_action,
} from './battle-ui-input';
import {
  _apply_level_up,
  _process_move_learning,
  _schedule_pending_evolution,
  enqueue_exp_gain,
} from './battle-ui-moves';
import { render_pack_menu, update } from './battle-ui-render';
import {
  BACKGROUND_COLOUR,
  BATTLE_TEXT_ADVANCE_DELAY_FRAMES,
  BATTLE_TEXT_COLOUR,
  BattleTextDelayFlag,
  BattleUIPhase,
  HPBarAnimationState,
  HP_ANIM_STEP_FRAMES,
} from './battle-ui-state';
import type { BattleUIState } from './battle-ui-state';

export {
  BattleMenu,
  BattleTextDelayFlag,
  BACKGROUND_COLOUR,
  BATTLE_TEXT_COLOUR,
  HP_BAR_LENGTH_PX,
  HP_GREEN,
  HP_YELLOW,
  HP_RED,
  create_battle_ui,
  begin_battle,
  end_battle,
  handle_event,
  handle_show_text,
  handle_input,
  get_player_input,
  enqueue_exp_gain,
  forward_pack_menu_inputs,
  update,
  render_pack_menu,
  start_trainer_intro,
  start_trainer_exit_animation,
  trainer_intro_active,
  start_trainer_victory_slide,
  show_trainer_sprites,
  should_block_state_advance,
  set_waiting_for_input,
  is_waiting_for_input,
  set_text_box_visible,
  is_text_box_visible,
  set_game_state,
  set_audio_engine,
  show_yes_no_prompt,
  get_yes_no_prompt_result,
  clear_yes_no_prompt,
  force_party_menu_selection,
  release_force_party_menu,
  reset_menu_selection,
  build_trainer_entrance_animation,
  _apply_level_up,
  _process_move_learning,
  _schedule_pending_evolution,
  _text_window_target,
  _apply_runtime_postprocessing,
  _draw_battle_sprites,
  _dispatch_bag_event,
  _resolve_pack_menu_action,
  menu_visible,
  menu_header_for_battle,
  BATTLE_TEXT_ADVANCE_DELAY_FRAMES,
  HPBarAnimationState,
  HP_ANIM_STEP_FRAMES,
};

export type { BattleUIState, BattleUILayout };
