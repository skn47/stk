#!/usr/bin/env bash
# stk-rewrite-hook-version: 1
# Auto-rewrite hook for Claude Code PreToolUse:Bash.
# Transparently rewrites raw cargo/git commands to their stk-compressed equivalent.
# Uses `stk rewrite` as the single source of truth -- no mapping logic duplicated here.
#
# Exit code protocol for `stk rewrite`:
#   0 + stdout   Rewrite found -> auto-allow the rewritten command
#   1            No rewrite (not cargo/git, already stk, or an unsafe shape) -> pass through unchanged

_stk_audit_log() {
  if [ "${STK_HOOK_AUDIT:-0}" != "1" ]; then return; fi
  # Escape any literal "|" in the fields so the pipe-delimited format stays unambiguous.
  local original="${1//|/\\|}" rewritten="${2//|/\\|}"
  local dir="${STK_AUDIT_DIR:-${HOME}/.local/share/stk}"
  mkdir -p "$dir"
  printf '%s | %s | %s\n' \
    "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$original" "$rewritten" \
    >> "${dir}/hook-audit.log"
}

# Hook subprocesses may run with a minimal PATH that omits ~/.cargo/bin, where `cargo
# install --path .` puts stk -- widen it here rather than hardcoding a single absolute path.
export PATH="${HOME}/.cargo/bin:${PATH}"

# Guards: skip silently if dependencies missing.
if ! command -v stk &>/dev/null || ! command -v jq &>/dev/null; then
  exit 0
fi

set -euo pipefail

INPUT=$(cat)
# `|| exit 0` (not bare `set -e`) so malformed stdin JSON degrades to "pass through
# unchanged" rather than killing the hook with an uncontrolled nonzero exit.
CMD=$(echo "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null) || exit 0

if [ -z "$CMD" ]; then
  exit 0
fi

REWRITTEN=""
if REWRITTEN=$(stk rewrite "$CMD" 2>/dev/null); then
  _stk_audit_log "$CMD" "$REWRITTEN"

  ORIGINAL_INPUT=$(echo "$INPUT" | jq -c '.tool_input')
  UPDATED_INPUT=$(echo "$ORIGINAL_INPUT" | jq --arg cmd "$REWRITTEN" '.command = $cmd')

  jq -n \
    --argjson updated "$UPDATED_INPUT" \
    '{
      "hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "allow",
        "permissionDecisionReason": "stk auto-rewrite (compressed output)",
        "updatedInput": $updated
      }
    }'
else
  # No rewrite (stk rewrite exited 1) -- pass the command through unchanged.
  exit 0
fi
