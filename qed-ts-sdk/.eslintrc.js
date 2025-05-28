// .eslintrc.js
// eslint-disable-next-line no-undef
module.exports = {
    parser: '@typescript-eslint/parser',
    plugins: ['@typescript-eslint','import'],
    parserOptions: {
        ecmaVersion: 2020,
        sourceType: 'commonjs'
    },
    extends: [
        'eslint:recommended',
        'plugin:@typescript-eslint/recommended',
        'prettier'
    ],
    env: {
        node: true,
        jest: true
    },
    rules: {
        // '@typescript-eslint/no-explicit-any': 'warn',
        // 'import/no-unresolved': 'error',
        // 'import/no-webpack-loader-syntax': 'off',
        // 'import/no-useless-path-segments': 'warn',
        'unused-imports/no-unused-imports': 'error',
        'unused-imports/no-unused-vars': [
            'warn',
            {
                vars: 'all',
                varsIgnorePattern: '^_',
                args: 'after-used',
                argsIgnorePattern: '^_',
            },
        ],
    },
};