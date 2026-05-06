export function valores<T extends number>(e: Record<string, number>): T[] {
  return Object.values(e).filter((v) => typeof v === "number") as T[];
}

export function createCircuitBreaker(limit: number = 10000, name: string = "Unnamed Loop") {
  let count = 0;
  return () => {
    if (++count > limit) {
      const tempError = new Error();
      let locationInfo = "unknown location";
      if (tempError.stack) {
        const stackLines = tempError.stack.split("\n");
        // Typically line 0 is 'Error', line 1 is createCircuitBreaker, line 2 is the caller.
        if (stackLines.length > 2) {
          const callerLine = stackLines[2].trim();
          locationInfo = callerLine;
        }
      }
      throw new Error(`Circuit breaker triggered! ${name} exceeded ${limit} iterations at ${locationInfo}. Potential infinite loop detected.`);
    }
  };
}
