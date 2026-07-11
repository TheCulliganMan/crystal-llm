from __future__ import annotations

import argparse
import hashlib
from pathlib import Path

from pyboy import PyBoy


EXPECTED_ROM_SHA1 = "f4cd194bdee0d04ca4eac29e09b8e4e9d818c133"


def main() -> None:
    parser = argparse.ArgumentParser(description="Boot the local pokecrystal ROM headlessly")
    parser.add_argument("rom", type=Path)
    parser.add_argument("--frames", type=int, default=120)
    args = parser.parse_args()

    digest = hashlib.sha1(args.rom.read_bytes()).hexdigest()
    if digest != EXPECTED_ROM_SHA1:
        raise SystemExit(f"unexpected ROM SHA-1: expected {EXPECTED_ROM_SHA1}, found {digest}")

    pyboy = PyBoy(str(args.rom), window="null")
    try:
        for _ in range(args.frames):
            pyboy.tick()
        print(f"ASM oracle booted {args.rom} for {pyboy.frame_count} frames")
    finally:
        pyboy.stop()


if __name__ == "__main__":
    main()
