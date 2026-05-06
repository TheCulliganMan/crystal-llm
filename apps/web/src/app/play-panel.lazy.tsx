"use client";

import dynamic from "next/dynamic";
import type { PlayPanelProps } from "./play-panel";

const PlayPanel = dynamic<PlayPanelProps>(() => import("./play-panel"), {
  // Keep the highly interactive panel client-only to avoid SSR/client hydration drift.
  ssr: false,
  loading: () => <div data-testid="play-panel-loading">Loading play panel...</div>,
});

export default PlayPanel;
