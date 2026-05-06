# @pokecrystal/core

ASM-faithful TypeScript core runtime for game state, engine logic, input handling, UI primitives, and shared types.

## Purpose
- Provide the reusable runtime consumed by the web app, CLI, and supporting packages.
- Keep gameplay logic and renderer-facing primitives outside any single app host.

## Package Boundaries
- App-specific integrations such as Next.js routes, Supabase wiring, browser bootstrap, and deployment concerns should stay outside this package.
- Host-specific behavior should be introduced through app-local adapters instead of direct framework coupling inside `@pokecrystal/core`.

## Development Commands

```bash
npm run build --workspace @pokecrystal/core
npm run lint:types --workspace @pokecrystal/core
npm run test --workspace @pokecrystal/core
```

## Release Notes
- Package name: `@pokecrystal/core`
- Published output is built from `dist/`
- This package is intended to be imported by other workspaces and external consumers
