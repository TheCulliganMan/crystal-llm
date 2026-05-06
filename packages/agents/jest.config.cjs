/** @type {import('jest').Config} */
const config = {
  testEnvironment: "node",
  setupFilesAfterEnv: ["<rootDir>/../../apps/web/jest.setup.js"],
  moduleDirectories: ["node_modules", "<rootDir>/src"],
  moduleNameMapper: {
    "^@pokecrystal/agents$": "<rootDir>/src/index.ts",
    "^@pokecrystal/agents/(.*)$": "<rootDir>/src/$1",
    "^(\\.{1,2}/.*)\\.js$": "$1"
  },
  transform: {
    "^.+\\.(js|mjs|ts|tsx)$": [
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
  transformIgnorePatterns: [],
  testPathIgnorePatterns: ["<rootDir>/dist/", "<rootDir>/src/.* 2\\.(ts|tsx)$"]
};

module.exports = config;
