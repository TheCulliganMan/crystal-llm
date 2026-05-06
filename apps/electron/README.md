# @pokecrystal/electron

Electron desktop host for the PokeCrystal web runtime.

## Status
- This workspace is private and is not intended for npm publication.
- It packages the `apps/web` runtime into a self-contained desktop app for the current host platform.

## Prerequisites
- Run commands from the repository root or this workspace with root dependencies installed.
- Build and packaging depend on the web workspace and the local disassembly setup described in the root README.

## Commands
- `npm run dev`: start the web workspace in development mode and open the `/desktop` route inside Electron.
- `npm run build`: generate desktop icons, build the dedicated `.next-electron` web output, and prepare packaging inputs.
- `npm run package`: run `electron-builder` and emit desktop artifacts into `apps/electron/dist`.
- `npm run smoke`: package the app, verify bundled runtime files, launch the unpacked app, and wait for the desktop shell to render.
- `npm run test`: run launcher and packaged-server tests with Node's built-in test runner.
- `npm run clean`: remove generated Electron build output.

## Packaging Notes
- The packaged app starts a local Next server from the bundled `apps/web/.next-electron/standalone` output.
- `KRABBY_DESKTOP_URL` can still override the desktop target during development-style runs.
- Desktop packaging writes artifacts to `apps/electron/dist`.
- The `/desktop` route stays local-first for save behavior, even if Supabase environment variables are present.

## Release Notes
- `apps/electron/assets/icon.png`, `icon.ico`, and `icon.icns` are generated assets. See [`assets/README.md`](assets/README.md).
- Packaging targets are configured per platform in `apps/electron/package.json`.
- The default product name is `KrabbyClaw Desktop`.
