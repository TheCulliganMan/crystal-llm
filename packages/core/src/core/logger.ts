const shouldEmitDebugLogs = (): boolean => {
  const value = process.env.NEXT_PUBLIC_DEBUG_LOGS ?? process.env.DEBUG_LOGS;
  return value === "1" || value === "true";
};

const logger = {
  info: (...args: unknown[]) => {
    console.log(...args);
  },
  debug: (...args: unknown[]) => {
    if (shouldEmitDebugLogs()) {
      console.log(...args);
    }
  },
  warn: (...args: unknown[]) => {
    console.warn(...args);
  },
  error: (...args: unknown[]) => {
    console.error(...args);
  },
};

export default logger;
