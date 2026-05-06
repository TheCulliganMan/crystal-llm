import type { GameState } from "@pokecrystal/core/core/state";

type UnownOverlayOverworld = {
  input_capture_active?: boolean;
} | null | undefined;

type UnownOverlayLockRuntime = GameState & {
  __unown_overlay_lock_depth__?: number;
  __unown_overlay_prev_state__?: number;
  __unown_overlay_capture_owner__?: {
    input_capture_active?: boolean;
  } | null;
  __unown_overlay_prev_capture__?: boolean;
};

export const getUnownOverlayLockDepth = (game_state: GameState): number =>
  (game_state as UnownOverlayLockRuntime).__unown_overlay_lock_depth__ ?? 0;

export const acquireUnownOverlayLock = (
  game_state: GameState,
  overworld: UnownOverlayOverworld,
): (() => void) => {
  const runtime = game_state as UnownOverlayLockRuntime;
  const depth = runtime.__unown_overlay_lock_depth__ ?? 0;

  if (depth === 0) {
    runtime.__unown_overlay_prev_state__ = game_state.wram.wUnownState ?? 0;
    runtime.__unown_overlay_capture_owner__ = overworld ?? null;
    runtime.__unown_overlay_prev_capture__ = Boolean(overworld?.input_capture_active);
  } else if (!runtime.__unown_overlay_capture_owner__ && overworld) {
    runtime.__unown_overlay_capture_owner__ = overworld;
    runtime.__unown_overlay_prev_capture__ = Boolean(overworld.input_capture_active);
  }

  runtime.__unown_overlay_lock_depth__ = depth + 1;
  if (overworld) {
    overworld.input_capture_active = true;
  }
  game_state.wram.wUnownState = 1;

  let released = false;
  return () => {
    if (released) {
      return;
    }
    released = true;

    const currentDepth = runtime.__unown_overlay_lock_depth__ ?? 0;
    const nextDepth = Math.max(0, currentDepth - 1);
    runtime.__unown_overlay_lock_depth__ = nextDepth;
    if (nextDepth > 0) {
      return;
    }

    game_state.wram.wUnownState = runtime.__unown_overlay_prev_state__ ?? 0;
    const captureOwner = runtime.__unown_overlay_capture_owner__ ?? null;
    if (captureOwner) {
      captureOwner.input_capture_active = Boolean(runtime.__unown_overlay_prev_capture__);
    }

    delete runtime.__unown_overlay_prev_state__;
    delete runtime.__unown_overlay_capture_owner__;
    delete runtime.__unown_overlay_prev_capture__;
  };
};
