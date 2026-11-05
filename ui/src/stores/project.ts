import { inject, provide, type InjectionKey } from 'vue'

import { createProjectApi, type ProjectSummary } from '../api'
import { createNavigation } from './navigation'
import { createRepo } from './repo'
import { createWorkingTree } from './workingTree'

/**
 * Creates everything a tab knows about the project open in it: the commands
 * that act on it, its repository and working tree, and its stack of views.
 */
export function createProject(summary: ProjectSummary) {
  const api = createProjectApi(summary.id)
  const workingTree = createWorkingTree(api)
  const repo = createRepo(api, workingTree.refreshWorkingTree)
  const navigation = createNavigation()

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

  return { summary, api, load, ...workingTree, ...repo, ...navigation }
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
