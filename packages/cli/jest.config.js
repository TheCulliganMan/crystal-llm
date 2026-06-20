/** @type {import('jest').Config} */
const config = {
  testEnvironment: "node",
  setupFilesAfterEnv: ["<rootDir>/../../apps/web/jest.setup.js"],
  moduleDirectories: ["node_modules", "<rootDir>/src"],
  moduleNameMapper: {
    "^@/(.*)$": "<rootDir>/../../apps/web/src/$1",
    "^@pokecrystal/agents$": "<rootDir>/../agents/src/index.ts",
    "^@pokecrystal/agents/(.*)$": "<rootDir>/../agents/src/$1",
    "^@pokecrystal/assets$": "<rootDir>/../assets/src/index.ts",
    "^@pokecrystal/assets/(.*)$": "<rootDir>/../assets/src/$1",
    "^@pokecrystal/cli$": "<rootDir>/src/index.ts",
    "^@pokecrystal/cli/(.*)$": "<rootDir>/src/$1",
    "^@pokecrystal/core$": "<rootDir>/../core/src/index.ts",
    "^@pokecrystal/core/(.*)$": "<rootDir>/../core/src/$1",
    "^@pokecrystal/exporters$": "<rootDir>/../exporters/src/index.ts",
    "^@pokecrystal/exporters/(.*)$": "<rootDir>/../exporters/src/$1",
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
