
import { FacingDirection } from "../../core/enums";

export class Player {
    public x: number;
    public y: number;
    public direction: FacingDirection;

    constructor(x: number, y: number, direction: FacingDirection) {
        this.x = x;
        this.y = y;
        this.direction = direction;
    }

    public move(direction: FacingDirection): void {
        this.direction = direction;
        switch (direction) {
            case FacingDirection.UP:
                this.y--;
                break;
            case FacingDirection.DOWN:
                this.y++;
                break;
            case FacingDirection.LEFT:
                this.x--;
                break;
            case FacingDirection.RIGHT:
                this.x++;
                break;
        }
    }
}
