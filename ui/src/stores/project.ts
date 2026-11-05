import { inject, provide, watch, type InjectionKey } from 'vue'

import { createProjectApi, type ProjectSummary } from '../api'
import { createNavigation } from './navigation'
import { createCommitSelection } from './commitSelection'
import { createRepo } from './repo'
import { createWorkingTree } from './workingTree'

/**
 * Creates everything a tab knows about the project open in it: the commands
 * that act on it, its repository and working tree, its stack of views, and
 * the commits checked to act on together.
 */
export function createProject(summary: ProjectSummary) {
  const api = createProjectApi(summary.id)
  const workingTree = createWorkingTree(api)
  const repo = createRepo(api, workingTree.refreshWorkingTree)
  const navigation = createNavigation()
  const commitSelection = createCommitSelection(api)

  // A selection belongs to the list it was made in, which a change of view
  // leaves, e.g. through the mouse's back button.
  watch(() => navigation.navigation.stack.length, commitSelection.endSelection)

  /** Loads what the tab first shows; live updates keep it current from then on. */
  async function load(): Promise<void> {
    await repo.refreshOverview()
    void workingTree.refreshWorkingTree()
    try {
      repo.applyFetchStatus(await api.fetchStatus())
    } catch {
      // The status also comes with the next fetch event.
    }
  }

  return { summary, api, load, ...workingTree, ...repo, ...navigation, ...commitSelection }
}

export type Project = ReturnType<typeof createProject>

const PROJECT: InjectionKey<Project> = Symbol('project')

/** Makes `project` the one that every view below the calling component acts on. */
export function provideProject(project: Project): void {
  provide(PROJECT, project)
}

/** The project of the tab the calling component is in. */
export function useProject(): Project {
  const project = inject(PROJECT)
  if (!project) {
    throw new Error('useProject() is only available inside a project tab')
  }
  return project
}
