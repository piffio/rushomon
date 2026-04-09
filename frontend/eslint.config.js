import js from "@eslint/js";
import prettierConfig from "eslint-config-prettier";
import prettier from "eslint-plugin-prettier";
import svelte from "eslint-plugin-svelte";
import globals from "globals";
import tseslint from "typescript-eslint";

export default [
  js.configs.recommended,
  ...tseslint.configs.recommended,
  prettierConfig,
  ...svelte.configs["flat/base"],
  ...svelte.configs["flat/recommended"],
  {
    files: ["**/*.svelte"],
    languageOptions: {
      parser: svelte.parser,
      parserOptions: {
        parser: tseslint.parser
      },
      globals: {
        ...globals.browser
      }
    },
    rules: {
      "prefer-const": "off"
    }
  },
  {
    files: ["**/*.js", "**/*.ts"],
    languageOptions: {
      parser: tseslint.parser,
      globals: {
        ...globals.browser
      }
    }
  },
  {
    files: ["scripts/**/*.js", "src/routes/sitemap.xml/+server.ts"],
    languageOptions: {
      globals: {
        ...globals.node
      }
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
      "svelte/no-navigation-without-resolve": "off",
      "svelte/prefer-writable-derived": "off",
      "@typescript-eslint/no-unused-vars": [
        "error",
        {
          argsIgnorePattern: "^_",
          varsIgnorePattern: "^_",
          caughtErrorsIgnorePattern: "^_"
        }
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
    files: ["**/*.svelte"],
    rules: {
      "prefer-const": "off"
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
