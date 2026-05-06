import { MoveName } from "../../../core/enums/move";
import { createTestPokemon } from "../../world/story-events/test-utils";
import { learnMove, resolveTmhmMove, tmhmItemName, TMHMResolutionError } from "../tmhm";

describe("tmhm", () => {
  describe("resolveTmhmMove", () => {
    it("should resolve a TM by number", () => {
      const [move, isHm] = resolveTmhmMove("TM01");
      expect(move).toBe(MoveName.DYNAMICPUNCH);
      expect(isHm).toBe(false);
    });

    it("should resolve an HM by number", () => {
      const [move, isHm] = resolveTmhmMove("HM01");
      expect(move).toBe(MoveName.CUT);
      expect(isHm).toBe(true);
    });

    it("should resolve an HM by name", () => {
      const [move, isHm] = resolveTmhmMove("HM_CUT");
      expect(move).toBe(MoveName.CUT);
      expect(isHm).toBe(true);
    });

    it("should throw an error for an invalid TM/HM", () => {
      expect(() => resolveTmhmMove("INVALID")).toThrow(TMHMResolutionError);
    });
  });

  describe("tmhmItemName", () => {
    it("should return the correct TM name for a TM index", () => {
      expect(tmhmItemName(0)).toBe("TM01");
    });

    it("should return the correct HM name for an HM index", () => {
      expect(tmhmItemName(50)).toBe("HM01");
    });

    it("should throw an error for an out-of-range index", () => {
      expect(() => tmhmItemName(999)).toThrow(TMHMResolutionError);
    });
  });

  describe("learnMove", () => {
    it("teaches a TM with the move's full base PP", () => {
      const pokemon = createTestPokemon("MACHOP", 66, { moves: [] });

      learnMove(pokemon, MoveName.DYNAMICPUNCH);

      expect(pokemon.moves).toEqual([
        { name: MoveName.DYNAMICPUNCH, current_pp: 5 },
      ]);
    });
  });
});
