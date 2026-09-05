# ASM oracle

This is a development-only headless reference runner. It loads the ROM built
from the pinned `vendor/pokecrystal` checkout and is intentionally separate
from the Rust runtime. Future parity scenarios will use this adapter to read
semantic checkpoints and replay controlled RTC/RNG inputs.

The first presentation scenario is specified in
[`MOM_DIALOGUE_PARITY.md`](MOM_DIALOGUE_PARITY.md). It deliberately uses ROM
LCD frames as the oracle; Rust labels and Rust text-layout helpers cannot prove
their own correctness.

Run it after building the reference ROM with:

```sh
uv run --project tools/asm-oracle python tools/asm-oracle/boot_smoke.py \
  vendor/pokecrystal/pokecrystal.gbc
```

Semantic checkpoints can be emitted from a JSON action scenario. The output
contains map/tile, party, money, RTC, and `hRandomAdd`/`hRandomSub` fields for
comparison with Rust parity adapters:

```sh
uv run --project tools/asm-oracle python tools/asm-oracle/trace.py \
  vendor/pokecrystal/pokecrystal.gbc --frames 120
```

CrystalIntro checkpoints run the unmodified boot sequence and capture the
first and last LCD surface of all 28 source jumptable scenes, together with
tilemap, attrmap, exact OAM bytes and hash, palette, scanline, and
display-register hashes:

```sh
uv run --project tools/asm-oracle python tools/asm-oracle/intro_trace.py \
  vendor/pokecrystal/pokecrystal.gbc
```

Pass `--ppm-dir DIR` to retain all 56 scene-boundary LCD images. Pass
`--timing` to include every intro `Decompress` call's exact resource symbol,
bridge helper, start/end frame, crossed frame boundaries, and elapsed CPU
T-cycles, plus each intervening VBlank handler's state and elapsed T-cycles up
to `GameTimer`. Each VBlank sample also captures the complete pre-update
`wAudio` state, `_UpdateSound` duration, and every `ParseMusic` call's channel,
full duration, command-byte prefix, terminal note byte, and terminal channel
timing state. `ParseMusic` loops back to its own entry after each command; the
oracle retains the original call boundary across those loop entries. While a
decompression is active it also attributes pitch-slide,
vibrato, noise, channel-write, danger, and fade helper durations without
paying hook overhead during the rest of the intro. The runner composes those
helper and parser bodies with the source-certified inactive/channel-class and
note-over overheads, rejects any remainder that is not an exact note-over
multiple, and records the derived note-over count. It also records the pinned
boot's frame-cycle origin. Every decompression and instrumented intro helper
includes its start phase within the 70,224-T-cycle frame; the runner independently
recomputes and verifies every observed frame-boundary count from that phase plus
elapsed cycles before emitting the trace. It also emits 1,752 complete outer-loop
intervals from `JoyTextDelay` entry through the central `DelayFrame` return,
including source-order call offsets, per-scene dispatcher ticks, exact elapsed
T-cycles, and phase-derived frame crossings. The pinned run proves those intervals
cross 2,441 frame boundaries in total; two iterations cross none and twenty cross
two, so host-frame execution cannot assume one dispatcher iteration per frame.
The entry-to-first-input interval independently composes as 59 source machine
cycles plus one 27-machine-cycle LCD interrupt, advancing the captured phase
from 2,980 to 3,324 T-cycles without resetting the clock to an oracle sample.
Each `JoyTextDelay` sample also records its in-call LCD/timer interrupt offsets
and must reduce uniquely to a certified source body. In the no-input boot,
1,395 calls take the 107-machine-cycle repeat-suppression path and 358 take the
110-machine-cycle restart path; the separate pressed-input path is 101 cycles.
The runner also replays the source instruction-cost sequence for all 1,753 calls
and proves the exact LCD/timer admission boundary, interrupt priority, callback
branch cost, interrupt count, and total elapsed T-cycles for every one. The
4,096-Hz timer requests every 262,144 T-cycles, with its first request 258,428
T-cycles after intro entry; five calls contain one timer interrupt, including two
whose admission is deferred behind an active LCD handler.
The event records include LY, LCDC, SCX/SCY, WX/WY, callback pointer, and OAM
identity. They independently establish the hardware calendar used by Rust:
456 T-cycles per scanline, mode-0/HBlank requested at scanline T-cycle 250,
and VBlank requested after the 144th visible line at frame T-cycle 65,664.
It currently reconciles all 74 sampled
VBlank sound updates exactly. The runner locks the
observed 2,356-T-cycle non-audio prefix and
the 1,364-T-cycle all-channels-inactive sound path. These ROM timings are
oracle evidence; production timing continues to come from the source-certified
exporter model.

Pass `--all-vblanks` together with `--timing` to retain the complete VBlank
state-transition corpus. It contains exactly one ordered pre-update audio and
display-register snapshot plus measured handler duration for each of intro
frames 1 through 2,441. The default timing trace keeps the smaller 74-sample
decompression-overlap corpus with full per-helper audio attribution.

Credits presentation checkpoints use a temporary ROM copy whose
`CrystalIntro` entry performs the normal farcall to the unmodified `RedCredits`
route. This retains the real fade, display initialization, font loads, white
palettes, delay, and `Credits` call. The canonical ROM is SHA-checked before
that development-only dispatch patch, and the vendored ROM is never changed:

```sh
uv run --project tools/asm-oracle python tools/asm-oracle/credits_trace.py \
  vendor/pokecrystal/pokecrystal.gbc --accelerate
```

Omit `--accelerate` for original wait durations. Pass `--ppm-dir DIR` to retain
the event checkpoint frames as dependency-free PPM images. Each checkpoint
includes raw RGBA and normalized RGB5 LCD hashes plus visible BG tilemap and
attrmap hashes. The runner also verifies that `CREDITS_END` changes none of
those retained-display hashes relative to the immediately preceding frame.
