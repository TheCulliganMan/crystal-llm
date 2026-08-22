# Mom dialogue parity proof

## Purpose

Prove that the Rust client presents Mom's first `MeetMomScript` interaction the
same way as the pinned Pokemon Crystal ROM. Rust state, exported text labels,
and Rust text-layout helpers are not acceptable oracles for this proof.

The reference is `vendor/pokecrystal/pokecrystal.gbc` with SHA-1
`f4cd194bdee0d04ca4eac29e09b8e4e9d818c133`, executed by the independently
locked PyBoy runner in this directory.

## Correctness contract

The proof must fail for each observed regression:

- a page is skipped;
- a page or carried `<CONT>` line is printed twice;
- a single input advances more than one blocking boundary;
- a page advances without an A/B edge;
- Mom moves while the ROM still owns a visible textbox or prompt;
- dialogue remains visible after Mom begins her return movement;
- the words, line breaks, glyphs, textbox, or prompt visible to the player do
  not match the ROM.

Passing script execution or final-story-state tests is insufficient. This is a
presentation and input-ownership proof.

## Reference recording

Add an honest, deterministic input tape at
`tools/asm-oracle/scenarios/mom-initial.json`. It starts at power-on, completes
new-game setup, walks from the bedroom to Mom's coordinate event, and chooses
the documented yes/no path. The recorder must never write ROM, WRAM, HRAM,
registers, or the program counter. It may only send Game Boy buttons and read
the LCD and symbols from the pinned `pokecrystal.sym`.

The oracle runner records every frame from the first shock emote until field
control returns:

- the 160x144 LCD image;
- buttons pressed on that frame;
- `wTextboxFlags`, `wScriptRunning`, `wScriptBank`, and `wScriptPos`;
- the player and Mom object map/sprite coordinates and movement fields;
- map and scene identifiers.

Symbol addresses must be parsed from `pokecrystal.sym`, not copied into the
scenario. The run rejects a ROM or symbol-file hash mismatch.

## Semantic checkpoints

Raw frame numbers are diagnostic only; Rust is not required to take the same
number of host frames as the Game Boy. Runs are aligned at observable input
boundaries.

For each boundary, the oracle runner waits until the textbox interior is
stable, ignoring only the animated prompt-arrow cells. It then records:

1. a four-color-normalized PNG of the complete 160x144 LCD;
2. a hash of the textbox interior and border;
3. Mom's map and sprite coordinates;
4. whether the next accepted edge is A/B, a yes/no choice, or no input;
5. the first frame on which the next boundary becomes visible.

The committed manifest contains the ordered checkpoints and hashes. PNGs are
retained on failure so a mismatch is inspectable rather than just a changed
digest.

The recorder also performs three gating probes while the first multi-page text
is complete:

- send no input for 120 frames;
- send Start and all four directions separately;
- send one A press followed by a release.

The first two probes must leave the masked LCD, script cursor, and Mom position
unchanged. The A edge must advance exactly one boundary.

## Rust recording

Add a small `crystal-bevy` parity executable that replays the same semantic
input tape through the production `App`, input system, script runtime,
typewriter, and renderer. It must capture the offscreen 160x144 logical game
surface after Bevy has rendered it. It must not call
`render_visible_script_text_pages`, `visible_field_dialog_pages`, or inspect
text labels to synthesize expected output.

The Rust recorder emits the same checkpoint schema. Its boundary detector may
observe public modal/input ownership, but checkpoint content always comes from
the rendered surface and live object transforms.

## Comparison

`npm run parity:mom-dialogue` performs these steps:

1. verify the pinned ASM source and ROM;
2. replay the ROM tape and produce a fresh oracle trace;
3. replay the Rust production path and produce a fresh Rust trace;
4. align both traces by input boundary;
5. compare ordered boundary kinds, normalized LCD pixels, Mom position, and
   the gating-probe invariants;
6. write a side-by-side image and pixel-diff image for every mismatch.

The command passes only when all boundaries match and both implementations
return field control after the final close/movement sequence. A final-state-only
match cannot pass.

## Test layers

The ROM differential is the acceptance gate. Smaller Rust tests remain useful
for localization, but are not evidence of parity:

- text macro/layout unit tests identify `text`/`line`/`para`/`cont` mistakes;
- script-boundary tests identify cursor or input-consumption mistakes;
- the Mom ROM differential proves the player-visible composition of those
  systems.

## Required first red run

Before fixing Rust, preserve one failing comparison artifact from the current
broken build. It must demonstrate at least the reported skip/repeat or
movement-while-dialogue mismatch. The proof is not accepted if it was only ever
run against the repaired implementation.
