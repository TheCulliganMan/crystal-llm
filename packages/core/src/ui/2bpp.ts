import { Surface } from './game-engine';

type RGBTuple = [number, number, number];

export function decode2bppTiles(data: Buffer, palette: RGBTuple[]): Surface[] {
    const tiles: Surface[] = [];
    const tileWidth = 8;
    const tileHeight = 8;

    const safePalette = [...palette];
    while (safePalette.length < 4) {
        safePalette.push([0, 0, 0]);
    }

    for (let i = 0; i < data.length; i += 16) {
        const tileData = data.slice(i, i + 16);
        const surface = new Surface(tileWidth, tileHeight);
        for (let y = 0; y < tileHeight; y++) {
            const byte1 = tileData[y * 2];
            const byte2 = tileData[y * 2 + 1];
            for (let x = 0; x < tileWidth; x++) {
                const bit1 = (byte1 >> (7 - x)) & 1;
                const bit2 = (byte2 >> (7 - x)) & 1;
                const colorIndex = (bit2 << 1) | bit1;
                const color = safePalette[colorIndex];
                surface.set_at([x, y], [...color, 255]);
            }
        }
        tiles.push(surface);
    }

    return tiles;
}
