import '@testing-library/jest-dom/vitest';

Object.defineProperty(globalThis.navigator, 'clipboard', {
  configurable: true,
  value: {
    writeText: async () => undefined,
  },
});
