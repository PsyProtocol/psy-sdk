import resolve from '@rollup/plugin-node-resolve';
import commonjs from '@rollup/plugin-commonjs';
import typescript from '@rollup/plugin-typescript';

export default {
    input: 'src/index.ts',
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
