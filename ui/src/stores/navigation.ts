import { reactive } from 'vue'

export type View =
  | { kind: 'main' }
  | { kind: 'fileHistory'; path: string }
  | { kind: 'commit'; id: string; fromFile: string | null }
  | { kind: 'pending' }

export type MainTab = 'commits' | 'files'

/**
 * Views stack inside the single window: the main view at the bottom, then
 * file history and commit views. Views below the top stay mounted, so going
 * back restores their scroll position and state.
 */
export const navigation = reactive({
  stack: [{ kind: 'main' }] as View[],
  tab: 'commits' as MainTab,
  directory: '',
})

export function openCommit(id: string, fromFile: string | null = null): void {
  navigation.stack.push({ kind: 'commit', id, fromFile })
}

export function openPendingChanges(): void {
  navigation.stack.push({ kind: 'pending' })
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
