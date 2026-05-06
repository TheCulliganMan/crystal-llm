// ASM mapping: pokecrystal_disassembly/engine/menus/menu_2.asm (PlaceMoneyTopRight/DisplayMoneyAndCoinBalance).
import { GameState } from "@pokecrystal/core/core/state";
import { Event, type EventManager } from "@pokecrystal/core/engine/events/events";
import { ScriptRunner, setRunnerValue } from "./utils";
import type { Overworld } from "@pokecrystal/core/types/overworld";

type EventManagerLike = Pick<EventManager, "dispatch">;

const formatMoney = (amount: number): string => String(Math.max(0, amount)).padStart(6, "0");

export function place_money_top_right(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld; event_manager?: EventManager } = {}
): number {
  // ASM: engine/menus/menu_2.asm::PlaceMoneyTopRight
  void overworld;
  const money = Math.max(0, Number(game_state.sram.money ?? 0));
  if (event_manager?.dispatch) {
    event_manager.dispatch(
      new Event("show_money", {
        source: "special",
        overlay: { x: 11, y: 0, width: 9, height: 2, value: money },
      })
    );
  }
  if (runner) {
    if (!runner.string_buffers) {
      runner.string_buffers = {};
    }
    runner.string_buffers.STRING_BUFFER_1 = formatMoney(money);
    setRunnerValue(runner, money, { truthy: true });
  }
  return money;
}

export function display_money_and_coin_balance(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld; event_manager?: EventManager } = {}
): { money: number; coins: number } {
  // ASM: engine/menus/menu_2.asm::DisplayMoneyAndCoinBalance
  void overworld;
  const money = Math.max(0, Number(game_state.sram.money ?? 0));
  const coins = Math.max(0, Number(game_state.sram.coins ?? 0));
  if (event_manager?.dispatch) {
    event_manager.dispatch(
      new Event("show_money_and_coins", {
        source: "special",
        overlay: { money, coins },
      })
    );
  }
  if (runner) {
    if (!runner.string_buffers) {
      runner.string_buffers = {};
    }
    runner.string_buffers.STRING_BUFFER_1 = formatMoney(money);
    runner.string_buffers.STRING_BUFFER_2 = String(coins).padStart(4, "0");
    setRunnerValue(runner, money, { truthy: true });
  }
  return { money, coins };
}
