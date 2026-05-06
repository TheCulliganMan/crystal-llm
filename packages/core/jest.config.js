/** @type {import('jest').Config} */
const config = {
  testEnvironment: "node",
  setupFilesAfterEnv: ["<rootDir>/../../apps/web/jest.setup.js"],
  moduleDirectories: ["node_modules", "<rootDir>/src", "<rootDir>/../../apps/web/src"],
  moduleNameMapper: {
    "^@pokecrystal/core$": "<rootDir>/src/index.ts",
    "^@pokecrystal/core/(.*)$": "<rootDir>/src/$1",
    "^@pokecrystal/assets$": "<rootDir>/../assets/src/index.ts",
    "^@pokecrystal/assets/(.*)$": "<rootDir>/../assets/src/$1",
    "^@pokecrystal/exporters$": "<rootDir>/../exporters/src/index.ts",
    "^@pokecrystal/exporters/(.*)$": "<rootDir>/../exporters/src/$1",
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
          ["@babel/preset-react", { runtime: "automatic" }],
          "@babel/preset-typescript"
        ]
      }
    ]
  },
  testPathIgnorePatterns: ["<rootDir>/dist/"]
};

module.exports = config;
