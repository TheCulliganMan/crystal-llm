import { z } from 'zod';
import { HardwareRNG } from './rng';

export const CARD_TYPES = [
  'ODDISH',
  'POLIWAG',
  'PIKACHU',
  'JIGGLYPUFF',
  'RATTATA',
  'VOLTORB',
];

export const CardFlipResultSchema = z.object({
  cardIndex: z.number(),
  cardName: z.string(),
  payout: z.number(),
});

export type CardFlipResult = z.infer<typeof CardFlipResultSchema>;

export class CardFlipGame {
  rng: HardwareRNG;
  revealed: boolean[];
  deck: string[];

  constructor(rng: HardwareRNG, deck?: string[]) {
    this.rng = rng;
    this.revealed = Array(24).fill(false);
    this.deck = deck || this.buildDeck();
  }

  private buildDeck(): string[] {
    const deck: string[] = [];
    for (const name of CARD_TYPES) {
      deck.push(...Array(4).fill(name));
    }
    return deck;
  }

  shuffle(): void {
    for (let index = this.deck.length - 1; index > 0; index--) {
      const swapIndex = this.rng.randrange(index + 1);
      [this.deck[index], this.deck[swapIndex]] = [
        this.deck[swapIndex],
        this.deck[index],
      ];
    }
    this.revealed = Array(this.deck.length).fill(false);
  }

  remainingOf(target: string): number {
    return this.deck.reduce(
      (count, card, index) =>
        !this.revealed[index] && card === target ? count + 1 : count,
      0,
    );
  }

  private payoutFor(cardName: string): number {
    const remaining = this.remainingOf(cardName);
    if (cardName === 'PIKACHU') {
      const payouts: { [key: number]: number } = {
        6: 6,
        5: 12,
        4: 24,
        3: 36,
        2: 48,
        1: 72,
      };
      return payouts[remaining] || 6;
    }
    const payouts: { [key: number]: number } = { 4: 6, 3: 12, 2: 18, 1: 36 };
    return payouts[remaining] || 6;
  }

  flip(index: number): CardFlipResult {
    if (index < 0 || index >= this.deck.length) {
      throw new Error('card index out of range');
    }
    if (this.revealed[index]) {
      throw new Error('card already revealed');
    }

    this.revealed[index] = true;
    const cardName = this.deck[index];
    const payout = this.payoutFor(cardName);
    return { cardIndex: index, cardName, payout };
  }

  serialize(): string[] {
    return [...this.deck];
  }
}
