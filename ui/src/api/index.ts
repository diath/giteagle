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
  FileSearchResult,
  GitHubPage,
  HistoryCursor,
  HistoryPage,
  Overview,
  PendingChanges,
  PendingSummary,
  PickTarget,
  RevertMode,
  RewritableCommit,
  SearchTag,
  StashDetails,
  StashSummary,
  StatusEntry,
} from './types'

export const api = {
  overview: () => invoke<Overview>('get_overview'),
  reload: () => invoke<void>('reload'),
  fetchStatus: () => invoke<FetchStatus>('get_fetch_status'),
  fetchNow: () => invoke<void>('fetch_now'),
  openWorkdir: () => invoke<void>('open_workdir'),
  openGitHubPage: (page: GitHubPage) => invoke<void>('open_github_page', { page }),
  history: (cursor: HistoryCursor | null, query: string) =>
    invoke<HistoryPage>('load_history', { cursor, query }),
  searchTags: () => invoke<SearchTag[]>('list_search_tags'),
  fileHistory: (path: string, cursor: HistoryCursor | null) =>
    invoke<HistoryPage>('load_file_history', { path, cursor }),
  authors: () => invoke<AuthorList>('list_authors'),
  authorLines: (head: string) => invoke<AuthorList>('count_author_lines', { head }),
  commit: (id: string) => invoke<CommitDetails>('get_commit', { id }),
  listDirectory: (path: string) => invoke<DirectoryEntry[]>('list_directory', { path }),
  lastCommits: (path: string) =>
    invoke<Record<string, CommitBrief>>('directory_last_commits', { path }),
  searchFiles: (path: string, query: string) =>
    invoke<FileSearchResult>('search_files', { path, query }),
  workingTreeStatus: () => invoke<StatusEntry[]>('get_working_tree_status'),
  pendingSummary: () => invoke<PendingSummary>('get_pending_summary'),
  pendingChanges: () => invoke<PendingChanges>('get_pending_changes'),
  branches: () => invoke<BranchList>('list_branches'),
  switchBranch: (name: string, isRemote: boolean) =>
    invoke<string>('switch_branch', { name, isRemote }),
  deleteInfo: (name: string) => invoke<DeleteInfo>('get_delete_info', { name }),
  deleteBranch: (name: string, deleteRemote: boolean) =>
    invoke<void>('delete_branch', { name, deleteRemote }),
  pickTargets: (id: string) => invoke<PickTarget[]>('list_pick_targets', { id }),
  cherryPick: (id: string, branches: string[]) => invoke<void>('cherry_pick', { id, branches }),
  revertCommit: (id: string, mode: RevertMode) => invoke<void>('revert_commit', { id, mode }),
  /** The commits that can be reworded, squashed or dropped together with `id`, newest first. */
  rewritableCommits: (id: string) => invoke<RewritableCommit[]>('list_rewritable_commits', { id }),
  rewordCommit: (id: string, message: string) => invoke<void>('reword_commit', { id, message }),
  squashCommits: (id: string, others: string[], message: string) =>
    invoke<void>('squash_commits', { id, others, message }),
  dropCommit: (id: string) => invoke<void>('drop_commit', { id }),
  stashes: () => invoke<StashSummary[]>('list_stashes'),
  stash: (id: string) => invoke<StashDetails>('get_stash', { id }),
  saveStash: (message: string, includeUntracked: boolean) =>
    invoke<void>('save_stash', { message, includeUntracked }),
  /** Resolves to the name the stash had, e.g. `stash@{1}`; likewise below. */
  applyStash: (id: string) => invoke<string>('apply_stash', { id }),
  popStash: (id: string) => invoke<string>('pop_stash', { id }),
  dropStash: (id: string) => invoke<string>('drop_stash', { id }),
  pull: () => invoke<string>('pull'),
  push: () => invoke<string>('push'),
  avatar: (email: string) => invoke<string | null>('get_avatar', { email }),
}

export const events = {
  onRepoChanged: (handler: () => void): Promise<UnlistenFn> =>
    listen('repo-changed', () => handler()),
  onWorkingTreeChanged: (handler: (entries: StatusEntry[]) => void): Promise<UnlistenFn> =>
    listen<StatusEntry[]>('working-tree-changed', (event) => handler(event.payload)),
  onPendingChanged: (handler: (summary: PendingSummary) => void): Promise<UnlistenFn> =>
    listen<PendingSummary>('pending-changed', (event) => handler(event.payload)),
  onStashesChanged: (handler: () => void): Promise<UnlistenFn> =>
    listen('stashes-changed', () => handler()),
  onFetchStatus: (handler: (status: FetchStatus) => void): Promise<UnlistenFn> =>
    listen<FetchStatus>('fetch-status', (event) => handler(event.payload)),
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
