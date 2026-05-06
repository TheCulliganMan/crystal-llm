/**
 * This module contains a TypeScript implementation of the LZ decompression algorithm
 * used in the PokeCrystal disassembly.
 *
 * ASM reference: pokecrystal_disassembly/home/decompress.asm (Decompress:: rewrite commands).
 */

export function decompress(data: Uint8Array): Uint8Array {
    const src = data;
    const dest: number[] = [];
    let src_idx = 0;

    while (src_idx < src.length) {
        // The compressed data stream is terminated by the byte 0xFF.
        if (src[src_idx] === 0xFF) {
            break;
        }

        const ctrl = src[src_idx];
        src_idx += 1;

        let cmd = ctrl & 0xE0;
        let length = 0;

        if (cmd === 0xE0) { // LZ_LONG: command 7, extended length
            cmd = (ctrl & 0x1C) << 3;
            const len_hi = ctrl & 0x03;
            const len_lo = src[src_idx];
            src_idx += 1;
            length = ((len_hi << 8) | len_lo) + 1;
        } else {
            length = (ctrl & 0x1F) + 1;
        }

        if (cmd < 0x80) { // Non-rewrite commands
            if (cmd === 0x00) { // LZ_LITERAL: copy n+1 bytes
                for (let i = 0; i < length; i++) {
                    dest.push(src[src_idx + i]);
                }
                src_idx += length;
            } else if (cmd === 0x20) { // LZ_ITERATE: repeat next byte n+1 times
                const byte = src[src_idx];
                src_idx += 1;
                for (let i = 0; i < length; i++) {
                    dest.push(byte);
                }
            } else if (cmd === 0x40) { // LZ_ALTERNATE: alternate two bytes n+1 times
                const b1 = src[src_idx];
                const b2 = src[src_idx + 1];
                src_idx += 2;
                for (let i = 0; i < length; i++) {
                    dest.push(i % 2 === 0 ? b1 : b2);
                }
            } else if (cmd === 0x60) { // LZ_ZERO: write 0 n+1 times
                for (let i = 0; i < length; i++) {
                    dest.push(0);
                }
            }
        } else { // Rewrite commands
            const offset_byte1 = src[src_idx];
            src_idx += 1;
            let rw_idx = 0;

            if (offset_byte1 & 0x80) { // Negative offset
                const offset = offset_byte1 & 0x7F;
                rw_idx = dest.length - offset - 1;
            } else { // Positive offset
                const offset_byte2 = src[src_idx];
                src_idx += 1;
                rw_idx = ((offset_byte1 & 0x7F) << 8) | offset_byte2;
            }

            if (cmd === 0x80) { // LZ_REPEAT: copy n+1 bytes from output
                for (let i = 0; i < length; i++) {
                    dest.push(dest[rw_idx]);
                    rw_idx += 1;
                }
            } else if (cmd === 0xA0) { // LZ_FLIP: copy n+1 bit-flipped bytes
                for (let i = 0; i < length; i++) {
                    let original_byte = dest[rw_idx];
                    let flipped_byte = 0;
                    for (let j = 0; j < 8; j++) {
                        flipped_byte = (flipped_byte << 1) | (original_byte & 1);
                        original_byte >>= 1;
                    }
                    dest.push(flipped_byte);
                    rw_idx += 1;
                }
            } else if (cmd === 0xC0) { // LZ_REVERSE: copy n+1 bytes in reverse
                let idx = rw_idx;
                for (let i = 0; i < length; i++) {
                    if (idx < 0 || idx >= dest.length) {
                        throw new Error(
                            `reverse oob idx=${idx} length=${length} rw_idx=${rw_idx} len(dest)=${dest.length}`
                        );
                    }
                    dest.push(dest[idx]);
                    idx -= 1;
                }
            }
        }
    }

    return new Uint8Array(dest);
}
