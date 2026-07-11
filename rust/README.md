# PokeCrystal Rust Port

This workspace is the Rust game port. It intentionally excludes the existing
MCP, web UI, CLI agent tooling, and TypeScript app surfaces.

## Crates

- `crystal-core`: deterministic game state, timing, input, battle/world rules,
  save state, and multiplayer-ready simulation boundaries.
- `crystal-assets`: loaders for ASM-derived data exported from
  `vendor/pokecrystal` and the existing TypeScript asset pipeline.
- `crystal-audio`: music, SFX, cries, and audio command playback data.
- `crystal-net`: transport-neutral multiplayer protocol types.
- `crystal-bevy`: desktop game shell for rendering, input, and audio.

The port should move file by file from the game runtime surfaces under
`packages/core/src`, `packages/assets/src`, `packages/exporters/src`, and
`vendor/pokecrystal`. Do not port MCP, web routes, agent workflows, or desktop
packaging.

## Play

The playable target is the Bevy shell. It reads one definitive compiled pack and
starts at the title screen by default:

```sh
cargo run -p crystal-bevy -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --save-path /tmp/pokecrystal.crystalsave
```

Keyboard controls are arrows for the D-pad, `Z` for A, `X` for B, `Enter` for
Start, and Right Shift for Select.

Existing saves can be loaded directly:

```sh
cargo run -p crystal-bevy -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --load-save /tmp/pokecrystal.crystalsave \
  --save-path /tmp/pokecrystal.crystalsave
```

For targeted smoke/debug commands, `--spawn <spawn-id>` bypasses the title
screen and starts a new game directly at a compiled spawn. Bare `--spawn` is
not a playable interactive launch path; omit it to enter through the ASM
intro/title/Oak flow.

Use `--list-spawns` with the same `--pack` to print compiled spawn ids. The
launcher has no stdin command shell and no web, MCP, agent, or Electron surface.

## Verification

Verify the pinned local ASM checkout and reference ROM before exporting:

```sh
npm run verify:asm
npm run verify:pack
npm run asm:boot
```

Run the Rust compile gate from this directory:

```sh
cargo test --workspace --no-run
```

From the repository root, rebuild the definitive core pack:

```sh
npm run export:core
```

Then smoke the compiled pack through Bevy without opening a long-running shell:

```sh
cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --list-spawns

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --list-field-targets DarkCaveVioletEntrance

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-save /tmp/core-modular-smoke.crystalsave \
  --smoke-script 'right;down;left;up'

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --smoke-load-save /tmp/core-modular-smoke.crystalsave \
  --save-path /tmp/core-modular-smoke-roundtrip.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-field-move Strength:CHIKORITA:80:STRENGTH \
  --smoke-save /tmp/strength-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-field-move Flash:CHIKORITA:80:FLASH \
  --smoke-save /tmp/flash-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-field-item Repel:REPEL:1 \
  --smoke-save /tmp/repel-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-field-item TmHm:HM_CUT:1:CHIKORITA:5 \
  --smoke-save /tmp/hm-cut-item-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-field-item EvolutionStone:THUNDERSTONE:1:PIKACHU:20:RAICHU \
  --smoke-save /tmp/thunderstone-evolution-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-field-item Register:BICYCLE:1 \
  --smoke-save /tmp/register-key-item-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-field-item Pokegear:POKEGEAR:1 \
  --smoke-save /tmp/pokegear-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-field-item Box:NORMAL_BOX:1 \
  --smoke-save /tmp/normal-box-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-party CHIKORITA:5 \
  --smoke-party-recovery FullHeal \
  --smoke-save /tmp/party-full-heal-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-party CHIKORITA:5 \
  --smoke-party-recovery Blackout \
  --smoke-save /tmp/party-blackout-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-roamers \
  --smoke-save /tmp/roamers-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-magikarp-length \
  --smoke-save /tmp/magikarp-length-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-odd-egg \
  --smoke-save /tmp/odd-egg-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-mystery-gift \
  --smoke-save /tmp/mystery-gift-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-buena-password \
  --smoke-save /tmp/buena-password-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-shuckie \
  --smoke-save /tmp/shuckie-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-dratini \
  --smoke-save /tmp/dratini-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-kurt-apricorn \
  --smoke-save /tmp/kurt-apricorn-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-bills-grandfather \
  --smoke-save /tmp/bills-grandfather-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-unown-printer \
  --smoke-save /tmp/unown-printer-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-map-radio \
  --smoke-save /tmp/map-radio-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-name-rater \
  --smoke-save /tmp/name-rater-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-poke-seer \
  --smoke-save /tmp/poke-seer-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-bank-of-mom \
  --smoke-save /tmp/bank-of-mom-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-link-records \
  --smoke-save /tmp/link-records-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-link-rooms \
  --smoke-save /tmp/link-rooms-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-link-handshake \
  --smoke-save /tmp/link-handshake-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-battle-tower \
  --smoke-party CHIKORITA:80 \
  --smoke-party CYNDAQUIL:80 \
  --smoke-party TOTODILE:80 \
  --smoke-save /tmp/battle-tower-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-bug-contest \
  --smoke-party CHIKORITA:20 \
  --smoke-party CYNDAQUIL:20 \
  --smoke-save /tmp/bug-contest-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-day-care \
  --smoke-party CHIKORITA:20 \
  --smoke-party CYNDAQUIL:20 \
  --smoke-save /tmp/day-care-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map CherrygroveCity:10:8 \
  --smoke-field-item Bicycle:BICYCLE:1 \
  --smoke-save /tmp/bicycle-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-field-item Itemfinder:ITEMFINDER:1 \
  --smoke-save /tmp/itemfinder-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-field-item TownMap:TOWN_MAP:1 \
  --smoke-save /tmp/town-map-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map Route32:6:78 \
  --smoke-script 'down' \
  --smoke-field-item EscapeRope:ESCAPE_ROPE:1 \
  --smoke-save /tmp/escape-rope-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map Route32:4:41 \
  --smoke-script 'down' \
  --smoke-field-move Surf:CHIKORITA:80:SURF \
  --smoke-save /tmp/surf-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map CherrygroveCity:10:8 \
  --smoke-set-flag ENGINE_FLYPOINT_NEW_BARK \
  --smoke-field-move Fly:CHIKORITA:80:FLY:14:ENGINE_FLYPOINT_NEW_BARK \
  --smoke-save /tmp/fly-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map Route32:6:78 \
  --smoke-script 'down' \
  --smoke-field-move Dig:CHIKORITA:80:DIG \
  --smoke-save /tmp/dig-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map CherrygroveCity:10:8 \
  --smoke-field-move Teleport:CHIKORITA:80:TELEPORT \
  --smoke-save /tmp/teleport-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map Route32:4:41 \
  --smoke-script 'down' \
  --smoke-fishing Item:GOOD_ROD \
  --smoke-party CHIKORITA:80 \
  --smoke-save /tmp/fishing-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map Route32:4:41 \
  --smoke-script 'down' \
  --smoke-field-move SweetScent:CHIKORITA:80:SWEET_SCENT:Grass \
  --smoke-save /tmp/sweet-scent-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map Route36:16:13 \
  --smoke-field-move Headbutt:CHIKORITA:80:HEADBUTT:0 \
  --smoke-save /tmp/headbutt-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map DarkCaveVioletEntrance:27:5 \
  --smoke-field-move RockSmash:CHIKORITA:80:ROCK_SMASH \
  --smoke-save /tmp/rock-smash-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map AzaleaMart:2:6 \
  --smoke-shop AzaleaMart:AzaleaMartClerkScript:1 \
  --smoke-money 1000 \
  --smoke-buy POTION:1 \
  --smoke-sell POTION:1 \
  --smoke-save /tmp/shop-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map GoldenrodGameCorner:2:12 \
  --smoke-menu 'GoldenrodGameCorner:GoldenrodGameCornerTMVendor_LoopScript:1:2:0:TM25    5500' \
  --smoke-save /tmp/menu-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map GoldenrodDeptStoreElevator:1:3 \
  --smoke-elevfloor GoldenrodDeptStoreElevator:GoldenrodDeptStoreElevatorData:2:GoldenrodDeptStore1F \
  --smoke-save /tmp/elevfloor-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-elevator GoldenrodDeptStoreElevator:GoldenrodDeptStoreElevatorData:GoldenrodDeptStoreElevatorScript:1:1:FLOOR_1F:4:GoldenrodDeptStore1F \
  --smoke-save /tmp/elevator-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map PlayersHouse2F:2:2 \
  --smoke-script 'up;a' \
  --smoke-interact \
  --smoke-save /tmp/interact-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-pc-storage \
  --smoke-party CHIKORITA:80 \
  --smoke-party CYNDAQUIL:80 \
  --smoke-pc-item POTION:2 \
  --smoke-save /tmp/pc-storage-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-link-journal \
  --smoke-script 'right;down;left;up'

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --list-gift-pokemon

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map ElmsLab:6:5 \
  --smoke-gift-pokemon ElmsLab:ChikoritaPokeBallScript:22 \
  --smoke-save /tmp/chikorita-gift-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map VioletPokecenter1F:5:5 \
  --smoke-gift-pokemon VioletPokecenter1F:VioletPokecenter1F_ElmsAideScript:9 \
  --smoke-save /tmp/togepi-egg-gift-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map EcruteakGym:4:15 \
  --smoke-script-warp EcruteakGym:EcruteakGymClosed:12:EcruteakCity \
  --smoke-save /tmp/script-warp-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map EcruteakGym:4:15 \
  --smoke-script-warp EcruteakGym:EcruteakGymClosed:12:EcruteakCity \
  --smoke-script-warp-pending \
  --smoke-save /tmp/script-warp-pending-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map GoldenrodMagnetTrainStation:9:9 \
  --smoke-script-map-pending GoldenrodMagnetTrainStation:GoldenrodMagnetTrainStationOfficerScript:21:newloadmap \
  --smoke-save /tmp/script-map-pending-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map AzaleaGym:5:7 \
  --smoke-script-text-pending AzaleaGym:AzaleaGymBugsyScript:1:5:waitbutton \
  --smoke-save /tmp/script-text-pending-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map VioletGym:4:13 \
  --smoke-trainer-battle VioletGym:VioletGymFalknerScript:9 \
  --smoke-party CHIKORITA:80 \
  --smoke-save /tmp/falkner-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map LakeOfRage:10:10 \
  --smoke-wild-battle LakeOfRage:RedGyarados:7 \
  --smoke-party CHIKORITA:80 \
  --smoke-player-action Move:0 \
  --smoke-enemy-action Move:0 \
  --smoke-save /tmp/wild-battle-turn-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map LakeOfRage:10:10 \
  --smoke-wild-battle LakeOfRage:RedGyarados:7 \
  --smoke-party CHIKORITA:80 \
  --smoke-capture-ball MASTER_BALL \
  --smoke-save /tmp/red-gyarados-capture-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map LakeOfRage:10:10 \
  --smoke-wild-battle LakeOfRage:RedGyarados:7 \
  --smoke-party CHIKORITA:80 \
  --smoke-battle-item X_ATTACK \
  --smoke-save /tmp/wild-battle-item-smoke.crystalsave

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map VioletGym:4:13 \
  --smoke-visible-trainer-battle VioletGym:VioletGymFalknerScript:9 \
  --smoke-party CHIKORITA:80

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-visible-start-menu /tmp/visible-start-menu-smoke.crystalsave \
  --smoke-party CHIKORITA:80 \
  --smoke-visible-bag-item POTION:1

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-visible-party \
  --smoke-party CHIKORITA:80 \
  --smoke-party CYNDAQUIL:80

cargo run -p crystal-bevy --manifest-path rust/Cargo.toml -- \
  --repo /path/to/crystal-llm \
  --pack content-packs/core-modular.crystalpack \
  --spawn 0 \
  --smoke-start-map LakeOfRage:10:10 \
  --smoke-visible-wild-battle LakeOfRage:RedGyarados:7 \
  --smoke-party CHIKORITA:80 \
  --smoke-visible-bag-item MASTER_BALL:1
```
