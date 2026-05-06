import { GameState } from '../../core/state';

const _LFSR_POLY = 0xb400;

/**
 * A facsimile of the Game Boy's hardware-based pseudo-random number generator.
 *
 * This class simulates the behavior of the original game's `Random` routine,
 * which was used for minigames and other non-cryptographic random events. It
 * uses a Linear Feedback Shift Register (LFSR) to emulate the hardware
 * divider register.
 */
export class HardwareRNG {
  private state: GameState;

  constructor(state: GameState) {
    this.state = state;
  }

  /**
   * Return the next divider byte using a 16-bit Galois LFSR.
   */
  private _stepDivider(): number {
    let divider = this.state.hram.hardware_divider ?? 0;
    if (divider === 0) {
      divider = 0xace1;
    }

    const feedback = divider & 1;
    divider >>= 1;
    if (feedback) {
      divider ^= _LFSR_POLY;
    }

    this.state.hram.hardware_divider = divider;
    return divider & 0xff;
  }

  /**
   * Generates the next pseudo-random byte.
   *
   * This method advances the RNG state by one step and returns the resulting
   * random byte, mimicking the `Random` routine from the original game.
   *
   * @returns A pseudo-random integer between 0 and 255.
   */
  public nextByte(): number {
    let addAcc = this.state.hram.hRandomAdd ?? 0;
    let subAcc = this.state.hram.hRandomSub ?? 0;

    const divider = this._stepDivider();
    addAcc = (addAcc + divider) & 0xff;
    subAcc = (subAcc - divider) & 0xff;

    this.state.hram.hRandomAdd = addAcc;
    this.state.hram.hRandomSub = subAcc;
    return subAcc;
  }

  /**
   * Return a value in `[0, upperBound)` mirroring `RandomRange`.
   */
  public randrange(upperBound: number): number {
    if (upperBound <= 0) {
      throw new Error('upperBound must be positive');
    }

    let mask = 1;
    while (mask < upperBound) {
      mask = (mask << 1) | 1;
    }

    // Generate enough random bytes to fill the mask width; otherwise large ranges
    // would only ever sample the low 8 bits and bias outcomes (e.g., catch wobble
    // rolls would always succeed).
    const bitLength = mask.toString(2).length;
    const byteCount = Math.max(1, Math.ceil(bitLength / 8));

    while (true) {
      let value = 0;
      for (let i = 0; i < byteCount; i++) {
        value = (value << 8) | this.nextByte();
      }
      value &= mask;
      if (value < upperBound) {
        return value;
      }
    }
  }

  /**
   * Return `true` with the provided probability.
   */
  public coinFlip(probability: number): boolean {
    const threshold = Math.floor(probability * 256);
    return this.nextByte() < threshold;
  }

  /**
   * Return a random integer N such that a <= N <= b.
   */
  public randint(a: number, b: number): number {
    return a + this.randrange(b - a + 1);
  }

  /**
   * Return a random element from the non-empty sequence seq.
   */
  public choice<T>(seq: T[]): T {
    if (seq.length === 0) {
      throw new Error('Cannot choose from an empty sequence');
    }
    return seq[this.randrange(seq.length)];
  }

  public peekHRandomAdd(): number {
    return this.state.hram.hRandomAdd ?? 0;
  }
}
