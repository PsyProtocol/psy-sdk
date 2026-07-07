import resolve from '@rollup/plugin-node-resolve';
import commonjs from '@rollup/plugin-commonjs';
import typescript from '@rollup/plugin-typescript';

const input = {
    index: 'src/index.ts',
    'expert/index': 'src/expert/index.ts',
    'local-web-prover/index': 'src/local-web-prover/index.ts',
    'local-web-compiler/index': 'src/local-web-compiler/index.ts',
    'local-web-compiler/compiler': 'src/local-web-compiler/compiler.ts',
    // Keep raw wasm-bindgen entry as explicit Rollup input so exports like
    // init_chain/create_account/deploy_contract/call_contract are not pruned.
    'local-web-compiler/psy_compiler': 'src/local-web-compiler/psy_compiler.js',
    'local-web-compiler/wasm-binary': 'src/local-web-compiler/wasm-binary.ts',
};

export default {
    input,
    output: [
        {
            dir: 'dist',
            format: 'esm',
            exports: 'named',
            preserveModules: true,
            preserveModulesRoot: 'src',
            entryFileNames: '[name].mjs',
        },
        {
            dir: 'dist',
            format: 'cjs',
            exports: 'named',
            preserveModules: true,
            preserveModulesRoot: 'src',
            entryFileNames: '[name].cjs',
        }
    ],
    plugins: [
        resolve({
            extensions: ['.ts', '.js'],
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
        })
    ]
};
