# Full TypeScript to Godot Parity Checklist

Generated from game-scope runtime TypeScript files under `packages/core/src`. This intentionally excludes `*.test.ts`, declarations, CLI, MCP, agent tooling, multiplayer, and the web shell unless those become explicit Godot-port scope.

The current Godot audit is not full parity. A domain is not complete until every unchecked file below has a Godot implementation, a TS-oracle parity test, visual/audio verification where applicable, and no placeholder/debug-only presentation in the live client.

## Summary

- [ ] Full game-scope runtime parity: 0/478 TS runtime files signed off.
- [ ] Replace broad domain audit with file/subsystem-level parity checks.
- [ ] Remove or hide all live debug harness UI from shipped scenes.
- [ ] Add golden frame or deterministic render checks for title, intro, overworld, menus, Pokedex/PC/Pokegear, battle, credits, and special overlays.
- [ ] Add deterministic end-to-end run from boot through overworld, battle, menus, save/load, and credits-relevant flows.

## Domain Counts

| Domain | Runtime TS files | Current Godot status |
|---|---:|---|
| `engine/world` | 146 | Partial/stub unless checked below |
| `ui` | 179 | Partial/stub unless checked below |
| `core` | 84 | Partial/stub unless checked below |
| `engine/battle` | 33 | Partial/stub unless checked below |
| `engine/systems` | 18 | Partial/stub unless checked below |
| `input` | 12 | Partial/stub unless checked below |
| `engine/games` | 6 | Partial/stub unless checked below |

## engine/world (146)

Overworld gameplay, maps, objects, collision, story integration, encounters, field moves, time, rendering, and map audio. Current Godot files are broad state/render stubs, not file-level ports.

Current Godot files touching this domain:
- `apps/godot/scripts/overworld_state.gd`
- `apps/godot/scripts/overworld_runtime.gd`
- `apps/godot/scripts/map_data.gd`
- `apps/godot/scripts/story_events_state.gd`
- `apps/godot/scripts/special_events_state.gd`

Checklist:
- [ ] `engine/world/events.ts`
- [ ] `engine/world/index.ts`
- [ ] `engine/world/map-music.ts`
- [ ] `engine/world/maps.ts`
- [ ] `engine/world/npc.ts`
- [ ] `engine/world/overworld/audio-controller.ts`
- [ ] `engine/world/overworld/audio-guards.ts`
- [ ] `engine/world/overworld/collision-data.ts`
- [ ] `engine/world/overworld/collision-rules.ts`
- [ ] `engine/world/overworld/connection-composite.ts`
- [ ] `engine/world/overworld/constants.ts`
- [ ] `engine/world/overworld/counter.ts`
- [ ] `engine/world/overworld/debug-collision.ts`
- [ ] `engine/world/overworld/dialogue-controller.ts`
- [ ] `engine/world/overworld/dialogue-types.ts`
- [ ] `engine/world/overworld/events.ts`
- [ ] `engine/world/overworld/field-move-animation.ts`
- [ ] `engine/world/overworld/field-move-sprite-anim.ts`
- [ ] `engine/world/overworld/fishing.ts`
- [ ] `engine/world/overworld/flag-collection.ts`
- [ ] `engine/world/overworld/grass-rustle.ts`
- [ ] `engine/world/overworld/index.ts`
- [ ] `engine/world/overworld/jump-offsets.ts`
- [ ] `engine/world/overworld/ledge.ts`
- [ ] `engine/world/overworld/logger.ts`
- [ ] `engine/world/overworld/map-geometry.ts`
- [ ] `engine/world/overworld/map-sign.ts`
- [ ] `engine/world/overworld/npc-autonomous-controller.ts`
- [ ] `engine/world/overworld/npc-movement.ts`
- [ ] `engine/world/overworld/npc-sprites.ts`
- [ ] `engine/world/overworld/overworld-base.ts`
- [ ] `engine/world/overworld/overworld-field-moves.ts`
- [ ] `engine/world/overworld/overworld-input.ts`
- [ ] `engine/world/overworld/overworld-input.types.ts`
- [ ] `engine/world/overworld/overworld-map-manager.ts`
- [ ] `engine/world/overworld/overworld-map.ts`
- [ ] `engine/world/overworld/overworld-movement.ts`
- [ ] `engine/world/overworld/overworld-npc-manager.ts`
- [ ] `engine/world/overworld/overworld-object.ts`
- [ ] `engine/world/overworld/overworld-rendering.ts`
- [ ] `engine/world/overworld/overworld-script-queue.ts`
- [ ] `engine/world/overworld/overworld-tileset.ts`
- [ ] `engine/world/overworld/overworld.ts`
- [ ] `engine/world/overworld/palette.ts`
- [ ] `engine/world/overworld/pending-event-flag-updates.ts`
- [ ] `engine/world/overworld/playable-character.ts`
- [ ] `engine/world/overworld/player-state-flags.ts`
- [ ] `engine/world/overworld/route-render.ts`
- [ ] `engine/world/overworld/script-tasks/delay-task.ts`
- [ ] `engine/world/overworld/script-tasks/follow-task.ts`
- [ ] `engine/world/overworld/script-tasks/index.ts`
- [ ] `engine/world/overworld/script-tasks/movement-task.ts`
- [ ] `engine/world/overworld/script-tasks/script-queue.ts`
- [ ] `engine/world/overworld/script-tasks/script-task.ts`
- [ ] `engine/world/overworld/sprite-palettes.ts`
- [ ] `engine/world/overworld/swarm.ts`
- [ ] `engine/world/overworld/temporary-event-flags.ts`
- [ ] `engine/world/overworld/text-status.ts`
- [ ] `engine/world/overworld/tile-coords.ts`
- [ ] `engine/world/overworld/tile-events.ts`
- [ ] `engine/world/overworld/tileset-animation.ts`
- [ ] `engine/world/overworld/tileset-types.ts`
- [ ] `engine/world/overworld/time-system.ts`
- [ ] `engine/world/overworld/trainer-sightlines.ts`
- [ ] `engine/world/overworld/wild-encounters.ts`
- [ ] `engine/world/player.ts`
- [ ] `engine/world/poison.ts`
- [ ] `engine/world/radio-music.ts`
- [ ] `engine/world/radio.ts`
- [ ] `engine/world/roamers.ts`
- [ ] `engine/world/safari-zone.ts`
- [ ] `engine/world/special-events/audio.ts`
- [ ] `engine/world/special-events/battle-tower-loader.ts`
- [ ] `engine/world/special-events/battle-tower.ts`
- [ ] `engine/world/special-events/buena-events.ts`
- [ ] `engine/world/special-events/buena.ts`
- [ ] `engine/world/special-events/bug-contest.ts`
- [ ] `engine/world/special-events/day-care.ts`
- [ ] `engine/world/special-events/decorations.ts`
- [ ] `engine/world/special-events/games.ts`
- [ ] `engine/world/special-events/gifts.ts`
- [ ] `engine/world/special-events/graphics.ts`
- [ ] `engine/world/special-events/haircuts.ts`
- [ ] `engine/world/special-events/index.ts`
- [ ] `engine/world/special-events/kurt.ts`
- [ ] `engine/world/special-events/link.ts`
- [ ] `engine/world/special-events/lucky-number.ts`
- [ ] `engine/world/special-events/magikarp.ts`
- [ ] `engine/world/special-events/magnet-train.ts`
- [ ] `engine/world/special-events/map.ts`
- [ ] `engine/world/special-events/misc.ts`
- [ ] `engine/world/special-events/mom.ts`
- [ ] `engine/world/special-events/money.ts`
- [ ] `engine/world/special-events/mystery-gift.ts`
- [ ] `engine/world/special-events/odd-egg.ts`
- [ ] `engine/world/special-events/pc-helpers.ts`
- [ ] `engine/world/special-events/pc.ts`
- [ ] `engine/world/special-events/placeholders-mobile.ts`
- [ ] `engine/world/special-events/placeholders.ts`
- [ ] `engine/world/special-events/player.ts`
- [ ] `engine/world/special-events/pokemon.ts`
- [ ] `engine/world/special-events/registry.ts`
- [ ] `engine/world/special-events/roamers.ts`
- [ ] `engine/world/special-events/services.ts`
- [ ] `engine/world/special-events/snorlax.ts`
- [ ] `engine/world/special-events/special-types.ts`
- [ ] `engine/world/special-events/sprites.ts`
- [ ] `engine/world/special-events/system.ts`
- [ ] `engine/world/special-events/time-capsule.ts`
- [ ] `engine/world/special-events/time.ts`
- [ ] `engine/world/special-events/unown-overlay-lock.ts`
- [ ] `engine/world/special-events/unown.ts`
- [ ] `engine/world/special-events/utils.ts`
- [ ] `engine/world/special_events/index.ts`
- [ ] `engine/world/special_events/pc.ts`
- [ ] `engine/world/special_events/registry.ts`
- [ ] `engine/world/story-events/command-factory.ts`
- [ ] `engine/world/story-events/commands/base.ts`
- [ ] `engine/world/story-events/commands/battle.ts`
- [ ] `engine/world/story-events/commands/credits.ts`
- [ ] `engine/world/story-events/commands/events.ts`
- [ ] `engine/world/story-events/commands/fruit-tree.ts`
- [ ] `engine/world/story-events/commands/hall-of-fame.ts`
- [ ] `engine/world/story-events/commands/index.ts`
- [ ] `engine/world/story-events/commands/items.ts`
- [ ] `engine/world/story-events/commands/memory.ts`
- [ ] `engine/world/story-events/commands/movement.ts`
- [ ] `engine/world/story-events/commands/overworld.ts`
- [ ] `engine/world/story-events/commands/special.ts`
- [ ] `engine/world/story-events/commands/text.ts`
- [ ] `engine/world/story-events/common.ts`
- [ ] `engine/world/story-events/event-flags.ts`
- [ ] `engine/world/story-events/index.ts`
- [ ] `engine/world/story-events/runner.ts`
- [ ] `engine/world/story-events/script-constants.ts`
- [ ] `engine/world/story-events/specials/fortune.ts`
- [ ] `engine/world/story-events/specials/handlers.ts`
- [ ] `engine/world/story-events/specials/helpers.ts`
- [ ] `engine/world/story-events/specials/index.ts`
- [ ] `engine/world/story-events/test-utils.ts`
- [ ] `engine/world/story-events/text-formatter.ts`
- [ ] `engine/world/story-events/text-helpers.ts`
- [ ] `engine/world/story-events/types.ts`
- [ ] `engine/world/tile.ts`
- [ ] `engine/world/tile/constants.ts`
- [ ] `engine/world/whiteout.ts`

## ui (179)

All visual gameplay UI: menus, overlays, battle HUD, boot screens, text, renderer surfaces, PC/Pokedex/Pokegear/party/bag/trainer card, animations.

Current Godot files touching this domain:
- `apps/godot/scripts/menu_state.gd`
- `apps/godot/scripts/menu_stack.gd`
- `apps/godot/scripts/ui_shell.gd`
- `apps/godot/scripts/text_box.gd`
- `apps/godot/scripts/battle_ui*.gd`
- `apps/godot/scripts/title_runtime.gd`
- `apps/godot/scripts/intro_runtime.gd`

Checklist:
- [ ] `ui/2bpp.ts`
- [ ] `ui/animation-clock.ts`
- [ ] `ui/async-loop.ts`
- [ ] `ui/base-components.ts`
- [ ] `ui/base-ui.ts`
- [ ] `ui/bg-map-sync.ts`
- [ ] `ui/canvas-ui.ts`
- [ ] `ui/composite-ui.ts`
- [ ] `ui/control-lines.ts`
- [ ] `ui/deferred-assets.ts`
- [ ] `ui/dom-canvas-ui.ts`
- [ ] `ui/flare-plot-renderer.ts`
- [ ] `ui/font-renderer.ts`
- [ ] `ui/game-engine.ts`
- [ ] `ui/graphics/place-graphic.ts`
- [ ] `ui/headless-canvas.ts`
- [ ] `ui/index.ts`
- [ ] `ui/menus/bag-menu-layout.ts`
- [ ] `ui/menus/bag-menu.ts`
- [ ] `ui/menus/evolution-item-handler.ts`
- [ ] `ui/menus/field-item-handler.ts`
- [ ] `ui/menus/main-menu.ts`
- [ ] `ui/menus/mart.ts`
- [ ] `ui/menus/menu-state.ts`
- [ ] `ui/menus/menu.ts`
- [ ] `ui/menus/move-reorder-menu.ts`
- [ ] `ui/menus/options-menu.ts`
- [ ] `ui/menus/party-menu-icons.ts`
- [ ] `ui/menus/party-menu-layout.ts`
- [ ] `ui/menus/party-menu-qualities.ts`
- [ ] `ui/menus/pc-asm-render.ts`
- [ ] `ui/menus/pc-auxiliary.ts`
- [ ] `ui/menus/pc-components.ts`
- [ ] `ui/menus/pc-hub-prompt.ts`
- [ ] `ui/menus/pc-layout.ts`
- [ ] `ui/menus/pc-menu.ts`
- [ ] `ui/menus/pc-player-menu.ts`
- [ ] `ui/menus/pc-render-audit.ts`
- [ ] `ui/menus/pc-views.ts`
- [ ] `ui/menus/pc-wallpaper.ts`
- [ ] `ui/menus/pokedex-assets.ts`
- [ ] `ui/menus/pokedex-behaviors.ts`
- [ ] `ui/menus/pokedex-cursor.ts`
- [ ] `ui/menus/pokedex-entry-loader.ts`
- [ ] `ui/menus/pokedex-layout.ts`
- [ ] `ui/menus/pokedex-render-audit.ts`
- [ ] `ui/menus/pokedex-render.ts`
- [ ] `ui/menus/pokedex-state.ts`
- [ ] `ui/menus/pokedex.ts`
- [ ] `ui/menus/pokegear-bg.ts`
- [ ] `ui/menus/pokegear-contacts.ts`
- [ ] `ui/menus/pokegear-labels.ts`
- [ ] `ui/menus/pokegear-state.ts`
- [ ] `ui/menus/pokegear.ts`
- [ ] `ui/menus/pokemon-menu.ts`
- [ ] `ui/menus/pokemon-stats.ts`
- [ ] `ui/menus/start-menu.ts`
- [ ] `ui/menus/trainer-card-layout.ts`
- [ ] `ui/menus/trainer-card.ts`
- [ ] `ui/menus/types.ts`
- [ ] `ui/overlays/_battle-anim-data.ts`
- [ ] `ui/overlays/_battle-anim-math.ts`
- [ ] `ui/overlays/_battle-anim-runtime.ts`
- [ ] `ui/overlays/_battle-animation-helpers.ts`
- [ ] `ui/overlays/_battle-animation-loader.ts`
- [ ] `ui/overlays/_battle-animation-sound.ts`
- [ ] `ui/overlays/_battle-animation-state.ts`
- [ ] `ui/overlays/_battle-animation-util.ts`
- [ ] `ui/overlays/_battle-animation.ts`
- [ ] `ui/overlays/_battle-background.ts`
- [ ] `ui/overlays/_battle-bg-effects.ts`
- [ ] `ui/overlays/_battle-constants.ts`
- [ ] `ui/overlays/_battle-dialogue.ts`
- [ ] `ui/overlays/_battle-hud-helpers.ts`
- [ ] `ui/overlays/_battle-layout-validation.ts`
- [ ] `ui/overlays/_battle-layout.ts`
- [ ] `ui/overlays/_battle-menu.ts`
- [ ] `ui/overlays/_battle-menus.ts`
- [ ] `ui/overlays/_battle-palettes.ts`
- [ ] `ui/overlays/_battle-party-menu.ts`
- [ ] `ui/overlays/_battle-vram.ts`
- [ ] `ui/overlays/_hp-bar-renderer.ts`
- [ ] `ui/overlays/battle-anim-data.ts`
- [ ] `ui/overlays/battle-anim-math.ts`
- [ ] `ui/overlays/battle-anim-runtime-impl.ts`
- [ ] `ui/overlays/battle-anim-runtime.ts`
- [ ] `ui/overlays/battle-animation-audit.ts`
- [ ] `ui/overlays/battle-animation-helpers.ts`
- [ ] `ui/overlays/battle-animation-loader.ts`
- [ ] `ui/overlays/battle-animation-sound.ts`
- [ ] `ui/overlays/battle-animation-state.ts`
- [ ] `ui/overlays/battle-animation-util.ts`
- [ ] `ui/overlays/battle-animation.ts`
- [ ] `ui/overlays/battle-background.ts`
- [ ] `ui/overlays/battle-bars.ts`
- [ ] `ui/overlays/battle-bg-effects.ts`
- [ ] `ui/overlays/battle-constants.ts`
- [ ] `ui/overlays/battle-dialogue.ts`
- [ ] `ui/overlays/battle-evolution.ts`
- [ ] `ui/overlays/battle-experience.ts`
- [ ] `ui/overlays/battle-input.ts`
- [ ] `ui/overlays/battle-intro.ts`
- [ ] `ui/overlays/battle-move-name.ts`
- [ ] `ui/overlays/battle-oam.ts`
- [ ] `ui/overlays/battle-scene.ts`
- [ ] `ui/overlays/battle-ui-core.ts`
- [ ] `ui/overlays/battle-ui-draw.ts`
- [ ] `ui/overlays/battle-ui-input.ts`
- [ ] `ui/overlays/battle-ui-menu-utils.ts`
- [ ] `ui/overlays/battle-ui-moves.ts`
- [ ] `ui/overlays/battle-ui-render.ts`
- [ ] `ui/overlays/battle-ui-sprites.ts`
- [ ] `ui/overlays/battle-ui-state.ts`
- [ ] `ui/overlays/battle-ui.ts`
- [ ] `ui/overlays/egg-hatch.ts`
- [ ] `ui/overlays/fly-map-prompt.ts`
- [ ] `ui/overlays/index.ts`
- [ ] `ui/overlays/phone-call-overlay.ts`
- [ ] `ui/overlays/pokemon-frontpic-animation.ts`
- [ ] `ui/overlays/pokepic.ts`
- [ ] `ui/overlays/slot-machine.ts`
- [ ] `ui/overlays/town-map-coords.ts`
- [ ] `ui/overlays/town-map-marker.ts`
- [ ] `ui/overlays/town-map-overlay.ts`
- [ ] `ui/overlays/trainer-entrance.ts`
- [ ] `ui/overlays/trainer-sprite-id.ts`
- [ ] `ui/overlays/trainer-victory.ts`
- [ ] `ui/overlays/ui-types.ts`
- [ ] `ui/overlays/unown-puzzle.ts`
- [ ] `ui/player-backpics.ts`
- [ ] `ui/renderer-factory.ts`
- [ ] `ui/screens/clock-reset-screen.ts`
- [ ] `ui/screens/continue-screen.ts`
- [ ] `ui/screens/credits-data.ts`
- [ ] `ui/screens/credits.ts`
- [ ] `ui/screens/day-of-week-screen.ts`
- [ ] `ui/screens/delete-save-screen.ts`
- [ ] `ui/screens/heal-machine-animation.ts`
- [ ] `ui/screens/index.ts`
- [ ] `ui/screens/intro/asm-data.ts`
- [ ] `ui/screens/intro/boot-textbox-renderer.ts`
- [ ] `ui/screens/intro/gender-selection.ts`
- [ ] `ui/screens/intro/index.ts`
- [ ] `ui/screens/intro/intro-graphics.ts`
- [ ] `ui/screens/intro/intro-schemas.ts`
- [ ] `ui/screens/intro/intro-sequence.ts`
- [ ] `ui/screens/intro/oak-intro-sequence.ts`
- [ ] `ui/screens/intro/rendering.ts`
- [ ] `ui/screens/intro/tilemap-defaults.ts`
- [ ] `ui/screens/intro/time-set-screen.ts`
- [ ] `ui/screens/magnet-train-animation.ts`
- [ ] `ui/screens/magnet-train-graphics.ts`
- [ ] `ui/screens/mystery-gift-screen.ts`
- [ ] `ui/screens/name-entry-screen.ts`
- [ ] `ui/screens/pokedex-screen.ts`
- [ ] `ui/screens/pokemon-center.ts`
- [ ] `ui/screens/prompt-screen-snapshot.ts`
- [ ] `ui/screens/screen-types.ts`
- [ ] `ui/screens/title-graphics.ts`
- [ ] `ui/screens/title-screen.ts`
- [ ] `ui/surface.ts`
- [ ] `ui/text-overlays.ts`
- [ ] `ui/text-snapshot-render.ts`
- [ ] `ui/text-ui.ts`
- [ ] `ui/text/bitmap-font.ts`
- [ ] `ui/text/colors.ts`
- [ ] `ui/text/constants.ts`
- [ ] `ui/text/dialogue.ts`
- [ ] `ui/text/glyph-map.ts`
- [ ] `ui/text/prompt-context.ts`
- [ ] `ui/text/prompts.ts`
- [ ] `ui/text/render-font.ts`
- [ ] `ui/text/text-renderer.ts`
- [ ] `ui/text/tile-font.ts`
- [ ] `ui/textbox.ts`
- [ ] `ui/tile-layout.ts`
- [ ] `ui/tilemap-surface.ts`
- [ ] `ui/trainer-portraits.ts`
- [ ] `ui/z-index.ts`

## core (84)

Core models, save/state, content loading, timing, constants, DVs, badges, enums, map block decoding, text loading, asset pathing.

Current Godot files touching this domain:
- `apps/godot/scripts/game_state.gd`
- `apps/godot/scripts/save_store.gd`
- `apps/godot/scripts/asset_index.gd`
- `apps/godot/scripts/gb_tile_decoder.gd`
- `apps/godot/scripts/repo_paths.gd`

Checklist:
- [ ] `core/asm-layouts.ts`
- [ ] `core/asm-move-descriptions-loader.ts`
- [ ] `core/asm-move-names-loader.ts`
- [ ] `core/asm-string-loader.ts`
- [ ] `core/asm-text-loader.ts`
- [ ] `core/asset-manifest.ts`
- [ ] `core/asset-reader.ts`
- [ ] `core/audio-formats.ts`
- [ ] `core/badges.ts`
- [ ] `core/base-data.ts`
- [ ] `core/browser-asset-paths.ts`
- [ ] `core/config.ts`
- [ ] `core/constants.ts`
- [ ] `core/content-packs.ts`
- [ ] `core/cry-data.ts`
- [ ] `core/data-loader.ts`
- [ ] `core/data-structures.ts`
- [ ] `core/debug-flags.ts`
- [ ] `core/debug-log.ts`
- [ ] `core/enums.ts`
- [ ] `core/enums/battle.ts`
- [ ] `core/enums/index.ts`
- [ ] `core/enums/item.ts`
- [ ] `core/enums/mon-menu.ts`
- [ ] `core/enums/move.ts`
- [ ] `core/enums/overworld.ts`
- [ ] `core/enums/party-menu.ts`
- [ ] `core/enums/pokedex.ts`
- [ ] `core/enums/pokemon-type.ts`
- [ ] `core/enums/pokemon.ts`
- [ ] `core/enums/ui-enums.ts`
- [ ] `core/gb-timing.ts`
- [ ] `core/gbc-colors.ts`
- [ ] `core/guest-session-storage.ts`
- [ ] `core/home.ts`
- [ ] `core/index.ts`
- [ ] `core/joypad.ts`
- [ ] `core/keycodes.ts`
- [ ] `core/logger.ts`
- [ ] `core/lz.ts`
- [ ] `core/mail.ts`
- [ ] `core/map-blocks.ts`
- [ ] `core/mcp-identity-context.server.ts`
- [ ] `core/memory/hram.ts`
- [ ] `core/memory/index.ts`
- [ ] `core/memory/layouts.ts`
- [ ] `core/memory/mmu.ts`
- [ ] `core/memory/palettes.ts`
- [ ] `core/memory/registers.ts`
- [ ] `core/memory/script-memory.ts`
- [ ] `core/memory/sram.ts`
- [ ] `core/memory/vram.ts`
- [ ] `core/memory/wram.ts`
- [ ] `core/models.ts`
- [ ] `core/models/box.ts`
- [ ] `core/models/bug-contest.ts`
- [ ] `core/models/date.ts`
- [ ] `core/models/day-care.ts`
- [ ] `core/models/daycare.ts`
- [ ] `core/models/index.ts`
- [ ] `core/models/item.ts`
- [ ] `core/models/map.ts`
- [ ] `core/models/move.ts`
- [ ] `core/models/party.ts`
- [ ] `core/models/pokemon.ts`
- [ ] `core/models/settings.ts`
- [ ] `core/models/time.ts`
- [ ] `core/models/trainer.ts`
- [ ] `core/move.ts`
- [ ] `core/path-utils.ts`
- [ ] `core/paths.ts`
- [ ] `core/pokedex.ts`
- [ ] `core/pokemon-dvs.ts`
- [ ] `core/random.ts`
- [ ] `core/rom.ts`
- [ ] `core/save-slots.ts`
- [ ] `core/save.ts`
- [ ] `core/state.ts`
- [ ] `core/tests/save-test-harness.ts`
- [ ] `core/text-constants.ts`
- [ ] `core/textbox-frame.ts`
- [ ] `core/tileset-data.ts`
- [ ] `core/tmhm.ts`
- [ ] `core/types.ts`

## engine/battle (33)

Battle rules and battle lifecycle: setup, turn order, move execution, damage, statuses, items, AI, experience, flee, trainer battles.

Current Godot files touching this domain:
- `apps/godot/scripts/battle_state.gd`
- `apps/godot/scripts/battle_runtime.gd`
- `apps/godot/scripts/battle_assets.gd`
- `apps/godot/scripts/battle_dialogue.gd`
- `apps/godot/scripts/battle_ui*.gd`

Checklist:
- [ ] `engine/battle/ai.ts`
- [ ] `engine/battle/auto-input.ts`
- [ ] `engine/battle/battle/battle-context.ts`
- [ ] `engine/battle/battle/battle-finalization.ts`
- [ ] `engine/battle/battle/battle-logic.ts`
- [ ] `engine/battle/battle/battle-setup.ts`
- [ ] `engine/battle/battle/battle-skill-audit.ts`
- [ ] `engine/battle/battle/battle-transition.ts`
- [ ] `engine/battle/battle/between-turn-effects.ts`
- [ ] `engine/battle/battle/damage-calculation.ts`
- [ ] `engine/battle/battle/experience.ts`
- [ ] `engine/battle/battle/flee-constants.ts`
- [ ] `engine/battle/battle/flee-logic.ts`
- [ ] `engine/battle/battle/hazards.ts`
- [ ] `engine/battle/battle/index.ts`
- [ ] `engine/battle/battle/item-effects.ts`
- [ ] `engine/battle/battle/item-lookup.ts`
- [ ] `engine/battle/battle/item-timeline.ts`
- [ ] `engine/battle/battle/move-effects.ts`
- [ ] `engine/battle/battle/move-execution.ts`
- [ ] `engine/battle/battle/music.ts`
- [ ] `engine/battle/battle/residual-effects.ts`
- [ ] `engine/battle/battle/stat-stages.ts`
- [ ] `engine/battle/battle/stats.ts`
- [ ] `engine/battle/battle/status-effects.ts`
- [ ] `engine/battle/battle/status-queue.ts`
- [ ] `engine/battle/battle/trainer-battle.ts`
- [ ] `engine/battle/battle/transform-state.ts`
- [ ] `engine/battle/battle/turn-order.ts`
- [ ] `engine/battle/index.ts`
- [ ] `engine/battle/stats.ts`
- [ ] `engine/battle/tmhm-teacher.ts`
- [ ] `engine/battle/tutorial.ts`

## engine/systems (18)

General gameplay systems: audio, breeding, day care, evolution, items, shop, daily/time, step events, mystery gift, printer, TM/HM.

Current Godot files touching this domain:
- `apps/godot/scripts/core_systems_state.gd`
- `apps/godot/scripts/audio_assets.gd`

Checklist:
- [ ] `engine/systems/animation.ts`
- [ ] `engine/systems/audio-aliases.ts`
- [ ] `engine/systems/audio.ts`
- [ ] `engine/systems/breeding.ts`
- [ ] `engine/systems/daily-events.ts`
- [ ] `engine/systems/day-care.ts`
- [ ] `engine/systems/evolution.ts`
- [ ] `engine/systems/experience.ts`
- [ ] `engine/systems/index.ts`
- [ ] `engine/systems/items.ts`
- [ ] `engine/systems/learnsets.ts`
- [ ] `engine/systems/mystery-gift.ts`
- [ ] `engine/systems/pokemon.ts`
- [ ] `engine/systems/printer.ts`
- [ ] `engine/systems/shop.ts`
- [ ] `engine/systems/step-events.ts`
- [ ] `engine/systems/time.ts`
- [ ] `engine/systems/tmhm.ts`

## input (12)

Input bindings, joypad semantics, script input adapters, key mapping, auto movement input.

Current Godot files touching this domain:
- `apps/godot/scripts/input_latch.gd`

Checklist:
- [ ] `input/adapters.ts`
- [ ] `input/auto-move-adapter.ts`
- [ ] `input/bindings.ts`
- [ ] `input/buttons.ts`
- [ ] `input/config.ts`
- [ ] `input/controls.ts`
- [ ] `input/index.ts`
- [ ] `input/joypad.ts`
- [ ] `input/keycodes.ts`
- [ ] `input/script-adapter.ts`
- [ ] `input/script-tokens.ts`
- [ ] `input/user-bindings.ts`

## engine/games (6)

Game Corner and side-game logic: slots, card flip, memory, Unown puzzle, RNG.

Current Godot files touching this domain:
- `apps/godot/scripts/game_corner_state.gd`

Checklist:
- [ ] `engine/games/card-flip.ts`
- [ ] `engine/games/index.ts`
- [ ] `engine/games/memory-game.ts`
- [ ] `engine/games/rng.ts`
- [ ] `engine/games/slots.ts`
- [ ] `engine/games/unown-puzzle.ts`

