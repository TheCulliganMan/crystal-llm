export type StoryStep = {
  id: string;
  title: string;
  asmLabel: string;
  description: string;
  prerequisites: string[];
  setFlag: string;
  applyMovement: string;
  mapRoute: string;
};

/**
 * ASM mapping for route-level game completion planning.
 *
 * This list intentionally tracks mandatory gates needed for a New Bark -> Red clear.
 * Each step records:
 * - script label (`asmLabel`)
 * - key event flag transition (`setFlag`)
 * - representative movement/script segment (`applyMovement`)
 */
export const STORY_STEPS: readonly StoryStep[] = [
  {
    id: "starter",
    title: "Starter + Pokédex",
    asmLabel: "ElmsLab_AfterCyndaquilTotodileChikorita",
    description: "Pick starter, receive Pokédex and Poké Balls to unlock wild captures.",
    prerequisites: [],
    setFlag: "EVENT_GOT_A_POKEMON_FROM_ELM",
    applyMovement: "ElmsLabElmWalksToPlayerMovement",
    mapRoute: "New Bark Town -> Elm's Lab",
  },
  {
    id: "mr-pokemon",
    title: "Mr. Pokémon + Mystery Egg",
    asmLabel: "MrPokemonEventScript",
    description: "Deliver Elm's errand and trigger Pokédex progression return path.",
    prerequisites: ["starter"],
    setFlag: "EVENT_GOT_MYSTERY_EGG_FROM_MR_POKEMON",
    applyMovement: "MrPokemonHouseOakGivesPokedexMovement",
    mapRoute: "Route 30 -> Mr. Pokémon's House",
  },
  {
    id: "mom-bank",
    title: "Mom + money setup",
    asmLabel: "MomScript",
    description: "Return the Mystery Egg to Elm, then talk to Mom and clear her money-saving prompt before leaving New Bark.",
    prerequisites: ["mr-pokemon"],
    setFlag: "EVENT_TALKED_TO_MOM_AFTER_MYSTERY_EGG_QUEST",
    applyMovement: "MomWalksToPlayerMovement",
    mapRoute: "New Bark Town -> Elm's Lab -> Player's House",
  },
  {
    id: "violet-badge",
    title: "Zephyr Badge",
    asmLabel: "VioletGymFalknerScript",
    description: "Defeat Falkner and establish first badge gate progression.",
    prerequisites: ["mom-bank"],
    setFlag: "ENGINE_ZEPHYRBADGE",
    applyMovement: "VioletGymGuyWalksToPlayer",
    mapRoute: "Violet City -> Violet Gym",
  },
  {
    id: "union-cave",
    title: "Union Cave Transit",
    asmLabel: "UnionCaveB1F_MapScripts",
    description: "Cross Union Cave to access Azalea branch and Johto midgame scripts.",
    prerequisites: ["violet-badge"],
    setFlag: "EVENT_RIVAL_AZALEA_TOWN",
    applyMovement: "AzaleaTownRivalBattleApproachMovement",
    mapRoute: "Route 32 -> Union Cave -> Azalea",
  },
  {
    id: "slowpoke-well",
    title: "Slowpoke Well Clear",
    asmLabel: "SlowpokeWellB1FRocketScript",
    description: "Defeat Rocket in well and unlock Kurt + Azalea Gym completion.",
    prerequisites: ["union-cave"],
    setFlag: "EVENT_CLEARED_SLOWPOKE_WELL",
    applyMovement: "SlowpokeWellKurtWalksToPlayerMovement",
    mapRoute: "Azalea Town -> Slowpoke Well",
  },
  {
    id: "hive-badge",
    title: "Hive Badge",
    asmLabel: "AzaleaGymBugsyScript",
    description: "Defeat Bugsy and unlock Cut progression through Ilex Forest.",
    prerequisites: ["slowpoke-well"],
    setFlag: "ENGINE_HIVEBADGE",
    applyMovement: "AzaleaGymBugsyBadgeGiveMovement",
    mapRoute: "Azalea Town -> Azalea Gym",
  },
  {
    id: "ilex-cut",
    title: "Ilex Forest + Cut Gate",
    asmLabel: "IlexForestFarfetchdScript",
    description: "Complete charcoal/Farfetch'd sequence and pass through forest to Goldenrod.",
    prerequisites: ["hive-badge"],
    setFlag: "EVENT_CHARCOAL_KILN_BOSS",
    applyMovement: "IlexForestFarfetchdChaseMovement",
    mapRoute: "Azalea -> Ilex Forest -> Route 34",
  },
  {
    id: "plain-badge",
    title: "Plain Badge",
    asmLabel: "GoldenrodGymWhitneyScript",
    description: "Defeat Whitney to continue normal city progression and radio access pathing.",
    prerequisites: ["ilex-cut"],
    setFlag: "ENGINE_PLAINBADGE",
    applyMovement: "GoldenrodGymWhitneyCryingMovement",
    mapRoute: "Goldenrod City -> Goldenrod Gym",
  },
  {
    id: "fog-badge",
    title: "Fog Badge",
    asmLabel: "EcruteakGymMortyScript",
    description: "Defeat Morty to unlock Surf-dependent route branches.",
    prerequisites: ["plain-badge"],
    setFlag: "ENGINE_FOGBADGE",
    applyMovement: "EcruteakGymMortyWalkToPlayerMovement",
    mapRoute: "Ecruteak City -> Ecruteak Gym",
  },
  {
    id: "cianwood-medicine",
    title: "Cianwood Medicine",
    asmLabel: "CianwoodPharmacyJasmineAmphyScript",
    description: "Deliver SecretPotion to Ampharos and reopen Olivine gym progression.",
    prerequisites: ["fog-badge"],
    setFlag: "EVENT_GOT_SECRETPOTION_FROM_PHARMACY",
    applyMovement: "OlivineLighthouseJasmineRequestsMedicineMovement",
    mapRoute: "Ecruteak -> Olivine -> Cianwood -> Olivine Lighthouse",
  },
  {
    id: "storm-badge",
    title: "Storm Badge",
    asmLabel: "CianwoodGymChuckScript",
    description: "Defeat Chuck and earn Fly utility for efficient Johto progression.",
    prerequisites: ["fog-badge"],
    setFlag: "ENGINE_STORMBADGE",
    applyMovement: "CianwoodGymChuckLiftBoulderMovement",
    mapRoute: "Cianwood City -> Cianwood Gym",
  },
  {
    id: "mineral-badge",
    title: "Mineral Badge",
    asmLabel: "OlivineGymJasmineScript",
    description: "Defeat Jasmine after medicine event and close western Johto badge gate.",
    prerequisites: ["cianwood-medicine"],
    setFlag: "ENGINE_MINERALBADGE",
    applyMovement: "OlivineGymJasmineApproachMovement",
    mapRoute: "Olivine City -> Olivine Gym",
  },
  {
    id: "mahogany-rocket",
    title: "Mahogany Hideout",
    asmLabel: "TeamRocketBaseB3FTrapDoorScript",
    description: "Shut down Lake of Rage signal in Team Rocket hideout.",
    prerequisites: ["mineral-badge", "storm-badge"],
    setFlag: "EVENT_CLEARED_ROCKET_HIDEOUT",
    applyMovement: "TeamRocketBaseLanceHyperBeamMovement",
    mapRoute: "Mahogany -> Rocket Hideout",
  },
  {
    id: "glacier-badge",
    title: "Glacier Badge",
    asmLabel: "MahoganyGymPryceScript",
    description: "Defeat Pryce to satisfy Radio Tower takeover trigger conditions.",
    prerequisites: ["mahogany-rocket"],
    setFlag: "ENGINE_GLACIERBADGE",
    applyMovement: "MahoganyGymPryceBadgeGiveMovement",
    mapRoute: "Mahogany Town -> Mahogany Gym",
  },
  {
    id: "goldenrod-rocket",
    title: "Goldenrod Rocket Takeover",
    asmLabel: "RadioTowerRocketsScript",
    description: "Clear Radio Tower and obtain Basement Key/Card Key progression.",
    prerequisites: ["glacier-badge"],
    setFlag: "EVENT_CLEARED_RADIO_TOWER",
    applyMovement: "RadioTowerDirectorGivesClearBellMovement",
    mapRoute: "Goldenrod Underground -> Radio Tower",
  },
  {
    id: "rising-badge",
    title: "Rising Badge",
    asmLabel: "BlackthornGymClairScript",
    description: "Defeat Clair and complete Dragon's Den test for final Johto badge.",
    prerequisites: ["goldenrod-rocket"],
    setFlag: "ENGINE_RISINGBADGE",
    applyMovement: "DragonsDenElderDratiniQuizMovement",
    mapRoute: "Blackthorn Gym -> Dragon's Den",
  },
  {
    id: "kimono-clear-bell",
    title: "Kimono Girls + Clear Bell",
    asmLabel: "EcruteakTinTowerEntranceScript",
    description: "Resolve Kimono Girls sequence and unlock Tin Tower/legendary path.",
    prerequisites: ["goldenrod-rocket"],
    setFlag: "EVENT_GOT_CLEAR_BELL",
    applyMovement: "KimonoGirlsDanceTheaterExitMovement",
    mapRoute: "Ecruteak Theater -> Tin Tower",
  },
  {
    id: "johto-champion",
    title: "Elite Four Champion",
    asmLabel: "LancesRoomLanceScript",
    description: "Defeat Elite Four + Lance to unlock S.S. Aqua and Kanto.",
    prerequisites: ["rising-badge"],
    setFlag: "EVENT_BEAT_CHAMPION_LANCE",
    applyMovement: "HallOfFameOakMaryEscortMovement",
    mapRoute: "Victory Road -> Indigo Plateau",
  },
  {
    id: "power-plant",
    title: "Power Plant Restored",
    asmLabel: "PowerPlantManagerScript",
    description: "Return Machine Part and reactivate Kanto rail/radio progression gates.",
    prerequisites: ["johto-champion"],
    setFlag: "EVENT_RETURNED_MACHINE_PART",
    applyMovement: "PowerPlantManagerRewardsPlayerMovement",
    mapRoute: "Cerulean Gym -> Power Plant",
  },
  {
    id: "blue-unlocked",
    title: "Viridian Gym Unlocked",
    asmLabel: "BlueHouseDaisyScript",
    description: "Resolve Blue's availability by completing Cinnabar/Mt. Moon conditions.",
    prerequisites: ["power-plant"],
    setFlag: "EVENT_BLUE_IN_CINNABAR",
    applyMovement: "ViridianGymBlueReturnsMovement",
    mapRoute: "Cinnabar -> Seafoam -> Viridian",
  },
  {
    id: "kanto-badges",
    title: "All 16 Badges",
    asmLabel: "ViridianGymBlueScript",
    description: "Collect all Johto + Kanto badges and trigger Mt. Silver access from Oak.",
    prerequisites: ["blue-unlocked"],
    setFlag: "ENGINE_EARTHBADGE",
    applyMovement: "PalletTownOakUnlocksMtSilverMovement",
    mapRoute: "All Kanto gyms + Viridian Gym",
  },
  {
    id: "red-defeated",
    title: "Defeat Red",
    asmLabel: "RedScript",
    description: "Final battle clear condition for Crystal end-to-end completion.",
    prerequisites: ["kanto-badges"],
    setFlag: "EVENT_BEAT_RED",
    applyMovement: "MtSilverSummitRedDespawnMovement",
    mapRoute: "Mt. Silver Summit",
  },
];

export type ProgressValidation = {
  isAcyclic: boolean;
  hasDanglingReferences: boolean;
  isRedReachable: boolean;
  orderedStepIds: string[];
  danglingPrerequisites: string[];
};

const topologicalSort = (steps: readonly StoryStep[]) => {
  const stepMap = new Map(steps.map((step) => [step.id, step]));
  const inDegree = new Map<string, number>(steps.map((step) => [step.id, 0]));

  for (const step of steps) {
    for (const prerequisite of step.prerequisites) {
      inDegree.set(step.id, (inDegree.get(step.id) ?? 0) + 1);
      if (!stepMap.has(prerequisite)) {
        continue;
      }
    }
  }

  const queue = Array.from(inDegree.entries())
    .filter(([, degree]) => degree === 0)
    .map(([id]) => id);

  const order: string[] = [];
  while (queue.length > 0) {
    const id = queue.shift();
    if (!id) {
      break;
    }
    order.push(id);

    for (const step of steps) {
      if (!step.prerequisites.includes(id)) {
        continue;
      }
      const nextDegree = (inDegree.get(step.id) ?? 0) - 1;
      inDegree.set(step.id, nextDegree);
      if (nextDegree === 0) {
        queue.push(step.id);
      }
    }
  }

  return order;
};

export const validateStoryGraph = (steps: readonly StoryStep[]): ProgressValidation => {
  const stepIds = new Set(steps.map((step) => step.id));
  const danglingPrerequisites = steps
    .flatMap((step) => step.prerequisites.map((prerequisite) => ({ stepId: step.id, prerequisite })))
    .filter(({ prerequisite }) => !stepIds.has(prerequisite))
    .map(({ stepId, prerequisite }) => `${stepId}:${prerequisite}`);

  const orderedStepIds = topologicalSort(steps);
  const isAcyclic = orderedStepIds.length === steps.length;

  const seen = new Set<string>();
  const walk = (stepId: string): boolean => {
    if (stepId === "red-defeated") {
      return true;
    }
    if (seen.has(stepId)) {
      return false;
    }
    seen.add(stepId);
    const dependents = steps.filter((step) => step.prerequisites.includes(stepId));
    return dependents.some((step) => walk(step.id));
  };

  return {
    isAcyclic,
    hasDanglingReferences: danglingPrerequisites.length > 0,
    isRedReachable: walk("starter"),
    orderedStepIds,
    danglingPrerequisites,
  };
};

export const getAvailableStepIds = (steps: readonly StoryStep[], completedStepIds: readonly string[]) => {
  const completed = new Set(completedStepIds);
  return steps
    .filter((step) => !completed.has(step.id) && step.prerequisites.every((required) => completed.has(required)))
    .map((step) => step.id);
};

export const buildMermaidDiagram = (steps: readonly StoryStep[], completedStepIds: readonly string[]) => {
  const completed = new Set(completedStepIds);
  const available = new Set(getAvailableStepIds(steps, completedStepIds));
  const lines = ["flowchart TD", "  start([New Bark Town])"];

  for (const step of steps) {
    lines.push(`  ${step.id}[\"${step.title}\\n${step.mapRoute}\\n${step.asmLabel}\"]`);
    if (step.prerequisites.length === 0) {
      lines.push(`  start --> ${step.id}`);
    }
    for (const requirement of step.prerequisites) {
      lines.push(`  ${requirement} --> ${step.id}`);
    }

    if (completed.has(step.id)) {
      lines.push(`  class ${step.id} done;`);
    } else if (available.has(step.id)) {
      lines.push(`  class ${step.id} ready;`);
    } else {
      lines.push(`  class ${step.id} todo;`);
    }
  }

  lines.push("  classDef done fill:#14532d,color:#ecfccb,stroke:#86efac,stroke-width:2px;");
  lines.push("  classDef ready fill:#92400e,color:#fffbeb,stroke:#f59e0b,stroke-width:2px;");
  lines.push("  classDef todo fill:#27272a,color:#e4e4e7,stroke:#a1a1aa,stroke-width:1px;");

  return lines.join("\n");
};

export const calculateCompletionPercent = (steps: readonly StoryStep[], completedStepIds: readonly string[]) => {
  if (steps.length === 0) {
    return 0;
  }

  const uniqueCompleted = new Set(completedStepIds);
  const completedCount = steps.reduce((count, step) => (uniqueCompleted.has(step.id) ? count + 1 : count), 0);
  return Math.round((completedCount / steps.length) * 100);
};
