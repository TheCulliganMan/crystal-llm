import type { Config } from "tailwindcss";

const config: Config = {
  content: [
    "./src/app/**/*.{ts,tsx}",
    "./src/components/**/*.{ts,tsx}",
    "./src/arena/**/*.{ts,tsx}",
  ],
  theme: {
    extend: {
      fontFamily: {
        display: ["var(--font-space-grotesk)", "sans-serif"],
        body: ["var(--font-space-grotesk)", "sans-serif"],
      },
      colors: {
        surface: "var(--color-surface)",
        ink: "var(--color-ink)",
        muted: "var(--color-muted)",
        line: "var(--color-line)",
        accent: "var(--color-accent)",
        "accent-soft": "var(--color-accent-soft)",
        crystal: {
          50: "#f2f6ff",
          100: "#d9e7ff",
          200: "#a9c8ff",
          300: "#7ca7ff",
          400: "#5889ff",
          500: "#3c6eff",
          600: "#2d55e6",
          700: "#2342b4",
          800: "#1d3790",
          900: "#182c74",
        },
      },
      boxShadow: {
        glow: "0 10px 60px rgba(88, 137, 255, 0.25)",
      },
      backgroundImage: {
        "grid-sheen":
          "linear-gradient(90deg, rgba(255,255,255,0.03) 1px, transparent 1px), linear-gradient(0deg, rgba(255,255,255,0.03) 1px, transparent 1px)",
      },
    },
  },
};

export default config;
