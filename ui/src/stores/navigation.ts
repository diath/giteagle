import { markRaw, reactive } from 'vue'

import type { CommitSummary } from '../api'

/** A list of commits, newest first, that a Commit View can step through. */
export interface CommitSequence {
  /** The commits loaded so far. */
  readonly commits: readonly CommitSummary[]
  /** Whether older commits are still to be loaded. */
  readonly hasMore: boolean
  /** Loads the next page of older commits. */
  loadMore(): Promise<void>
}

export type View =
  | { kind: 'main' }
  | { kind: 'fileHistory'; path: string }
  | { kind: 'commit'; id: string; fromFile: string | null; sequence: CommitSequence | null }
  | { kind: 'pending' }
  | { kind: 'stash'; id: string }

export type MainTab = 'commits' | 'files' | 'stash' | 'authors'

/**
 * Views stack inside the single window: the main view at the bottom, then
 * file history, commit and stash views. Views below the top stay mounted, so going
 * back restores their scroll position and state.
 */
export const navigation = reactive({
  stack: [{ kind: 'main' }] as View[],
  tab: 'commits' as MainTab,
  directory: '',
  /** A search started outside the Commits tab, which runs and clears it. */
  commitSearch: null as string | null,
})

/** Shows the Commits tab with `query` in its search field, already run. */
export function searchCommits(query: string): void {
  navigation.commitSearch = query
  navigation.tab = 'commits'
}

export interface OpenCommitOptions {
  /** The file whose history the commit was opened from. */
  fromFile?: string | null
  /** The list the commit was opened from, to step to its neighbors. */
  sequence?: CommitSequence | null
}

export function openCommit(id: string, { fromFile = null, sequence = null }: OpenCommitOptions = {}): void {
  // Kept raw, as the list it reads from is reactive on its own.
  navigation.stack.push({ kind: 'commit', id, fromFile, sequence: sequence && markRaw(sequence) })
}

/**
 * Shows commit `to` in the Commit View on top while it still shows `from`,
 * e.g. to step through its list, so going back still returns to the list.
 */
export function replaceCommit(from: string, to: string): void {
  const top = navigation.stack.length - 1
  const view = navigation.stack[top]
  if (view.kind === 'commit' && view.id === from) {
    navigation.stack.splice(top, 1, { ...view, id: to })
  }
}

export function openPendingChanges(): void {
  navigation.stack.push({ kind: 'pending' })
}

export function openStash(id: string): void {
  navigation.stack.push({ kind: 'stash', id })
}

export function openFileHistory(path: string): void {
  navigation.stack.push({ kind: 'fileHistory', path })
}

export function goBack(): void {
  if (navigation.stack.length > 1) {
    navigation.stack.pop()
  }
}

/** Drops every view above the main one, e.g. after a branch switch. */
export function returnToMain(): void {
  navigation.stack.splice(1)
}
