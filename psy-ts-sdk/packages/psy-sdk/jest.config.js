/* eslint-env node */
export default {
    preset: 'ts-jest/presets/default-esm',
    testEnvironment: 'jest-environment-node',
    roots: ['./src'],
    testMatch: [
        '**/__tests__/**/*.test.ts',
        '**/?(*.)+(spec|test).ts'
    ],
    transform: {
        '^.+\\.(ts|tsx|js|jsx|mjs|mts)$': ['ts-jest', { tsconfig: 'tsconfig.json', useESM: true }],
    },
    extensionsToTreatAsEsm: ['.ts', '.tsx'],
    moduleNameMapper: {
        '^(\\.{1,2}/.*)\\.js$': '$1',
    },
    transformIgnorePatterns: [
        '/node_modules/(?!.*(?:@noble/secp256k1|doge-sdk))/'
    ],
    collectCoverageFrom: [
        'src/**/*.ts',
        '!src/**/*.d.ts',
        '!src/**/__tests__/**',
        '!src/**/index.ts'
    ],
    coverageDirectory: 'coverage',
    coverageReporters: ['text', 'lcov', 'html'],
    testTimeout: 30000,
    setupFilesAfterEnv: ['./jest.setup.ts'],
    verbose: true
};