# Contributing

Thanks for helping improve this project.

The supported workflow is now the TypeScript monorepo:
- `apps/web` for the Next.js app and browser-facing UI.
- `packages/core`, `packages/assets`, and `packages/exporters` for shared runtime and generated content.

The goal is still ASM-faithful behavior: visuals, audio, timing, and game logic should match the original Pokémon Crystal disassembly.

## Prerequisites
- Node.js 24.x + npm.
- Git LFS only if you are working with large assets or captures.

## Repo Setup

```bash
git clone <repo>
cd pokecrystal-python
npm install
npm run dev
```

Open `http://localhost:3000`.

If a task needs LFS-backed files, run `git lfs install && git lfs pull`.

## Common Commands

```bash
npm run dev
npm run build
npm run test
npm run lint
```

For targeted work:

```bash
npm run test --workspace @pokecrystal/core
npm run test --workspace @pokecrystal/assets
npm run test --workspace @pokecrystal/exporters
npm run build --workspace @pokecrystal/web
```

## Verification
- If you change TypeScript behavior, make sure `npm run build` passes.
- If you change shared runtime code, run the relevant workspace tests as well as the root build.
- If you touch generated assets, regenerate the canonical pack with the Rust `./export` path instead of editing the compiled artifact by hand.

## Generated Data Policy
- `pokecrystal_disassembly/` is the source of truth.
- Generated outputs are consumed by the TypeScript packages and app.
- Do not hand-edit generated data.

## LLM Notes
- The deployed web app serves `/llms.txt`.
- The repo root `llms.txt` is synced into `apps/web/public/llms.txt` during dev/build.

## Pull Requests
- Keep PRs focused and small when possible.
- For behavior changes, include what changed, why it is faithful, and how to test it.
- If you touch the TypeScript workspace, ensure the root build passes.
