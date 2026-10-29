import type { TokenizeRequest, TokenizeResponse } from './highlight.worker'

/** Larger files render as plain text; tokenizing them would hold up the
 * highlighting of every file after them. */
const MAX_HIGHLIGHT_CHARACTERS = 300_000

export interface Token {
  content: string
  color?: string
  /** Shiki's FontStyle bit flags: 1 italic, 2 bold, 4 underline. */
  fontStyle?: number
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

let worker: Worker | null = null
let workerFailed = false
let nextRequestId = 0
const pendingRequests = new Map<number, (tokens: Token[][] | null) => void>()

function getWorker(): Worker {
  if (!worker) {
    worker = new Worker(new URL('./highlight.worker.ts', import.meta.url), { type: 'module' })
    worker.addEventListener('message', (event: MessageEvent<TokenizeResponse>) => {
      pendingRequests.get(event.data.id)?.(event.data.tokens)
      pendingRequests.delete(event.data.id)
    })
    // The worker catches its own errors, so this means it couldn't start.
    // Files then stay plain text instead of waiting for it forever.
    worker.addEventListener('error', () => {
      workerFailed = true
      pendingRequests.forEach((resolve) => resolve(null))
      pendingRequests.clear()
    })
  }
  return worker
}

/**
 * Tokenizes a whole file at once, so multi-line constructs such as block
 * comments are colored correctly, and returns the tokens per line. Resolves
 * to `null` when the file can't or shouldn't be highlighted.
 */
export function tokenizeFile(code: string, language: string | null): Promise<Token[][] | null> {
  if (!language || workerFailed || code.length > MAX_HIGHLIGHT_CHARACTERS) {
    return Promise.resolve(null)
  }
  const id = (nextRequestId += 1)
  return new Promise((resolve) => {
    pendingRequests.set(id, resolve)
    const request: TokenizeRequest = { id, code, language }
    getWorker().postMessage(request)
  })
}
