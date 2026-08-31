import { faDice } from "@fortawesome/free-solid-svg-icons/faDice";
import { faEye } from "@fortawesome/free-solid-svg-icons/faEye";
import { faPenNib } from "@fortawesome/free-solid-svg-icons/faPenNib";
import { faPlay } from "@fortawesome/free-solid-svg-icons/faPlay";
import { faTrophy } from "@fortawesome/free-solid-svg-icons/faTrophy";

export type NavItem = Readonly<{
  label: string;
  href: `/${string}`;
  icon: typeof faPlay;
}>;

export const MAIN_NAV_ITEMS: readonly NavItem[] = [
  { label: "Play", href: "/", icon: faPlay },
  { label: "Connect", href: "/mcp", icon: faPenNib },
  { label: "Leaderboard", href: "/leaderboard", icon: faTrophy },
  { label: "Watch", href: "/watch", icon: faEye },
  { label: "Game Corner", href: "/game-corner", icon: faDice },
] as const;

export const canonicalizeNavigationPath = (pathname: string): string => {
  if (pathname.startsWith("/arena") || pathname.startsWith("/leaderboard")) {
    return "/leaderboard";
  }
  return pathname;
};

export const resolveTopBarLabel = (pathname: string): string => {
  const normalized = canonicalizeNavigationPath(pathname);
  const route = MAIN_NAV_ITEMS.find((item) =>
    item.href === "/" ? normalized === "/" : normalized.startsWith(item.href)
  );
  return route?.label ?? "Play";
};


export const GAME_CORNER_SUBNAV_ITEMS = [
  { label: "Game Corner", href: "/game-corner?tab=slot-machine" },
  { label: "Arena MCP/Skill", href: "/game-corner?tab=arena-mcp-skill" },
] as const;
