"use client";

import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent, type FocusEvent as ReactFocusEvent } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import {
  MAIN_NAV_ITEMS,
  canonicalizeNavigationPath,
} from "@/components/layout/navigation-config";
import { GAME_CORNER_TABS } from "@/app/game-corner/tabs";

type MainNavMode = "desktop" | "mobile" | "both";

type MainNavProps = {
  mode?: MainNavMode;
};

const DAISY_NAV_FOCUS_RING =
  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-base-200";
const NAV_BUTTON_BASE = `btn btn-ghost btn-square ${DAISY_NAV_FOCUS_RING}`;
const DESKTOP_NAV_BUTTON = `${NAV_BUTTON_BASE} h-14 w-14 rounded-lg border border-transparent text-base-content/80 transition-all duration-200 hover:border-base-300 hover:bg-base-300/70 hover:text-base-content`;
const DESKTOP_NAV_BUTTON_ACTIVE = "btn-active btn-primary border-primary/35 text-primary-content shadow-[0_10px_24px_rgba(6,10,24,0.38)] hover:brightness-110";
const GAME_CORNER_FLYOUT_LINK = `rounded-md text-base-content/90 transition-colors duration-150 hover:bg-primary/20 hover:text-base-content focus-visible:bg-primary/20 ${DAISY_NAV_FOCUS_RING}`;
const NAV_ICON_CLASS = "size-5";
const GAME_CORNER_FLYOUT_CLOSE_DELAY_MS = 220;

const isActivePath = (pathname: string | null, href: string): boolean => {
  if (!pathname) {
    return false;
  }
  if (href === "/") {
    return pathname === "/";
  }
  const normalized = canonicalizeNavigationPath(pathname);
  return normalized === href || normalized.startsWith(`${href}/`);
};

const gameCornerHref = (tabId: string): string => `/game-corner?tab=${tabId}`;

export const MainNav = ({ mode = "both" }: MainNavProps = {}) => {
  const pathname = usePathname();
  const [isHydrated, setIsHydrated] = useState(false);
  const [isGameCornerOpen, setIsGameCornerOpen] = useState(false);
  const gameCornerCloseTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const gameCornerContainerRef = useRef<HTMLLIElement | null>(null);

  useEffect(() => {
    setIsHydrated(true);
  }, []);

  useEffect(() => {
    return () => {
      if (gameCornerCloseTimerRef.current !== null) {
        clearTimeout(gameCornerCloseTimerRef.current);
      }
    };
  }, []);

  const resolvedPathname =
    isHydrated && pathname ? canonicalizeNavigationPath(pathname) : null;

  const isActive = (href: string) =>
    resolvedPathname !== null ? isActivePath(resolvedPathname, href) : false;

  const cancelGameCornerClose = () => {
    if (gameCornerCloseTimerRef.current !== null) {
      clearTimeout(gameCornerCloseTimerRef.current);
      gameCornerCloseTimerRef.current = null;
    }
  };

  const openGameCornerFlyout = () => {
    cancelGameCornerClose();
    setIsGameCornerOpen(true);
  };

  const scheduleGameCornerClose = () => {
    cancelGameCornerClose();
    gameCornerCloseTimerRef.current = setTimeout(() => {
      setIsGameCornerOpen(false);
      gameCornerCloseTimerRef.current = null;
    }, GAME_CORNER_FLYOUT_CLOSE_DELAY_MS);
  };

  const handleGameCornerPointerEnter = () => {
    openGameCornerFlyout();
  };

  const handleGameCornerPointerLeave = (event: ReactPointerEvent<HTMLLIElement | HTMLDivElement | HTMLUListElement>) => {
    const nextTarget = event.relatedTarget;
    if (
      nextTarget instanceof Node &&
      gameCornerContainerRef.current?.contains(nextTarget)
    ) {
      return;
    }
    scheduleGameCornerClose();
  };

  const handleGameCornerFocus = () => {
    openGameCornerFlyout();
  };

  const handleGameCornerBlur = (event: ReactFocusEvent<HTMLLIElement>) => {
    const nextTarget = event.relatedTarget;
    if (
      nextTarget instanceof Node &&
      gameCornerContainerRef.current?.contains(nextTarget)
    ) {
      return;
    }
    scheduleGameCornerClose();
  };

  const desktopNavLinks = MAIN_NAV_ITEMS.map((item) => {
    const active = isActive(item.href);

    if (item.href === "/game-corner") {
      return (
        <li
          key={`${item.href}-desktop`}
          ref={gameCornerContainerRef}
          className="relative"
          onPointerEnter={handleGameCornerPointerEnter}
          onPointerLeave={handleGameCornerPointerLeave}
          onFocus={handleGameCornerFocus}
          onBlur={handleGameCornerBlur}
        >
          <Link
            href={gameCornerHref("slot-machine")}
            className={`${DESKTOP_NAV_BUTTON} ${active ? DESKTOP_NAV_BUTTON_ACTIVE : ""}`}
            aria-current={active ? "page" : undefined}
            title={item.label}
            aria-label={item.label}
          >
            <FontAwesomeIcon icon={item.icon} className={NAV_ICON_CLASS} />
          </Link>
          <div
            aria-hidden="true"
            data-testid="game-corner-hover-bridge"
            className={`absolute left-full top-1/2 z-[85] h-24 w-3 -translate-y-1/2 ${isGameCornerOpen ? "pointer-events-auto" : "pointer-events-none"}`}
            onPointerEnter={handleGameCornerPointerEnter}
            onPointerLeave={handleGameCornerPointerLeave}
          />
          <ul
            className={`menu absolute left-full top-1/2 z-[90] ml-3 min-w-56 -translate-y-1/2 rounded-lg border border-base-300 bg-base-100 p-2 shadow-xl transition-opacity duration-150 ${isGameCornerOpen ? "visible pointer-events-auto opacity-100" : "invisible pointer-events-none opacity-0"}`}
            data-testid="game-corner-flyout"
          >
            {GAME_CORNER_TABS.map((tab) => {
              return (
                <li key={tab.id}>
                  <Link
                    href={gameCornerHref(tab.id)}
                    className={GAME_CORNER_FLYOUT_LINK}
                    aria-label={`Game Corner: ${tab.label}`}
                  >
                    {tab.label}
                  </Link>
                </li>
              );
            })}
          </ul>
        </li>
      );
    }

    return (
      <li key={`${item.href}-desktop`}>
        <Link
          href={item.href}
          className={`${DESKTOP_NAV_BUTTON} ${active ? DESKTOP_NAV_BUTTON_ACTIVE : ""}`}
          aria-current={active ? "page" : undefined}
          title={item.label}
          aria-label={item.label}
        >
          <FontAwesomeIcon icon={item.icon} className={NAV_ICON_CLASS} />
        </Link>
      </li>
    );
  });

  const mobileNavLinks = MAIN_NAV_ITEMS.map((item) => {
    const active = isActive(item.href);
    return (
      <Link
        key={`${item.href}-mobile`}
        href={item.href === "/game-corner" ? gameCornerHref("slot-machine") : item.href}
        className={`${active ? "dock-active" : ""} ${DAISY_NAV_FOCUS_RING}`}
        aria-current={active ? "page" : undefined}
        title={item.label}
        aria-label={item.label}
      >
        <FontAwesomeIcon icon={item.icon} className={NAV_ICON_CLASS} />
        <span className="dock-label">{item.label}</span>
      </Link>
    );
  });

  const includeDesktop = mode === "desktop" || mode === "both";
  const includeMobile = mode === "mobile" || mode === "both";

  return (
    <>
      {includeDesktop ? (
        <aside className="kc-surface-bar relative z-30 hidden min-h-dvh min-w-20 self-stretch overflow-visible border-r border-r-base-300 p-2 pt-2 md:sticky md:top-0 md:flex md:flex-col md:items-stretch">
          <Link
            href="/"
            className={`kc-surface-card ${DESKTOP_NAV_BUTTON} mx-1 overflow-hidden p-0`}
            aria-label="KrabbyClaw home"
          >
            <span
              className="kc-brand-sprite rounded-none"
              style={{ transform: "scale(1.68)", transformOrigin: "center" }}
              aria-hidden="true"
            />
          </Link>
          <nav
            className="relative z-10 mt-3 flex-1 overflow-visible"
            role="navigation"
            aria-label="Main desktop navigation"
          >
            <ul className="menu menu-md gap-1 overflow-visible px-1 py-2">{desktopNavLinks}</ul>
          </nav>
        </aside>
      ) : null}

      {includeMobile ? <nav className="kc-surface-bar dock dock-sm z-[80] overflow-visible border-t border-base-300 md:hidden">{mobileNavLinks}</nav> : null}
    </>
  );
};
