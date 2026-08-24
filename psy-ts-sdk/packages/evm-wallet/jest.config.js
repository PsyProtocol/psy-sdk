/** Mirrors the sibling packages' ts-jest setup. ESM package → ts-jest ESM preset. */
export default {
  preset: 'ts-jest/presets/default-esm',
  testEnvironment: 'node',
  extensionsToTreatAsEsm: ['.ts'],
  moduleNameMapper: {
    '^(\\.{1,2}/.*)\\.js$': '$1',
  },
  transform: {
    '^.+\\.tsx?$': [
      'ts-jest',
      { useESM: true, tsconfig: 'tsconfig.json' },
    ],
  },
  testMatch: ['**/__tests__/**/*.test.ts', '**/src/**/*.test.ts'],
};
