import js from '@eslint/js'
import { defineConfig, globalIgnores } from 'eslint/config'
import pluginVue from 'eslint-plugin-vue'
import globals from 'globals'
import tseslint from 'typescript-eslint'

const HEX_COLOR = /#(?:[0-9a-fA-F]{8}|[0-9a-fA-F]{6}|[0-9a-fA-F]{3,4})\b/g

// Components must take colors from the palette. This scans the whole file
// text, so it also covers <style> blocks and template attributes, which no
// syntax-based rule sees.
const noRawColors = {
  meta: {
    type: 'problem',
    messages: { rawColor: 'Use a palette token instead of the raw color {{color}}.' },
    schema: [],
  },
  create(context) {
    return {
      Program() {
        const source = context.sourceCode
        for (const match of source.text.matchAll(HEX_COLOR)) {
          context.report({
            loc: {
              start: source.getLocFromIndex(match.index),
              end: source.getLocFromIndex(match.index + match[0].length),
            },
            messageId: 'rawColor',
            data: { color: match[0] },
          })
        }
      },
    }
  },
}

export default defineConfig([
  globalIgnores(['dist/**', 'node_modules/**']),
  js.configs.recommended,
  tseslint.configs.recommended,
  pluginVue.configs['flat/recommended'],
  {
    files: ['**/*.vue'],
    languageOptions: {
      parserOptions: { parser: tseslint.parser, extraFileExtensions: ['.vue'] },
    },
  },
  {
    files: ['src/**/*.{ts,vue}'],
    languageOptions: { globals: globals.browser },
  },
  {
    files: ['*.{js,ts}'],
    languageOptions: { globals: globals.node },
  },
  {
    files: ['**/*.vue'],
    plugins: { giteagle: { rules: { 'no-raw-colors': noRawColors } } },
    rules: { 'giteagle/no-raw-colors': 'error' },
  },
  {
    rules: {
      // Layout of template markup is left to the author, as with the TS code.
      'vue/max-attributes-per-line': 'off',
      'vue/singleline-html-element-content-newline': 'off',
      'vue/multiline-html-element-content-newline': 'off',
      'vue/html-self-closing': 'off',
      'vue/html-indent': 'off',
      'vue/html-closing-bracket-newline': 'off',
    },
  },
])
