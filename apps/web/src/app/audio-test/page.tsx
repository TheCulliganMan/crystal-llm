import type { Metadata } from "next";
import { AudioTestClient } from "./audio-test-client";
import { buildAudioTestCatalog } from "./catalog";

export const metadata: Metadata = {
  title: "Audio Test",
};

const AudioTestPage = () => {
  const catalog = buildAudioTestCatalog();

  return (
    <main
      data-testid="audio-test-page"
      className="h-[calc(100dvh-var(--nav-top-height)-var(--nav-mobile-height)-0.35rem)] overflow-hidden md:h-[calc(100dvh-var(--nav-top-height)-1rem)]"
    >
      <AudioTestClient entries={catalog.entries} stats={catalog.stats} />
    </main>
  );
};

export default AudioTestPage;
