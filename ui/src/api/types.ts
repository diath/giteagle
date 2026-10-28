// Mirrors of the structs the Rust commands return (serde camelCase). Kept in
// one file so drift from the backend is easy to spot.

export interface Person {
  name: string
  email: string
}

export interface CommitBrief {
  id: string
  shortId: string
  subject: string
  authorTime: number
}

export interface CommitSummary {
  id: string
  shortId: string
  subject: string
  body: string
  authors: Person[]
  authorTime: number
  isPushed: boolean | null
}

export interface HistoryCursor {
  head: string
  skip: number
}

export interface HistoryPage {
  head: string | null
  commits: CommitSummary[]
  next: HistoryCursor | null
  truncated: boolean
}

export type RepoState =
  | 'clean'
  | 'merge'
  | 'rebase'
  | 'cherryPick'
  | 'revert'
  | 'bisect'
  | 'applyMailbox'

export type HeadKind = 'branch' | 'detached' | 'unborn'

export interface HeadInfo {
  kind: HeadKind
  name: string | null
  tip: CommitSummary | null
  upstream: string | null
  ahead: number | null
  behind: number | null
}

export interface Overview {
  repoName: string
  workdir: string
  state: RepoState
  isShallow: boolean
  head: HeadInfo
  commitCount: number
  remotes: string[]
  hasOrigin: boolean
  branchCount: number
}

export interface BranchRow {
  name: string
  isRemote: boolean
  isCurrent: boolean
  tip: CommitBrief
  upstream: string | null
  ahead: number | null
  behind: number | null
  trackedBy: string | null
}

export interface BranchList {
  local: BranchRow[]
  remote: BranchRow[]
}

export interface RemoteBranch {
  remote: string
  branch: string
  displayName: string
}

export interface DeleteInfo {
  remoteBranch: RemoteBranch | null
  lostCommits: number
  lostCommitsWithRemote: number
}

export type EntryKind = 'directory' | 'file' | 'symlink' | 'submodule'

export interface DirectoryEntry {
  name: string
  path: string
  kind: EntryKind
}

export interface FileSearchResult {
  entries: DirectoryEntry[]
  truncated: boolean
}

export type FileStatus =
  | 'modified'
  | 'staged'
  | 'added'
  | 'deleted'
  | 'renamed'
  | 'typeChange'
  | 'conflicted'

export interface StatusEntry {
  path: string
  status: FileStatus
}

export type ChangeKind = 'added' | 'deleted' | 'modified' | 'renamed' | 'copied' | 'typeChange'

export type LineKind = 'context' | 'added' | 'removed'

export interface DiffLine {
  kind: LineKind
  oldLine: number | null
  newLine: number | null
  text: string
}

export interface Hunk {
  oldStart: number
  oldLines: number
  newStart: number
  newLines: number
  section: string
  lines: DiffLine[]
}

export interface FileDiff {
  path: string
  oldPath: string | null
  kind: ChangeKind
  oldMode: number | null
  newMode: number | null
  isBinary: boolean
  isSubmodule: boolean
  tooLarge: boolean
  additions: number
  deletions: number
  hunks: Hunk[]
  oldContent: string | null
  newContent: string | null
}

export interface DiffStats {
  filesChanged: number
  additions: number
  deletions: number
}

export interface ParentRef {
  id: string
  shortId: string
}

export interface CommitDetails {
  id: string
  shortId: string
  subject: string
  body: string
  authors: Person[]
  authorTime: number
  committer: Person
  committerTime: number
  parents: ParentRef[]
  stats: DiffStats
  files: FileDiff[]
}

export interface FetchStatus {
  inProgress: boolean
  lastSuccess: number | null
  lastError: string | null
}

export type ErrorKind =
  | 'notFound'
  | 'conflict'
  | 'invalidState'
  | 'invalidInput'
  | 'git'
  | 'io'
  | 'internal'

export interface AppError {
  kind: ErrorKind
  message: string
}
