import js from "@eslint/js";
import tseslint from "typescript-eslint";
import svelte from "eslint-plugin-svelte";
import prettier from "eslint-plugin-prettier";
import prettierConfig from "eslint-config-prettier";

export default [
  js.configs.recommended,
  ...tseslint.configs.recommended,
  prettierConfig,
  {
    files: ["**/*.svelte"],
    languageOptions: {
      parser: svelte.parser,
      parserOptions: {
        parser: tseslint.parser
      }
    }
  },
  {
    files: ["**/*.js", "**/*.ts"],
    languageOptions: {
      parser: tseslint.parser
    }
  },
  {
    files: ["**/*.js", "**/*.ts", "**/*.svelte"],
    plugins: {
      "@typescript-eslint": tseslint.plugin,
      svelte,
      prettier
    },
    rules: {
      "svelte/no-at-html-tags": "off",
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_" }
      ],
      "@typescript-eslint/explicit-function-return-type": "off",
      "@typescript-eslint/explicit-module-boundary-types": "off",
      "@typescript-eslint/no-explicit-any": "warn",
      "prefer-const": "error",
      "no-var": "error",
      "prettier/prettier": "error"
    }
  },
  {
    ignores: [
      "build/",
      ".svelte-kit/",
      "dist/",
      "node_modules/",
      "*.config.js",
      "*.config.ts"
    ]
  }
];
