#!/usr/bin/env bash
# harvest.test.sh - offline regression for harvest.sh PR mode (#1660).
# Throwaway bare origin + host clone; a stateful fake `gh` first on PATH serves JSON fixtures and
# applies the real `--jq` filter with jq (so harvest.sh's jq expressions are exercised), records
# every call, keeps PR bodies, and remembers PRs it created. Checks open/skip/fail paths, that a
# second real run neither reopens a PR nor re-comments, and that harvest.sh never closes tickets,
# merges, pushes main, force-pushes, deletes branches or runs cargo.
# Run: bash .sandcastle/harvest.test.sh   (needs git, jq, python3; pure ASCII, bash 3.2 compatible)
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
T="$(mktemp -d "${TMPDIR:-/tmp}/harvest-test.XXXXXX")"
trap 'rm -rf "$T"' EXIT
pass=0
fail=0
check() { if eval "$2"; then pass=$((pass+1)); echo "ok   - $1"; else fail=$((fail+1)); echo "FAIL - $1"; fi; }

# ---- 1. closing-keyword regex, taken verbatim from harvest.sh ----
CLOSE_RE=$(sed -n "s/^CLOSE_RE='\(.*\)'\$/\1/p" "$HERE/harvest.sh")
check "CLOSE_RE extracted" '[ -n "$CLOSE_RE" ]'
kw() { LC_ALL=C grep -Eiq "$CLOSE_RE" <<<"$1"; }
for s in "fix: #103" "Fixes #12" "closes: #12" "Fix:#9" "FIXES owner/repo#3" "Resolved #4" "close #1 and fix #2" \
         "Fixes https://github.com/o/r/issues/5" "fixed #7." "x resolves o/r#18"; do
  check "keyword hit: $s" 'kw "$s"'
done
for s in "perf(#1629): x" "chore(probe): #1630" "fix(#1629): x" "prefix #7" "fixed-point #8" "refs #15" \
         "closing #20" "Refs #1660" "fix the bug in 1630" "closed-form #3"; do
  check "keyword miss: $s" '! kw "$s"'
done

# ---- 2. repo fixture ----
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
git clone -q "$T/origin.git" "$T/other" 2>/dev/null

mk_branch() { # branch-suffix message
  git -C "$H" checkout -q -b "sandcastle/issue-$1" main
  echo "$1" > "$H/t$1.txt"
  git -C "$H" add "t$1.txt" && git -C "$H" commit -q -m "$2"
  git -C "$H" checkout -q main
}
ev() { printf '{"ticket":%s,"phase":"%s","status":"%s"}\n' "$1" "$2" "$3" >> "$H/.sandcastle/logs/workers.jsonl"; }
remote_push_extra() { # ticket message: remote gets local branch + one more commit
  git -C "$H" push -q origin "sandcastle/issue-$1"
  git -C "$T/other" fetch -q origin
  git -C "$T/other" checkout -q -B "sandcastle/issue-$1" "origin/sandcastle/issue-$1"
  echo extra > "$T/other/x$1.txt" && git -C "$T/other" add -A && git -C "$T/other" commit -q -m "$2"
  git -C "$T/other" push -q origin "sandcastle/issue-$1"
}

F="$T/fake"; mkdir -p "$F/bodies"
mk_branch 101 "probe: #101 add reading"            # reviewed, open, no PR -> push + PR + register
mk_branch 102 "probe: #102 tweak"                  # open PR exists -> push only, no comment
mk_branch 103 "fix: #103 broken thing"             # closing keyword -> FAIL, not pushed
mk_branch 104 "probe: #104 wip"                    # reviewer not completed for latest round -> skip
mk_branch 105 "probe: #105 done"                   # ticket closed -> skip
mk_branch 106 "probe: #106 blocked"                # 2 open + 1 closed blocker -> skip (2)
mk_branch 107 "probe: #107 diverged"               # remote diverged -> FAIL
mk_branch 109 "probe: #109 merged"                 # merged PR, remote branch deleted -> skip, not pushed
mk_branch 110 "probe: #110 local"                  # remote ahead (clean) -> no push, PR opened
mk_branch 111 "probe: #111 local"                  # remote ahead with closing keyword -> FAIL
mk_branch 112 "probe: #112 ok"                     # ticket title carries closing keyword -> FAIL
git -C "$H" branch -q sandcastle/issue-113 main    # nothing new -> skip (already in origin/main)
mk_branch 114 "fix: #114 reworded on remote"       # open PR whose head was rewritten -> skip, no FAIL
mk_branch 1660-2 "probe: odd name"                 # not a ticket branch -> skip
for n in 101 102 103 105 106 107 109 110 111 112 113 114; do ev $n implementer started; ev $n reviewer completed; done
ev 104 implementer started; ev 104 reviewer completed; ev 104 implementer started
git -C "$T/other" checkout -q -b sandcastle/issue-107 origin/main
echo other > "$T/other/o.txt" && git -C "$T/other" add -A && git -C "$T/other" commit -q -m "other side"
git -C "$T/other" push -q origin sandcastle/issue-107
remote_push_extra 110 "review: #110 polish"
remote_push_extra 111 "review: Closes #111"
git -C "$T/other" checkout -q -B sandcastle/issue-114 origin/main
echo 114 > "$T/other/t114.txt" && git -C "$T/other" add -A && git -C "$T/other" commit -q -m "probe: #114 reworded"
git -C "$T/other" push -q origin sandcastle/issue-114
# host main ahead of origin main: a push of main/HEAD would be visible.
echo local > "$H/local.txt" && git -C "$H" add local.txt && git -C "$H" commit -q -m "local only"

printf '{"state":"CLOSED","title":"closed ticket","blockedBy":{"nodes":[],"totalCount":0}}' > "$F/issue-105.json"
printf '{"state":"OPEN","title":"blocked","blockedBy":{"nodes":[{"state":"OPEN"},{"state":"CLOSED"},{"state":"OPEN"}],"totalCount":3}}' > "$F/issue-106.json"
printf '{"state":"OPEN","title":"Fix #1588 follow-up","blockedBy":{"nodes":[],"totalCount":0}}' > "$F/issue-112.json"
printf '[{"state":"OPEN","url":"https://example.invalid/pull/9102"}]' > "$F/prs-sandcastle_issue-102.json"
printf '[{"state":"MERGED","url":"https://example.invalid/pull/9109"}]' > "$F/prs-sandcastle_issue-109.json"
printf '[{"state":"OPEN","url":"https://example.invalid/pull/9114"}]' > "$F/prs-sandcastle_issue-114.json"

mkdir -p "$T/bin"
cat > "$T/bin/gh" <<'GHEOF'
#!/usr/bin/env bash
# fake gh: fixtures in $FAKE_DIR, real jq for --jq, stateful pr create.
echo "$*" >> "$FAKE_GH_LOG"
jqf=""; head=""; body=""; title=""; prev=""
for a in "$@"; do
  case "$prev" in --jq) jqf="$a" ;; --head) head="$a" ;; --body-file) body="$a" ;; --title) title="$a" ;; esac
  prev="$a"
done
out() { if [ -n "$jqf" ]; then jq -r "$jqf"; else cat; fi; }
case "$1 $2" in
  "issue view")
    f="$FAKE_DIR/issue-$3.json"
    if [ -f "$f" ]; then out < "$f"; else printf '{"state":"OPEN","title":"ticket %s","blockedBy":{"nodes":[],"totalCount":0}}' "$3" | out; fi ;;
  "pr list")
    f="$FAKE_DIR/prs-$(echo "$head" | tr / _).json"
    if [ -f "$f" ]; then out < "$f"; else echo '[]' | out; fi ;;
  "pr create")
    k=$(echo "$head" | tr / _)
    cp "$body" "$FAKE_DIR/bodies/$k.md"; echo "$title" > "$FAKE_DIR/bodies/$k.title"
    printf '[{"state":"OPEN","url":"https://example.invalid/pull/new-%s"}]' "$k" > "$FAKE_DIR/prs-$k.json"
    echo "https://example.invalid/pull/new-$k" ;;
  "issue comment") cp "$body" "$FAKE_DIR/bodies/comment-$3.md" ;;
  *) echo "unexpected gh call: $*" >&2; exit 9 ;;
esac
GHEOF
printf '#!/usr/bin/env bash\necho "cargo $*" >> "$FAKE_GH_LOG"; exit 87\n' > "$T/bin/cargo"
printf '#!/usr/bin/env bash\nexit 0\n' > "$T/bin/osascript"
chmod +x "$T/bin/gh" "$T/bin/cargo" "$T/bin/osascript"
export FAKE_GH_LOG="$T/gh.log" FAKE_DIR="$F"
: > "$FAKE_GH_LOG"
main_before=$(git -C "$T/origin.git" rev-parse main)
run_h() { PATH="$T/bin:$PATH" SANDCASTLE_HOST_REPO="$H" SKIP_ISSUES="" HARVEST_GH_CWD="$T" "$@" bash "$H/.sandcastle/harvest.sh" 2>&1; }
remote_has() { git -C "$T/origin.git" rev-parse -q --verify "refs/heads/$1" >/dev/null; }
remote_sha() { git -C "$T/origin.git" rev-parse "refs/heads/$1"; }

# ---- 3. first run ----
rc=0; out=$(run_h env) || rc=$?
echo "$out" | sed 's/^/    | /'
check "exit 1 because of failures (fail-loud)" '[ $rc = 1 ]'
check "101 PR opened" 'grep -q "OPENED sandcastle/issue-101" <<<"$out"'
check "101 pushed at local tip" '[ "$(remote_sha sandcastle/issue-101)" = "$(git -C "$H" rev-parse sandcastle/issue-101)" ]'
check "101 PR: base main, body file, explicit --repo" 'grep -q "pr create --base main --head sandcastle/issue-101 .*--body-file .*--repo xy7365527-lang/NewChanlun" "$FAKE_GH_LOG"'
check "101 PR body has Refs and no closing keyword" 'grep -q "^Refs #101$" "$F/bodies/sandcastle_issue-101.md" && ! kw "$(cat "$F/bodies/sandcastle_issue-101.md")"'
check "101 registration comment, says ticket stays open" 'grep -q "stays open" "$F/bodies/comment-101.md"'
check "102 open PR reused: push only, no PR, no comment" 'grep -q "PUSHED sandcastle/issue-102 (open PR exists" <<<"$out" && ! grep -q "pr create .*head sandcastle/issue-102" "$FAKE_GH_LOG" && [ ! -f "$F/bodies/comment-102.md" ]'
check "103 closing keyword fails, not pushed" 'grep -q "FAIL sandcastle/issue-103 (commit message carries a closing keyword" <<<"$out" && ! remote_has sandcastle/issue-103'
check "104 skipped (reviewer not completed)" 'grep -q "SKIP sandcastle/issue-104 (reviewer phase not completed" <<<"$out" && ! remote_has sandcastle/issue-104'
check "105 skipped (ticket closed)" 'grep -q "SKIP sandcastle/issue-105 (ticket CLOSED)" <<<"$out" && ! remote_has sandcastle/issue-105'
check "106 skipped, counts only open blockers (jq ran)" 'grep -q "SKIP sandcastle/issue-106 (ticket has 2 open blocker" <<<"$out" && ! remote_has sandcastle/issue-106'
check "107 diverged remote fails, remote untouched" 'grep -q "FAIL sandcastle/issue-107 (remote branch diverged" <<<"$out" && [ "$(git -C "$T/origin.git" log -1 --format=%s sandcastle/issue-107)" = "other side" ]'
check "109 merged PR: skipped and not resurrected" 'grep -q "SKIP sandcastle/issue-109 (earlier PR closed or merged" <<<"$out" && ! remote_has sandcastle/issue-109'
check "110 remote ahead: no push, PR lists remote commit" 'grep -q "OPENED sandcastle/issue-110" <<<"$out" && grep -q "review: #110 polish" "$F/bodies/sandcastle_issue-110.md" && [ "$(git -C "$T/origin.git" log -1 --format=%s sandcastle/issue-110)" = "review: #110 polish" ]'
check "111 remote-ahead commit with closing keyword fails" 'grep -q "FAIL sandcastle/issue-111 (commit message carries a closing keyword" <<<"$out" && ! grep -q "pr create .*head sandcastle/issue-111" "$FAKE_GH_LOG"'
check "112 ticket title with closing keyword fails, not pushed" 'grep -q "FAIL sandcastle/issue-112 (PR title from ticket title carries a closing keyword" <<<"$out" && ! remote_has sandcastle/issue-112'
check "113 nothing new: skipped" 'grep -q "SKIP sandcastle/issue-113 (already in origin/main)" <<<"$out"'
check "114 open PR with rewritten head: skipped, not failed, remote untouched" 'grep -q "SKIP sandcastle/issue-114 (local diverged from open PR head" <<<"$out" && [ "$(git -C "$T/origin.git" log -1 --format=%s sandcastle/issue-114)" = "probe: #114 reworded" ] && ! grep -q "issue-114" "$H/.sandcastle/logs/harvest-failures.md"'
check "odd branch name not parsed as a ticket" 'grep -q "SKIP sandcastle/issue-1660-2 (not a ticket branch)" <<<"$out"'
check "failure ledger written" 'grep -q "sandcastle/issue-103" "$H/.sandcastle/logs/harvest-failures.md"'
check "origin main untouched although host main is ahead" '[ "$(git -C "$T/origin.git" rev-parse main)" = "$main_before" ]'
check "local branches kept" '[ "$(git -C "$H" for-each-ref refs/heads/sandcastle | wc -l | tr -d " ")" = 14 ]'

# ---- 4. second real run: idempotent ----
cp "$FAKE_GH_LOG" "$T/gh.run1.log"; : > "$FAKE_GH_LOG"
rm -f "$F/bodies/comment-"*
rc2=0; out2=$(run_h env SKIP_ISSUES="103 107 111 112") || rc2=$?
check "second run exits 0" '[ $rc2 = 0 ]'
check "second run: 101/110 push-only via their open PRs" 'grep -q "PUSHED sandcastle/issue-101 (open PR exists" <<<"$out2" && grep -q "PUSHED sandcastle/issue-110 (open PR exists" <<<"$out2"'
check "second run: no PR created, no comment" '! grep -qE "pr create|issue comment" "$FAKE_GH_LOG"'

# ---- 5. dry run on a new branch ----
mk_branch 108 "probe: #108 new"
ev 108 implementer started; ev 108 reviewer completed
: > "$FAKE_GH_LOG"
out3=$(run_h env SKIP_ISSUES="103 107 111 112" HARVEST_DRY_RUN=1) || true
check "dry run announces 108" 'grep -q "DRY push sandcastle/issue-108" <<<"$out3" && grep -q "DRY pr create sandcastle/issue-108" <<<"$out3"'
check "dry run pushes/creates/comments nothing" '! remote_has sandcastle/issue-108 && ! grep -qE "pr create|issue comment" "$FAKE_GH_LOG"'

# ---- 6. token rotation marker blocks the whole run ----
touch "$H/.sandcastle/logs/GH_TOKEN_ROTATION_REQUIRED"; : > "$FAKE_GH_LOG"
rc4=0; out4=$(run_h env) || rc4=$?
check "token rotation marker: fail-loud, no gh calls" '[ $rc4 = 1 ] && grep -q "GitHub token rotation required" <<<"$out4" && [ ! -s "$FAKE_GH_LOG" ]'
rm -f "$H/.sandcastle/logs/GH_TOKEN_ROTATION_REQUIRED"

# ---- 7. forbidden actions across all runs ----
cat "$T/gh.run1.log" "$FAKE_GH_LOG" > "$T/gh.all.log"
check "never closes tickets" '! grep -q "issue close" "$T/gh.all.log"'
check "never merges PRs" '! grep -q "pr merge" "$T/gh.all.log"'
check "never runs cargo" '! grep -q "^cargo" "$T/gh.all.log"'
check "script code has no force push / branch delete / push to main / close / merge / cargo" '! grep -vE "^[[:space:]]*#" "$HERE/harvest.sh" | grep -nE "push .*(-f |--force|[[:space:]]\+)|branch -[dD]|:refs/heads/main|:main[[:space:]\"]|issue close|pr merge|cargo (test|check|build|fmt|clippy)"'

echo "harvest.test.sh: pass=$pass fail=$fail"
[ $fail = 0 ]
