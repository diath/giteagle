import { reactive } from 'vue'

import type { CommitSummary } from '../api'

/**
 * Creates a project's selection of commits to cherry-pick together. While it
 * is active, commit lists show a checkbox on every commit that can be
 * cherry-picked, and the controls around them wait until it ends.
 */
export function createPickSelection() {
  const pickSelection = reactive({
    active: false,
    /** The checked commits, in the order they were checked. */
    commits: [] as CommitSummary[],
  })

  function startPickSelection(first: CommitSummary): void {
    pickSelection.active = true
    pickSelection.commits = [first]
  }

  function isPicked(id: string): boolean {
    return pickSelection.commits.some((commit) => commit.id === id)
  }

  function togglePicked(commit: CommitSummary): void {
    if (isPicked(commit.id)) {
      pickSelection.commits = pickSelection.commits.filter((picked) => picked.id !== commit.id)
    } else {
      pickSelection.commits.push(commit)
    }
  }

  function endPickSelection(): void {
    pickSelection.active = false
    pickSelection.commits = []
  }

  return { pickSelection, startPickSelection, isPicked, togglePicked, endPickSelection }
}
