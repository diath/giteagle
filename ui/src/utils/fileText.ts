import { computed, onMounted, ref, shallowRef } from 'vue'

import { errorMessage, type FileAtCommit, type ProjectApi } from '../api'
import { buildSegments, splitLines } from './diff'
import { languageFor, tokenizeFile, type Token } from './highlight'

/** Says why a view shows no lines of a file. */
export interface FileNotice {
  text: string
  /** The class it is shown in. */
  tone: 'error' | 'muted' | 'faint'
}

function formatSize(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} ${bytes === 1 ? 'byte' : 'bytes'}`
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

/**
 * Loads the file at `path` as it is in commit `commit` once the calling
 * component mounts, and highlights it.
 */
export function useFileAt(api: ProjectApi, commit: string, path: string) {
  const file = shallowRef<FileAtCommit | null>(null)
  const error = ref<string | null>(null)
  const tokens = shallowRef<Token[][] | null>(null)

  const lines = computed(() => splitLines(file.value?.text ?? null) ?? [])

  // Plain until the tokens arrive, so the text shows at once.
  const rows = computed(() => lines.value.map((text, index) => buildSegments(text, tokens.value?.[index], undefined)))

  const notice = computed<FileNotice | null>(() => {
    if (error.value) {
      return { text: error.value, tone: 'error' }
    }
    const loaded = file.value
    if (!loaded) {
      return { text: 'Loading the file…', tone: 'faint' }
    }
    if (loaded.isBinary) {
      return { text: `This is a binary file of ${formatSize(loaded.size)}.`, tone: 'muted' }
    }
    if (loaded.text === null) {
      return { text: `This file is too large to show (${formatSize(loaded.size)}).`, tone: 'muted' }
    }
    if (lines.value.length === 0) {
      return { text: 'This file is empty.', tone: 'muted' }
    }
    return null
  })

  onMounted(async () => {
    try {
      const loaded = await api.fileAt(commit, path)
      file.value = loaded
      if (loaded.text !== null) {
        tokens.value = await tokenizeFile(loaded.text, languageFor(path))
      }
    } catch (caught) {
      error.value = errorMessage(caught)
    }
  })

  return { lines, rows, notice }
}
