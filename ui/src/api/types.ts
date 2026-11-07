// Mirrors of the structs the Rust commands return (serde camelCase). Kept in
// one file so drift from the backend is easy to spot.

/** Names an open project for as long as the app runs. */
export type ProjectId = number

export interface ProjectSummary {
  id: ProjectId
  name: string
  workdir: string
}

/** How large the interface is drawn. */
export type FontSize = 'small' | 'normal' | 'large' | 'extraLarge'

/** The colors the interface is drawn in. */
export type ThemeName = 'darkBlack' | 'darkBrown' | 'lightBeige' | 'lightWhite'

/** What finds who last changed each line: the system `git`, which is far faster, or libgit2. */
export type BlameBackend = 'systemGit' | 'git2'

/** Mirrors the settings and their defaults in `src/settings.rs`. */
export interface Settings {
  theme: ThemeName
  fontSize: FontSize
  /** How many projects the Recent menu remembers. */
  recentLimit: number
  /** The folders of recently opened projects, the latest first. */
  recentProjects: string[]
  /** The folders of the projects open in tabs, in tab order, to reopen at the next start. */
  openProjects: string[]
  /** The folder of the project in the shown tab, or empty for a tab without one. */
  activeProject: string
  blameBackend: BlameBackend
}

/** The tabs to show when the window opens. */
export interface Session {
  projects: ProjectSummary[]
  /** The project whose tab is shown first. */
  active: ProjectId | null
  /** The projects that were open last time but couldn't be reopened. */
  failures: { folder: string; message: string }[]
}

/** A file as it is in one commit. */
export interface FileAtCommit {
  /** In bytes. */
  size: number
  isBinary: boolean
  /** `null` for a binary file or one too large to show. */
  text: string | null
}

/** The versions of a file around the one in a commit. */
export interface FileVersions {
  /** The closest older commit with a version of the file. */
  older: CommitBrief | null
  /** The closest newer one, only looked for on the current branch. */
  newer: CommitBrief | null
  /** Whether the commit is on the current branch. */
  onBranch: boolean
}

export interface Person {
  name: string
  email: string
}

/** A commit that last changed some lines of a blamed file. */
export interface BlameCommit {
  id: string
  shortId: string
  subject: string
  author: Person
  authorTime: number
}

/** The version of a file that a commit changed some lines of, to blame them as they were before. */
export interface EarlierVersion {
  /** The commit's first parent. */
  id: string
  shortId: string
  /** The file's path there, which differs when the commit renamed it. */
  path: string
  /** The line there that the first changed line replaced, or the one it was inserted after. */
  line: number
}

/** Lines in a row that the same commit last changed. */
export interface BlameHunk {
  /** Counting from 1. */
  startLine: number
  lineCount: number
  /** A key of `FileBlame.commits`. */
  commitId: string
  /** `null` when the file didn't exist before that commit. */
  earlier: EarlierVersion | null
}

export interface FileBlame {
  /** In line order, covering every line. */
  hunks: BlameHunk[]
  commits: Record<string, BlameCommit>
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

/** A `tag:value` filter of the commit search, as listed by the backend. */
export interface SearchTag {
  name: string
  description: string
  example: string
}

export interface LineCounts {
  additions: number
  deletions: number
}

export interface AuthorStats {
  person: Person
  /** Newest first, so starting with `person.email`. */
  emails: string[]
  commits: number
  lastCommitTime: number
  /** Null until lines are counted. */
  lines: LineCounts | null
}

export interface AuthorList {
  head: string | null
  authors: AuthorStats[]
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
  /** From the upstream of the current branch, or else of `master` or `main`. */
  github: GitHubRepository | null
}

export interface GitHubRepository {
  owner: string
  name: string
}

export type GitHubPage = 'issues' | 'pullRequests'

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

export interface PickTarget {
  name: string
  isCurrent: boolean
  /** The commit is already in the branch's history. */
  hasCommit: boolean
  inOtherWorktree: boolean
}

/**
 * `newCommit` adds a commit that undoes the changes, and `workingTree` undoes
 * them in the working tree and stages them.
 */
export type RevertMode = 'newCommit' | 'workingTree'

export interface RewritableCommit {
  id: string
  shortId: string
  subject: string
  /** The full message, for editing. */
  message: string
  authorTime: number
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
  /**
   * Set for an uncommitted text file changed in place, whose hunks can be
   * committed one at a time. It changes with any change to the hunks.
   */
  fingerprint: string | null
}

export interface DiffStats {
  filesChanged: number
  additions: number
  deletions: number
}

/** A partly staged file counts as both staged and unstaged. */
export interface PendingSummary {
  staged: number
  unstaged: number
  untracked: number
  conflicted: number
  additions: number
  deletions: number
}

export interface PendingFile {
  path: string
  staged: boolean
  unstaged: boolean
  untracked: boolean
  conflicted: boolean
}

export interface PendingChanges {
  summary: PendingSummary
  states: PendingFile[]
  stats: DiffStats
  files: FileDiff[]
}

/** Hunks picked from an uncommitted file, to commit them without the rest. */
export interface HunkPick {
  /** The `fingerprint` of the file's diff they were picked from. */
  fingerprint: string
  /** Positions in that diff's hunks. */
  indices: number[]
}

/** An uncommitted file to commit, whole or in part. */
export interface FilePick {
  path: string
  /** Where a renamed file was, which the commit removes. */
  oldPath: string | null
  /** `null` commits the whole file. */
  hunks: HunkPick | null
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

export interface StashSummary {
  /** The position in the stash, as in `stash@{0}`; the newest is 0. */
  index: number
  id: string
  shortId: string
  /** Null for a stash Git named "WIP on …" after its base commit. */
  message: string | null
  /** Null for a stash made on a detached HEAD. */
  branch: string | null
  /** The commit it was stashed on. */
  base: CommitBrief
  author: Person
  authorTime: number
  /** The stashed files in each state, and the lines they change in `base`. */
  changes: PendingSummary
}

export interface StashDetails extends StashSummary {
  states: PendingFile[]
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
