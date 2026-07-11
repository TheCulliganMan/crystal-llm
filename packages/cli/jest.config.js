/** @type {import('jest').Config} */
const config = {
  testEnvironment: "node",
  setupFilesAfterEnv: ["<rootDir>/../../apps/web/jest.setup.js"],
  moduleDirectories: ["node_modules", "<rootDir>/src"],
  moduleNameMapper: {
    "^@pokecrystal/cli$": "<rootDir>/src/index.ts",
    "^@pokecrystal/cli/(.*)$": "<rootDir>/src/$1",
    "^@pokecrystal/core$": "<rootDir>/../../packages/core/src/index.ts",
    "^@pokecrystal/core/(.*)$": "<rootDir>/../../packages/core/src/$1",
    "^@pokecrystal/assets$": "<rootDir>/../../packages/assets/src/index.ts",
    "^@pokecrystal/assets/(.*)$": "<rootDir>/../../packages/assets/src/$1",
    "^@pokecrystal/exporters$": "<rootDir>/../../packages/exporters/src/index.ts",
    "^@pokecrystal/exporters/(.*)$": "<rootDir>/../../packages/exporters/src/$1",
    "^@/core/(.*)$": "<rootDir>/../../packages/core/src/core/$1",
    "^@/engine/(.*)$": "<rootDir>/../../packages/core/src/engine/$1",
    "^@/ui/(.*)$": "<rootDir>/../../packages/core/src/ui/$1",
    "^@/backend/(.*)$": "<rootDir>/../../packages/core/src/backend/$1",
    "^@/input/(.*)$": "<rootDir>/../../packages/core/src/input/$1",
    "^@/types/(.*)$": "<rootDir>/../../packages/core/src/types/$1",
    "^@/content/(.*)$": "<rootDir>/../../packages/assets/src/content/$1",
    "^@/data/(.*)$": "<rootDir>/../../packages/assets/src/data/$1",
    "^@/(.*)$": "<rootDir>/../../apps/web/src/$1",
    "^(\\.{1,2}/.*)\\.js$": "$1"
  },
  transform: {
    "^.+\\.(ts|tsx)$": [
      "babel-jest",
      {
        presets: [
          [
            "@babel/preset-env",
            {
              targets: { node: "current" },
              modules: "commonjs"
            }
          ],
          "@babel/preset-typescript"
        ]
      }
    ]
  },
  testPathIgnorePatterns: ["<rootDir>/dist/"]
};

module.exports = config;
