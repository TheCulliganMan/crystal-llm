# @pokecrystal/exporters

TypeScript exporters for turning `vendor/pokecrystal/` inputs into checked runtime data and generated assets.

## Purpose
- Read the local pret disassembly checkout.
- Emit the normalized data consumed by `@pokecrystal/assets`, `@pokecrystal/core`, and the app workspaces.
- Keep ASM-derived generation logic in one package instead of spreading it across runtime code.

## Prerequisites
- A local `vendor/pokecrystal/` checkout
- Root workspace dependencies installed

## Core Export

```bash
npm run export:core --workspace @pokecrystal/exporters
```

This is the canonical exporter path for regenerating core runtime data.

## Development Commands

```bash
npm run build --workspace @pokecrystal/exporters
npm run lint:types --workspace @pokecrystal/exporters
npm run test --workspace @pokecrystal/exporters
```

## Modular Content-Pack Output
- Core export emits an enabled `core-modular` content pack under `assets/data/content-packs/`.
- It splits key datasets such as Pokemon, moves, learnsets, egg moves, evolutions, encounters, map data, NPCs, Pokegear landmarks, items, trainers, pokedex content, story events, and phone scripts into per-entity or per-map JSON files.
- NPC overrides live under `content-packs/<pack>/npcs/` as per-map records, for example `{ "AzaleaGym": [/* object_event entries */] }`.
- It appends a low-priority `core-modular` entry to `content-packs/index.json` as canonical base data. Higher-priority custom packs can still override it.
- It also emits disabled module-pack entries such as `module-pokemon-*`, `module-move-*`, `module-learnset-*`, `module-egg-move-*`, `module-route-*`, `module-npc-*`, `module-pokegear-landmarks-*`, `module-item-*`, `module-trainer-*`, `module-pokedex-*`, `module-story-*`, and `module-phone-*` for granular opt-in templates and overrides.
