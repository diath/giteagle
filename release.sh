#!/usr/bin/env bash
# Remakes the website's screenshots, checks the code and pushes master to the
# release branch, whose workflow builds the installers and publishes the crate.

set -euo pipefail

cd "$(dirname "$0")"

if [ "$(git branch --show-current)" != "master" ]; then
    echo "Run this on the master branch." >&2
    exit 1
fi

# The screenshots and the checks use the working tree, so it has to match what
# gets released.
if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
    echo "Commit or stash your changes first." >&2
    exit 1
fi

python3 tools/screenshots/generate.py

git add docs/screenshots
# Identical screenshots leave nothing to commit, which shouldn't stop a release.
if ! git diff --cached --quiet -- docs/screenshots; then
    git commit -m "Update the website screenshots"
fi
git push origin master

cd ui
npm run typecheck
npm run lint
npm run build
cd ..

cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test

git push origin master:release
