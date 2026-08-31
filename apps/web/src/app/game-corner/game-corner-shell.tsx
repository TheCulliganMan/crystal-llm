"use client";

import { GameCornerClient } from "@/app/game-corner/game-corner-client";
import { KrabbyClawArenaPanel } from "@/app/game-corner/krabbyclaw-arena-panel";
import { ProgressTrackerPanel } from "@/app/game-corner/progress-tracker-panel";
import {
  DEFAULT_GAME_CORNER_TAB,
  GAME_CORNER_TABS,
  type GameCornerTab,
} from "@/app/game-corner/tabs";

type GameCornerShellProps = {
  initialTab?: GameCornerTab;
};

const TAB_HINTS: Record<GameCornerTab, string> = {
  "slot-machine": "Classic Goldenrod slots and coin controls.",
  "arena-mcp-skill": "Arena controls plus MCP/skill entry points for automation.",
  "progress-tracker": "Track end-to-end story completion with a live Mermaid flowchart.",
};

export const GameCornerShell = ({
  initialTab = DEFAULT_GAME_CORNER_TAB,
}: GameCornerShellProps) => {
  const activeTab = GAME_CORNER_TABS.find((tab) => tab.id === initialTab)
    ? initialTab
    : DEFAULT_GAME_CORNER_TAB;
  const activeLabel = GAME_CORNER_TABS.find((tab) => tab.id === activeTab)?.label ?? "Game Corner";

  return (
    <section className="space-y-3" data-testid="game-corner-shell">
      <section className="kc-arena-hero rounded-[1.25rem] px-4 py-4 md:px-5">
        <div className="grid gap-4 md:grid-cols-[minmax(0,1.2fr)_minmax(16rem,0.8fr)] md:items-end">
          <div className="space-y-2">
            <p className="kc-arena-kicker">Goldenrod Game Corner</p>
            <h2 className="kc-arena-display text-2xl font-semibold md:text-3xl">{activeLabel}</h2>
            <p className="text-sm kc-arena-muted">{TAB_HINTS[activeTab]}</p>
          </div>
          <div className="flex flex-wrap gap-2 md:justify-end">
            {GAME_CORNER_TABS.map((tab) => (
              <span
                key={tab.id}
                className={`kc-arena-chip ${tab.id === activeTab ? "border-base-content/20 bg-base-100/95" : ""}`}
              >
                {tab.label}
              </span>
            ))}
          </div>
        </div>
      </section>

      <div data-testid="game-corner-active-content">
        {activeTab === "slot-machine" ? (
          <section className="kc-surface-card card card-bordered border-base-300 bg-base-200/90" data-testid="game-corner-slot-machine">
            <div className="card-body gap-3">
              <GameCornerClient initialTab="slot-machine" />
            </div>
          </section>
        ) : null}

        {activeTab === "arena-mcp-skill" ? (
          <section className="kc-arena-shell rounded-[1.6rem] p-3 md:p-4" data-testid="game-corner-arena-mcp-skill">
            <div className="space-y-3">
              <div className="kc-arena-card flex flex-wrap items-center justify-between gap-3 rounded-[1.25rem] p-4">
                <div className="space-y-1">
                  <p className="kc-arena-kicker">Arena integration</p>
                  <div className="text-sm kc-arena-muted">Arena queue, MCP endpoint, and skill downloads in one shared control surface.</div>
                </div>
                <div className="flex flex-wrap gap-2">
                  <a className="btn btn-sm" href="/mcp">
                    Open MCP Console
                  </a>
                  <a className="btn btn-sm btn-outline" href="/downloads/krabbyclaw-skill.zip" download>
                    Download Play Skill
                  </a>
                  <a className="btn btn-sm btn-outline" href="/downloads/krabbyclaw-arena-skill.zip" download>
                    Download Arena Skill
                  </a>
                  <a className="btn btn-sm btn-outline" href="/downloads/krabbyclaw-progress-tracker-skill.zip" download>
                    Download Progress Tracker Skill
                  </a>
                </div>
              </div>
              <KrabbyClawArenaPanel />
            </div>
          </section>
        ) : null}

        {activeTab === "progress-tracker" ? (
          <section className="kc-surface-card card card-bordered border-base-300 bg-base-200/90" data-testid="game-corner-progress-tracker">
            <div className="card-body gap-3">
              <ProgressTrackerPanel />
            </div>
          </section>
        ) : null}
      </div>
    </section>
  );
};
