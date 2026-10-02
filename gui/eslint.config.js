// Configuración ESLint 9 (flat config) para la GUI de Pact.
//
// Se lintean únicamente los archivos JavaScript. La corrección sintáctica y de
// tipos de los componentes `.svelte` se valida con `svelte-check` (ver
// package.json). No se activa `no-undef` porque el proyecto no tipa los `.js`
// (jsconfig.json -> checkJs: false) y depende de globals del navegador y de
// Vitest; `svelte-check` cubre esa responsabilidad.

export default [
  {
    // `src/wasm/` lo genera `wasm-pack` (ver `make build-wasm`): es código
    // ajeno y cambia en cada build, así que no se lintea.
    ignores: [
      'dist/**',
      'node_modules/**',
      '.svelte-kit/**',
      'coverage/**',
      'src/wasm/**',
    ],
  },
  {
    files: ['**/*.js'],
    languageOptions: {
      ecmaVersion: 2023,
      sourceType: 'module',
    },
    rules: {
      'no-unused-vars': ['warn', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
      'no-var': 'warn',
      'prefer-const': 'warn',
      eqeqeq: ['warn', 'smart'],
      'no-console': 'off',
      'no-dupe-keys': 'error',
      'no-unreachable': 'error',
      'valid-typeof': 'error',
      'use-isnan': 'error',
      'no-constant-condition': 'warn',
      'no-empty': 'warn',
    },
  },
];
