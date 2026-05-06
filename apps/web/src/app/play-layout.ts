export const GAMEBOY_ASPECT_RATIO = 160 / 144;

type FullscreenCanvasLayoutInput = {
  viewportWidth: number;
  viewportHeight: number;
};

export type FullscreenCanvasLayout = {
  frameWidth: number;
  frameHeight: number;
  framePadding: number;
  shellPaddingX: number;
  shellPaddingY: number;
};

const clamp = (value: number, min: number, max: number): number => Math.min(Math.max(value, min), max);

const fitAspectRatio = (availableWidth: number, availableHeight: number): { frameWidth: number; frameHeight: number } => {
  if (availableWidth / availableHeight > GAMEBOY_ASPECT_RATIO) {
    const frameHeight = Math.max(1, Math.floor(availableHeight));
    const frameWidth = Math.max(1, Math.floor(frameHeight * GAMEBOY_ASPECT_RATIO));
    return { frameWidth, frameHeight };
  }

  const frameWidth = Math.max(1, Math.floor(availableWidth));
  const frameHeight = Math.max(1, Math.floor(frameWidth / GAMEBOY_ASPECT_RATIO));
  return { frameWidth, frameHeight };
};

export const computeFullscreenCanvasLayout = ({
  viewportWidth,
  viewportHeight,
}: FullscreenCanvasLayoutInput): FullscreenCanvasLayout => {
  const width = Number.isFinite(viewportWidth) && viewportWidth > 0 ? viewportWidth : 1280;
  const height = Number.isFinite(viewportHeight) && viewportHeight > 0 ? viewportHeight : 720;

  const shellPaddingX = clamp(Math.round(width * 0.035), 10, 32);
  const shellPaddingY = clamp(Math.round(height * 0.03), 8, 24);
  const framePadding = clamp(Math.round(Math.min(width, height) * 0.02), 8, 22);

  const availableWidth = Math.max(1, width - shellPaddingX * 2 - framePadding * 2);
  const availableHeight = Math.max(1, height - shellPaddingY * 2 - framePadding * 2);

  const { frameWidth, frameHeight } = fitAspectRatio(availableWidth, availableHeight);

  return {
    frameWidth,
    frameHeight,
    framePadding,
    shellPaddingX,
    shellPaddingY,
  };
};
