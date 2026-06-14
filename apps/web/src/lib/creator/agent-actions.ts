export type CreatorMapKind = "town" | "route" | "cave";

export type CreatorPokemonStats = {
  hp: number;
  attack: number;
  defense: number;
  speed: number;
  special_attack: number;
  special_defense: number;
};

export type CreatorProject = {
  id: string;
  title: string;
  packId: string;
  description?: string;
  premise?: string;
  maps: CreatorMapDraft[];
  storyBeats: CreatorStoryBeat[];
  pokemon: CreatorPokemon[];
  trainers: CreatorTrainer[];
  encounterTables: CreatorEncounterTable[];
  npcs: CreatorNpc[];
  audioTokens: string[];
  updatedAt: string;
};

export type CreatorMapDraft = {
  id: string;
  name: string;
  kind: CreatorMapKind;
  width: number;
  height: number;
  terrain: string[];
  collisions: string[];
};

export type CreatorStoryBeat = {
  id: string;
  type: string;
  title: string;
  map?: string;
  text?: string;
  species?: string;
  level?: number;
  item?: string;
  quantity?: number;
  flag?: string;
  script?: string;
};

export type CreatorPokemon = {
  id: string;
  name: string;
  types: string[];
  base_stats: CreatorPokemonStats;
  catch_rate: number;
  base_exp: number;
};

export type CreatorTrainer = {
  id: string;
  name: string;
  trainer_class: string;
  party: Array<{ species: string; level: number; moves?: string[] }>;
  win_quote?: string;
  lose_quote?: string;
};

export type CreatorEncounterTable = {
  id: string;
  map: string;
  encounters: Array<{ species: string; minLevel: number; maxLevel: number; rate: number }>;
};

export type CreatorNpc = {
  id: string;
  map: string;
  name: string;
  dialogue: string;
};

const slugify = (value: string): string =>
  value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "") || "untitled";

const nowIso = (): string => new Date().toISOString();

const uniqueById = <T extends { id: string }>(items: T[], next: T): T[] => {
  const index = items.findIndex((item) => item.id === next.id);
  if (index === -1) {
    return [...items, next];
  }
  return items.map((item, itemIndex) => (itemIndex === index ? next : item));
};

const makeTerrain = (kind: CreatorMapKind, width: number, height: number): string[] => {
  const edge = kind === "cave" ? "rock" : "tree";
  const floor = kind === "town" ? "path" : kind === "cave" ? "cave_floor" : "grass";
  return Array.from({ length: height }, (_, y) =>
    Array.from({ length: width }, (_, x) => (x === 0 || y === 0 || x === width - 1 || y === height - 1 ? edge : floor)).join(",")
  );
};

const makeCollision = (width: number, height: number): string[] =>
  Array.from({ length: height }, (_, y) =>
    Array.from({ length: width }, (_, x) => (x === 0 || y === 0 || x === width - 1 || y === height - 1 ? "1" : "0")).join("")
  );

export const createEmptyCreatorProject = (title: string): CreatorProject => {
  const id = slugify(title);
  return {
    id,
    title,
    packId: id,
    maps: [],
    storyBeats: [],
    pokemon: [],
    trainers: [],
    encounterTables: [],
    npcs: [],
    audioTokens: [],
    updatedAt: nowIso(),
  };
};

export const applyCreatorBrief = (
  project: CreatorProject,
  brief: { title?: string; packId?: string; description?: string; premise?: string }
): CreatorProject => ({
  ...project,
  title: brief.title ?? project.title,
  packId: brief.packId ? slugify(brief.packId) : project.packId,
  description: brief.description ?? project.description,
  premise: brief.premise ?? project.premise,
  updatedAt: nowIso(),
});

export const addCreatorMapDraft = (
  project: CreatorProject,
  input: { name: string; kind: CreatorMapKind; width?: number; height?: number }
): CreatorProject => {
  const width = input.width ?? (input.kind === "route" ? 16 : 12);
  const height = input.height ?? (input.kind === "route" ? 10 : 12);
  const map: CreatorMapDraft = {
    id: slugify(input.name),
    name: input.name,
    kind: input.kind,
    width,
    height,
    terrain: makeTerrain(input.kind, width, height),
    collisions: makeCollision(width, height),
  };
  return { ...project, maps: uniqueById(project.maps, map), updatedAt: nowIso() };
};

export const addCreatorStoryBeat = (
  project: CreatorProject,
  input: Omit<CreatorStoryBeat, "id">
): CreatorProject => {
  const beat: CreatorStoryBeat = {
    ...input,
    id: `${project.storyBeats.length + 1}-${slugify(input.title)}`,
  };
  return { ...project, storyBeats: [...project.storyBeats, beat], updatedAt: nowIso() };
};

export const upsertCreatorPokemon = (
  project: CreatorProject,
  input: {
    id: string;
    name?: string;
    types?: string[];
    base_stats?: Partial<CreatorPokemonStats>;
    catch_rate?: number;
    base_exp?: number;
  }
): CreatorProject => {
  const existing = project.pokemon.find((pokemon) => pokemon.id === input.id);
  const pokemon: CreatorPokemon = {
    id: input.id,
    name: input.name ?? existing?.name ?? input.id,
    types: input.types ?? existing?.types ?? ["NORMAL"],
    base_stats: {
      hp: input.base_stats?.hp ?? existing?.base_stats.hp ?? 45,
      attack: input.base_stats?.attack ?? existing?.base_stats.attack ?? 45,
      defense: input.base_stats?.defense ?? existing?.base_stats.defense ?? 45,
      speed: input.base_stats?.speed ?? existing?.base_stats.speed ?? 45,
      special_attack: input.base_stats?.special_attack ?? existing?.base_stats.special_attack ?? 45,
      special_defense: input.base_stats?.special_defense ?? existing?.base_stats.special_defense ?? 45,
    },
    catch_rate: input.catch_rate ?? existing?.catch_rate ?? 190,
    base_exp: input.base_exp ?? existing?.base_exp ?? 64,
  };
  return { ...project, pokemon: uniqueById(project.pokemon, pokemon), updatedAt: nowIso() };
};

export const upsertCreatorTrainer = (
  project: CreatorProject,
  input: {
    name: string;
    trainer_id?: string;
    trainer_class?: string;
    party?: Array<{ species: string; level: number; moves?: string[] }>;
    win_quote?: string;
    lose_quote?: string;
  }
): CreatorProject => {
  const id = input.trainer_id ?? slugify(input.name);
  const existing = project.trainers.find((trainer) => trainer.id === id);
  const trainer: CreatorTrainer = {
    id,
    name: input.name,
    trainer_class: input.trainer_class ?? existing?.trainer_class ?? "TRAINER",
    party: input.party ?? existing?.party ?? [],
    win_quote: input.win_quote ?? existing?.win_quote,
    lose_quote: input.lose_quote ?? existing?.lose_quote,
  };
  return { ...project, trainers: uniqueById(project.trainers, trainer), updatedAt: nowIso() };
};

export const buildCreatorVerticalSlice = (
  project: CreatorProject,
  input: { townName?: string; routeName?: string; caveName?: string; premise?: string }
): CreatorProject => {
  const townName = input.townName ?? "HomeTown";
  const routeName = input.routeName ?? "RouteOne";
  const caveName = input.caveName ?? "CrystalCave";
  let next = applyCreatorBrief(project, { premise: input.premise });
  next = addCreatorMapDraft(next, { name: townName, kind: "town", width: 12, height: 12 });
  next = addCreatorMapDraft(next, { name: routeName, kind: "route", width: 16, height: 10 });
  next = addCreatorMapDraft(next, { name: caveName, kind: "cave", width: 14, height: 12 });
  for (const pokemon of [
    { id: "sproutcub", name: "Sproutcub", types: ["GRASS"] },
    { id: "flintot", name: "Flintot", types: ["FIRE"] },
    { id: "bubblit", name: "Bubblit", types: ["WATER"] },
    { id: "mothpin", name: "Mothpin", types: ["BUG"] },
    { id: "stoneel", name: "Stoneel", types: ["ROCK"] },
  ]) {
    next = upsertCreatorPokemon(next, pokemon);
  }
  next = upsertCreatorTrainer(next, {
    name: "Route Scout",
    trainer_class: "YOUNGSTER",
    party: [{ species: "mothpin", level: 4 }],
  });
  next = upsertCreatorTrainer(next, {
    name: "Cave Guide",
    trainer_class: "HIKER",
    party: [{ species: "stoneel", level: 6 }],
  });
  next = {
    ...next,
    encounterTables: [
      {
        id: slugify(routeName),
        map: routeName,
        encounters: [
          { species: "mothpin", minLevel: 2, maxLevel: 4, rate: 60 },
          { species: "sproutcub", minLevel: 3, maxLevel: 5, rate: 40 },
        ],
      },
      {
        id: slugify(caveName),
        map: caveName,
        encounters: [
          { species: "stoneel", minLevel: 4, maxLevel: 6, rate: 70 },
          { species: "bubblit", minLevel: 3, maxLevel: 5, rate: 30 },
        ],
      },
    ],
    npcs: [
      { id: `${slugify(townName)}-guide`, map: townName, name: "Guide", dialogue: "The road ahead is open." },
      { id: `${slugify(routeName)}-sign`, map: routeName, name: "Sign", dialogue: "Wild grass starts here." },
    ],
    audioTokens: ["town-theme", "route-theme", "cave-theme", "battle-sting"],
    storyBeats: [
      { id: "1-departure", type: "scene", title: "Departure", map: townName, text: input.premise ?? next.premise },
      { id: "2-first-route", type: "battle", title: "First Route", map: routeName },
      { id: "3-cave-crossing", type: "trigger", title: "Cave Crossing", map: caveName },
    ],
    updatedAt: nowIso(),
  };
  return next;
};

export const summarizeCreatorProjectForAgent = (project: CreatorProject) => ({
  id: project.id,
  packId: project.packId,
  title: project.title,
  description: project.description,
  premise: project.premise,
  counts: {
    maps: project.maps.length,
    storyBeats: project.storyBeats.length,
    pokemon: project.pokemon.length,
    trainers: project.trainers.length,
    encounterTables: project.encounterTables.length,
    npcs: project.npcs.length,
    audioTokens: project.audioTokens.length,
  },
  maps: project.maps.map(({ id, name, kind, width, height }) => ({ id, name, kind, width, height })),
  pokemon: project.pokemon.map(({ id, name, types }) => ({ id, name, types })),
  trainers: project.trainers.map(({ id, name, trainer_class }) => ({ id, name, trainer_class })),
  updatedAt: project.updatedAt,
});
