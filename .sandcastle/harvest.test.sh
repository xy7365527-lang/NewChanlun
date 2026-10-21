#!/usr/bin/env bash
# harvest.test.sh - offline regression for harvest.sh PR mode (#1660).
# Builds a throwaway bare origin + host clone, puts a fake `gh` first on PATH that records
# every call, runs harvest.sh, and checks: PR opened + ticket registered for a reviewed branch;
# no duplicate PR/comment when one is open; closing keyword => fail-loud and not pushed;
# reviewer not completed / ticket closed / open blocker => skipped; diverged remote => fail;
# and never `issue close`, `pr merge`, a push of main, or local cargo.
# Run: bash .sandcastle/harvest.test.sh   (pure ASCII, bash 3.2 compatible)
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
T="$(mktemp -d "${TMPDIR:-/tmp}/harvest-test.XXXXXX")"
trap 'rm -rf "$T"' EXIT
pass=0
fail=0
check() { if eval "$2"; then pass=$((pass+1)); echo "ok   - $1"; else fail=$((fail+1)); echo "FAIL - $1"; fi; }

export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@example.invalid GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@example.invalid
git init -q --bare "$T/origin.git"
git -C "$T/origin.git" symbolic-ref HEAD refs/heads/main
git clone -q "$T/origin.git" "$T/host" 2>/dev/null
H="$T/host"
git -C "$H" checkout -q -b main
mkdir -p "$H/.sandcastle/logs"
cp "$HERE/harvest.sh" "$H/.sandcastle/harvest.sh"
echo base > "$H/f.txt"
git -C "$H" add -A && git -C "$H" commit -q -m base && git -C "$H" push -q origin main

mk_branch() { # ticket message
  git -C "$H" checkout -q -b "sandcastle/issue-$1" main
  echo "$1" > "$H/t$1.txt"
  git -C "$H" add "t$1.txt" && git -C "$H" commit -q -m "$2"
  git -C "$H" checkout -q main
}
ev() { printf '{"ticket":%s,"phase":"%s","status":"%s"}\n' "$1" "$2" "$3" >> "$H/.sandcastle/logs/workers.jsonl"; }

mk_branch 101 "probe: #101 add reading"                 # reviewed, open, no PR -> PR + register
mk_branch 102 "probe: #102 tweak"                       # reviewed, open PR exists -> push only
mk_branch 103 "fix: #103 broken thing"                  # closing keyword -> FAIL, not pushed
mk_branch 104 "probe: #104 wip"                         # reviewer not completed -> skip
mk_branch 105 "probe: #105 done"                        # ticket closed -> skip
mk_branch 106 "probe: #106 blocked"                     # open blocker -> skip
mk_branch 107 "probe: #107 diverged"                    # remote diverged -> FAIL
for n in 101 102 103 105 106 107; do ev $n implementer started; ev $n reviewer completed; done
ev 104 implementer started; ev 104 reviewer completed; ev 104 implementer started
# diverge 107 on the remote
git clone -q "$T/origin.git" "$T/other" 2>/dev/null
git -C "$T/other" checkout -q -b sandcastle/issue-107 origin/main
echo other > "$T/other/o.txt" && git -C "$T/other" add -A && git -C "$T/other" commit -q -m "other side"
git -C "$T/other" push -q origin sandcastle/issue-107

# fake gh: records calls; answers issue view / pr list / pr create / issue comment.
mkdir -p "$T/bin"
cat > "$T/bin/gh" <<'GHEOF'
#!/usr/bin/env bash
echo "$*" >> "$FAKE_GH_LOG"
case "$1 $2" in
  "issue view")
    case "$3" in
      105) printf 'CLOSED\t0\tclosed ticket\n' ;;
      106) printf 'OPEN\t2\tblocked ticket\n' ;;
      *)   printf 'OPEN\t0\tticket %s\n' "$3" ;;
    esac ;;
  "pr list")
    case "$*" in *sandcastle/issue-102*) echo "OPEN https://example.invalid/pull/9102" ;; esac ;;
  "pr create") echo "https://example.invalid/pull/9001" ;;
  "issue comment") : ;;
  *) echo "unexpected gh call: $*" >&2; exit 9 ;;
esac
GHEOF
cat > "$T/bin/cargo" <<'CEOF'
#!/usr/bin/env bash
echo "cargo $*" >> "$FAKE_GH_LOG"; exit 87
CEOF
cat > "$T/bin/osascript" <<'OEOF'
#!/usr/bin/env bash
exit 0
OEOF
chmod +x "$T/bin/gh" "$T/bin/cargo" "$T/bin/osascript"
export FAKE_GH_LOG="$T/gh.log"
: > "$FAKE_GH_LOG"

rc=0
out=$(PATH="$T/bin:$PATH" SANDCASTLE_HOST_REPO="$H" SKIP_ISSUES="" HARVEST_GH_CWD="$T" \
  bash "$H/.sandcastle/harvest.sh" 2>&1) || rc=$?
echo "$out" | sed 's/^/    | /'
remote_has() { git -C "$T/origin.git" rev-parse -q --verify "refs/heads/$1" >/dev/null; }

check "exit 1 because of failures (fail-loud)" '[ $rc = 1 ]'
check "101 PR opened" 'grep -q "OPENED sandcastle/issue-101" <<<"$out"'
check "101 branch pushed" 'remote_has sandcastle/issue-101'
check "101 PR body via file, base main, explicit --repo" 'grep -q "pr create --base main --head sandcastle/issue-101 .*--body-file .*--repo xy7365527-lang/NewChanlun" "$FAKE_GH_LOG"'
check "101 ticket registration comment" 'grep -q "^issue comment 101 --body-file" "$FAKE_GH_LOG"'
check "102 open PR reused, no new PR" 'grep -q "PUSHED sandcastle/issue-102 (open PR exists" <<<"$out" && ! grep -q "head sandcastle/issue-102" <(grep "pr create" "$FAKE_GH_LOG")'
check "102 no registration comment" '! grep -q "^issue comment 102" "$FAKE_GH_LOG"'
check "103 closing keyword fails" 'grep -q "FAIL sandcastle/issue-103 (commit message carries a closing keyword" <<<"$out"'
check "103 not pushed" '! remote_has sandcastle/issue-103'
check "104 skipped (reviewer not completed)" 'grep -q "SKIP sandcastle/issue-104 (reviewer phase not completed" <<<"$out" && ! remote_has sandcastle/issue-104'
check "105 skipped (ticket closed)" 'grep -q "SKIP sandcastle/issue-105 (ticket CLOSED)" <<<"$out" && ! remote_has sandcastle/issue-105'
check "106 skipped (open blocker)" 'grep -q "SKIP sandcastle/issue-106 (ticket has 2 open blocker" <<<"$out" && ! remote_has sandcastle/issue-106'
check "107 diverged remote fails" 'grep -q "FAIL sandcastle/issue-107 (remote branch diverged" <<<"$out"'
check "failure ledger written" 'grep -q "sandcastle/issue-103" "$H/.sandcastle/logs/harvest-failures.md"'
check "never closes tickets" '! grep -q "issue close" "$FAKE_GH_LOG"'
check "never merges PRs" '! grep -q "pr merge" "$FAKE_GH_LOG"'
check "never runs cargo" '! grep -q "^cargo" "$FAKE_GH_LOG"'
check "origin main untouched" '[ "$(git -C "$T/origin.git" rev-parse main)" = "$(git -C "$H" rev-parse main)" ]'
check "local branches kept" '[ "$(git -C "$H" for-each-ref refs/heads/sandcastle | wc -l | tr -d " ")" = 7 ]'

# second run: nothing new to open; 101 now has an open PR in the fake? (fake only knows 102) ->
# dry-run must not push, create or comment anything.
: > "$FAKE_GH_LOG"
git -C "$H" checkout -q -b sandcastle/issue-108 main
echo 108 > "$H/t108.txt" && git -C "$H" add t108.txt && git -C "$H" commit -q -m "probe: #108 new"
git -C "$H" checkout -q main
ev 108 implementer started; ev 108 reviewer completed
out2=$(PATH="$T/bin:$PATH" SANDCASTLE_HOST_REPO="$H" SKIP_ISSUES="103 107" HARVEST_GH_CWD="$T" HARVEST_DRY_RUN=1 \
  bash "$H/.sandcastle/harvest.sh" 2>&1) || true
check "dry run announces 108" 'grep -q "DRY push sandcastle/issue-108" <<<"$out2" && grep -q "DRY pr create sandcastle/issue-108" <<<"$out2"'
check "dry run pushes nothing" '! remote_has sandcastle/issue-108'
check "dry run creates/comments nothing" '! grep -qE "pr create|issue comment" "$FAKE_GH_LOG"'

echo "harvest.test.sh: pass=$pass fail=$fail"
[ $fail = 0 ]
