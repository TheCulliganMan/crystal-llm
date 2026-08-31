import { GameCornerShell } from "@/app/game-corner/game-corner-shell";
import { DEFAULT_GAME_CORNER_TAB, type GameCornerTab } from "@/app/game-corner/tabs";

type GameCornerPageContentProps = {
  initialTab?: GameCornerTab;
};

export const GameCornerPageContent = ({
  initialTab = DEFAULT_GAME_CORNER_TAB,
}: GameCornerPageContentProps) => (
  <main data-testid="route-game-corner" className="mx-auto w-full max-w-6xl px-4">
    <section className="w-full space-y-4 pt-2 pb-4 md:pt-3">
      <article className="kc-arena-shell rounded-[1.6rem] p-3 md:p-4">
        <div className="kc-arena-hero rounded-[1.3rem] px-5 py-6 md:px-7 md:py-7">
          <div className="flex flex-col gap-5 lg:flex-row lg:items-end lg:justify-between">
            <div className="max-w-3xl space-y-3">
              <p className="kc-arena-kicker">Casino Floor</p>
              <div className="space-y-2">
                <h1 className="kc-arena-display text-3xl font-semibold md:text-4xl">Game Corner</h1>
                <p className="max-w-2xl text-sm leading-6 kc-arena-muted md:text-base">
                  One control room for spectacle and operation: arcade play, Arena automation, and a story-tracking
                  planner that feels deliberate instead of dashboard-heavy.
                </p>
              </div>
            </div>

            <div className="kc-arena-stat-grid w-full max-w-xl">
              <div className="kc-arena-stat-card">
                <span>Slots</span>
                <strong>Live</strong>
              </div>
              <div className="kc-arena-stat-card">
                <span>Arena</span>
                <strong>MCP + Skills</strong>
              </div>
            </div>
          </div>
        </div>
      </article>

      <article className="kc-arena-shell rounded-[1.6rem] p-3 md:p-4">
        <div className="rounded-[1.3rem]">
          <GameCornerShell initialTab={initialTab} />
        </div>
      </article>
    </section>
  </main>
);
