import fs from "fs";
import { assetExists } from "@pokecrystal/core/core/asset-manifest";
import { gameEngine } from "./game-engine";

const pendingImagePreloads = new Set<string>();

export const asset_can_defer_in_browser = (path: string): boolean =>
  assetExists(path) || fs.existsSync(path);

export const ensure_image_preload = (path: string): boolean => {
  if (!asset_can_defer_in_browser(path)) {
    return false;
  }
  if (pendingImagePreloads.has(path)) {
    return true;
  }
  const preload = gameEngine.image.preload;
  if (typeof preload !== "function") {
    return false;
  }
  pendingImagePreloads.add(path);
  void preload(path)
    .catch(() => null)
    .finally(() => {
      pendingImagePreloads.delete(path);
    });
  return true;
};

export const is_image_preload_pending = (path: string): boolean => pendingImagePreloads.has(path);

export const reset_deferred_image_preloads_for_test = (): void => {
  pendingImagePreloads.clear();
};
