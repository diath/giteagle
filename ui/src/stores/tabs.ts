import { computed, markRaw, reactive, type Raw } from 'vue'

import { appApi, errorMessage, type ProjectId, type ProjectSummary } from '../api'
import { createProject, type Project } from './project'
import { showToast } from './toasts'

export interface Tab {
  /** Unique for the app's lifetime, so a tab keeps its page while others open and close. */
  key: number
  /** Null while the tab asks for a project to open. */
  project: Raw<Project> | null
}

/** The tabs across the top of the window, one project each. */
export const tabs = reactive({
  list: [] as Tab[],
  activeKey: 0,
})

let lastKey = 0

// Projects are kept raw: they hold refs of their own, which a reactive
// wrapper would unwrap.
function newTab(summary: ProjectSummary | null): Tab {
  lastKey += 1
  return { key: lastKey, project: summary && markRaw(createProject(summary)) }
}

function indexOf(key: number): number {
  return tabs.list.findIndex((tab) => tab.key === key)
}

export const activeTab = computed(() => tabs.list.find((tab) => tab.key === tabs.activeKey) ?? null)

export function activateTab(key: number): void {
  tabs.activeKey = key
}

/** Adds a tab that asks for a project to open, and shows it. */
export function openNewTab(): void {
  const tab = newTab(null)
  tabs.list.push(tab)
  activateTab(tab.key)
}

export function projectById(id: ProjectId): Raw<Project> | null {
  return tabOf(id)?.project ?? null
}

/**
 * Starts with a tab for each project the backend already has open, such as
 * the one named on the command line, or with one that asks for a project.
 */
export async function restoreTabs(): Promise<void> {
  let projects: ProjectSummary[] = []
  try {
    projects = await appApi.projects()
  } catch (error) {
    showToast('error', `Couldn't list the open projects: ${errorMessage(error)}`)
  }
  tabs.list.push(...projects.map(newTab))
  if (tabs.list.length === 0) {
    tabs.list.push(newTab(null))
  }
  activateTab(tabs.list[0].key)
}

/**
 * Opens the project at `path` in tab `key`, which asked for one. A project
 * that is already open is shown in its own tab instead, and this one closes.
 * Rejects when the folder holds no repository GitEagle can open.
 */
export async function openProjectInTab(key: number, path: string): Promise<void> {
  const summary = await appApi.openProject(path)
  const index = indexOf(key)
  const existing = tabOf(summary.id)
  if (existing) {
    if (index !== -1) {
      tabs.list.splice(index, 1)
    }
    activateTab(existing.key)
    showToast('info', `${summary.name} is already open.`)
    return
  }
  if (index === -1) {
    // The tab was closed while the project opened.
    closeInBackend(summary.id)
    return
  }
  tabs.list.splice(index, 1, { key, project: markRaw(createProject(summary)) })
}

/**
 * Opens the project at `path` in the shown tab if that one asks for a
 * project, or else in a new tab. A project that is already open is shown in
 * its own tab instead. Rejects when the folder holds no repository GitEagle
 * can open.
 */
export async function openProject(path: string): Promise<void> {
  const summary = await appApi.openProject(path)
  const existing = tabOf(summary.id)
  if (existing) {
    activateTab(existing.key)
    return
  }
  const shown = activeTab.value
  if (shown && shown.project === null) {
    tabs.list.splice(indexOf(shown.key), 1, { key: shown.key, project: markRaw(createProject(summary)) })
    return
  }
  const tab = newTab(summary)
  tabs.list.push(tab)
  activateTab(tab.key)
}

function tabOf(id: ProjectId): Tab | null {
  return tabs.list.find((tab) => tab.project?.summary.id === id) ?? null
}

/** The only tab, when it asks for a project, has nothing to close into. */
export function canCloseTab(tab: Tab): boolean {
  return tabs.list.length > 1 || tab.project !== null
}

export function closeActiveTab(): void {
  const tab = activeTab.value
  if (tab && canCloseTab(tab)) {
    closeTab(tab.key)
  }
}

/**
 * Closes tab `key` and its project. The tab to its right is shown next, or
 * the one to its left for the last tab, and closing the only tab leaves one
 * that asks for a project.
 */
export function closeTab(key: number): void {
  const index = indexOf(key)
  if (index === -1) {
    return
  }
  const [closed] = tabs.list.splice(index, 1)
  if (closed.project) {
    closeInBackend(closed.project.summary.id)
  }
  if (tabs.list.length === 0) {
    tabs.list.push(newTab(null))
  }
  if (tabs.activeKey === key) {
    activateTab(tabs.list[Math.min(index, tabs.list.length - 1)].key)
  }
}

function closeInBackend(id: ProjectId): void {
  appApi.closeProject(id).catch((error: unknown) => {
    showToast('error', `Couldn't close the project: ${errorMessage(error)}`)
  })
}
