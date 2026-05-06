import { EvolutionItemHandler } from "./evolution-item-handler";

describe("EvolutionItemHandler text resolution", () => {
  it("throws instead of prettifying unknown evolution text labels", () => {
    const handler = new EvolutionItemHandler({});

    expect(() =>
      (
        handler as unknown as {
          resolveText: (label: string) => string;
        }
      ).resolveText("TotallyMissingEvolutionText"),
    ).toThrow("Missing ASM text for label 'TotallyMissingEvolutionText'.");
  });
});
