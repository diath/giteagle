import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import type {
  AppError,
  BranchList,
  CommitBrief,
  CommitDetails,
  DeleteInfo,
  DirectoryEntry,
  FetchStatus,
  FileSearchResult,
  HistoryCursor,
  HistoryPage,
  Overview,
  PendingChanges,
  PendingSummary,
  StatusEntry,
} from './types'

export const api = {
  overview: () => invoke<Overview>('get_overview'),
  reload: () => invoke<void>('reload'),
  fetchStatus: () => invoke<FetchStatus>('get_fetch_status'),
  fetchNow: () => invoke<void>('fetch_now'),
  history: (cursor: HistoryCursor | null, query: string) =>
    invoke<HistoryPage>('load_history', { cursor, query }),
  fileHistory: (path: string, cursor: HistoryCursor | null) =>
    invoke<HistoryPage>('load_file_history', { path, cursor }),
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
