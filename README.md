# Pokemon Crystal LLM Benchmarking Toolkit

Terminal and native desktop Pokemon Crystal runtime for live play and LLM
gameplay experiments.

## Start

Install once:

```bash
npm install
```

Start the TUI:

```bash
npm run start:tui
```

Start the native desktop app:

```bash
npm run start:desktop
```

## Controls

- Move: arrow keys, `WASD`, or `HJKL`
- A: `Z`, `J`, or Space
- B: `X`, `K`
- Start: Enter
- Select: Tab
- Quit the TUI: `:q!`

## Notes

- Use Node.js `24.x` and npm `10.5+`.
- `npm run tui` and `npm run desktop` are shorter aliases.
- On first run, the start command fetches the upstream `pret/pokecrystal`
  sources and exports core runtime data only when local assets are missing.
- To require preexisting local assets and skip that bootstrap, run with
  `POKECRYSTAL_BOOTSTRAP_ASSETS=0`.
- TUI audio uses PCM; `ffmpeg` and MP3 audio export are not required to start
  the game.
