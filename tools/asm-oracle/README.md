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
