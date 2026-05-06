import { defineConfig, globalIgnores } from "eslint/config";
import nextConfig from "eslint-config-next";

const nextTypescriptConfig = nextConfig.find(
  (entry) => entry && typeof entry === "object" && entry.plugins && entry.plugins["@typescript-eslint"],
);
const typescriptEslintPlugin = nextTypescriptConfig?.plugins?.["@typescript-eslint"];

const nextBaseConfig = nextConfig.find(
  (entry) => entry && typeof entry === "object" && entry.plugins && entry.plugins["react-hooks"],
);
const reactHooksPlugin = nextBaseConfig?.plugins?.["react-hooks"];

const eslintConfig = defineConfig([
  // eslint-config-next exports a flat-config array.
  ...nextConfig,
  {
    files: ["**/*.ts", "**/*.tsx", "**/*.js", "**/*.jsx"],
    plugins: reactHooksPlugin ? { "react-hooks": reactHooksPlugin } : {},
    rules: {
      // This rule is overly strict for our app shell, and currently flags a few benign mount-time patterns.
      "react-hooks/set-state-in-effect": "off",
      "@next/next/no-assign-module-variable": "off",
    },
  },
  {
    files: ["**/*.ts", "**/*.tsx"],
    plugins: typescriptEslintPlugin ? { "@typescript-eslint": typescriptEslintPlugin } : {},
    rules: {
      "@typescript-eslint/no-require-imports": "off",
      "@typescript-eslint/no-unused-vars": "off",
    }
  },
  // Override default ignores of eslint-config-next.
  globalIgnores([
    // Default ignores of eslint-config-next:
    ".next/**",
    ".next-build/**",
    ".next-dev/**",
    ".vercel/**",
    "out/**",
    "build/**",
    "next-env.d.ts",
    "dist/**",
    "node_modules/**",
    ".generated/**",
    "assets/**"
  ])
]);

export default eslintConfig;
