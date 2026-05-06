# Desktop Build Assets

This directory contains generated desktop packaging assets for `@pokecrystal/electron`.

## Icons
- `icon.png`, `icon.ico`, and `icon.icns` are generated during the Electron build flow.
- The preferred source is `apps/web/assets/gfx/pokemon/krabby/front.png` when that asset is available.
- `icon-source.svg` is the fallback and reference source used when the runtime asset is unavailable.

## Refreshing Assets

```bash
npm run build --workspace @pokecrystal/electron
```

or

```bash
npm run package --workspace @pokecrystal/electron
```

Do not hand-edit generated icon binaries unless the generation workflow itself changes.
