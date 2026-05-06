export enum FacingDirection {
    DOWN = 0,
    UP = 1,
    LEFT = 2,
    RIGHT = 3
}

export function facingDirectionFromString(value: string): FacingDirection {
    switch (value.toLowerCase()) {
        case "down":
            return FacingDirection.DOWN;
        case "up":
            return FacingDirection.UP;
        case "left":
            return FacingDirection.LEFT;
        case "right":
            return FacingDirection.RIGHT;
        default:
            throw new Error(`Unknown facing direction '${value}'`);
    }
}

export function facingDirectionQuadrantIndices(direction: FacingDirection): [number, number] {
    switch (direction) {
        case FacingDirection.DOWN:
            return [2, 3];
        case FacingDirection.UP:
            return [0, 1];
        case FacingDirection.LEFT:
            return [0, 2];
        case FacingDirection.RIGHT:
            return [1, 3];
        default:
            throw new Error(`Unknown facing direction ${direction}`);
    }
}

export enum PlayerState {
    NORMAL = "NORMAL",
    BIKE = "BIKE",
    SKATE = "SKATE",
    SURF = "SURF",
    SURF_PIKA = "SURF_PIKA",
}

export namespace FacingDirection {
    export const fromString = facingDirectionFromString;
    export const quadrantIndices = facingDirectionQuadrantIndices;
}
