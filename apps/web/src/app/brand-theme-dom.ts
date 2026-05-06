import type { BrandThemeKey } from "./theme-preferences";

const BRAND_THEME_FAVICON_PATHS: Record<BrandThemeKey, string> = {
  krabby: "/favicon.png",
  kingler: "/favicon.png",
  heracross: "/favicon.png",
  gligar: "/favicon.png",
  scizor: "/favicon.png",
  sneasel: "/favicon.png",
  teddiursa: "/favicon.png",
  ursaring: "/favicon.png",
  totodile: "/favicon.png",
  croconaw: "/favicon.png",
  feraligatr: "/favicon.png",
  pinsir: "/favicon.png",
};

const BRAND_THEME_FAVICON_SPRITES: Record<BrandThemeKey, string> = {
  krabby: "/assets/gfx/pokemon/krabby/front.png",
  kingler: "/assets/gfx/pokemon/kingler/front.png",
  heracross: "/assets/gfx/pokemon/heracross/front.png",
  gligar: "/assets/gfx/pokemon/gligar/front.png",
  scizor: "/assets/gfx/pokemon/scizor/front.png",
  sneasel: "/assets/gfx/pokemon/sneasel/front.png",
  teddiursa: "/assets/gfx/pokemon/teddiursa/front.png",
  ursaring: "/assets/gfx/pokemon/ursaring/front.png",
  totodile: "/assets/gfx/pokemon/totodile/front.png",
  croconaw: "/assets/gfx/pokemon/croconaw/front.png",
  feraligatr: "/assets/gfx/pokemon/feraligatr/front.png",
  pinsir: "/assets/gfx/pokemon/pinsir/front.png",
};

const FAVICON_ANIMATION_FRAME_MS = 180;
const FAVICON_SIZE = 32;

let faviconAnimationStop: (() => void) | null = null;
let faviconAnimationTheme: BrandThemeKey | null = null;
let faviconAnimationToken = 0;

const stopFaviconAnimation = (): void => {
  if (!faviconAnimationStop) {
    return;
  }
  faviconAnimationStop();
  faviconAnimationStop = null;
  faviconAnimationTheme = null;
};

const updateFaviconHref = (href: string): void => {
  const iconLinks = document.querySelectorAll<HTMLLinkElement>('link[rel~="icon"]');

  if (iconLinks.length === 0) {
    const favicon = document.createElement("link");
    favicon.rel = "icon";
    favicon.type = "image/png";
    favicon.href = href;
    document.head.appendChild(favicon);
    return;
  }

  for (const iconLink of iconLinks) {
    iconLink.href = href;
  }
};

const startFaviconAnimation = (brandTheme: BrandThemeKey): void => {
  if (typeof window === "undefined") {
    return;
  }

  if (faviconAnimationTheme === brandTheme && faviconAnimationStop) {
    return;
  }

  stopFaviconAnimation();
  const token = ++faviconAnimationToken;
  const spriteSource = BRAND_THEME_FAVICON_SPRITES[brandTheme];
  const image = new Image();

  image.onload = () => {
    if (token !== faviconAnimationToken) {
      return;
    }

    const frameSize = image.naturalWidth;
    const frameCount = Math.max(1, Math.floor(image.naturalHeight / Math.max(frameSize, 1)));
    if (frameSize <= 0 || frameCount <= 1) {
      updateFaviconHref(BRAND_THEME_FAVICON_PATHS[brandTheme]);
      return;
    }

    const canvas = document.createElement("canvas");
    canvas.width = FAVICON_SIZE;
    canvas.height = FAVICON_SIZE;
    const context = canvas.getContext("2d");
    if (!context) {
      updateFaviconHref(BRAND_THEME_FAVICON_PATHS[brandTheme]);
      return;
    }

    let frameIndex = 0;
    const drawFrame = () => {
      context.clearRect(0, 0, FAVICON_SIZE, FAVICON_SIZE);
      context.drawImage(
        image,
        0,
        frameIndex * frameSize,
        frameSize,
        frameSize,
        0,
        0,
        FAVICON_SIZE,
        FAVICON_SIZE,
      );
      updateFaviconHref(canvas.toDataURL("image/png"));
      frameIndex = (frameIndex + 1) % frameCount;
    };

    drawFrame();
    const intervalId = window.setInterval(drawFrame, FAVICON_ANIMATION_FRAME_MS);
    faviconAnimationTheme = brandTheme;
    faviconAnimationStop = () => window.clearInterval(intervalId);
  };

  image.onerror = () => {
    if (token !== faviconAnimationToken) {
      return;
    }
    updateFaviconHref(BRAND_THEME_FAVICON_PATHS[brandTheme]);
  };

  image.src = spriteSource;
};

export const applyBrandThemeToDocument = (brandTheme: BrandThemeKey): void => {
  if (typeof document === "undefined") {
    return;
  }

  document.documentElement.setAttribute("data-brand-theme", brandTheme);
  updateFaviconHref(BRAND_THEME_FAVICON_PATHS[brandTheme]);
  startFaviconAnimation(brandTheme);
};
