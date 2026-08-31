#!/usr/bin/env bash
# migrate-surface.test.sh -- sandbox regression tests for migrate-surface.sh (#1235).
#
# The sandbox image has no rsync, so a fake-rsync (full mirror via cp -a) simulates
# `rsync -a --delete` semantics. These tests only verify the safety properties of the
# "stop-write -> final copy -> atomic switch -> health check -> rollback -> read-only ->
# lock" process; real rsync flag semantics are verified on the host with real rsync
# (RUNBOOK.md acceptance section).
#
# The migration primitive and fake-rsync are invoked with "$BASH" (the same bash as
# this runner), so running this file under macOS bash 3.2 exercises the 3.2 code path.
#
# This file is intentionally ASCII-only for the same byte-robustness reason as
# migrate-surface.sh (bash 3.2 / transfer-mangling, issue #1235 host gate).
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MIGRATE="$HERE/migrate-surface.sh"
FAILURES=0

say()  { printf '%s\n' "[test] $*" >&2; }
assert() {
  local desc="$1" cond="$2"
  if eval "$cond"; then
    printf '%s\n' "  PASS: $desc"
  else
    printf '%s\n' "  FAIL: $desc"
    FAILURES=$((FAILURES + 1))
  fi
}

make_fake_rsync() {
  local dir="$1"
  {
    # Run fake-rsync with the same bash as the test runner (bash 3.2 on macOS),
    # so the bash 3.2 code path is exercised on the host, not just bash 5.
    printf '#!%s\n' "$BASH"
    cat <<'EOF'
# fake-rsync: simulate the full-mirror semantics of `rsync <opts...> <src>/ <dst>/`.
# migrate-surface.sh always calls it as: rsync <opts...> <src>/ <dst/> (last two args).
set -e
args=("$@")
# bash 3.2 has no negative array subscripts (${args[-1]}); compute indices explicitly.
n=${#args[@]}
src="${args[$((n-2))]}"
dst="${args[$((n-1))]}"
rm -rf "$dst"
mkdir -p "$dst"
cp -a "$src"/. "$dst"/
EOF
  } > "$dir/fake-rsync"
  chmod +x "$dir/fake-rsync"
}

# Byte-robustness guard: the migration primitive must stay ASCII-only so that
# transfer/encoding mangling cannot break the bash 3.2 parser (issue #1235 host gate).
byte_guard() {
  say "byte guard: migrate-surface.sh is ASCII-only"
  if LC_ALL=C grep -q '[^ -~]' "$MIGRATE"; then
    printf '%s\n' "  FAIL: migrate-surface.sh contains non-ASCII bytes"
    FAILURES=$((FAILURES + 1))
  else
    printf '%s\n' "  PASS: migrate-surface.sh is ASCII-only"
  fi
}

# -- Scenario A: full succeeds + no active writer + structure check + old path read-only --
scenario_a() {
  say "scenario A: full success, stop-write effective, atomic switch, read-only old path"
  local base; base=$(mktemp -d)
  local SRC="$base/src" DST="$base/dst" LOCK="$base/lock"
  mkdir -p "$SRC/sub"
  printf 'v1\n' > "$SRC/a.txt"
  printf 'x\n' > "$SRC/sub/b.txt"
  make_fake_rsync "$base"

  # Simulate a live writer: append a line every 20ms until it sees the stop flag.
  ( while [ ! -f "$base/stopflag" ]; do printf 'tick\n' >> "$SRC/live.txt"; sleep 0.02; done ) &
  local writer=$!
  printf '%s\n' "$writer" > "$base/writer.pid"

  local stop_hook="touch $base/stopflag"
  local start_hook="touch $base/started"
  local health_hook="test -f $base/started"

  "$BASH" "$MIGRATE" full \
    --src "$SRC" --dst "$DST" --name "canary-a" \
    --lock-dir "$LOCK" --rsync "$base/fake-rsync" \
    --stop-hook "$stop_hook" --start-hook "$start_hook" --health-hook "$health_hook" \
    >/dev/null 2>&1
  local rc=$?
  wait "$writer" 2>/dev/null || true

  assert "full exit code is 0" "[ $rc -eq 0 ]"
  assert "src is a symlink to dst" "[ -L \"$SRC\" ] && [ \"\$(readlink \"$SRC\")\" = \"$DST\" ]"
  assert "dst has a.txt" "[ -f \"$DST/a.txt\" ]"
  assert "dst/a.txt content is correct" "[ \"\$(cat \"$DST/a.txt\")\" = v1 ]"
  assert "dst has sub/b.txt" "[ -f \"$DST/sub/b.txt\" ]"
  assert "dst has live.txt (live data copied)" "[ -f \"$DST/live.txt\" ]"
  assert "lock released" "[ ! -e \"$LOCK\" ]"
  assert "manifest exists" "[ -f \"$DST/.migrate-surface-manifest\" ]"
  assert "writer stopped (no double write)" "! kill -0 \"$writer\" 2>/dev/null"
  assert "start hook executed" "[ -f \"$base/started\" ]"

  # Old path read-only
  "$BASH" "$MIGRATE" readonly-old --src "$SRC" --dst "$DST" --name "canary-a" \
    --lock-dir "$LOCK" >/dev/null 2>&1
  assert "readonly-old exit code is 0" "[ $? -eq 0 ]"
  local backup; backup=$(ls -d "$SRC".migrate-backup.* 2>/dev/null | head -n1)
  if [ -n "$backup" ]; then
    assert "old-path backup is not writable" "! touch \"$backup/.wtest\" 2>/dev/null"
    rm -f "$backup/.wtest"
  else
    assert "old-path backup exists" "false"
  fi

  # Restore writability before cleanup (readonly-old set a-w).
  [ -n "$backup" ] && chmod -R u+w "$backup" 2>/dev/null || true
  rm -rf "$base"
}

# -- Scenario B: healthcheck failure -> auto rollback, old path restored --
scenario_b() {
  say "scenario B: healthcheck failure auto-rolls-back, old path restored as-is"
  local base; base=$(mktemp -d)
  local SRC="$base/src" DST="$base/dst" LOCK="$base/lock"
  mkdir -p "$SRC"
  printf 'important\n' > "$SRC/data.txt"
  make_fake_rsync "$base"

  "$BASH" "$MIGRATE" full \
    --src "$SRC" --dst "$DST" --name "canary-b" \
    --lock-dir "$LOCK" --rsync "$base/fake-rsync" \
    --stop-hook "true" --start-hook "true" --health-hook "exit 1" \
    >/dev/null 2>&1
  local rc=$?

  assert "full exit code is non-zero (healthcheck failed)" "[ $rc -ne 0 ]"
  assert "src restored as a real directory (not symlink)" "[ -d \"$SRC\" ] && [ ! -L \"$SRC\" ]"
  assert "src/data.txt content is unchanged" "[ \"\$(cat \"$SRC/data.txt\")\" = important ]"
  assert "lock released" "[ ! -e \"$LOCK\" ]"
  assert "no leftover backup directory" "[ -z \"\$(ls -d \"$SRC\".migrate-backup.* 2>/dev/null || true)\" ]"

  rm -rf "$base"
}

# -- Scenario C: in-flight lock blocks a second preflight --
scenario_c() {
  say "scenario C: in-flight lock blocks a second preflight"
  local base; base=$(mktemp -d)
  local SRC="$base/src" DST="$base/dst" LOCK="$base/lock"
  mkdir -p "$SRC"
  make_fake_rsync "$base"

  "$BASH" "$MIGRATE" preflight --src "$SRC" --dst "$DST" --name "canary-c" \
    --lock-dir "$LOCK" --rsync "$base/fake-rsync" >/dev/null 2>&1
  assert "first preflight succeeds" "[ $? -eq 0 ]"
  assert "lock exists" "[ -e \"$LOCK\" ]"

  "$BASH" "$MIGRATE" preflight --src "$SRC" --dst "$DST" --name "canary-c" \
    --lock-dir "$LOCK" --rsync "$base/fake-rsync" >/dev/null 2>&1
  assert "second preflight rejected (non-zero)" "[ $? -ne 0 ]"

  "$BASH" "$MIGRATE" done --src "$SRC" --dst "$DST" --name "canary-c" \
    --lock-dir "$LOCK" >/dev/null 2>&1
  assert "done releases the lock" "[ ! -e \"$LOCK\" ]"

  rm -rf "$base"
}

# -- Scenario D: --dry-run prints but changes nothing --
scenario_d() {
  say "scenario D: --dry-run prints only, changes nothing"
  local base; base=$(mktemp -d)
  local SRC="$base/src" DST="$base/dst" LOCK="$base/lock"
  mkdir -p "$SRC"; printf 'd\n' > "$SRC/d.txt"
  make_fake_rsync "$base"

  "$BASH" "$MIGRATE" full --dry-run \
    --src "$SRC" --dst "$DST" --name "canary-d" \
    --lock-dir "$LOCK" --rsync "$base/fake-rsync" \
    --stop-hook "true" --start-hook "true" --health-hook "true" \
    >/dev/null 2>&1
  assert "dry-run full exit code 0" "[ $? -eq 0 ]"
  assert "src still a real directory after dry-run" "[ -d \"$SRC\" ] && [ ! -L \"$SRC\" ]"
  assert "dst not created after dry-run" "[ ! -e \"$DST\" ]"
  assert "no lock residue after dry-run" "[ ! -e \"$LOCK\" ]"

  rm -rf "$base"
}

# -- Scenario E: manual flow mid-way (stopped but not switched) rollback restarts old service --
scenario_e() {
  say "scenario E: rollback after stop-write (before switch) restarts old service, source stays real dir"
  local base; base=$(mktemp -d)
  local SRC="$base/src" DST="$base/dst" LOCK="$base/lock"
  mkdir -p "$SRC"; printf 'e\n' > "$SRC/e.txt"
  make_fake_rsync "$base"

  "$BASH" "$MIGRATE" preflight --src "$SRC" --dst "$DST" --name "canary-e"     --lock-dir "$LOCK" --rsync "$base/fake-rsync" >/dev/null 2>&1
  "$BASH" "$MIGRATE" baseline --src "$SRC" --dst "$DST" --name "canary-e"     --lock-dir "$LOCK" --rsync "$base/fake-rsync" >/dev/null 2>&1
  "$BASH" "$MIGRATE" stop-write --src "$SRC" --dst "$DST" --name "canary-e"     --lock-dir "$LOCK" --stop-hook "touch $base/stopped" >/dev/null 2>&1
  assert "stop hook executed" "[ -f \"$base/stopped\" ]"

  "$BASH" "$MIGRATE" rollback --src "$SRC" --dst "$DST" --name "canary-e"     --lock-dir "$LOCK" --start-hook "touch $base/restarted" >/dev/null 2>&1
  assert "rollback exit code 0" "[ $? -eq 0 ]"
  assert "start hook executed after rollback (old service restarted)" "[ -f \"$base/restarted\" ]"
  assert "source is still a real directory" "[ -d \"$SRC\" ] && [ ! -L \"$SRC\" ]"
  assert "source data unchanged" "[ \"\$(cat \"$SRC/e.txt\")\" = e ]"
  assert "lock released" "[ ! -e \"$LOCK\" ]"

  rm -rf "$base"
}

scenario_f() {
  say "scenario F: pre-switch-check hook runs before switch; failure aborts and rolls back"
  local base; base=$(mktemp -d)
  local SRC="$base/src" DST="$base/dst" LOCK="$base/lock"
  mkdir -p "$SRC"; printf 'f1\n' > "$SRC/f.txt"
  make_fake_rsync "$base"

  # Success path: the hook reads the old path as a real directory (i.e. before mv).
  "$BASH" "$MIGRATE" full \
    --src "$SRC" --dst "$DST" --name "canary-f1" --lock-dir "$LOCK" --rsync "$base/fake-rsync" \
    --stop-hook "true" --start-hook "true" --health-hook "true" \
    --pre-switch-check-hook "test ! -L '$SRC'" >/dev/null 2>&1
  assert "full with pre-switch-check succeeds" "[ $? -eq 0 ]"
  assert "src is a symlink to dst after switch" "[ -L \"$SRC\" ] && [ \"\$(readlink \"$SRC\")\" = \"$DST\" ]"
  rm -rf "$base"

  # Failure path: hook non-zero -> switch aborted, rollback restores old path.
  local base2; base2=$(mktemp -d)
  local SRC2="$base2/src" DST2="$base2/dst" LOCK2="$base2/lock"
  mkdir -p "$SRC2"; printf 'f2\n' > "$SRC2/f.txt"
  make_fake_rsync "$base2"
  "$BASH" "$MIGRATE" full \
    --src "$SRC2" --dst "$DST2" --name "canary-f2" --lock-dir "$LOCK2" --rsync "$base2/fake-rsync" \
    --stop-hook "true" --start-hook "true" --health-hook "true" \
    --pre-switch-check-hook "exit 1" >/dev/null 2>&1
  assert "pre-switch-check failure makes full non-zero" "[ $? -ne 0 ]"
  assert "switch aborted, src still a real directory" "[ -d \"$SRC2\" ] && [ ! -L \"$SRC2\" ]"
  assert "no backup residue after failure" "[ -z \"\$(ls -d \"$SRC2\".migrate-backup.* 2>/dev/null || true)\" ]"
  assert "lock released" "[ ! -e \"$LOCK2\" ]"
  rm -rf "$base2"
}

scenario_g() {
  say "scenario G: full runs preflight; clean rejection when target is a regular file"
  local base; base=$(mktemp -d)
  local SRC="$base/src" DST="$base/dstfile" LOCK="$base/lock"
  mkdir -p "$SRC"; printf 'g\n' > "$SRC/g.txt"
  printf 'occupied\n' > "$DST"
  make_fake_rsync "$base"

  "$BASH" "$MIGRATE" full \
    --src "$SRC" --dst "$DST" --name "canary-g" --lock-dir "$LOCK" --rsync "$base/fake-rsync" \
    --stop-hook "true" --start-hook "true" --health-hook "true" >/dev/null 2>&1
  assert "full rejects target being a regular file (non-zero)" "[ $? -ne 0 ]"
  assert "target regular file content unchanged" "[ \"\$(cat \"$DST\")\" = occupied ]"
  assert "src still a real directory (not switched)" "[ -d \"$SRC\" ] && [ ! -L \"$SRC\" ]"
  assert "lock released" "[ ! -e \"$LOCK\" ]"
  rm -rf "$base"
}

byte_guard
scenario_a
scenario_b
scenario_c
scenario_d
scenario_e
scenario_f
scenario_g

say "--------------------------------"
if [ "$FAILURES" -eq 0 ]; then
  say "all passed"
  exit 0
else
  say "$FAILURES failed"
  exit 1
fi
