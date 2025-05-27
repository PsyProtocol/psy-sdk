// .eslintrc.js
// eslint-disable-next-line no-undef
module.exports = {
    parser: '@typescript-eslint/parser',
    plugins: ['@typescript-eslint'],
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
    },
};