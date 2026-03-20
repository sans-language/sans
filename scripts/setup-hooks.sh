#!/bin/sh
# Symlink the tracked pre-commit hook into .git/hooks.
set -e

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HOOK_SRC="$REPO_ROOT/scripts/pre-commit"
HOOK_DST="$REPO_ROOT/.git/hooks/pre-commit"

if [ -e "$HOOK_DST" ] || [ -L "$HOOK_DST" ]; then
  printf 'Removing existing .git/hooks/pre-commit\n'
  rm "$HOOK_DST"
fi

ln -s "$HOOK_SRC" "$HOOK_DST"
printf 'Installed pre-commit hook: %s -> %s\n' "$HOOK_DST" "$HOOK_SRC"
