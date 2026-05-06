"use client";

import type { ReactNode } from "react";
import { AppRouterCacheProvider } from "@/components/ui/app-router-cache";

type TypographyOptions = Record<string, string | number>;

type ThemeOptions = {
  typography: Record<string, TypographyOptions | string>;
  shape: { borderRadius: number };
  palette?: {
    mode?: "light" | "dark";
    primary?: { main: string; contrastText?: string };
    secondary?: { main: string; contrastText?: string };
    error?: { main: string };
    background?: { default?: string; paper?: string };
    text?: { primary?: string; secondary?: string };
    divider?: string;
  };
};

const baseThemeOptions: ThemeOptions = {
  typography: {
    fontFamily: "var(--font-space-grotesk), sans-serif",
    h1: { fontWeight: 800, letterSpacing: "-0.035em", lineHeight: 1.02 },
    h2: { fontWeight: 800, letterSpacing: "-0.03em", lineHeight: 1.04 },
    h3: { fontWeight: 780, letterSpacing: "-0.022em", lineHeight: 1.08 },
    h4: { fontWeight: 760, letterSpacing: "-0.018em", lineHeight: 1.1 },
    h5: { fontWeight: 700, letterSpacing: "-0.01em", lineHeight: 1.14 },
    h6: { fontWeight: 680, lineHeight: 1.16 },
    subtitle1: { fontWeight: 600 },
    overline: { fontWeight: 700, letterSpacing: "0.24em" },
    button: { fontWeight: 700, textTransform: "none", letterSpacing: "0.01em" },
  },
  shape: {
    borderRadius: 14,
  },
};

export const __test_only__baseThemeOptions = baseThemeOptions;
const paletteThemeOptions: ThemeOptions["palette"] = {
  mode: "light",
  primary: {
    main: "var(--color-accent)",
    contrastText: "var(--color-ink)",
  },
  secondary: {
    main: "var(--color-ember)",
    contrastText: "var(--color-ink)",
  },
  error: {
    main: "var(--color-danger)",
  },
  background: {
    default: "var(--color-surface)",
    paper: "var(--color-panel)",
  },
  text: {
    primary: "var(--color-ink)",
    secondary: "var(--color-muted)",
  },
  divider: "var(--color-line)",
};

export const __test_only__paletteThemeOptions = paletteThemeOptions;

export const ThemeRegistry = ({ children }: { children: ReactNode }) => {
  return (
    <AppRouterCacheProvider>
      {children}
    </AppRouterCacheProvider>
  );
};
