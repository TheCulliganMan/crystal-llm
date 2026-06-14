import { createPokemonCrystalMastra, createPokemonCrystalWorkflow } from "./workflow.js";

describe("workflow factory", () => {
  it("creates the workflow and registers it on a Mastra instance", () => {
    const workflow = createPokemonCrystalWorkflow();
    const mastra = createPokemonCrystalMastra({});

    expect(workflow.id).toBe("pokemon-crystal-taskmaster-workflow");
    expect(mastra.getWorkflow("pokemonCrystalWorkflow")).toBeDefined();
  });
});
