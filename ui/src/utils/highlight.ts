import type { HighlighterCore, LanguageInput } from 'shiki/core'

import { SYNTAX_THEME_NAME, syntaxTheme } from '../theme/syntax-theme'

/** Larger files render as plain text; tokenizing them would stall the UI. */
const MAX_HIGHLIGHT_CHARACTERS = 300_000

export interface Token {
  content: string
  color?: string
  /** Shiki's FontStyle bit flags: 1 italic, 2 bold, 4 underline. */
  fontStyle?: number
}

const GRAMMARS: Record<string, () => LanguageInput> = {
  bash: () => import('@shikijs/langs/bash'),
  c: () => import('@shikijs/langs/c'),
  cmake: () => import('@shikijs/langs/cmake'),
  cpp: () => import('@shikijs/langs/cpp'),
  csharp: () => import('@shikijs/langs/csharp'),
  css: () => import('@shikijs/langs/css'),
  dart: () => import('@shikijs/langs/dart'),
  diff: () => import('@shikijs/langs/diff'),
  docker: () => import('@shikijs/langs/docker'),
  go: () => import('@shikijs/langs/go'),
  graphql: () => import('@shikijs/langs/graphql'),
  html: () => import('@shikijs/langs/html'),
  ini: () => import('@shikijs/langs/ini'),
  java: () => import('@shikijs/langs/java'),
  javascript: () => import('@shikijs/langs/javascript'),
  json: () => import('@shikijs/langs/json'),
  jsx: () => import('@shikijs/langs/jsx'),
  kotlin: () => import('@shikijs/langs/kotlin'),
  less: () => import('@shikijs/langs/less'),
  lua: () => import('@shikijs/langs/lua'),
  make: () => import('@shikijs/langs/make'),
  markdown: () => import('@shikijs/langs/markdown'),
  php: () => import('@shikijs/langs/php'),
  powershell: () => import('@shikijs/langs/powershell'),
  python: () => import('@shikijs/langs/python'),
  ruby: () => import('@shikijs/langs/ruby'),
  rust: () => import('@shikijs/langs/rust'),
  scss: () => import('@shikijs/langs/scss'),
  sql: () => import('@shikijs/langs/sql'),
  swift: () => import('@shikijs/langs/swift'),
  toml: () => import('@shikijs/langs/toml'),
  tsx: () => import('@shikijs/langs/tsx'),
  typescript: () => import('@shikijs/langs/typescript'),
  vue: () => import('@shikijs/langs/vue'),
  xml: () => import('@shikijs/langs/xml'),
  yaml: () => import('@shikijs/langs/yaml'),
}

const LANGUAGES_BY_EXTENSION: Record<string, string> = {
  bash: 'bash',
  sh: 'bash',
  zsh: 'bash',
  c: 'c',
  h: 'c',
  cc: 'cpp',
  cpp: 'cpp',
  cxx: 'cpp',
  hh: 'cpp',
  hpp: 'cpp',
  cs: 'csharp',
  css: 'css',
  dart: 'dart',
  diff: 'diff',
  patch: 'diff',
  go: 'go',
  graphql: 'graphql',
  gql: 'graphql',
  htm: 'html',
  html: 'html',
  ini: 'ini',
  cfg: 'ini',
  java: 'java',
  js: 'javascript',
  cjs: 'javascript',
  mjs: 'javascript',
  json: 'json',
  jsx: 'jsx',
  kt: 'kotlin',
  kts: 'kotlin',
  less: 'less',
  lua: 'lua',
  md: 'markdown',
  markdown: 'markdown',
  php: 'php',
  ps1: 'powershell',
  py: 'python',
  rb: 'ruby',
  rs: 'rust',
  scss: 'scss',
  sql: 'sql',
  swift: 'swift',
  toml: 'toml',
  ts: 'typescript',
  cts: 'typescript',
  mts: 'typescript',
  tsx: 'tsx',
  vue: 'vue',
  xml: 'xml',
  svg: 'xml',
  yaml: 'yaml',
  yml: 'yaml',
}

const LANGUAGES_BY_FILE_NAME: Record<string, string> = {
  'cmakelists.txt': 'cmake',
  dockerfile: 'docker',
  makefile: 'make',
  'cargo.lock': 'toml',
  '.bashrc': 'bash',
  '.zshrc': 'bash',
  '.gitconfig': 'ini',
  '.editorconfig': 'ini',
}

export function languageFor(path: string): string | null {
  const fileName = path.slice(path.lastIndexOf('/') + 1).toLowerCase()
  const byName = LANGUAGES_BY_FILE_NAME[fileName]
  if (byName) {
    return byName
  }
  const dot = fileName.lastIndexOf('.')
  if (dot <= 0) {
    return null
  }
  return LANGUAGES_BY_EXTENSION[fileName.slice(dot + 1)] ?? null
}

let highlighter: Promise<HighlighterCore> | null = null
const loadedLanguages = new Map<string, Promise<void>>()

function getHighlighter(): Promise<HighlighterCore> {
  // Shiki is loaded on first use so it doesn't slow down startup. The
  // JavaScript regex engine avoids shipping and instantiating WASM.
  highlighter ??= Promise.all([import('shiki/core'), import('shiki/engine/javascript')]).then(
    ([{ createHighlighterCore }, { createJavaScriptRegexEngine }]) =>
      createHighlighterCore({
        themes: [syntaxTheme],
        langs: [],
        engine: createJavaScriptRegexEngine({ forgiving: true }),
      }),
  )
  return highlighter
}

function ensureLanguage(instance: HighlighterCore, language: string): Promise<void> {
  let loading = loadedLanguages.get(language)
  if (!loading) {
    loading = instance.loadLanguage(GRAMMARS[language]())
    loadedLanguages.set(language, loading)
  }
  return loading
}

/**
 * Tokenizes a whole file at once, so multi-line constructs such as block
 * comments are colored correctly, and returns the tokens per line. Resolves
 * to `null` when the file can't or shouldn't be highlighted.
 */
export async function tokenizeFile(code: string, language: string | null): Promise<Token[][] | null> {
  if (!language || !(language in GRAMMARS) || code.length > MAX_HIGHLIGHT_CHARACTERS) {
    return null
  }
  try {
    const instance = await getHighlighter()
    await ensureLanguage(instance, language)
    const { tokens } = instance.codeToTokens(code.replace(/\r\n/g, '\n'), {
      lang: language,
      theme: SYNTAX_THEME_NAME,
    })
    return tokens.map((line) =>
      line.map((token) => ({ content: token.content, color: token.color, fontStyle: token.fontStyle })),
    )
  } catch {
    return null
  }
}
