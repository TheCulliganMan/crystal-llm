# @pokecrystal/desktop

Zero Native desktop host for the PokeCrystal web runtime.

## Status
- This workspace is private and is not intended for npm publication.
- The native shell is Zero Native. It loads the existing `apps/web` `/desktop` route.
- The packaged app loads the static `/desktop` output through `zero://app`.

## Prerequisites
- Run commands from the repository root or this workspace with root dependencies installed.
- Install Zig `0.16.0+`; Zero Native uses Zig for the native app build.
- Build and packaging depend on the web workspace and the local disassembly setup described in the root README.

## Commands
- `npm run dev`: start the web workspace in development mode and open `/desktop` inside the Zero Native shell.
- `npm run build`: generate desktop icons, build the dedicated `.next-desktop` web output, stage desktop resources, and compile the Zero Native binary.
- `npm run package`: package the Zero Native app into `apps/desktop/dist/KrabbyClaw.app`.
- `npm run test`: run Zero Native Zig tests.
- `npm run clean`: remove generated desktop build output.

## Packaging Notes
- Desktop web resources are staged in `apps/desktop/dist/resources`.
- The staged resources include `index.html`, `_next/static`, filtered public assets, and the app icon.
- `KRABBY_DESKTOP_URL` can override the desktop target during development-style runs.
- The `/desktop` route stays local-first for save behavior, even if Supabase environment variables are present.

## Release Notes
- `apps/desktop/assets/icon.png`, `icon.ico`, and `icon.icns` are generated assets. See [`assets/README.md`](assets/README.md).
- Packaging metadata lives in `apps/desktop/app.zon`.
