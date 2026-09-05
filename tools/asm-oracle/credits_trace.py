from __future__ import annotations

import argparse
import hashlib
import json
import re
import tempfile
from pathlib import Path
from typing import Any

from pyboy import PyBoy


EXPECTED_ROM_SHA1 = "f4cd194bdee0d04ca4eac29e09b8e4e9d818c133"

ADDR = {
    "credits_pos": 0xCD20,
    "credits_timer": 0xCD22,
    "jumptable_index": 0xCF63,
    "credits_border_frame": 0xCF64,
    "credits_border_mon": 0xCF65,
    "credits_ly_override": 0xCF66,
    "h_rom_bank": 0xFF9D,
    "h_vblank": 0xFF9E,
    "h_bg_map_mode": 0xFFD4,
    "h_bg_map_third": 0xFFD5,
}

VBLANK_CREDITS = 5
ALLOW_SKIPPING_CREDITS = 1 << 6
JUMPTABLE_EXIT = 1 << 7


def load_symbols(path: Path) -> dict[str, tuple[int, int]]:
    symbols: dict[str, tuple[int, int]] = {}
    pattern = re.compile(r"^([0-9a-fA-F]{2}):([0-9a-fA-F]{4})\s+(\S+)$")
    for raw_line in path.read_text().splitlines():
        match = pattern.match(raw_line)
        if match:
            symbols[match.group(3)] = (int(match.group(1), 16), int(match.group(2), 16))
    return symbols


def rom_offset(bank: int, address: int) -> int:
    if bank == 0:
        return address
    if not 0x4000 <= address < 0x8000:
        raise ValueError(f"banked address is outside the ROM window: {bank:02x}:{address:04x}")
    return bank * 0x4000 + address - 0x4000


def credits_script_length(rom: bytes, bank: int, address: int) -> int:
    start = rom_offset(bank, address)
    end = rom.find(b"\xff", start)
    if end < 0:
        raise ValueError("CreditsScript has no terminating CREDITS_END byte")
    return end - start + 1


def patch_intro_dispatch(
    rom: bytes,
    intro: tuple[int, int],
    red_credits: tuple[int, int],
    status_flags_address: int,
    allow_skip: bool,
) -> bytes:
    intro_bank, intro_address = intro
    red_credits_bank, red_credits_address = red_credits
    if red_credits_bank > 0xFF:
        raise ValueError("RedCredits bank does not fit the farcall ABI")
    patch = bytearray()
    if allow_skip:
        # Model a post-Hall-of-Fame save, which is the real condition that
        # permits RedCredits' B-button acceleration.
        patch.extend(
            (
                0x3E,
                ALLOW_SKIPPING_CREDITS,
                0xEA,
                status_flags_address & 0xFF,
                status_flags_address >> 8,
            )
        )
    # ld a, BANK(RedCredits); ld hl, RedCredits; rst FarCall; ret
    patch.extend(
        (
            0x3E,
            red_credits_bank,
            0x21,
            red_credits_address & 0xFF,
            red_credits_address >> 8,
            0xCF,
            0xC9,
        )
    )
    result = bytearray(rom)
    start = rom_offset(intro_bank, intro_address)
    result[start : start + len(patch)] = patch
    return bytes(result)


def memory_u16(pyboy: PyBoy, address: int) -> int:
    return pyboy.memory[address] | pyboy.memory[address + 1] << 8


def snapshot(pyboy: PyBoy, event: str, absolute_frame: int, credits_frame: int) -> dict[str, Any]:
    pixels = bytes(pyboy.screen.ndarray.tobytes())
    rgb5_pixels = bytes(
        component >> 3
        for offset, component in enumerate(pixels)
        if offset % 4 != 3
    )
    saved_vram_bank = pyboy.memory[0xFF4F]
    pyboy.memory[0xFF4F] = 0
    tilemap = b"".join(bytes(pyboy.memory[0x9800 + row * 32 : 0x9814 + row * 32]) for row in range(18))
    pyboy.memory[0xFF4F] = 1
    attrmap = b"".join(bytes(pyboy.memory[0x9800 + row * 32 : 0x9814 + row * 32]) for row in range(18))
    pyboy.memory[0xFF4F] = saved_vram_bank
    return {
        "event": event,
        "absolute_frame": absolute_frame,
        "credits_frame": credits_frame,
        "rgba_sha256": hashlib.sha256(pixels).hexdigest(),
        "rgb5_sha256": hashlib.sha256(rgb5_pixels).hexdigest(),
        "bg_tilemap_sha256": hashlib.sha256(tilemap).hexdigest(),
        "bg_attrmap_sha256": hashlib.sha256(attrmap).hexdigest(),
        "credits_pos": memory_u16(pyboy, ADDR["credits_pos"]),
        "credits_timer": pyboy.memory[ADDR["credits_timer"]],
        "jumptable_index": pyboy.memory[ADDR["jumptable_index"]],
        "h_vblank": pyboy.memory[ADDR["h_vblank"]],
        "h_bg_map_mode": pyboy.memory[ADDR["h_bg_map_mode"]],
        "h_bg_map_third": pyboy.memory[ADDR["h_bg_map_third"]],
        "credits_border_frame": pyboy.memory[ADDR["credits_border_frame"]],
        "credits_border_mon": pyboy.memory[ADDR["credits_border_mon"]],
        "credits_ly_override": pyboy.memory[ADDR["credits_ly_override"]],
    }


def write_ppm(path: Path, pyboy: PyBoy) -> None:
    rgba = pyboy.screen.ndarray
    height, width, _ = rgba.shape
    rgb = rgba[:, :, :3].tobytes()
    path.write_bytes(f"P6\n{width} {height}\n255\n".encode() + rgb)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Run the pinned ASM Credits routine and emit LCD checkpoints"
    )
    parser.add_argument("rom", type=Path)
    parser.add_argument("--symbols", type=Path)
    parser.add_argument("--frames", type=int, default=25000)
    parser.add_argument(
        "--accelerate",
        action="store_true",
        help="hold B after setup; pixels remain ROM-produced but wait durations are shortened",
    )
    parser.add_argument("--ppm-dir", type=Path, help="optionally write checkpoint images as PPM")
    args = parser.parse_args()

    canonical_rom = args.rom.read_bytes()
    digest = hashlib.sha1(canonical_rom).hexdigest()
    if digest != EXPECTED_ROM_SHA1:
        raise SystemExit(f"unexpected ROM SHA-1: expected {EXPECTED_ROM_SHA1}, found {digest}")
    symbol_path = args.symbols or args.rom.with_suffix(".sym")
    symbols = load_symbols(symbol_path)
    try:
        intro = symbols["CrystalIntro"]
        red_credits = symbols["RedCredits"]
        script = symbols["CreditsScript"]
        _, status_flags_address = symbols["wStatusFlags"]
    except KeyError as error:
        raise SystemExit(f"required symbol is missing: {error.args[0]}") from error
    script_length = credits_script_length(canonical_rom, *script)
    patched_rom = patch_intro_dispatch(
        canonical_rom,
        intro,
        red_credits,
        status_flags_address,
        allow_skip=args.accelerate,
    )

    if args.ppm_dir:
        args.ppm_dir.mkdir(parents=True, exist_ok=True)
    checkpoints: list[dict[str, Any]] = []
    seen: set[str] = set()

    with tempfile.TemporaryDirectory(prefix="pokecrystal-credits-oracle-") as temp_dir:
        rom_path = Path(temp_dir) / "oracle.gbc"
        rom_path.write_bytes(patched_rom)
        pyboy = PyBoy(str(rom_path), window="null", log_level="ERROR")
        try:
            credits_frame = 0
            setup_started = False
            awaiting_exit = False
            for absolute_frame in range(1, args.frames + 1):
                if args.accelerate and setup_started and not awaiting_exit:
                    pyboy.button_press("b")
                pyboy.tick()
                if args.accelerate and setup_started and not awaiting_exit:
                    pyboy.button_release("b")

                if not setup_started and pyboy.memory[ADDR["credits_border_frame"]] == 0xFF:
                    setup_started = True
                    credits_frame = 1
                    event = "border_assets_loaded"
                elif setup_started:
                    credits_frame += 1
                    event = ""
                else:
                    continue

                position = memory_u16(pyboy, ADDR["credits_pos"])
                jumptable = pyboy.memory[ADDR["jumptable_index"]]
                candidates = [event] if event else []
                if pyboy.memory[ADDR["h_vblank"]] == VBLANK_CREDITS:
                    candidates.append("execution_ready")
                if position > 0:
                    candidates.append("first_script_parse")
                if position >= 5:
                    candidates.append(f"first_bg_third_{pyboy.memory[ADDR['h_bg_map_third']]}")
                if (
                    position == 5
                    and pyboy.memory[ADDR["h_bg_map_mode"]] == 0
                    and (jumptable & 0x0F) >= 4
                ):
                    candidates.append("first_transfer_complete")
                if position == script_length - 1:
                    candidates.append("the_end_wait")
                    if pyboy.memory[ADDR["credits_timer"]] == 0 and jumptable & 0x0F == 0:
                        candidates.append("pre_exit")
                if jumptable & JUMPTABLE_EXIT:
                    candidates.append("awaiting_exit")
                    awaiting_exit = True

                for candidate in candidates:
                    if not candidate or candidate in seen:
                        continue
                    seen.add(candidate)
                    checkpoints.append(snapshot(pyboy, candidate, absolute_frame, credits_frame))
                    if args.ppm_dir:
                        write_ppm(args.ppm_dir / f"{len(checkpoints):02d}-{candidate}.ppm", pyboy)
                if awaiting_exit:
                    break
            else:
                raise SystemExit(f"Credits did not reach its exit state within {args.frames} frames")

            by_event = {checkpoint["event"]: checkpoint for checkpoint in checkpoints}
            for field in ("rgba_sha256", "rgb5_sha256", "bg_tilemap_sha256", "bg_attrmap_sha256"):
                if by_event["pre_exit"][field] != by_event["awaiting_exit"][field]:
                    raise SystemExit(
                        f"CREDITS_END unexpectedly changed retained display field {field}"
                    )

            pyboy.button_press("a")
            for post_a_frame in range(1, 7):
                pyboy.tick()
                if post_a_frame == 1:
                    pyboy.button_release("a")
                checkpoint = snapshot(
                    pyboy,
                    f"post_a_{post_a_frame}",
                    pyboy.frame_count,
                    credits_frame + post_a_frame,
                )
                checkpoints.append(checkpoint)
                if args.ppm_dir:
                    write_ppm(args.ppm_dir / f"{len(checkpoints):02d}-post_a_{post_a_frame}.ppm", pyboy)
        finally:
            pyboy.stop()

    print(
        json.dumps(
            {
                "rom_sha1": digest,
                "dispatch_patch": "CrystalIntro -> RedCredits",
                "credits_script_length": script_length,
                "accelerated": args.accelerate,
                "checkpoints": checkpoints,
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
