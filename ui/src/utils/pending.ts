import type { PendingFile, PendingSummary } from '../api'

/** The states an uncommitted file can be in; each has a `state-*` class. */
export type PendingState = 'conflicted' | 'staged' | 'unstaged' | 'untracked'

const STATES: readonly PendingState[] = ['conflicted', 'staged', 'unstaged', 'untracked']

/** The states that have files in them, with how many, in a fixed order. */
export function pendingCounts(summary: PendingSummary): { state: PendingState; count: number }[] {
  return STATES.map((state) => ({ state, count: summary[state] })).filter(({ count }) => count > 0)
}

export function hasPendingChanges(summary: PendingSummary | null): boolean {
  return summary !== null && pendingCounts(summary).length > 0
}

export function statesOf(file: PendingFile): PendingState[] {
  return STATES.filter((state) => file[state])
}
