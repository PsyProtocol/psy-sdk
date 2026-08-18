import resolve from '@rollup/plugin-node-resolve';
import commonjs from '@rollup/plugin-commonjs';
import typescript from '@rollup/plugin-typescript';

// Mirrors packages/psy-sdk/rollup.config.js (dual-format, preserveModules) with
// one addition: EVERYTHING external. This package ships only its own source —
// wagmi/viem/ethers/react/@psy-protocol/psy-sdk are peers the consumer app
// already bundles (a second copy would mean two wagmi stores / two 22MB WASM
// blobs), and nostr-tools / poseidon-goldilocks-lite are ordinary deps the
// consumer's bundler resolves.
const input = {
    index: 'src/index.ts',
    'react/index': 'src/react/index.ts',
    // The prover Web Worker entry — a distinct, side-effectful module so apps
    // can do `import '@psy-protocol/evm-wallet/worker'` from their own
    // bundler-visible worker file (see README: createWorker).
    worker: 'src/worker.ts',
};

const external = (id) =>
    !id.startsWith('.') && !id.startsWith('/') && !id.startsWith('src/');

export default {
    input,
    external,
    output: [
        {
            dir: 'dist',
            format: 'esm',
            preserveModules: true,
            preserveModulesRoot: 'src',
            entryFileNames: '[name].mjs',
        },
        {
            dir: 'dist',
            format: 'cjs',
            preserveModules: true,
            preserveModulesRoot: 'src',
            entryFileNames: '[name].cjs',
        },
    ],
    plugins: [
        resolve({
            extensions: ['.ts', '.tsx', '.js'],
            moduleDirectories: ['node_modules'],
        }),
        commonjs({
            include: /node_modules/,
        }),
        typescript({
            tsconfig: 'tsconfig.build.json',
            declaration: true,
            outDir: 'dist',
            sourceMap: false,
        }),
    ],
};
