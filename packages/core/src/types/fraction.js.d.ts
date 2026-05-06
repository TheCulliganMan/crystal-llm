declare module 'fraction.js' {
  interface Fraction {
    n: number;
    d: number;
    s: number;
    mul(...args: (number | string | Fraction)[]): Fraction;
    div(...args: (number | string | Fraction)[]): Fraction;
    add(...args: (number | string | Fraction)[]): Fraction;
    sub(...args: (number | string | Fraction)[]): Fraction;
    compare(num: number | string | Fraction): number;
    valueOf(): number;
  }

  interface FractionConstructor {
    new(num?: number | string | Fraction, den?: number): Fraction;
    (num?: number | string | Fraction, den?: number): Fraction;
  }

  const Fraction: FractionConstructor;
  export = Fraction;
}
