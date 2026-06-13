import { buildFlowStateSnapshot } from "./flow-state";

describe("buildFlowStateSnapshot", () => {
  it("keeps gym and major public milestones named while masking Rocket beats", () => {
    const snapshot = buildFlowStateSnapshot({
      EVENT_GOT_A_POKEMON_FROM_ELM: true,
      EVENT_GOT_MYSTERY_EGG_FROM_MR_POKEMON: true,
      EVENT_TALKED_TO_MOM_AFTER_MYSTERY_EGG_QUEST: true,
      ENGINE_ZEPHYRBADGE: true,
      EVENT_RIVAL_AZALEA_TOWN: true,
      EVENT_CLEARED_SLOWPOKE_WELL: true,
      ENGINE_HIVEBADGE: true,
      EVENT_GOT_HM01_CUT: true,
      ENGINE_PLAINBADGE: true,
      ENGINE_FOGBADGE: true,
      EVENT_GOT_SECRETPOTION_FROM_PHARMACY: true,
      ENGINE_STORMBADGE: true,
      ENGINE_MINERALBADGE: true,
    });

    expect(snapshot.completed.some((step) => step.title === "Zephyr Badge")).toBe(true);
    expect(snapshot.remaining.some((step) => step.title === "???")).toBe(true);
    expect(snapshot.next_goal?.title).toBe("???");
  });

  it("uses Mt. Silver as the public terminal completion target", () => {
    const snapshot = buildFlowStateSnapshot({
      EVENT_GOT_A_POKEMON_FROM_ELM: true,
      EVENT_GOT_MYSTERY_EGG_FROM_MR_POKEMON: true,
      EVENT_TALKED_TO_MOM_AFTER_MYSTERY_EGG_QUEST: true,
      ENGINE_ZEPHYRBADGE: true,
      EVENT_RIVAL_AZALEA_TOWN: true,
      EVENT_CLEARED_SLOWPOKE_WELL: true,
      ENGINE_HIVEBADGE: true,
      EVENT_GOT_HM01_CUT: true,
      ENGINE_PLAINBADGE: true,
      ENGINE_FOGBADGE: true,
      EVENT_GOT_SECRETPOTION_FROM_PHARMACY: true,
      ENGINE_STORMBADGE: true,
      ENGINE_MINERALBADGE: true,
      EVENT_CLEARED_ROCKET_HIDEOUT: true,
      ENGINE_GLACIERBADGE: true,
      EVENT_CLEARED_RADIO_TOWER: true,
      ENGINE_RISINGBADGE: true,
      EVENT_GOT_CLEAR_BELL: true,
      EVENT_BEAT_CHAMPION_LANCE: true,
      EVENT_RETURNED_MACHINE_PART: true,
      EVENT_BLUE_IN_CINNABAR: true,
      ENGINE_EARTHBADGE: true,
      EVENT_BEAT_RED: true,
    });

    expect(snapshot.completion_target).toEqual({
      id: "mt-silver",
      title: "Beat Mt. Silver",
    });
    expect(snapshot.remaining_path.some((step) => step.title === "Beat Mt. Silver")).toBe(false);
  });

  it("does not mark later steps complete when their prerequisites are missing", () => {
    const snapshot = buildFlowStateSnapshot({
      EVENT_RIVAL_AZALEA_TOWN: true,
    });

    expect(snapshot.completed_count).toBe(0);
    expect(snapshot.completed).toEqual([]);
    expect(snapshot.next_goal?.title).toBe("Starter + Pokédex");
    expect(snapshot.available.map((step) => step.title)).toEqual(["Starter + Pokédex"]);
  });

  it("routes through Mom's banking prompt after the Mystery Egg before Violet", () => {
    const snapshot = buildFlowStateSnapshot({
      EVENT_GOT_A_POKEMON_FROM_ELM: true,
      EVENT_GOT_MYSTERY_EGG_FROM_MR_POKEMON: true,
    });

    expect(snapshot.next_goal?.title).toBe("Mom + money setup");
    expect(snapshot.available.map((step) => step.title)).toEqual(["Mom + money setup"]);
    expect(snapshot.remaining_path.map((step) => step.title).slice(0, 2)).toEqual([
      "Mom + money setup",
      "Zephyr Badge",
    ]);
  });

  it("uses the canonical badge state for badge milestones", () => {
    const snapshot = buildFlowStateSnapshot(
      {
        EVENT_GOT_A_POKEMON_FROM_ELM: true,
        EVENT_GOT_MYSTERY_EGG_FROM_MR_POKEMON: true,
        EVENT_TALKED_TO_MOM_AFTER_MYSTERY_EGG_QUEST: true,
        EVENT_RIVAL_AZALEA_TOWN: true,
        EVENT_CLEARED_SLOWPOKE_WELL: true,
        EVENT_GOT_HM01_CUT: true,
      },
      {
        johto: [true, true, false, false, false, false, false, false],
        kanto: [false, false, false, false, false, false, false, false],
      }
    );

    expect(snapshot.completed.map((step) => step.title)).toContain("Zephyr Badge");
    expect(snapshot.completed.map((step) => step.title)).toContain("Hive Badge");
    expect(snapshot.completed.map((step) => step.title)).toContain("Ilex Forest + Cut Gate");
    expect(snapshot.next_goal?.title).toBe("Plain Badge");
  });

  it("does not let Mineral Badge bypass the Cianwood medicine prerequisite", () => {
    const snapshot = buildFlowStateSnapshot({
      EVENT_GOT_A_POKEMON_FROM_ELM: true,
      EVENT_GOT_MYSTERY_EGG_FROM_MR_POKEMON: true,
      EVENT_TALKED_TO_MOM_AFTER_MYSTERY_EGG_QUEST: true,
      ENGINE_ZEPHYRBADGE: true,
      EVENT_RIVAL_AZALEA_TOWN: true,
      EVENT_CLEARED_SLOWPOKE_WELL: true,
      ENGINE_HIVEBADGE: true,
      EVENT_GOT_HM01_CUT: true,
      ENGINE_PLAINBADGE: true,
      ENGINE_FOGBADGE: true,
      ENGINE_MINERALBADGE: true,
    });

    expect(snapshot.completed.map((step) => step.title)).not.toContain("Mineral Badge");
    expect(snapshot.next_goal?.title).toBe("Cianwood Medicine");
    expect(snapshot.available.map((step) => step.title)).toEqual([
      "Cianwood Medicine",
      "Storm Badge",
    ]);
  });

  it("routes from medicine pickup to Mineral Badge without repeating Cianwood Medicine", () => {
    const snapshot = buildFlowStateSnapshot({
      EVENT_GOT_A_POKEMON_FROM_ELM: true,
      EVENT_GOT_MYSTERY_EGG_FROM_MR_POKEMON: true,
      EVENT_TALKED_TO_MOM_AFTER_MYSTERY_EGG_QUEST: true,
      ENGINE_ZEPHYRBADGE: true,
      EVENT_RIVAL_AZALEA_TOWN: true,
      EVENT_CLEARED_SLOWPOKE_WELL: true,
      ENGINE_HIVEBADGE: true,
      EVENT_GOT_HM01_CUT: true,
      ENGINE_PLAINBADGE: true,
      ENGINE_FOGBADGE: true,
      EVENT_GOT_SECRETPOTION_FROM_PHARMACY: true,
    });

    expect(snapshot.completed.map((step) => step.title)).toContain("Cianwood Medicine");
    expect(snapshot.next_goal?.title).toBe("Storm Badge");
    expect(snapshot.available.map((step) => step.title)).toEqual([
      "Storm Badge",
      "Mineral Badge",
    ]);
  });

  it("routes to Storm Badge after Jasmine when Chuck is still unfinished", () => {
    const snapshot = buildFlowStateSnapshot({
      EVENT_GOT_A_POKEMON_FROM_ELM: true,
      EVENT_GOT_MYSTERY_EGG_FROM_MR_POKEMON: true,
      EVENT_TALKED_TO_MOM_AFTER_MYSTERY_EGG_QUEST: true,
      ENGINE_ZEPHYRBADGE: true,
      EVENT_RIVAL_AZALEA_TOWN: true,
      EVENT_CLEARED_SLOWPOKE_WELL: true,
      ENGINE_HIVEBADGE: true,
      EVENT_GOT_HM01_CUT: true,
      ENGINE_PLAINBADGE: true,
      ENGINE_FOGBADGE: true,
      EVENT_GOT_SECRETPOTION_FROM_PHARMACY: true,
      ENGINE_MINERALBADGE: true,
    });

    expect(snapshot.completed.map((step) => step.title)).toContain("Mineral Badge");
    expect(snapshot.completed.map((step) => step.title)).not.toContain("Storm Badge");
    expect(snapshot.next_goal?.title).toBe("Storm Badge");
    expect(snapshot.available.map((step) => step.title)).toEqual(["Storm Badge"]);
  });
});
