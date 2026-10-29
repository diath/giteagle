# GitEagle

A desktop viewer for one Git repository: branches, history and diffs. It has a
Tauri 2 shell, a Rust backend using git2, and a Vue 3 + TypeScript frontend in
`ui/`. It launches as `giteagle [path]` and finds the repository by searching
upward from the path; bare repositories are rejected at launch.

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
  commands in `src/lib.rs`.
- **`src/external_git.rs`** runs network operations (fetch, pull, push, remote
  deletes) through the system `git`, so the user's credentials and SSH config
  apply. It sets `GIT_TERMINAL_PROMPT=0` so nothing prompts. Everything else
  uses git2.
- **`src/watcher.rs` and `src/fetcher.rs`** do background work and report to
  the UI through Tauri events:
  - The watcher polls every 3 seconds for outside changes and the working tree
    status.
  - The fetcher fetches at startup and then every 5 minutes.
- **`tests/`** holds integration tests that build temporary repositories with
  `TempRepo` from `tests/common`.
- **`ui/src/api/`** holds typed `invoke` wrappers and DTO types. The DTOs mirror
  the Rust structs (serde `camelCase`) by hand, so change both sides together.
- **`ui/src/stores/navigation.ts`** holds the view stack. Screens switch inside
  the single window and never open new OS windows.

## Product rules

- **Out of scope:** rebase, blame, stash, merge and conflict UI, staging and
  committing, force push, submodules, LFS, bare repositories, multiple
  repositories in one window, and a light theme.
- **Switching branches** behaves like `git checkout`: non-conflicting changes
  carry over. If the switch would overwrite local changes or fails, nothing
  changes.
- **Pull** is fast-forward only.
- **Push** pushes the current branch to its upstream. Without an upstream it
  runs `git push -u origin <branch>`, and with no `origin` it is disabled.
- **Background fetches never prompt.** A failed fetch is shown as a small
  indicator, not a dialog.
- **Outside changes** to HEAD or branches show a non-blocking reload prompt.
  Working tree changes update silently.
- **Dates** are shown in the local timezone, and lists show the author date.

## UI rules

- **Colors:**
  - The theme is dark only.
  - Colors come from `ui/src/theme/palette.ts`, exposed as CSS variables in
    `ui/src/theme/base.css`.
  - The `giteagle/no-raw-colors` ESLint rule bans raw hex values in components.
  - Don't use pure colors (`#000`, `#fff`, `#f00`) or gradients. Shadows are
    barely visible at most; prefer borders.
- **Font:** IBM Plex Mono everywhere. Never use system UI fonts.
- **Sizes:**
  - Write lengths in design pixels as `calc(N * var(--px))` in CSS, or
    `scaled(N)` in scripts, so `UI_SCALE` in `ui/src/theme/scale.ts` resizes
    everything.
  - Prefer the tokens in `base.css`, such as `--space-*`, `--font-size-*` and
    `--radius`.
  - 1px hairlines and focus rings stay unscaled.
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
