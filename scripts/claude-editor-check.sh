#!/usr/bin/env bash
# Claude Code hook: remind about VSCode extension updates after commits
# that touch feature-related files but not the editor extension.

set -euo pipefail

cd "$(git rev-parse --show-toplevel 2>/dev/null)" || exit 0

# Get files changed in the most recent commit
changed=$(git diff HEAD~1 --name-only 2>/dev/null) || exit 0

if [ -z "$changed" ]; then
  exit 0
fi

# Check if any trigger files were touched
trigger=false
while IFS= read -r file; do
  case "$file" in
    docs/*|compiler/typeck.sans|compiler/constants.sans|compiler/ir.sans|compiler/codegen.sans|runtime/*)
      trigger=true
      break
      ;;
  esac
done <<< "$changed"

if [ "$trigger" = false ]; then
  exit 0
fi

# Check if editor files were also touched
editor_updated=false
while IFS= read -r file; do
  case "$file" in
    editors/vscode-sans/src/extension.ts|editors/vscode-sans/syntaxes/sans.tmLanguage.json)
      editor_updated=true
      break
      ;;
  esac
done <<< "$changed"

if [ "$editor_updated" = false ]; then
  echo "REMINDER: This commit touched feature files (docs/, compiler/, or runtime/) but did NOT update the VSCode extension. If adding/changing a feature, also update:"
  echo "  - editors/vscode-sans/src/extension.ts (HOVER_DATA)"
  echo "  - editors/vscode-sans/syntaxes/sans.tmLanguage.json (syntax highlighting)"
fi
