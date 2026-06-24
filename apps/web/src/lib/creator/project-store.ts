import fs from "node:fs/promises";
import path from "node:path";
import {
  createEmptyCreatorProject,
} from "./agent-actions";
import type { CreatorProject } from "./agent-actions";

type CreatorValidation = {
  errors: string[];
  warnings: string[];
  generatedFiles: string[];
  nextActions: string[];
};

type ContentPackIndex = {
  version: number;
  packs: Array<{
    id: string;
    name?: string;
    enabled?: boolean;
    path: string;
    priority?: number;
    files?: Record<string, string[]>;
  }>;
};

const CONTENT_PACK_CATEGORIES = [
  "maps",
  "pokemon",
  "trainers",
  "wild_encounters",
  "npcs",
  "story_events",
] as const;

const creatorAssetsRoot = (): string =>
  process.env.POKECRYSTAL_CREATOR_ASSETS_ROOT ?? path.resolve(process.cwd(), "apps/web/assets");

const contentPacksRoot = (): string => path.join(creatorAssetsRoot(), "data", "content-packs");

const projectDir = (projectId: string): string => path.join(contentPacksRoot(), projectId);

const projectPath = (projectId: string): string => path.join(projectDir(projectId), "creator-project.json");

const indexPath = (): string => path.join(contentPacksRoot(), "index.json");

const readJson = async <T>(filePath: string): Promise<T> => JSON.parse(await fs.readFile(filePath, "utf8")) as T;

const writeJson = async (filePath: string, value: unknown): Promise<void> => {
  await fs.mkdir(path.dirname(filePath), { recursive: true });
  await fs.writeFile(filePath, `${JSON.stringify(value, null, 2)}\n`);
};

const readIndex = async (): Promise<ContentPackIndex> => {
  try {
    return await readJson<ContentPackIndex>(indexPath());
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") {
      return { version: 1, packs: [] };
    }
    throw error;
  }
};

const normalizeProject = (project: CreatorProject): CreatorProject => ({
  ...project,
  packId: project.packId || project.id,
  maps: project.maps ?? [],
  storyBeats: project.storyBeats ?? [],
  pokemon: project.pokemon ?? [],
  trainers: project.trainers ?? [],
  encounterTables: project.encounterTables ?? [],
  npcs: project.npcs ?? [],
  audioTokens: project.audioTokens ?? [],
});

const categoryFile = (project: CreatorProject, category: (typeof CONTENT_PACK_CATEGORIES)[number]): string =>
  `content-packs/${project.packId}/${category}/${project.id}.json`;

const writeContentPackFiles = async (project: CreatorProject): Promise<Record<string, string[]>> => {
  const files: Record<string, string[]> = {};
  const payloads: Record<(typeof CONTENT_PACK_CATEGORIES)[number], unknown> = {
    maps: project.maps,
    pokemon: project.pokemon,
    trainers: project.trainers,
    wild_encounters: project.encounterTables,
    npcs: project.npcs,
    story_events: project.storyBeats,
  };

  for (const category of CONTENT_PACK_CATEGORIES) {
    const relativePath = categoryFile(project, category);
    files[category] = [relativePath];
    await writeJson(path.join(creatorAssetsRoot(), "data", relativePath), payloads[category]);
  }

  return files;
};

export const createCreatorProject = async (title: string): Promise<CreatorProject> => {
  const project = createEmptyCreatorProject(title);
  return saveCreatorProject(project);
};

export const saveCreatorProject = async (project: CreatorProject): Promise<CreatorProject> => {
  const normalized = normalizeProject({ ...project, updatedAt: new Date().toISOString() });
  await writeJson(projectPath(normalized.id), normalized);
  return normalized;
};

export const readCreatorProject = async (projectId: string): Promise<CreatorProject> =>
  normalizeProject(await readJson<CreatorProject>(projectPath(projectId)));

export const listCreatorProjects = async (): Promise<Array<{ id: string; title: string; packId: string; updatedAt: string }>> => {
  try {
    const entries = await fs.readdir(contentPacksRoot(), { withFileTypes: true });
    const projects = await Promise.all(
      entries
        .filter((entry) => entry.isDirectory())
        .map(async (entry) => {
          try {
            const project = await readCreatorProject(entry.name);
            return {
              id: project.id,
              title: project.title,
              packId: project.packId,
              updatedAt: project.updatedAt,
            };
          } catch {
            return undefined;
          }
        })
    );
    return projects
      .filter((project): project is { id: string; title: string; packId: string; updatedAt: string } => Boolean(project))
      .sort((a, b) => a.title.localeCompare(b.title));
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") {
      return [];
    }
    throw error;
  }
};

export const validateCreatorProject = (project: CreatorProject): CreatorValidation => {
  const errors: string[] = [];
  const warnings: string[] = [];

  if (!project.title.trim()) {
    errors.push("Project title is required.");
  }
  if (project.maps.length === 0) {
    warnings.push("Add at least one map before activating the content pack.");
  }
  if (project.pokemon.length === 0) {
    warnings.push("Add at least one Pokemon before activating the content pack.");
  }
  for (const trainer of project.trainers) {
    if (trainer.party.length === 0) {
      warnings.push(`Trainer ${trainer.name} has no party members.`);
    }
  }

  return {
    errors,
    warnings,
    generatedFiles: CONTENT_PACK_CATEGORIES.map((category) => categoryFile(project, category)),
    nextActions: errors.length > 0 ? ["Fix validation errors."] : ["Activate the project when ready."],
  };
};

export const activateCreatorProject = async (projectId: string): Promise<CreatorProject> => {
  const project = await readCreatorProject(projectId);
  const validation = validateCreatorProject(project);
  if (validation.errors.length > 0) {
    throw new Error(`Cannot activate creator project ${projectId}: ${validation.errors.join(" ")}`);
  }

  const files = await writeContentPackFiles(project);
  const index = await readIndex();
  const pack = {
    id: project.packId,
    name: project.title,
    enabled: true,
    path: `content-packs/${project.packId}`,
    priority: 100,
    files,
  };
  const packs = index.packs.filter((entry) => entry.id !== project.packId);
  await writeJson(indexPath(), { version: index.version || 1, packs: [...packs, pack] });
  return project;
};
