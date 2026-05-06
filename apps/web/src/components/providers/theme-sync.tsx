"use client";

import { useEffect } from "react";
import { createSupabaseBrowserClient } from "@/lib/supabase/browser";
import { TimeOfDay } from "@pokecrystal/core/core/enums";
import { canonicaliseTimeOfDay } from "@pokecrystal/core/engine/systems/time";
import {
  BRAND_THEME_STORAGE_KEY,
  THEME_STORAGE_KEY,
  type ThemeKey,
  isBrandThemeKey,
  isThemeKey,
} from "@/app/theme-preferences";
import { applyBrandThemeToDocument } from "@/app/brand-theme-dom";
import { DAY_HOUR, MORN_HOUR, NITE_HOUR } from "@pokecrystal/core/engine/systems/time";

const applyThemeAttributes = (theme?: string | null, brandTheme?: string | null) => {
  if (typeof document === "undefined") {
    return;
  }
  const root = document.documentElement;
  if (isThemeKey(theme)) {
    root.setAttribute("data-theme", theme);
  }
  if (isBrandThemeKey(brandTheme)) {
    applyBrandThemeToDocument(brandTheme);
  }
};

const toThemeKey = (value?: string | null): ThemeKey | null => {
  if (isThemeKey(value)) {
    return value;
  }
  if (!value) {
    return null;
  }
  const canonical = canonicaliseTimeOfDay(value);
  if (canonical === TimeOfDay.MORN) {
    return "morning";
  }
  if (canonical === TimeOfDay.NIGHT) {
    return "night";
  }
  return "day";
};

export const ThemeSync = () => {
  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }

    const resolveLocalTheme = (): ThemeKey => {
      const hour = new Date().getHours();
      if (hour < MORN_HOUR) {
        return "night";
      }
      if (hour < DAY_HOUR) {
        return "morning";
      }
      if (hour < NITE_HOUR) {
        return "day";
      }
      return "night";
    };

    let localTheme = window.localStorage.getItem(THEME_STORAGE_KEY);
    if (!isThemeKey(localTheme)) {
      localTheme = resolveLocalTheme();
      window.localStorage.setItem(THEME_STORAGE_KEY, localTheme);
    }
    const localBrandTheme = window.localStorage.getItem(BRAND_THEME_STORAGE_KEY);
    applyThemeAttributes(localTheme, localBrandTheme);

    const supabase = createSupabaseBrowserClient();
    if (!supabase) {
      return;
    }

    let active = true;
    const syncFromUser = async (userId?: string | null) => {
      if (!active || !userId) {
        return;
      }
      const { data, error } = await supabase
        .from("play_user_settings")
        .select("time_of_day, brand_theme")
        .eq("user_id", userId)
        .maybeSingle();
      if (!active || error || !data) {
        return;
      }
      const storedTheme = toThemeKey(data.time_of_day);
      const storedBrandTheme = typeof data.brand_theme === "string" ? data.brand_theme : null;
      applyThemeAttributes(storedTheme, storedBrandTheme);
      if (isThemeKey(storedTheme)) {
        window.localStorage.setItem(THEME_STORAGE_KEY, storedTheme);
      }
      if (isBrandThemeKey(storedBrandTheme)) {
        window.localStorage.setItem(BRAND_THEME_STORAGE_KEY, storedBrandTheme);
      }
    };

    void supabase.auth
      .getUser()
      .then(({ data }) => syncFromUser(data.user?.id ?? null))
      .catch(() => undefined);

    const { data: listener } = supabase.auth.onAuthStateChange((_event, session) => {
      void syncFromUser(session?.user?.id ?? null);
    });

    return () => {
      active = false;
      listener.subscription.unsubscribe();
    };
  }, []);

  return null;
};

export default ThemeSync;
