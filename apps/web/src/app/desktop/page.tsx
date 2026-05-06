import PlayPanel from "@/app/play-panel.lazy";

const HomeDesktopPage = () => (
  <div data-testid="desktop-page-shell" className="h-dvh min-h-0 w-full overflow-hidden">
    <PlayPanel variant="desktop" />
  </div>
);

export default HomeDesktopPage;
