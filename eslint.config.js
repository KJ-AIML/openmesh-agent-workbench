import js from "@eslint/js";
import vueParser from "vue-eslint-parser";
import tsParser from "@typescript-eslint/parser";
import vuePlugin from "eslint-plugin-vue";
import tseslint from "typescript-eslint";

export default [
  {
    ignores: [
      "node_modules/**",
      "dist/**",
      "src-tauri/target/**",
      "coverage/**",
      "*.d.ts",
    ],
  },
  // Base JS/TS rules for all source.
  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...vuePlugin.configs["flat/recommended"],
  {
    files: ["**/*.ts", "**/*.vue"],
    languageOptions: {
      parser: vueParser,
      parserOptions: {
        parser: tsParser,
        ecmaVersion: "latest",
        sourceType: "module",
        ecmaFeatures: {
          jsx: false,
        },
      },
      globals: {
        window: "readonly",
        document: "readonly",
        console: "readonly",
        setTimeout: "readonly",
        clearTimeout: "readonly",
        setInterval: "readonly",
        clearInterval: "readonly",
        process: "readonly",
        URL: "readonly",
        URLSearchParams: "readonly",
      },
    },
    rules: {
      // Allow explicit any during current cleanup phase; tighten later.
      "@typescript-eslint/no-explicit-any": "off",
      // Allow unused vars prefixed with _ (common for destructuring).
      "@typescript-eslint/no-unused-vars": [
        "warn",
        {
          argsIgnorePattern: "^_",
          varsIgnorePattern: "^_",
          caughtErrorsIgnorePattern: "^_",
        },
      ],
      // Vue component naming: allow multi-word except in generated/legacy areas.
      "vue/multi-word-component-names": "off",
      // Allow event names in any case for Tauri/cross-platform parity.
      "vue/component-definition-name-casing": "off",
    },
  },
  // Vite config uses plain Node — no Vue parser needed.
  {
    files: ["vite.config.ts", "vitest.config.ts", "eslint.config.js"],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: "latest",
        sourceType: "module",
      },
    },
    rules: {
      "@typescript-eslint/no-require-imports": "off",
      "@typescript-eslint/no-var-requires": "off",
    },
  },
];
