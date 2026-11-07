# GitEagle

A desktop viewer for Git repositories: branches, history and diffs, with each
repository open in a tab of its own. It has a Tauri 2 shell, a Rust backend
using git2, and a Vue 3 + TypeScript frontend in `ui/`. It launches as
`giteagle [path]` and reopens the projects that were open when it last
closed:

- With a path, it also opens a tab for the repository found by searching
  upward from the path, and shows it. It exits with an error if there is
  none.
- Without one, it shows the tab that was shown last, or a tab that asks for
  a project when there is nothing to reopen.

Bare repositories are rejected.

## Commands

Run these from the repository root:

- `cargo tauri dev -- -- <path>` runs the app against a repository.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
  `cargo test` check the backend.
- `npm run typecheck`, `npm run lint` and `npm run build` check the frontend.
  Run them in `ui/`.

CI (`.github/workflows/pr.yml`) runs all of these on Linux and Windows, so keep
every one passing.

## Layout

- **One Cargo package at the root.** There is no workspace and no `src-tauri/`.
  `build.rs`, `tauri.conf.json`, `icons/` and `capabilities/` sit next to
  `Cargo.toml`.
- **`src/git/`** holds all repository logic over git2. Its functions take a
  `&Repository` and return serializable structs. It never touches Tauri.
- **`src/commands/`** holds thin `#[tauri::command]` wrappers. Register new
  commands in `src/lib.rs`. Commands about a repository take the id of its
  `project`, and events name the project in their payload.
- **`src/app_state.rs`** keeps the open projects by id. **`src/project.rs`**
  holds one project's repository handle, caches and background schedule.
- **`src/settings.rs`** keeps the settings, saved as JSON in the app's config
  folder. Every setting has its default there, which also fixes the kind of
  value it holds. The frontend's settings are listed there too, and
  `ui/src/stores/settings.ts` repeats the defaults.
- **`src/external_git.rs`** runs network operations (fetch, pull, push, remote
  deletes) through the system `git`, so the user's credentials and SSH config
  apply. It sets `GIT_TERMINAL_PROMPT=0` so nothing prompts. Blame runs
  through it too unless Settings choose git2, as libgit2's is many times
  slower. Everything else uses git2.
- **`src/watcher.rs` and `src/fetcher.rs`** do background work for each
  project and report to the UI through Tauri events:
  - The watcher polls every 3 seconds for outside changes and the working tree
    status, but only for the project in the active tab.
  - The fetcher fetches when a project opens and then every 5 minutes.
- **`tests/`** holds integration tests that build temporary repositories with
  `TempRepo` from `tests/common`.
- **`ui/src/api/`** holds typed `invoke` wrappers and DTO types. The DTOs mirror
  the Rust structs (serde `camelCase`) by hand, so change both sides together.
- **`ui/src/stores/`** holds the app state:
  - `tabs.ts` holds the tabs.
  - `settings.ts` holds a copy of the settings and saves changes to them.
  - `project.ts` creates the stores of each tab's project: its commands,
    repository, working tree and view stack. Components get them with
    `useProject()`.
  - `navigation.ts` holds a project's view stack. Screens switch inside the
    single window and never open new OS windows.

## Product rules

- **Out of scope:** rebasing onto another branch, merge and conflict UI,
  staging, force push, submodules, LFS and bare repositories.
- **Tabs** each hold one project. Opening a project that is already open
  shows its tab instead. Closing the last tab leaves one that asks for a
  project. Ctrl+T opens a new tab and Ctrl+W closes the shown one.
- **Open tabs are remembered** as they change. A project that can't be
  reopened at the next start is reported and forgotten.
- **The Recent menu** lists the projects opened last, latest first. One that
  fails to open is taken off the list.
- **Settings** apply and are saved as soon as they change, so the Settings
  dialog has nothing to confirm.
- **Switching branches** behaves like `git checkout`: non-conflicting changes
  carry over. If the switch would overwrite local changes or fails, nothing
  changes.
- **Applying a stash** behaves like `git stash apply`, but only when it
  applies cleanly. If anything is staged, the stash conflicts with HEAD, or it
  would overwrite local changes, nothing changes. Apply, pop, drop and
  stashing all ask for confirmation first.
- **Cherry-pick and revert** are offered on the commits of a branch with an
  upstream, pushed or not, and both ask for confirmation first. Merge
  commits are taken against their first parent.
  - Cherry-pick copies the commit onto other local branches without checking
    them out. Ctrl+Click on it starts selecting several commits instead,
    which each branch then gets oldest first. Branches that already have one
    of them or are checked out in a worktree can't be chosen. If any chosen
    branch conflicts, no branch changes.
  - Revert adds a revert commit, or reverts in the working tree and stages
    the result. If anything is staged, it conflicts, or it would overwrite
    local changes, nothing changes.
- **Reword, squash and drop** are offered only on unpushed commits after the
  last merge, and all ask for confirmation first. As in `git rebase -i`, the
  commits after the changed one are recreated on top of it, in memory, and
  only the current branch moves. If any commit conflicts or local changes
  would be overwritten, nothing changes.
  - Reword changes only the message and never touches the working tree.
  - Squash folds the chosen commits into the one it was started from, which
    keeps its place and its author. Ctrl+Click on it starts checking them
    in the commit list instead.
- **Committing** is offered on the uncommitted changes row, onto the current
  branch but never a detached HEAD. The commit holds HEAD's files with only
  the chosen changes, whatever is staged. Afterwards the index holds the
  committed versions, staged changes to other files stay staged, and the
  working tree never changes.
  - Commit takes the checked files whole, as they are in the working tree.
- **Pull** is fast-forward only.
- **Push** pushes the current branch to its upstream. Without an upstream it
  runs `git push -u origin <branch>`, and with no `origin` it is disabled.
- **Background fetches never prompt.** A failed fetch is shown as a small
  indicator, not a dialog.
- **Outside changes** to HEAD or branches show a non-blocking reload prompt.
  Working tree and stash changes update silently.
- **Blame** shows the commit that last changed each line of a file as it is
  in one commit, following renames. A commit's diff blames that commit's
  version, and uncommitted changes and stashes the one they are compared
  with. Each run of lines can step back to the blame from before its change.
  The system `git` finds it by default; Settings can switch to git2.
- **Dates** are shown in the local timezone, and lists show the author date.

## UI rules

- **Colors:**
  - Settings offers four themes: Dark (Black), the default, Dark (Brown),
    Light (Beige) and Light (White). A theme applies as soon as it is chosen.
  - Colors come from `ui/src/theme/palette.ts`, exposed as CSS variables in
    `ui/src/theme/base.css`. Each theme gives the same roles their colors, so
    every color must work in all of them.
  - `src/theme.rs` repeats each theme's app background for the window, which
    shows before the page draws.
  - The `giteagle/no-raw-colors` ESLint rule bans raw hex values in components.
  - Don't use pure colors (`#000`, `#fff`, `#f00`) or gradients. Shadows are
    barely visible at most; prefer borders.
- **Font:** IBM Plex Mono everywhere. Never use system UI fonts.
- **Sizes:**
  - Write lengths in design pixels as `calc(N * var(--px))` in CSS, or
    `scaled(N)` in scripts, so the font size setting resizes everything
    through `ui/src/theme/scale.ts`. Call `scaled()` when the length is
    used, since the setting can change while the app runs.
  - Prefer the tokens in `base.css`, such as `--space-*`, `--font-size-*` and
    `--radius`.
  - 1px hairlines and focus rings stay unscaled.
- **Buttons** look like buttons: use `.button`, with its border and
  background. Never style one as plain text or a link unless asked to.
  Small icon-only controls in rows and headers, such as a collapse chevron
  or a copy button, use `.icon-button`.
- **Icons** are inline SVG paths in `ui/src/icons/paths.ts`, drawn with
  `currentColor` through `AppIcon`.
- **Syntax highlighting** uses the Shiki theme built from the palette in
  `ui/src/theme/syntax-theme.ts`. Never use a built-in Shiki theme.
- **Polish:**
  - Never remove focus rings.
  - Use short transitions (120-180ms), and only on hover and active states.
  - Keep border radii small.
- **Vue whitespace:** Vue drops whitespace-only text at the start of an element
  and between elements separated by a line break. Use a margin such as
  `margin-left: 1ch`, or keep the text in one span.

## Code and docs

- Write plain, readable code with descriptive names and no clever tricks.
- Comments explain why or how, never what. Constants may say what they hold.
  Write for an unfamiliar reader, and never mention an agent's reasoning or
  planning.
- Don't add a README, docs or contribution guidelines; a human will write those
  later.
- `LICENSE` is MIT. The first copyright holder's email is base64-encoded on
  purpose: never decode it.
- Use plain dashes, never em dashes, in code, comments, docs and UI text.
- Versions have the form `<major>.<minor>.<build>`. Change major and minor only
  in `VERSION`. The release workflow stamps the full version into `Cargo.toml`
  and `tauri.conf.json`.

## Commits

- Keep commits small, one change each.
- **Subject:**
  - Imperative mood, sentence case, no trailing period.
  - Start with a plain verb, such as Add, Fix, Make or Move.
  - No prefixes, scopes or issue numbers.
  - Name things the way users see them, e.g. "Rework the Commit View header".
- **Body:** add one only when the subject isn't enough. It is either:
  - a short prose paragraph on why, wrapped at 72 columns, or
  - `*` bullets in the past tense ("* Fixed …"), with nested bullets indented
    by four spaces.
- Never commit local tool files such as `.claude/`.
