# @pokecrystal/assets

Generated assets, JSON content, and loader utilities consumed by the PokeCrystal runtime and app workspaces.

## Purpose
- Package ASM-derived content in a form that `@pokecrystal/core`, `@pokecrystal/web`, and related tools can consume directly.
- Keep generated content separate from handwritten runtime logic.

## Source of Truth
- Inputs come from `vendor/pokecrystal/` through the exporter pipeline.
- Generated outputs should be regenerated rather than hand-edited unless the generation contract itself changes.

## Development Commands

```bash
npm run build --workspace @pokecrystal/assets
npm run lint:types --workspace @pokecrystal/assets
npm run test --workspace @pokecrystal/assets
```

## Release Notes
- Package name: `@pokecrystal/assets`
- Published output is built from `dist/`
- This package depends on `@pokecrystal/core`
