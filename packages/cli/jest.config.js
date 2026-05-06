/** @type {import('jest').Config} */
const config = {
  testEnvironment: "node",
  setupFilesAfterEnv: ["<rootDir>/../../apps/web/jest.setup.js"],
  moduleDirectories: ["node_modules", "<rootDir>/src"],
  moduleNameMapper: {
    "^@pokecrystal/cli$": "<rootDir>/src/index.ts",
    "^@pokecrystal/cli/(.*)$": "<rootDir>/src/$1",
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
