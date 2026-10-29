// Tokenizes files for highlight.ts. Tokenizing a big file takes long enough
// to freeze the window, so it happens here instead of on the main thread.

import { createHighlighterCore, type HighlighterCore, type LanguageInput } from 'shiki/core'
import { createOnigurumaEngine } from 'shiki/engine/oniguruma'

import { SYNTAX_THEME_NAME, syntaxTheme } from '../theme/syntax-theme'
import type { Token } from './highlight'

export interface TokenizeRequest {
  id: number
  code: string
  language: string
}

export interface TokenizeResponse {
  id: number
  tokens: Token[][] | null
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

let highlighter: Promise<HighlighterCore> | null = null
const loadedLanguages = new Map<string, Promise<void>>()

function getHighlighter(): Promise<HighlighterCore> {
  // Oniguruma runs as WebAssembly. Shiki's JavaScript regex engine is far
  // slower on WebKitGTK, which the Linux build renders with: a 124 KB C++
  // file took 13 seconds there instead of 0.4.
  highlighter ??= createHighlighterCore({
    themes: [syntaxTheme],
    langs: [],
    engine: createOnigurumaEngine(import('shiki/wasm')),
  })
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

async function tokenize(code: string, language: string): Promise<Token[][] | null> {
  if (!(language in GRAMMARS)) {
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

addEventListener('message', (event: MessageEvent<TokenizeRequest>) => {
  const { id, code, language } = event.data
  void tokenize(code, language).then((tokens) => {
    const response: TokenizeResponse = { id, tokens }
    postMessage(response)
  })
})
