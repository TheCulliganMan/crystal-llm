import { Battle } from "./battle-logic";

export enum BattleStatusPhase {
    PRE_TURN,
    POST_TURN,
}

export class BattleStatusScheduler {
    private _handlers: Map<BattleStatusPhase, Array<() => void>>;
    private _battle: Battle;

    constructor(battle: Battle) {
        this._battle = battle;
        this._handlers = new Map<BattleStatusPhase, Array<() => void>>();
        for (const phase in BattleStatusPhase) {
            if (isNaN(Number(phase))) {
                this._handlers.set(BattleStatusPhase[phase as keyof typeof BattleStatusPhase], []);
            }
        }
    }

    public register(phase: BattleStatusPhase, handler: () => void): void {
        const phaseHandlers = this._handlers.get(phase);
        if (phaseHandlers) {
            phaseHandlers.push(handler);
        }
    }

    public process(phase: BattleStatusPhase): void {
        const handlers = this._handlers.get(phase);
        if (handlers) {
            for (const handler of handlers) {
                handler();
            }
        }
    }
}
