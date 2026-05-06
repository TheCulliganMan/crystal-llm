/** @type {import('jest').Config} */
const config = {
  testEnvironment: "node",
  setupFilesAfterEnv: ["<rootDir>/../../apps/web/jest.setup.js"],
  moduleDirectories: ["node_modules", "<rootDir>/src", "<rootDir>/../../apps/web/src"],
  moduleNameMapper: {
    "^@pokecrystal/core$": "<rootDir>/../core/src/index.ts",
    "^@pokecrystal/core/(.*)$": "<rootDir>/../core/src/$1",
    "^@pokecrystal/assets$": "<rootDir>/src/index.ts",
    "^@pokecrystal/assets/(.*)$": "<rootDir>/src/$1",
    "^@/(.*)$": "<rootDir>/src/$1",
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
