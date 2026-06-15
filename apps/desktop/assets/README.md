# Desktop Build Assets

This directory contains generated desktop packaging assets for `@pokecrystal/desktop`.

## Icons
- `icon.png`, `icon.ico`, and `icon.icns` are generated during the Zero Native desktop build flow.
- The preferred source is `apps/web/assets/gfx/pokemon/krabby/front.png` when that asset is available.
- `icon-source.svg` is the fallback and reference source used when the runtime asset is unavailable.

## Refreshing Assets

```bash
npm run build --workspace @pokecrystal/desktop
```

or

```bash
npm run package --workspace @pokecrystal/desktop
```

Do not hand-edit generated icon binaries unless the generation workflow itself changes.
