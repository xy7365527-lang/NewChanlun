#!/usr/bin/env bash
# harvest.sh - sandcastle branch harvest: push branch + open PR against main + register on ticket (#1660).
#
# What it does, per local sandcastle/issue-<n> branch in the host clone:
#   1. skip: skip-list / already in origin/main / no new commits vs origin/main /
#      latest worker round has no completed reviewer phase / ticket not open / open blocker;
#   2. fail-loud: any commit message carrying a GitHub closing keyword (close/fix/resolve #n)
#      would auto-close the ticket on merge, so the branch is NOT pushed;
#   3. push the branch to origin (never forced; a diverged remote branch is a failure);
#   4. open a PR against main (body uses "Refs #n") unless the head already has a PR,
#      then comment a delivery registration on the ticket.
#
# It never merges, never pushes main, never closes tickets and never deletes branches:
# merging is the human gate (#1003 ruling 3) and a partial slice must not close its ticket
# (docs/agents/delivery-discipline.md). Build and tests run in CI; no local cargo (#1588).
# History: #1003 scripted gate -> #1098 rc/ancestor fixes -> #1660 PR mode (host clone,
# no worktree on main, no auto close/merge).
#
# Env: SANDCASTLE_HOST_REPO (repo root, default = parent of this script's dir),
#      HARVEST_GH_REPO (default xy7365527-lang/NewChanlun), SKIP_ISSUES (space separated),
#      HARVEST_DRY_RUN=1 (print actions, push/PR/comment nothing), HARVEST_GH_CWD (default /tmp).
# Pure ASCII, bash 3.2 compatible.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
REPO="${SANDCASTLE_HOST_REPO:-$(cd "$SCRIPT_DIR/.." && pwd)}"
GH_SLUG="${HARVEST_GH_REPO:-xy7365527-lang/NewChanlun}"
GH_CWD="${HARVEST_GH_CWD:-/tmp}"
SKIP_ISSUES="${SKIP_ISSUES:-1024 1025}"
DRY="${HARVEST_DRY_RUN:-0}"
REG="$REPO/.sandcastle/logs/workers.jsonl"
# GitHub closing keywords followed by an issue ref: "#n", "owner/repo#n" or a full issues URL ("fix: #n" also closes).
CLOSE_RE='(^|[^[:alnum:]_])(close|closes|closed|fix|fixes|fixed|resolve|resolves|resolved)[[:space:]]*:?[[:space:]]*(([[:alnum:]_.-]+/[[:alnum:]_.-]+)?#[0-9]+|https?://github[.]com/[^/[:space:]]+/[^/[:space:]]+/issues/[0-9]+)'

# gh always runs from a neutral cwd with an explicit --repo (never infers the repo from cwd).
gh_() { (cd "$GH_CWD" && gh "$@" --repo "$GH_SLUG"); }

# Latest worker round for ticket $1 has a completed reviewer phase (no implementer start after it).
reviewer_done() {
  [ -f "$REG" ] || return 1
  python3 - "$REG" "$1" <<'PYEOF'
import json, sys
path, ticket = sys.argv[1], int(sys.argv[2])
last_impl, last_rev = -1, -1
for i, line in enumerate(open(path, encoding="utf-8")):
    line = line.strip()
    if not line:
        continue
    try:
        e = json.loads(line)
    except json.JSONDecodeError:
        continue
    if e.get("ticket") != ticket:
        continue
    if e.get("phase") == "implementer" and e.get("status") == "started":
        last_impl = i
    if e.get("phase") == "reviewer" and e.get("status") == "completed":
        last_rev = i
sys.exit(0 if last_rev > last_impl >= 0 else 1)
PYEOF
}

cd "$REPO"

opened=0
pushed_only=0
failed=0
fail_list=""
note_fail() { echo "FAIL $1 ($2)"; failed=$((failed+1)); fail_list="${fail_list}$1 ($2)"$'\n'; }
# Closing-keyword check in the C locale (stricter: CJK text right before "fix #n" still counts).
has_close_kw() { LC_ALL=C grep -Eiq "$CLOSE_RE" <<<"$1"; }

for f in "$REPO/.sandcastle/logs/GH_TOKEN_ROTATION_REQUIRED" "/Users/silencehan/Projects/NewChanlun/.sandcastle/logs/GH_TOKEN_ROTATION_REQUIRED"; do
  if [ -f "$f" ]; then note_fail "(all)" "GitHub token rotation required: $f"; fi
done
if [ $failed = 0 ] && ! git fetch -q origin main; then note_fail "(all)" "git fetch origin main failed"; fi
if [ $failed -gt 0 ]; then branches=""; else
  branches=$(git for-each-ref --format='%(refname)' 'refs/heads/sandcastle/issue-*' | sed 's#^refs/heads/##' | sort)
fi

for br in $branches; do
  n="${br#sandcastle/issue-}"
  case "$n" in ''|*[!0-9]*) echo "SKIP $br (not a ticket branch)"; continue ;; esac
  if [[ " $SKIP_ISSUES " == *" $n "* ]]; then echo "SKIP $br (skip-list)"; continue; fi

  # Pin the branch tip once; every later step (checks, push, PR text) uses this sha, so a worker
  # committing on the same branch mid-run cannot slip unreviewed commits into the push.
  sha=$(git rev-parse --verify -q "refs/heads/$br^{commit}") || { note_fail "$br" "cannot resolve branch"; continue; }
  if git merge-base --is-ancestor "$sha" origin/main 2>/dev/null; then echo "SKIP $br (already in origin/main)"; continue; fi
  if [ -z "$(git rev-list -n 1 "origin/main..$sha")" ]; then echo "SKIP $br (no new commits)"; continue; fi
  if ! reviewer_done "$n"; then echo "SKIP $br (reviewer phase not completed for latest round)"; continue; fi

  info=$(gh_ issue view "$n" --json state,blockedBy,title \
    --jq '[.state, ((if (.blockedBy.nodes | type) == "array" then ([.blockedBy.nodes[] | select(.state=="OPEN")] | length) else (.blockedBy.totalCount // 0) end) | tostring), .title] | @tsv' 2>/dev/null) || info=""
  if [ -z "$info" ]; then echo "SKIP $br (ticket read failed, retry later)"; continue; fi
  st=$(printf '%s' "$info" | cut -f1)
  blockers=$(printf '%s' "$info" | cut -f2)
  title=$(printf '%s' "$info" | cut -f3-)
  if [ "$st" != "OPEN" ]; then echo "SKIP $br (ticket $st)"; continue; fi
  if [ "$blockers" != "0" ]; then echo "SKIP $br (ticket has $blockers open blocker(s))"; continue; fi

  # PR state before any push: a closed/merged PR (possibly with its head branch deleted) is a
  # human decision; do not resurrect the branch.
  prs=$(gh_ pr list --head "$br" --state all --json state,url --jq '.[] | "\(.state) \(.url)"' 2>/dev/null) || {
    note_fail "$br" "PR lookup failed"; continue; }
  open_pr=$(printf '%s\n' "$prs" | awk '$1=="OPEN"{print $2; exit}')
  if [ -z "$open_pr" ] && [ -n "$prs" ]; then
    echo "SKIP $br (earlier PR closed or merged, needs a human decision: $(printf '%s' "$prs" | tr '\n' ' '))"
    continue
  fi

  # Remote state: equal / behind local (fast-forward push) / ahead of local (PR head = remote) / diverged.
  remote_sha=$(git ls-remote origin "refs/heads/$br" | cut -f1) || { note_fail "$br" "git ls-remote failed"; continue; }
  head_sha="$sha"
  need_push=1
  if [ -n "$remote_sha" ]; then
    if [ "$remote_sha" != "$sha" ] && ! git cat-file -e "$remote_sha^{commit}" 2>/dev/null; then
      git fetch -q origin "refs/heads/$br" 2>/dev/null || { note_fail "$br" "fetch of remote branch failed"; continue; }
    fi
    if [ "$remote_sha" = "$sha" ]; then
      need_push=0
    elif git merge-base --is-ancestor "$sha" "$remote_sha" 2>/dev/null; then
      need_push=0; head_sha="$remote_sha"
    elif ! git merge-base --is-ancestor "$remote_sha" "$sha" 2>/dev/null; then
      if [ -n "$open_pr" ]; then
        # The PR head was rewritten on the remote (e.g. reworded commits); the PR owns it now.
        echo "SKIP $br (local diverged from open PR head $open_pr; not pushing)"
        continue
      fi
      note_fail "$br" "remote branch diverged from local; not forcing"
      continue
    fi
  fi

  if has_close_kw "$(git log --format='%B' "origin/main..$head_sha")"; then
    note_fail "$br" "commit message carries a closing keyword; reword before PR"
    continue
  fi
  pr_title="Sandcastle #$n: $title"
  if [ -z "$open_pr" ] && has_close_kw "$pr_title"; then
    note_fail "$br" "PR title from ticket title carries a closing keyword"
    continue
  fi

  if [ $need_push = 1 ]; then
    if [ "$DRY" = 1 ]; then
      echo "DRY push $br ($sha)"
    elif ! git push -q origin "$sha:refs/heads/$br"; then
      note_fail "$br" "push failed"
      continue
    fi
  fi
  if [ -n "$open_pr" ]; then
    echo "PUSHED $br (open PR exists: $open_pr)"
    pushed_only=$((pushed_only+1))
    continue
  fi

  shas=$(git rev-list --reverse "origin/main..$head_sha")
  commit_lines=$(git log --reverse --format='- `%h` %s' "origin/main..$head_sha")
  body_file=$(mktemp "${TMPDIR:-/tmp}/harvest-pr.XXXXXX")
  {
    echo "Sandcastle two-phase delivery for #$n (implementer + reviewer worker), branch \`$br\`, based on main \`$(git rev-parse --short origin/main)\`."
    echo
    echo "Commits:"
    echo
    echo "$commit_lines"
    echo
    echo "Reviewer worker: completed (see \`.sandcastle/logs/sandcastle-issue-$n-reviewer.log\` on the host). Build and tests run in CI; harvest.sh does not run cargo locally."
    echo
    echo "Opened by harvest.sh. It does not merge and does not close tickets: merging is the human gate, and whether this delivery completes #$n is decided at review time against the ticket's acceptance criteria."
    echo
    echo "Refs #$n"
  } > "$body_file"
  if has_close_kw "$pr_title" || has_close_kw "$(cat "$body_file")"; then
    rm -f "$body_file"
    note_fail "$br" "generated PR title/body carries a closing keyword"
    continue
  fi
  if [ "$DRY" = 1 ]; then
    echo "DRY pr create $br -> main (#$n: $title)"
    rm -f "$body_file"
    continue
  fi
  pr_url=$(gh_ pr create --base main --head "$br" --title "$pr_title" --body-file "$body_file" 2>/dev/null) || pr_url=""
  rm -f "$body_file"
  if [ -z "$pr_url" ]; then note_fail "$br" "PR create failed"; continue; fi

  reg_file=$(mktemp "${TMPDIR:-/tmp}/harvest-reg.XXXXXX")
  {
    echo "> *This was generated by AI during wayfinding.*"
    echo
    echo "**Delivery registration (harvest.sh)**: branch \`$br\` pushed, PR $pr_url opened against main."
    echo
    echo "Commits: $(echo $shas | tr '\n' ' ')"
    echo
    echo "Reviewer worker phase completed. CI, independent review, merge and ticket closing are still pending; this ticket stays open."
  } > "$reg_file"
  if ! gh_ issue comment "$n" --body-file "$reg_file" >/dev/null 2>&1; then
    echo "WARN $br (ticket registration comment failed; PR $pr_url is open)"
  fi
  rm -f "$reg_file"
  echo "OPENED $br -> $pr_url"
  opened=$((opened+1))
done

# fail-loud (#1098): on any failure append a failure ledger + macOS popup + exit 1,
# so a red harvest cannot silently repeat round after round.
if [ $failed -gt 0 ] && [ "$DRY" = 1 ]; then
  echo "harvest dry run: opened=$opened pushed_only=$pushed_only failed=$failed (no ledger/popup in dry run)"
  exit 1
fi
if [ $failed -gt 0 ]; then
  mkdir -p .sandcastle/logs
  {
    echo "## harvest failed ($(date '+%Y-%m-%d %H:%M:%S'))"
    echo
    echo "opened=$opened pushed_only=$pushed_only failed=$failed"
    echo
    echo "failed branches:"
    printf '%s' "$fail_list" | sed '/^$/d; s/^/- /'
    echo
  } >> .sandcastle/logs/harvest-failures.md
  osascript -e "display notification \"Sandcastle harvest: ${failed} failed\" with title \"harvest.sh FAIL\"" 2>/dev/null || true
  exit 1
fi
echo "harvest done: opened=$opened pushed_only=$pushed_only failed=$failed"
