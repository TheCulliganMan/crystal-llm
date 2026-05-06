import PlayPanel from "./play-panel.lazy";

const HomePage = () => (
  <main
    data-testid="play-page-shell"
    className="h-[calc(100dvh-var(--nav-top-height)-var(--nav-mobile-height)-0.35rem)] w-full overflow-hidden md:h-[calc(100dvh-var(--nav-top-height)-1rem)]"
  >
    <PlayPanel />
  </main>
);

export default HomePage;
