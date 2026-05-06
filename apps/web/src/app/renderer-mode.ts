import type { RendererMode } from "./ui";

export const rendererModeCycle: ReadonlyArray<RendererMode> = ["tile", "both", "text"];

const rendererModeLabels: Record<RendererMode, string> = {
  tile: "Tile View",
  both: "Tile + Text",
  text: "Text View",
};

const rendererModeActionLabels: Record<RendererMode, string> = {
  tile: "Show Tile + Text",
  both: "Show Text View",
  text: "Show Tile View",
};

export const getNextRendererMode = (mode: RendererMode): RendererMode => {
  const currentIndex = rendererModeCycle.indexOf(mode);
  if (currentIndex === -1) {
    return rendererModeCycle[0];
  }
  const nextIndex = (currentIndex + 1) % rendererModeCycle.length;
  return rendererModeCycle[nextIndex];
};

export const getRendererModeLabel = (mode: RendererMode): string => rendererModeLabels[mode];

export const getRendererModeActionLabel = (mode: RendererMode): string => rendererModeActionLabels[mode];
