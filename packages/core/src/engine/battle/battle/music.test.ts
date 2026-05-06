import { createInitialGameState } from "@pokecrystal/core/core/state";
import { determineBattleMusic, determineVictoryMusic, isGymLeaderClass } from "./music";

const withTrainerClass = (trainerClass: string, trainerId = "") => {
  const gameState = createInitialGameState();
  gameState.wram.other_trainer_class = trainerClass;
  gameState.wram.other_trainer_id = trainerId;
  return gameState;
};

describe("battle music trainer class mapping", () => {
  it("treats the Elite Four as Johto gym leader battles", () => {
    for (const trainerClass of ["WILL", "KOGA", "BRUNO", "KAREN"]) {
      expect(determineBattleMusic(withTrainerClass(trainerClass))).toBe("MUSIC_JOHTO_GYM_LEADER_BATTLE");
      expect(determineVictoryMusic(trainerClass, true)).toBe("MUSIC_GYM_VICTORY");
      expect(isGymLeaderClass(trainerClass)).toBe(true);
    }
  });

  it("keeps Champion and Red on Champion battle music", () => {
    for (const trainerClass of ["CHAMPION", "RED"]) {
      expect(determineBattleMusic(withTrainerClass(trainerClass))).toBe("MUSIC_CHAMPION_BATTLE");
      expect(determineVictoryMusic(trainerClass, true)).toBe("MUSIC_GYM_VICTORY");
      expect(isGymLeaderClass(trainerClass)).toBe(true);
    }
  });
});
