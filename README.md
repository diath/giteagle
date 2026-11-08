<p align="center">
  <img src="docs/logo.png" width="128" height="128" alt="GitEagle logo">
</p>

<h1 align="center">GitEagle</h1>

<p align="center">
  A desktop viewer for Git repositories: branches, history, diffs, blame and
  more, with each repository open in a tab of its own.
</p>

<p align="center">
  <a href="https://github.com/diath/giteagle/releases"><b>Download</b></a>
  · Windows installer and Linux AppImage · Free and open source
</p>

<table>
  <tr>
    <td align="center">
      <a href="docs/screenshots/history.webp"><img src="docs/screenshots/thumbs/history.webp" width="280" alt="The commit list, with unpushed commits and uncommitted changes at the top"></a>
      <br><b>History</b>
    </td>
    <td align="center">
      <a href="docs/screenshots/diffs.webp"><img src="docs/screenshots/thumbs/diffs.webp" width="280" alt="A commit's diff with syntax highlighting and a tree of the changed files"></a>
      <br><b>Diffs</b>
    </td>
    <td align="center">
      <a href="docs/screenshots/blame.webp"><img src="docs/screenshots/thumbs/blame.webp" width="280" alt="A file with the commit and author that last changed each line"></a>
      <br><b>Blame</b>
    </td>
  </tr>
  <tr>
    <td align="center">
      <a href="docs/screenshots/branches.webp"><img src="docs/screenshots/thumbs/branches.webp" width="280" alt="The branch switcher listing local and remote branches"></a>
      <br><b>Branches</b>
    </td>
    <td align="center">
      <a href="docs/screenshots/pick-hunks.webp"><img src="docs/screenshots/thumbs/pick-hunks.webp" width="280" alt="Committing changes hunk by hunk"></a>
      <br><b>Pick hunks</b>
    </td>
    <td align="center">
      <a href="docs/screenshots/themes.webp"><img src="docs/screenshots/thumbs/themes.webp" width="280" alt="The same diff in the four themes"></a>
      <br><b>Themes</b>
    </td>
  </tr>
</table>

# Features

- **[History](docs/screenshots/history.webp)** - Commits newest first, with
  the ones not pushed yet marked and uncommitted changes on top. Every
  repository opens in a tab of its own, and the tabs that were open come back
  the next time you start GitEagle.
- **[Diffs](docs/screenshots/diffs.webp)** - Syntax-highlighted diffs that
  mark the words that changed, with a tree of the changed files and hidden
  lines a click away.
- **[Blame](docs/screenshots/blame.webp)** - See the commit that last changed
  each line, following renames, and step back to the blame from before any
  change.
- **[Files](docs/screenshots/files.webp)** - Browse the repository with the
  last commit that changed each file, then open a file, its history or its
  blame.
- **[Search](docs/screenshots/search.webp)** - Find commits by text, or narrow
  them down with `author:`, `message:`, `file:` and `since:`.
- **[Branches](docs/screenshots/branches.webp)** - Switch branches like
  `git checkout`: changes that don't conflict carry over. Fetch, pull
  (fast-forward only) and push are one click away, and fetches also run
  quietly in the background.
- **[Pick hunks](docs/screenshots/pick-hunks.webp)** - Commit whole files, or
  go through the changes hunk by hunk as with `git add --patch`, using the Y,
  N and Q keys.
- **[Reword, squash and drop](docs/screenshots/squash.webp)** - Tidy up
  unpushed commits as `git rebase -i` would. If any commit would conflict,
  nothing changes.
- **[Cherry-pick and revert](docs/screenshots/cherry-pick.webp)** - Copy
  commits onto other local branches without checking them out, or revert
  them. If any branch conflicts, none changes.
- **[Stashes](docs/screenshots/stashes.webp)** - Read each stash as a diff,
  then apply, pop or drop it. A stash is only applied when it applies cleanly.
- **[Authors](docs/screenshots/authors.webp)** - Everyone who worked on the
  branch, with their commits, the lines they added and removed, and when they
  were last active.
- **[Themes](docs/screenshots/themes.webp)** - Dark (Black), Dark (Brown),
  Light (Beige) and Light (White), applied as soon as you choose one.

# Installing

GitEagle needs `git` on your `PATH`: fetch, pull, push and blame run through
it, so your credentials and SSH config apply. It won't start without it.

## Pre-built binaries

Every release on the [releases page](https://github.com/diath/giteagle/releases)
has a Windows installer and a Linux AppImage. macOS isn't built yet.

- **Windows:** run `GitEagle_<version>_x64-setup.exe`. The installer isn't
  code-signed, so SmartScreen may warn about it; choose *More info*, then
  *Run anyway*.
- **Linux:** make the AppImage executable and run it:

  ```sh
  chmod +x GitEagle_<version>_amd64.AppImage
  ./GitEagle_<version>_amd64.AppImage
  ```

## From source

You need:

- Rust 1.91 or newer
- Node.js 24 with npm
- The Tauri CLI: `cargo install tauri-cli --version "^2.12" --locked`
- The system libraries Tauri needs, listed in
  [Tauri's prerequisites](https://v2.tauri.app/start/prerequisites/). On
  Debian and Ubuntu:

  ```sh
  sudo apt install build-essential libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf xdg-utils
  ```

Then build it:

```sh
git clone https://github.com/diath/giteagle.git
cd giteagle/ui
npm ci
cd ..
cargo tauri build
```

The installer or AppImage ends up in `target/release/bundle/`, and the bare
executable in `target/release/`. Add `--no-bundle` to skip the installer and
build only the executable.

To work on GitEagle, `cargo tauri dev -- -- <path>` runs it against a
repository and reloads the interface as you edit it.

## With cargo install

GitEagle isn't on crates.io, and `cargo install --git` can't build it, since
the interface has to be built with npm first. From a clone, with the
prerequisites above:

```sh
cd ui
npm ci
npm run build
cd ..
cargo install --path . --locked --features tauri/custom-protocol
```

This puts `giteagle` in `~/.cargo/bin`. The `tauri/custom-protocol` feature
makes it load the interface built into it instead of looking for a
development server, as `cargo tauri build` does.

## Running

```sh
giteagle [path]
```

GitEagle reopens the repositories that were open when it last closed. Given a
path, it also opens the repository that contains it, searching upward like
`git` does. Without one, it shows the tab you had open last, or a tab that
asks for a repository.

# Missing Features

GitEagle is a viewer first, so it leaves these out for now:

- Rebasing onto another branch, and the parts of `git rebase -i` beyond
  reword, squash and drop, such as reordering or editing commits
- Merging, and a UI for resolving conflicts
- Staging: commits are made straight from the working tree instead
- Force pushing
- Submodules
- Git LFS
- Bare repositories
- Tuning for very large repositories: the commit list loads lazily, but
  nothing else is optimized for them
- macOS builds

# License

GitEagle is licensed under the [MIT License](LICENSE).
