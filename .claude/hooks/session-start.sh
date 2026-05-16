#!/bin/bash
# SessionStart hook for emoji-niwa (Claude Code on the web).
#
# This project is a single self-contained index.html with ZERO dependencies
# and no build step, so there is nothing to install. Instead this hook just
# confirms the runtime is present and runs a fast JS syntax smoke check so a
# fresh web session immediately knows whether the codebase is healthy.
#
# Synchronous, web-only, idempotent, non-interactive. Always exits 0 so a
# pre-existing syntax issue never blocks the session — it is reported, and
# Claude can then fix it.
set -euo pipefail

# Only run in the remote (Claude Code on the web) environment.
if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"

echo "── emoji-niwa: session preparation ──────────────────────────"

if command -v node >/dev/null 2>&1; then
  echo "node    : $(node --version)"
else
  echo "node    : NOT FOUND (syntax check will be skipped)"
fi

if command -v python3 >/dev/null 2>&1; then
  echo "python3 : $(python3 --version 2>&1)"
else
  echo "python3 : NOT FOUND (needed for: python3 -m http.server 8000)"
fi

echo "dependencies : none (single-file, zero-dependency, no build step)"

if command -v node >/dev/null 2>&1; then
  echo "── JS syntax check (index.html) ─────────────────────────────"
  if node "$PROJECT_DIR/.claude/scripts/check-syntax.mjs"; then
    :
  else
    echo "‼️  index.html has a JS syntax error — fix it before continuing."
  fi
fi

cat <<'EOF'
── ready ────────────────────────────────────────────────────
Preview : python3 -m http.server 8000   → http://localhost:8000
Verify  : node .claude/scripts/check-syntax.mjs   (run after editing index.html)
Context : see CLAUDE.md (single-file / zero-dependency / JA-EN sync rules)
─────────────────────────────────────────────────────────────
EOF

exit 0
