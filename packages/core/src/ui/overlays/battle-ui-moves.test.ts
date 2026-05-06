import { MoveEffect, MoveName, PokemonType } from "../../core/enums";
import type { LearnedMove, Move, Pokemon } from "../../core/models";
import type { BattleUIState } from "./battle-ui-state";
import type { DialogueState } from "./battle-dialogue";
import { MoveLearningPhase } from "./battle-ui-state";
import { advance_dialogue } from "./battle-dialogue";
import { process_move_learning } from "./battle-ui-moves";
import { Surface } from "../surface";

type DialogueHarness = DialogueState;

const buildDialogue = (): DialogueHarness =>
  ({
    window: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
    dialogue: {
      open: jest.fn(),
      clear: jest.fn(),
      is_complete: jest.fn(() => true),
      has_more_pages: jest.fn(() => false),
      advance_page: jest.fn(),
      complete: jest.fn(),
    },
    queue: [],
    pending_waits: 0,
    forced_visible: false,
    auto_close_after_display: false,
  }) as unknown as DialogueHarness;

const clearDialogueWait = (state: BattleUIState): void => {
  state.dialogue.pending_waits = 0;
  state.dialogue.queue = [];
  state.dialogue.forced_visible = false;
};

const resolveYesNo = (state: BattleUIState, result: boolean): void => {
  state.yes_no_prompt.active = false;
  state.yes_no_prompt.result = result;
  advance_dialogue(state.dialogue);
};

const makeMove = (name: MoveName, pp: number): Move =>
  ({
    name,
    pp,
    type: PokemonType.NORMAL,
    power: 0,
    accuracy: 100,
    effect: MoveEffect.NORMAL_HIT,
    effect_chance: 0,
  }) as Move;

const makePokemon = (moves: MoveName[], nickname = "Quinn"): Pokemon =>
  ({
    nickname,
    species: { id: "test_mon" },
    moves: moves.map((name) => ({ name, current_pp: 5 } as LearnedMove)),
  }) as Pokemon;

const buildState = (moveMetadata?: Map<MoveName, Move>): BattleUIState =>
  ({
    ui: {
      screen: new Surface(160, 144),
      drawWindow: jest.fn(),
      font: { renderText: jest.fn() },
      update: jest.fn(),
    },
    dialogue: buildDialogue(),
    yes_no_prompt: { active: false, result: null, pending_activation: false, prompt: null },
    pending_move_learns: [],
    active_move_learn: null,
    move_forget_menu: null,
    manual_wait_override: false,
    block_on_move_learning: false,
    _move_metadata: moveMetadata ?? new Map<MoveName, Move>(),
  }) as unknown as BattleUIState;

describe("battle-ui-moves learn move dialogue", () => {
  it("teaches a move immediately when slots are available", () => {
    const pokemon = makePokemon([MoveName.TACKLE, MoveName.GROWL]);
    const moveMetadata = new Map<MoveName, Move>([[MoveName.THUNDER, makeMove(MoveName.THUNDER, 10)]]);
    const state = buildState(moveMetadata);
    state.pending_move_learns.push({
      pokemon,
      move: { name: MoveName.THUNDER, current_pp: 0 } as LearnedMove,
    });

    process_move_learning(state);

    expect(state.active_move_learn?.stage).toBe(MoveLearningPhase.DECIDE);
    expect((state.dialogue.dialogue.open as jest.Mock).mock.calls.at(-1)?.[0]).toBe(
      "Quinn is trying to learn THUNDER!",
    );

    clearDialogueWait(state);
    process_move_learning(state);
    expect(state.active_move_learn?.stage).toBe(MoveLearningPhase.LEARN_NEW_MOVE);

    process_move_learning(state);
    expect(state.active_move_learn?.stage).toBe(MoveLearningPhase.FINAL);
    expect(pokemon.moves?.map((move) => move?.name)).toContain(MoveName.THUNDER);
    const learned = pokemon.moves?.find((move) => move?.name === MoveName.THUNDER);
    expect(learned?.current_pp).toBe(10);
    expect((state.dialogue.dialogue.open as jest.Mock).mock.calls.at(-1)?.[0]).toBe(
      "Quinn learned THUNDER!",
    );

    clearDialogueWait(state);
    process_move_learning(state);
    expect(state.active_move_learn).toBeNull();
    expect(state.block_on_move_learning).toBe(false);
  });

  it("walks the stop-learning dialogue when the player declines to forget", () => {
    const pokemon = makePokemon([
      MoveName.TACKLE,
      MoveName.GROWL,
      MoveName.TAIL_WHIP,
      MoveName.LEER,
    ]);
    const moveMetadata = new Map<MoveName, Move>([[MoveName.THUNDER, makeMove(MoveName.THUNDER, 10)]]);
    const state = buildState(moveMetadata);
    state.pending_move_learns.push({
      pokemon,
      move: { name: MoveName.THUNDER, current_pp: 0 } as LearnedMove,
    });

    process_move_learning(state);
    clearDialogueWait(state);
    process_move_learning(state);
    expect(state.active_move_learn?.stage).toBe(MoveLearningPhase.ASK_FORGET);

    process_move_learning(state);
    expect((state.dialogue.dialogue.open as jest.Mock).mock.calls.at(-1)?.[0]).toBe(
      "But Quinn can't learn more than four moves.\n\nDelete an older move to make room for THUNDER?",
    );
    expect(state.active_move_learn?.stage).toBe(MoveLearningPhase.WAIT_FORGET_PROMPT);

    clearDialogueWait(state);
    process_move_learning(state);
    expect(state.yes_no_prompt.active).toBe(true);
    expect(state.active_move_learn?.stage).toBe(MoveLearningPhase.FORGET_PROMPT_RESULT);

    resolveYesNo(state, false);
    process_move_learning(state);
    expect(state.active_move_learn?.stage).toBe(MoveLearningPhase.STOP_PROMPT);

    process_move_learning(state);
    expect((state.dialogue.dialogue.open as jest.Mock).mock.calls.at(-1)?.[0]).toBe("Stop learning THUNDER?");
    expect(state.active_move_learn?.stage).toBe(MoveLearningPhase.WAIT_STOP_PROMPT);

    clearDialogueWait(state);
    process_move_learning(state);
    expect(state.active_move_learn?.stage).toBe(MoveLearningPhase.STOP_PROMPT_RESULT);

    resolveYesNo(state, true);
    process_move_learning(state);
    expect(state.active_move_learn?.stage).toBe(MoveLearningPhase.DID_NOT_LEARN);

    process_move_learning(state);
    expect((state.dialogue.dialogue.open as jest.Mock).mock.calls.at(-1)?.[0]).toBe(
      "Quinn did not learn THUNDER.",
    );

    clearDialogueWait(state);
    process_move_learning(state);
    expect(state.active_move_learn).toBeNull();
    expect(pokemon.moves?.map((move) => move?.name)).toEqual([
      MoveName.TACKLE,
      MoveName.GROWL,
      MoveName.TAIL_WHIP,
      MoveName.LEER,
    ]);
  });

  it("forgets the selected move and learns the new one", () => {
    const pokemon = makePokemon([
      MoveName.TACKLE,
      MoveName.GROWL,
      MoveName.TAIL_WHIP,
      MoveName.LEER,
    ]);
    const moveMetadata = new Map<MoveName, Move>([[MoveName.THUNDER, makeMove(MoveName.THUNDER, 15)]]);
    const state = buildState(moveMetadata);
    state.pending_move_learns.push({
      pokemon,
      move: { name: MoveName.THUNDER, current_pp: 0 } as LearnedMove,
    });

    process_move_learning(state);
    clearDialogueWait(state);
    process_move_learning(state);
    process_move_learning(state);
    clearDialogueWait(state);
    process_move_learning(state);
    resolveYesNo(state, true);
    process_move_learning(state);
    process_move_learning(state);
    clearDialogueWait(state);
    process_move_learning(state);

    expect(state.active_move_learn?.stage).toBe(MoveLearningPhase.FORGET_MENU);
    expect(state.move_forget_menu?.option_count).toBe(5);

    state.move_forget_menu = null;
    state.manual_wait_override = false;
    state.active_move_learn!.pending_selection = 1;
    state.active_move_learn!.stage = MoveLearningPhase.HANDLE_MENU_SELECTION;

    process_move_learning(state);
    expect(state.active_move_learn?.stage).toBe(MoveLearningPhase.FORGET_ANIMATION);
    expect(state.active_move_learn?.replace_index).toBe(1);
    expect(state.active_move_learn?.forget_move_name).toBe(MoveName.GROWL);

    process_move_learning(state);
    expect((state.dialogue.dialogue.open as jest.Mock).mock.calls.at(-1)?.[0]).toBe("1, 2 and...");
    expect(state.dialogue.queue.at(-1)?.text).toBe("Poof! Quinn forgot GROWL!\n\nAnd...");

    clearDialogueWait(state);
    process_move_learning(state);
    expect(state.active_move_learn?.stage).toBe(MoveLearningPhase.LEARN_NEW_MOVE);

    process_move_learning(state);
    expect(state.active_move_learn?.stage).toBe(MoveLearningPhase.FINAL);
    expect(pokemon.moves?.[1]?.name).toBe(MoveName.THUNDER);
    expect(pokemon.moves?.[1]?.current_pp).toBe(15);

    clearDialogueWait(state);
    process_move_learning(state);
    expect(state.active_move_learn).toBeNull();
  });

  it("warns when trying to forget an HM move and returns to the menu", () => {
    const pokemon = makePokemon([
      MoveName.CUT,
      MoveName.GROWL,
      MoveName.TAIL_WHIP,
      MoveName.LEER,
    ]);
    const moveMetadata = new Map<MoveName, Move>([[MoveName.THUNDER, makeMove(MoveName.THUNDER, 10)]]);
    const state = buildState(moveMetadata);
    state.pending_move_learns.push({
      pokemon,
      move: { name: MoveName.THUNDER, current_pp: 0 } as LearnedMove,
    });

    process_move_learning(state);
    clearDialogueWait(state);
    process_move_learning(state);
    process_move_learning(state);
    clearDialogueWait(state);
    process_move_learning(state);
    resolveYesNo(state, true);
    process_move_learning(state);
    process_move_learning(state);
    clearDialogueWait(state);
    process_move_learning(state);

    state.move_forget_menu = null;
    state.manual_wait_override = false;
    state.active_move_learn!.pending_selection = 0;
    state.active_move_learn!.stage = MoveLearningPhase.HANDLE_MENU_SELECTION;
    process_move_learning(state);
    expect((state.dialogue.dialogue.open as jest.Mock).mock.calls.at(-1)?.[0]).toBe(
      "HM moves can't be forgotten now.",
    );
    expect(state.active_move_learn?.stage).toBe(MoveLearningPhase.HM_WARNING);

    clearDialogueWait(state);
    process_move_learning(state);
    expect(state.active_move_learn?.stage).toBe(MoveLearningPhase.FORGET_MENU_TEXT);

    process_move_learning(state);
    expect((state.dialogue.dialogue.open as jest.Mock).mock.calls.at(-1)?.[0]).toBe(
      "Which move should be forgotten?",
    );
    expect(state.active_move_learn?.replace_index).toBeNull();
    expect(state.active_move_learn?.forget_move_name).toBeNull();
  });
});
