'use client';

import { useCallback, useEffect, useState } from "react";
import { usePathname } from "next/navigation";
import dynamic from "next/dynamic";
import { TimeOfDay } from "@pokecrystal/core/core/enums";
import { useSupabase } from "@/components/providers/supabase-provider";
import { resolveTopBarLabel } from "@/components/layout/navigation-config";
import {
  THEME_STORAGE_KEY,
  isThemeKey,
  type ThemeKey,
} from "@/app/theme-preferences";

const AuthPanel = dynamic(() => import("@/components/arena/auth-panel"), {
  loading: () => (
    <div role="status" aria-live="polite" className="skeleton h-20 w-full bg-base-200">
      <span className="sr-only">Loading account controls...</span>
    </div>
  ),
});

const DAISY_FOCUS_RING =
  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2";

const toTimeOfDay = (themeKey: ThemeKey): TimeOfDay => {
  switch (themeKey) {
    case "morning":
      return TimeOfDay.MORN;
    case "night":
      return TimeOfDay.NIGHT;
    default:
      return TimeOfDay.DAY;
  }
};

const TIME_OF_DAY_OPTIONS: Array<{ value: ThemeKey; label: string }> = [
  { value: "morning", label: "Morning" },
  { value: "day", label: "Day" },
  { value: "night", label: "Night" },
];

const ThemeIcon = ({ theme }: { theme: ThemeKey }) => {
  if (theme === "morning") {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true" className="h-3.5 w-3.5">
        <path
          d="M4 16h16M6 13h12M8 10h8M12 4v3m-5 9 2-2m8 2-2-2"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.8"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
    );
  }
  if (theme === "night") {
    return (
      <svg viewBox="0 0 24 24" aria-hidden="true" className="h-3.5 w-3.5">
        <path
          d="M16.5 3.8a8.8 8.8 0 1 0 3.7 13.8A9.3 9.3 0 0 1 16.5 3.8z"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.8"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
    );
  }
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true" className="h-3.5 w-3.5">
      <circle cx="12" cy="12" r="3.4" fill="none" stroke="currentColor" strokeWidth="1.8" />
      <path
        d="M12 2.5v2.2M12 19.3v2.2M2.5 12h2.2M19.3 12h2.2M5.6 5.6l1.6 1.6M16.8 16.8l1.6 1.6M18.4 5.6l-1.6 1.6M7.2 16.8l-1.6 1.6"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
      />
    </svg>
  );
};

const THEME_CHANGED_EVENT = "pokecrystal:theme-change";

export const TopBar = () => {
  const pathname = usePathname() || "/";
  const { supabaseClient, session } = useSupabase();
  const [authOpen, setAuthOpen] = useState(false);
  const [themeKey, setThemeKey] = useState<ThemeKey>("day");
  const [isHydrated, setIsHydrated] = useState(false);
  const routeLabel = resolveTopBarLabel(pathname);
  const [displayedUserLabel, setDisplayedUserLabel] = useState("Guest session");
  const [authLabel, setAuthLabel] = useState("Login");

  useEffect(() => {
    setIsHydrated(true);
  }, []);

  useEffect(() => {
    if (!isHydrated) {
      return;
    }
    setDisplayedUserLabel(session?.user?.email ?? "Guest session");
    setAuthLabel(session?.user ? "Account" : "Login");
  }, [
    isHydrated,
    session?.user?.email,
    session?.user,
  ]);

  useEffect(() => {
    const root = document.documentElement;
    const currentTheme = root.getAttribute("data-theme");
    const savedTheme = typeof window !== "undefined" ? window.localStorage.getItem(THEME_STORAGE_KEY) : null;

    const initialTheme: ThemeKey = isThemeKey(currentTheme)
      ? currentTheme
      : isThemeKey(savedTheme)
        ? savedTheme
        : "day";

    root.setAttribute("data-theme", initialTheme);
    setThemeKey(initialTheme);
  }, []);

  const persistThemeToServer = useCallback(
    (nextTheme: ThemeKey) => {
      if (!supabaseClient || !session?.user) {
        return;
      }

      const timeOfDay = toTimeOfDay(nextTheme);
      void (async () => {
        try {
          const { error } = await supabaseClient
            .from("play_user_settings")
            .upsert({ user_id: session.user.id, time_of_day: timeOfDay }, { onConflict: "user_id" });
          if (error) {
            console.warn("[top-bar] failed to persist time-of-day theme", error.message);
          }
        } catch {
          // Ignore persistence failures and keep local preference only.
        }
      })();
    },
    [session, supabaseClient]
  );

  const handleThemeChange = (nextTheme: ThemeKey) => {
    const root = document.documentElement;
    root.setAttribute("data-theme", nextTheme);
    setThemeKey(nextTheme);
    if (typeof window !== "undefined") {
      window.localStorage.setItem(THEME_STORAGE_KEY, nextTheme);
      window.dispatchEvent(
        new CustomEvent(THEME_CHANGED_EVENT, {
          detail: { themeKey: nextTheme },
        })
      );
    }
    persistThemeToServer(nextTheme);
  };

  return (
    <>
      <nav className="navbar kc-surface-bar min-h-[var(--nav-top-height)] gap-2 overflow-visible border-b border-base-300 px-2 py-1.5 shadow-sm sm:px-3 md:px-4 md:py-2">
        <div className="navbar-start min-w-0">
          <div className="flex min-w-0 flex-col">
            <span className="badge badge-xs badge-outline max-w-28 truncate border-base-300 text-base-content/60 sm:max-w-none">
              {routeLabel}
            </span>
            <span className="truncate text-sm font-semibold tracking-wide text-base-content">
              KrabbyClaw
            </span>
          </div>
        </div>

        <div className="navbar-end min-w-0 flex-1 gap-1.5 sm:gap-2" suppressHydrationWarning>
          <span
            className="hidden max-w-40 truncate text-sm text-base-content/70 lg:inline-block"
            suppressHydrationWarning
          >
            {displayedUserLabel}
          </span>
          <div className="flex min-w-0 items-center justify-end gap-1.5 sm:gap-2">
            <div className="flex min-w-0 items-center gap-1.5 sm:gap-2" role="group" aria-label="Time-of-day theme">
              <span className="hidden text-xs font-semibold uppercase tracking-[0.18em] text-base-content/65 sm:inline">
                Theme
              </span>
              <div className="join">
                {TIME_OF_DAY_OPTIONS.map((option) => (
                  <button
                    key={option.value}
                    type="button"
                    className={`btn btn-xs join-item min-h-8 w-8 px-0 ${themeKey === option.value ? "btn-primary" : "btn-outline"} ${DAISY_FOCUS_RING}`}
                    aria-label={option.label}
                    title={option.label}
                    disabled={!isHydrated}
                    onClick={() => {
                      if (!isHydrated) {
                        return;
                      }
                      if (isThemeKey(option.value)) {
                        handleThemeChange(option.value);
                      }
                    }}
                  >
                    <ThemeIcon theme={option.value} />
                  </button>
                ))}
              </div>
            </div>
            <button
              type="button"
              onClick={() => setAuthOpen(true)}
              className={`btn btn-sm btn-outline min-h-8 shrink-0 px-2 sm:px-3 ${DAISY_FOCUS_RING}`}
              disabled={!isHydrated}
            >
              {authLabel}
            </button>
          </div>
        </div>
      </nav>

      {authOpen ? (
        <div className="modal modal-open" role="dialog" aria-modal="true">
          <div className="modal-box w-full max-w-sm p-0">
            <div className="border-b border-base-300 px-4 py-3">
              <h2 className="text-sm font-semibold">{authLabel}</h2>
            </div>
            <div className="p-4">
              <AuthPanel />
            </div>
            <div className="modal-action border-t border-base-300 p-4">
              <form method="dialog" className="w-full">
                <button
                  type="submit"
                  className={`btn btn-sm btn-outline ${DAISY_FOCUS_RING}`}
                  onClick={() => setAuthOpen(false)}
                  aria-label="Close auth panel"
                >
                  Close
                </button>
              </form>
            </div>
          </div>
          <button
            type="button"
            className="modal-backdrop"
            onClick={() => setAuthOpen(false)}
            aria-label="Close auth panel"
          />
        </div>
      ) : null}
    </>
  );
};
