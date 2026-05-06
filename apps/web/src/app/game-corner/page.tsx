import { GameCornerPageContent } from "@/app/game-corner/page-content";
import { DEFAULT_GAME_CORNER_TAB, isGameCornerTab, type GameCornerTab } from "@/app/game-corner/tabs";

type GameCornerPageSearchParams = {
  tab?: string | string[];
};

type GameCornerPageProps = {
  searchParams?: Promise<GameCornerPageSearchParams>;
};

const resolveInitialTab = (searchParams?: GameCornerPageSearchParams): GameCornerTab => {
  const rawTab = searchParams?.tab;
  const requestedTab = Array.isArray(rawTab) ? rawTab[0] : rawTab;
  if (isGameCornerTab(requestedTab)) {
    return requestedTab;
  }
  return DEFAULT_GAME_CORNER_TAB;
};

const GameCornerPage = async ({ searchParams }: GameCornerPageProps) => {
  const resolvedSearchParams = searchParams ? await searchParams : undefined;
  return <GameCornerPageContent initialTab={resolveInitialTab(resolvedSearchParams)} />;
};

export default GameCornerPage;
