import { reactive } from 'vue'

import { errorMessage, type CommitSummary, type ProjectApi } from '../api'
import { showToast } from './toasts'

/** What the commits are checked for. */
export type SelectionPurpose = 'cherryPick' | 'squash'

/**
 * Creates a project's selection of commits to act on together, checked in
 * the commit list rather than in a dialog. While it is active, the commits
 * that can take part show a checkbox, and the controls around the list wait
 * until it ends.
 */
export function createCommitSelection(api: ProjectApi) {
  const commitSelection = reactive({
    active: false,
    purpose: 'cherryPick' as SelectionPurpose,
    /** The checked commits, in the order they were checked. */
    commits: [] as CommitSummary[],
    /** When squashing, the commit the others fold into, which stays checked. */
    target: null as CommitSummary | null,
    /** When squashing, the ids of the commits that can be folded into the target. */
    squashable: [] as string[],
  })

  function startCherryPickSelection(first: CommitSummary): void {
    Object.assign(commitSelection, {
      active: true,
      purpose: 'cherryPick',
      commits: [first],
      target: null,
      squashable: [],
    })
  }

  /** Starts checking the commits to squash into `target`, once it is known which can be. */
  async function startSquashSelection(target: CommitSummary): Promise<void> {
    let squashable: string[]
    try {
      squashable = (await api.rewritableCommits(target.id)).map((commit) => commit.id)
    } catch (error) {
      showToast('error', errorMessage(error))
      return
    }
    squashable = squashable.filter((id) => id !== target.id)
    if (squashable.length === 0) {
      showToast('info', `No other unpushed commits can be squashed into ${target.shortId}.`)
      return
    }
    Object.assign(commitSelection, { active: true, purpose: 'squash', commits: [], target, squashable })
  }

  /** Whether `commit` can be checked, or stays checked, in the current selection. */
  function canSelect(commit: CommitSummary): boolean {
    if (commitSelection.purpose === 'squash') {
      return commit.id === commitSelection.target?.id || commitSelection.squashable.includes(commit.id)
    }
    return commit.isPushed !== null
  }

  function isChecked(id: string): boolean {
    return id === commitSelection.target?.id || commitSelection.commits.some((commit) => commit.id === id)
  }

  function toggleChecked(commit: CommitSummary): void {
    if (commit.id === commitSelection.target?.id || !canSelect(commit)) {
      return
    }
    if (isChecked(commit.id)) {
      commitSelection.commits = commitSelection.commits.filter((checked) => checked.id !== commit.id)
    } else {
      commitSelection.commits.push(commit)
    }
  }

  function endSelection(): void {
    Object.assign(commitSelection, { active: false, commits: [], target: null, squashable: [] })
  }

  return {
    commitSelection,
    startCherryPickSelection,
    startSquashSelection,
    canSelect,
    isChecked,
    toggleChecked,
    endSelection,
  }
}
