import { readMapBlockBytes, readMapBlockBytesAsync } from "@pokecrystal/core/core/map-blocks";

const _MAP_BLOCK_CACHE: Map<string, Buffer> = new Map();
const _MAP_BLOCK_PENDING: Map<string, Promise<void>> = new Map();
const _MAP_BLOCK_ERRORS: Map<string, Error> = new Map();

export class OverworldMap {
    public readonly mapName: string;
    public dataLoader?: {
        get_text?: (label: string) => string | null;
        getText?: (label: string) => string | null;
    } | null;
    public refresh_event_flag?: (flagName: string, options?: { value?: boolean }) => void;
    public metatileIds: number[] = [];
    private _width: number;
    private _height: number;
    private _blocksLabel: string | null;

    public static preloadBlocks(mapName: string, blocksLabel?: string | null): Promise<void> {
        if (!mapName) {
            return Promise.resolve();
        }
        if (_MAP_BLOCK_CACHE.has(mapName)) {
            return Promise.resolve();
        }
        const cachedError = _MAP_BLOCK_ERRORS.get(mapName);
        if (cachedError) {
            return Promise.reject(cachedError);
        }
        const pending = _MAP_BLOCK_PENDING.get(mapName);
        if (pending) {
            return pending;
        }
        const loadPromise = readMapBlockBytesAsync(mapName, blocksLabel ?? null)
            .then((data) => {
                _MAP_BLOCK_CACHE.set(mapName, data);
                _MAP_BLOCK_PENDING.delete(mapName);
            })
            .catch((error: unknown) => {
                const resolvedError = error instanceof Error ? error : new Error(String(error));
                _MAP_BLOCK_ERRORS.set(mapName, resolvedError);
                _MAP_BLOCK_PENDING.delete(mapName);
                throw resolvedError;
            });
        _MAP_BLOCK_PENDING.set(mapName, loadPromise);
        return loadPromise;
    }

    constructor(mapName: string, width: number, height: number, blocksLabel?: string | null) {
        this.mapName = mapName;
        this._width = width;
        this._height = height;
        this._blocksLabel = blocksLabel ?? null;

        this._loadMapData();
    }

    public get width(): number {
        return this._width;
    }

    public get height(): number {
        return this._height;
    }

    public setDimensions(width: number, height: number): void {
        this._width = width;
        this._height = height;
    }

    private _loadMapData() {
        const cachedError = _MAP_BLOCK_ERRORS.get(this.mapName);
        if (cachedError) {
            throw cachedError;
        }
        let data = _MAP_BLOCK_CACHE.get(this.mapName);
        if (data === undefined) {
            data = readMapBlockBytes(this.mapName, this._blocksLabel);
            _MAP_BLOCK_CACHE.set(this.mapName, data);
        }
        this.metatileIds = Array.from(data);
    }

    public getMetatileAt(x: number, y: number): number {
        if (x >= 0 && x < this.width && y >= 0 && y < this.height) {
            return this.metatileIds[y * this.width + x];
        }
        throw new Error(
            `Metatile lookup out of range for ${this.mapName}: ` +
            `x=${x}, y=${y}, width=${this.width}, height=${this.height}`
        );
    }
}
