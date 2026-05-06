import { PlayerGender } from "@pokecrystal/core/core/enums";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import {
  cable_club_check_which_chris,
  check_link_timeout_receptionist,
  set_bits_for_battle_request,
} from "./link";

describe("link room helpers", () => {
  test("CableClubCheckWhichChris selects Chris1 for male player sprites", () => {
    const gameState = createInitialGameState();
    gameState.wram.player_gender = PlayerGender.MALE;
    const runner = {
      last_condition_result: false,
      last_value: null,
      variables: {},
    };

    const result = cable_club_check_which_chris(gameState, { runner: runner as any });

    expect(result).toBe(true);
    expect(runner.last_condition_result).toBe(true);
    expect(runner.last_value).toBe(1);
  });

  test("CableClubCheckWhichChris selects Chris2 for female player sprites", () => {
    const gameState = createInitialGameState();
    gameState.wram.player_gender = PlayerGender.FEMALE;
    const runner = {
      last_condition_result: true,
      last_value: null,
      variables: {},
    };

    const result = cable_club_check_which_chris(gameState, { runner: runner as any });

    expect(result).toBe(false);
    expect(runner.last_condition_result).toBe(false);
    expect(runner.last_value).toBe(0);
  });

  test("CheckLinkTimeoutReceptionist preserves the currently chosen battle room", () => {
    const gameState = createInitialGameState();
    const runner = {
      last_condition_result: false,
      last_value: null,
      variables: {},
    };

    set_bits_for_battle_request(gameState, { runner: runner as any });
    runner.variables = { _other_player_link_mode: 3 };

    const result = check_link_timeout_receptionist(gameState, { runner: runner as any });

    expect(result).toBe(true);
    expect(gameState.wram.wChosenCableClubRoom).toBe(2);
    expect(gameState.wram.wPlayerLinkAction).toBe(2);
    expect(gameState.wram.script_memory.wOtherPlayerLinkMode).toBe(3);
  });
});
