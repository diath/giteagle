import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import type {
  AppError,
  AuthorList,
  BranchList,
  CommitBrief,
  CommitDetails,
  DeleteInfo,
  DirectoryEntry,
  FetchStatus,
  FileAtCommit,
  FileSearchResult,
  GitHubPage,
  HistoryCursor,
  HistoryPage,
  Overview,
  PendingChanges,
  PendingSummary,
  PickTarget,
  ProjectId,
  ProjectSummary,
  RevertMode,
  RewritableCommit,
  SearchTag,
  Session,
  Settings,
  StashDetails,
  StashSummary,
  StatusEntry,
} from './types'

/** Commands about the app rather than one project. */
export const appApi = {
  /**
   * Reopens the projects open when the app last closed, and lists them with
   * the ones open already, e.g. the one named on the command line.
   */
  restoreSession: () => invoke<Session>('restore_session'),
  /** Resolves to the project already open for the folder, if there is one. */
  openProject: (path: string) => invoke<ProjectSummary>('open_project', { path }),
  closeProject: (project: ProjectId) => invoke<void>('close_project', { project }),
  /** Tells the backend which tab is shown; `null` for one without a project. */
  showProject: (project: ProjectId | null) => invoke<void>('show_project', { project }),
  /** Resolves to `null` when the folder picker is cancelled. */
  pickFolder: (start: string | null) => invoke<string | null>('pick_folder', { start }),
  searchTags: () => invoke<SearchTag[]>('list_search_tags'),
  avatar: (email: string) => invoke<string | null>('get_avatar', { email }),
}

/** The settings, which the backend stores. */
export const settingsApi = {
  all: () => invoke<Settings>('get_settings'),
  /** Saves the settings in `changes`; the backend doesn't announce these back. */
  set: (changes: Partial<Settings>) => invoke<void>('set_settings', { changes }),
}

/** The commands about project `project`, which each call names. */
export function createProjectApi(project: ProjectId) {
  const call = <T>(command: string, args: Record<string, unknown> = {}): Promise<T> =>
    invoke<T>(command, { ...args, project })
  return {
    overview: () => call<Overview>('get_overview'),
    reload: () => call<void>('reload'),
    fetchStatus: () => call<FetchStatus>('get_fetch_status'),
    fetchNow: () => call<void>('fetch_now'),
    openWorkdir: () => call<void>('open_workdir'),
    openGitHubPage: (page: GitHubPage) => call<void>('open_github_page', { page }),
    history: (cursor: HistoryCursor | null, query: string) =>
      call<HistoryPage>('load_history', { cursor, query }),
    fileHistory: (path: string, cursor: HistoryCursor | null) =>
      call<HistoryPage>('load_file_history', { path, cursor }),
    authors: () => call<AuthorList>('list_authors'),
    authorLines: (head: string) => call<AuthorList>('count_author_lines', { head }),
    commit: (id: string) => call<CommitDetails>('get_commit', { id }),
    listDirectory: (path: string) => call<DirectoryEntry[]>('list_directory', { path }),
    lastCommits: (path: string) =>
      call<Record<string, CommitBrief>>('directory_last_commits', { path }),
    fileAt: (id: string, path: string) => call<FileAtCommit>('get_file_at', { id, path }),
    searchFiles: (path: string, query: string) =>
      call<FileSearchResult>('search_files', { path, query }),
    workingTreeStatus: () => call<StatusEntry[]>('get_working_tree_status'),
    pendingSummary: () => call<PendingSummary>('get_pending_summary'),
    pendingChanges: () => call<PendingChanges>('get_pending_changes'),
    branches: () => call<BranchList>('list_branches'),
    switchBranch: (name: string, isRemote: boolean) =>
      call<string>('switch_branch', { name, isRemote }),
    deleteInfo: (name: string) => call<DeleteInfo>('get_delete_info', { name }),
    deleteBranch: (name: string, deleteRemote: boolean) =>
      call<void>('delete_branch', { name, deleteRemote }),
    pickTargets: (id: string) => call<PickTarget[]>('list_pick_targets', { id }),
    cherryPick: (id: string, branches: string[]) => call<void>('cherry_pick', { id, branches }),
    revertCommit: (id: string, mode: RevertMode) => call<void>('revert_commit', { id, mode }),
    /** The commits that can be reworded, squashed or dropped together with `id`, newest first. */
    rewritableCommits: (id: string) => call<RewritableCommit[]>('list_rewritable_commits', { id }),
    rewordCommit: (id: string, message: string) => call<void>('reword_commit', { id, message }),
    squashCommits: (id: string, others: string[], message: string) =>
      call<void>('squash_commits', { id, others, message }),
    dropCommit: (id: string) => call<void>('drop_commit', { id }),
    stashes: () => call<StashSummary[]>('list_stashes'),
    stash: (id: string) => call<StashDetails>('get_stash', { id }),
    saveStash: (message: string, includeUntracked: boolean) =>
      call<void>('save_stash', { message, includeUntracked }),
    /** Resolves to the name the stash had, e.g. `stash@{1}`; likewise below. */
    applyStash: (id: string) => call<string>('apply_stash', { id }),
    popStash: (id: string) => call<string>('pop_stash', { id }),
    dropStash: (id: string) => call<string>('drop_stash', { id }),
    pull: () => call<string>('pull'),
    push: () => call<string>('push'),
  }
}

export type ProjectApi = ReturnType<typeof createProjectApi>

/** The payload of every event, which names the project it is about. */
interface ProjectEvent<T> {
  project: ProjectId
  payload: T
}

function listenForProjects<T>(
  event: string,
  handler: (project: ProjectId, payload: T) => void,
): Promise<UnlistenFn> {
  return listen<ProjectEvent<T>>(event, ({ payload }) => handler(payload.project, payload.payload))
}

export const events = {
  onRepoChanged: (handler: (project: ProjectId) => void): Promise<UnlistenFn> =>
    listenForProjects<null>('repo-changed', (project) => handler(project)),
  onWorkingTreeChanged: (handler: (project: ProjectId, entries: StatusEntry[]) => void): Promise<UnlistenFn> =>
    listenForProjects('working-tree-changed', handler),
  onPendingChanged: (handler: (project: ProjectId, summary: PendingSummary) => void): Promise<UnlistenFn> =>
    listenForProjects('pending-changed', handler),
  onStashesChanged: (handler: (project: ProjectId) => void): Promise<UnlistenFn> =>
    listenForProjects<null>('stashes-changed', (project) => handler(project)),
  onFetchStatus: (handler: (project: ProjectId, status: FetchStatus) => void): Promise<UnlistenFn> =>
    listenForProjects('fetch-status', handler),
  /** Settings the backend changed, e.g. the recent projects when one opens. */
  onSettingsChanged: (handler: (changes: Partial<Settings>) => void): Promise<UnlistenFn> =>
    listen<Partial<Settings>>('settings-changed', ({ payload }) => handler(payload)),
}

function isAppError(error: unknown): error is AppError {
  return typeof error === 'object' && error !== null && 'message' in error && 'kind' in error
}

/** Commands reject with an `AppError`; anything else is shown as-is. */
export function errorMessage(error: unknown): string {
  if (isAppError(error)) {
    return error.message
  }
  if (error instanceof Error) {
    return error.message
  }
  return String(error)
}

export type * from './types'
