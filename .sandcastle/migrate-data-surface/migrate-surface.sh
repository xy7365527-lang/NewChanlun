#!/usr/bin/env bash
# migrate-surface.sh -- single-agent data-surface migration primitive for the
# "stop-write -> baseline copy -> final incremental copy -> atomic switch ->
# health check -> rollback" flow (issue #1235).
#
# It migrates the data surface only; it never copies global coordination state
# such as the supervisor Unix socket or owner registry (which live under
# $TMPDIR/prime-agent-<uid>/). Callers must exclude those explicitly via
# --exclude -- see RUNBOOK.md.
#
# Requirements: bash (3.2+, macOS / Linux; verified against GNU bash 3.2.57,
# the same major.minor as macOS) + rsync + coreutils.
#
# Usage:
#   migrate-surface.sh <subcommand> --src <old-path> --dst <new-path> --name <surface> [options]
#
# Subcommands (manual flow, in order; "full" is one-shot):
#   preflight    prechecks + acquire lock     done          release lock (after sign-off)
#   baseline     full copy while online       verify        post-switch structure check
#   stop-write   stop-write point (run stop hook)  readonly-old  make old path read-only
#   final-copy   final incremental copy after stop-write
#   switch       atomic switch (mv old -> backup; ln -s new)
#   start        run start hook               healthcheck   run health hook
#   rollback     rollback (unlink symlink / restore old path / restart)
#   full         preflight..verify one-shot (auto-rollback on failure)
#
# IMPORTANT: this file is intentionally ASCII-only. Non-ASCII bytes in an
# executable shell script can be mangled by transfer tools or file-system
# encoding, which breaks the bash 3.2 parser (issue #1235 host gate). Keep it
# ASCII-only; migrate-surface.test.sh asserts this invariant.
set -uo pipefail

## Global variables (filled by argument parsing)
SRC=""
DST=""
NAME=""
STOP_HOOK=""
START_HOOK=""
HEALTH_HOOK=""
PRE_SWITCH_CHECK_HOOK=""
LOCK_DIR=""
RSYNC_CMD="${RSYNC:-rsync}"
# Default rsync options are macOS-stock-rsync (2.6.9) compatible: -A/-X are
# rsync 3.x only. Opt in via --rsync-opts '-A -X' (Linux) or '-E' (macOS xattrs).
RSYNC_OPTS=(-a --numeric-ids --delete --partial)
EXCLUDES=()
DRY_RUN=0
BACKUP_DIR=""

## Utility functions
log()  { printf '%s\n' "[migrate-surface] $*" >&2; }
die()  { printf '%s\n' "[migrate-surface] error: $*" >&2; exit 1; }
err()  { printf '%s\n' "[migrate-surface] error: $*" >&2; }

run() {
  # Print and execute (--dry-run prints only). Returns the command's exit code.
  printf '%s\n' "+ $*" >&2
  if [ "$DRY_RUN" -eq 1 ]; then
    return 0
  fi
  "$@"
}

run_hook() {
  local hook="$1" kind="$2"
  if [ -z "$hook" ]; then
    log "(no $kind hook, skipping)"
    return 0
  fi
  log "running $kind hook: $hook"
  if [ "$DRY_RUN" -eq 1 ]; then
    return 0
  fi
  # Evaluate via shell to support "launchctl unload <plist>" style commands.
  bash -c "$hook" || { log "$kind hook failed" >&2; return 1; }
}

require_args() {
  [ -n "$SRC" ] || die "missing --src"
  [ -n "$DST" ] || die "missing --dst"
  [ -n "$NAME" ] || die "missing --name"
  # SRC must not be inside DST (would make mv + symlink self-referential).
  case "$SRC" in
    "$DST"|"$DST"/*) die "SRC must not be inside DST ($SRC vs $DST)" ;;
  esac
  # DST must not be inside SRC (would make rsync recurse into itself).
  case "$DST" in
    "$SRC"|"$SRC"/*) die "DST must not be inside SRC ($SRC vs $DST)" ;;
  esac
}

default_lock_dir() {
  printf '%s' "${HOME:-/tmp}/.migrate-surface-locks/$NAME"
}

## Lock (in-flight marker; prevents two concurrent migrations of one surface)
acquire_lock() {
  if [ "$DRY_RUN" -eq 1 ]; then
    log "(dry-run: skip lock acquisition)"
    return 0
  fi
  mkdir -p "$(dirname "$LOCK_DIR")" || return 1
  if [ -e "$LOCK_DIR" ]; then
    log "lock already exists: $LOCK_DIR" >&2
    log "  a migration for the same surface is in flight, or a previous run was not finalized with 'done'." >&2
    log "  if no migration is running, inspect and remove that directory manually, then retry." >&2
    return 1
  fi
  mkdir "$LOCK_DIR" || return 1
  printf '%s\n' "$$"                                   > "$LOCK_DIR/pid"
  printf '%s\n' "$SRC"                                 > "$LOCK_DIR/src"
  printf '%s\n' "$DST"                                 > "$LOCK_DIR/dst"
  printf '%s\n' "$NAME"                                > "$LOCK_DIR/name"
  printf '%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"       > "$LOCK_DIR/started"
  set_phase preflight || return 1
  log "lock acquired: $LOCK_DIR"
}

release_lock() {
  if [ "$DRY_RUN" -eq 1 ]; then
    return 0
  fi
  if [ -n "$LOCK_DIR" ] && [ -e "$LOCK_DIR" ]; then
    rm -rf "$LOCK_DIR" || { log "failed to release lock: $LOCK_DIR" >&2; return 1; }
    log "lock released: $LOCK_DIR"
  fi
}

set_phase() {
  [ "$DRY_RUN" -eq 1 ] && return 0
  if [ -n "$LOCK_DIR" ]; then
    mkdir -p "$LOCK_DIR" && printf '%s\n' "$1" > "$LOCK_DIR/phase"
  fi
}
get_phase() { [ -f "$LOCK_DIR/phase" ] && cat "$LOCK_DIR/phase" || printf 'none'; }

## Copy primitive
do_copy() {
  local kind="$1"
  log "$kind: $SRC/ -> $DST/"
  # bash 3.2 compatible: build args element by element (no ${arr[@]/#/...}).
  local -a cmd
  cmd=("$RSYNC_CMD" "${RSYNC_OPTS[@]}")
  # bash 3.2 + set -u treats an empty array's "${arr[@]}" as unbound; guard on length.
  if [ "${#EXCLUDES[@]}" -gt 0 ]; then
    local e
    for e in "${EXCLUDES[@]}"; do
      cmd+=("--exclude=$e")
    done
  fi
  cmd+=("$SRC/" "$DST/")
  printf '%s\n' "+ ${cmd[*]}" >&2
  if [ "$DRY_RUN" -eq 1 ]; then
    return 0
  fi
  mkdir -p "$DST" || return 1
  "${cmd[@]}" || { log "$kind copy failed" >&2; return 1; }
}

## Backup directory discovery
manifest_file() { printf '%s' "$DST/.migrate-surface-manifest"; }
read_manifest_backup() {
  local m; m=$(manifest_file)
  if [ -f "$m" ]; then
    sed -n 's/^backup=//p' "$m" | head -n1
  fi
}
discover_backup() {
  local b; b=$(read_manifest_backup)
  if [ -n "$b" ] && [ -e "$b" ]; then printf '%s' "$b"; return 0; fi
  local found=""
  local cand
  for cand in "$SRC".migrate-backup.*; do
    [ -e "$cand" ] || continue
    found="$cand"
  done
  printf '%s' "$found"
}

## Subcommands

# Portable non-empty directory check. macOS BSD find has no -quit, and
# find -print | head -n1 is needlessly wordy; ls -A works on both platforms.
dir_nonempty() {
  [ -n "$(ls -A "$1" 2>/dev/null | head -n1)" ]
}

# Prechecks (no lock; shared by preflight and full).
preflight_checks() {
  [ -e "$SRC" ] || { err "source path does not exist: $SRC"; return 1; }
  [ -L "$SRC" ] && { err "source path is already a symlink (possibly migrated; run verify): $SRC"; return 1; }
  [ -d "$SRC" ] || { err "source path is not a directory: $SRC"; return 1; }
  [ -L "$DST" ] && { err "target path must not be a symlink: $DST"; return 1; }
  if [ -e "$DST" ] && [ ! -d "$DST" ]; then
    err "target path exists and is not a directory: $DST"; return 1
  fi

  # Space precheck: refuse when target filesystem free < source usage (fail-closed).
  if [ "$DRY_RUN" -eq 0 ]; then
    local src_kb dst_avail_kb dst_dir
    src_kb=$(du -sk "$SRC" | awk '{print $1}') || { log "du on source failed (continue, non-blocking)"; src_kb=0; }
    dst_dir="$DST"; mkdir -p "$dst_dir" || return 1
    dst_avail_kb=$(df -Pk "$dst_dir" | awk 'NR==2 {print $4}')
    if [ -n "${dst_avail_kb:-}" ] && [ "${src_kb:-0}" -gt 0 ] && [ "$dst_avail_kb" -lt "$src_kb" ]; then
      err "not enough space: source $src_kb KiB > target free $dst_avail_kb KiB"; return 1
    fi
  fi
}

cmd_preflight() {
  require_args
  [ -n "$LOCK_DIR" ] || LOCK_DIR="$(default_lock_dir)"
  acquire_lock || return 1
  preflight_checks || return 1
  log "preflight passed: src=$SRC dst=$DST name=$NAME"
  log "manual flow: run 'done' after all verifications pass; 'full' finalizes automatically."
}

cmd_baseline() {
  require_args
  do_copy "baseline copy (service online)" || return 1
  set_phase baseline
  log "baseline copy done. It can be re-run; after stop-write, run final-copy to finish."
}

cmd_stop_write() {
  require_args
  log "-- stop-write point (old path must not be written from here on) --"
  run_hook "$STOP_HOOK" "stop" || return 1
  set_phase stop-write
  log "writes stopped. Confirm the service no longer writes the old path, then run final-copy."
}

cmd_final_copy() {
  require_args
  do_copy "final incremental copy (after stop-write)" || return 1
  set_phase final-copy
  log "final incremental copy done."
}

cmd_switch() {
  require_args
  if [ -L "$SRC" ]; then
    local t; t=$(readlink "$SRC" || true)
    if [ "$t" = "$DST" ]; then
      log "already switched: $SRC -> $DST (no-op)"
      return 0
    fi
    err "source path is already a symlink pointing elsewhere ($t), refusing switch"; return 1
  fi
  [ -d "$SRC" ] || { err "source path is not a directory, cannot switch: $SRC"; return 1; }
  # Target must have content (manifest or at least one entry) before switching.
  if [ "$DRY_RUN" -eq 0 ]; then
    if [ ! -e "$DST" ] || ! dir_nonempty "$DST"; then
      err "target is empty, refusing switch: $DST"; return 1
    fi
  fi

  # Re-confirm the old writer has stopped before switching (RUNBOOK sec 6.3).
  run_hook "$PRE_SWITCH_CHECK_HOOK" "pre-switch-check" || return 1

  local backup="${BACKUP_DIR:-$SRC.migrate-backup.$(date +%Y%m%d%H%M%S)}"
  [ -e "$backup" ] && { err "backup path already exists: $backup"; return 1; }

  run mv "$SRC" "$backup" || return 1
  run ln -s "$DST" "$SRC" || { log "symlink failed, attempting to restore backup"; run mv "$backup" "$SRC"; return 1; }

  # Record phase before writing manifest: even if the manifest write fails,
  # rollback can still restore correctly per the "switched" phase.
  set_phase switch

  # Write manifest (basis for rollback / readonly-old).
  if [ "$DRY_RUN" -eq 0 ]; then
    {
      printf 'src=%s\n' "$SRC"
      printf 'dst=%s\n' "$DST"
      printf 'backup=%s\n' "$backup"
      printf 'name=%s\n' "$NAME"
      printf 'ts=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    } > "$(manifest_file)" || { err "failed to write manifest (rollback basis missing, manual intervention required)"; return 1; }
  fi
  log "switched: $SRC -> $DST (backup at $backup)"
}

cmd_start() {
  require_args
  run_hook "$START_HOOK" "start" || return 1
  set_phase start
  log "new-path service started."
}

cmd_healthcheck() {
  require_args
  log "-- health check --"
  run_hook "$HEALTH_HOOK" "health" || return 1
  set_phase healthcheck
  log "health check passed."
}

cmd_verify() {
  require_args
  if [ "$DRY_RUN" -eq 1 ]; then
    log "(dry-run: skip structure verification)"
    return 0
  fi
  local ok=1
  if [ ! -L "$SRC" ]; then
    log "verify failed: $SRC is not a symlink to the new path" >&2; ok=0
  elif [ "$(readlink "$SRC")" != "$DST" ]; then
    log "verify failed: symlink points to $(readlink "$SRC"), expected $DST" >&2; ok=0
  fi
  [ -f "$(manifest_file)" ] || { log "verify failed: manifest missing" >&2; ok=0; }
  if [ ! -e "$DST" ] || ! dir_nonempty "$DST"; then
    log "verify failed: target is empty" >&2; ok=0
  fi
  [ "$ok" -eq 1 ] || return 1
  set_phase verify
  log "verify passed: $SRC -> $DST, manifest at $(manifest_file)"
}

cmd_readonly_old() {
  require_args
  local backup; backup=$(discover_backup)
  if [ -z "$backup" ] || [ ! -e "$backup" ]; then
    err "old-path backup not found (no manifest backup and no $SRC.migrate-backup.*)"; return 1
  fi
  run chmod -R a-w "$backup" || return 1
  # Verify read-only actually took effect.
  if [ "$DRY_RUN" -eq 0 ]; then
    if touch "$backup/.migrate-write-test" 2>/dev/null; then
      rm -f "$backup/.migrate-write-test"
      err "old-path backup is still writable, read-only did not take effect: $backup"; return 1
    fi
  fi
  log "old path set read-only: $backup (run 'chmod -R u+w' before an RTO rollback)"
}

cmd_rollback() {
  require_args
  [ -n "$LOCK_DIR" ] || LOCK_DIR="$(default_lock_dir)"
  local phase; phase=$(get_phase)
  local backup; backup=$(discover_backup)

  case "$phase" in
    preflight|baseline)
      log "rollback: writes not yet stopped, nothing to do."
      ;;
    stop-write|final-copy)
      log "rollback: writes stopped but not switched; restarting old-path service."
      run_hook "$START_HOOK" "start" || { log "rollback restart failed, manual intervention required" >&2; return 1; }
      ;;
    switch|start|healthcheck)
      log "rollback: already switched ($phase); restoring old path."
      run_hook "$STOP_HOOK" "stop" || log "(stop hook failed, continuing rollback)"
      if [ -L "$SRC" ]; then
        run rm "$SRC" || { log "failed to remove symlink" >&2; return 1; }
      fi
      if [ -n "$backup" ] && [ -e "$backup" ]; then
        run mv "$backup" "$SRC" || { log "failed to restore backup" >&2; return 1; }
      else
        log "warning: backup directory not found, old path not restored" >&2
      fi
      run_hook "$START_HOOK" "start" || { log "rollback restart failed, manual intervention required" >&2; return 1; }
      ;;
    verify|done)
      log "rollback: migration already finalized ($phase); this script does not auto-revert a verified migration. For rollback, follow the RTO section of RUNBOOK.md manually."
      return 1
      ;;
    *)
      log "rollback: unknown phase ($phase), releasing lock only."
      ;;
  esac
  release_lock
  log "rollback done."
}

cmd_done() {
  require_args
  [ -n "$LOCK_DIR" ] || LOCK_DIR="$(default_lock_dir)"
  local phase; phase=$(get_phase)
  [ "$phase" = "verify" ] || log "warning: current phase is $phase (usually 'verify'), still releasing lock."
  release_lock
  log "done: lock released."
}

cmd_full() {
  require_args
  [ -n "$LOCK_DIR" ] || LOCK_DIR="$(default_lock_dir)"
  acquire_lock || return 1
  local failed=1
  if preflight_checks && cmd_baseline && cmd_stop_write && cmd_final_copy \
      && cmd_switch && cmd_start && cmd_healthcheck && cmd_verify; then
    failed=0
  fi
  if [ "$failed" -ne 0 ]; then
    log "migration failed, rolling back automatically."
    cmd_rollback || die "rollback failed, manual intervention required (lock at $LOCK_DIR)"
    die "migration failed (rolled back)."
  fi
  release_lock
  log "migration done: $SRC -> $DST"
}

usage() {
  cat >&2 <<'EOF'
Usage: migrate-surface.sh <subcommand> --src <old-path> --dst <new-path> --name <surface> [options]

Subcommands:
  preflight | baseline | stop-write | final-copy | switch | start |
  healthcheck | verify | readonly-old | rollback | done | full

Options:
  --src <path>            old data-surface path (required)
  --dst <path>            new data-surface path (required, inside the XFS/VDO volume)
  --name <name>           surface name (required; used for the lock directory)
  --stop-hook <cmd>       stop-write command (e.g. launchctl unload <plist> / systemctl stop <unit>)
  --start-hook <cmd>      start command
  --health-hook <cmd>     health-check command (non-zero means failure)
  --pre-switch-check-hook <cmd>  extra guard before switch (e.g. confirm no active writer)
  --exclude <glob>        copy exclusion (repeatable; use for socket/registry)
  --rsync <path>          rsync executable (default $RSYNC)
  --rsync-opts <'...'>    append rsync options (one argument, split on whitespace)
  --lock-dir <path>       lock directory (default $HOME/.migrate-surface-locks/<name>)
  --backup-dir <path>     explicit backup path (default <src>.migrate-backup.<timestamp>)
  --dry-run               print only, do not execute
EOF
  exit 2
}

## Argument parsing
COMMAND=""
while [ $# -gt 0 ]; do
  case "$1" in
    preflight|baseline|stop-write|final-copy|switch|start|healthcheck|verify|readonly-old|rollback|done|full)
      if [ -z "$COMMAND" ]; then COMMAND="$1"; else log "only one subcommand allowed (already got $COMMAND)" >&2; exit 2; fi
      ;;
    --src) SRC="${2:-}"; shift ;;
    --dst) DST="${2:-}"; shift ;;
    --name) NAME="${2:-}"; shift ;;
    --stop-hook) STOP_HOOK="${2:-}"; shift ;;
    --start-hook) START_HOOK="${2:-}"; shift ;;
    --health-hook) HEALTH_HOOK="${2:-}"; shift ;;
    --pre-switch-check-hook) PRE_SWITCH_CHECK_HOOK="${2:-}"; shift ;;
    --exclude) EXCLUDES+=("${2:-}"); shift ;;
    --rsync) RSYNC_CMD="${2:-}"; shift ;;
    --rsync-opts)
      # bash 3.2: "read -a" on empty/whitespace input leaves the array unset,
      # and an empty array's "${arr[@]}" trips "set -u"; guard on length.
      if [ -n "${2:-}" ]; then
        read -r -a _rsync_extra <<< "$2" || true
        if [ "${#_rsync_extra[@]}" -gt 0 ]; then
          RSYNC_OPTS+=("${_rsync_extra[@]}")
        fi
      fi
      shift ;;
    --lock-dir) LOCK_DIR="${2:-}"; shift ;;
    --backup-dir) BACKUP_DIR="${2:-}"; shift ;;
    --dry-run) DRY_RUN=1 ;;
    -h|--help) usage ;;
    *) log "unknown option: $1" >&2; usage ;;
  esac
  shift
done

[ -n "$COMMAND" ] || usage

case "$COMMAND" in
  preflight)   cmd_preflight ;;
  baseline)    cmd_baseline ;;
  stop-write)  cmd_stop_write ;;
  final-copy)  cmd_final_copy ;;
  switch)      cmd_switch ;;
  start)       cmd_start ;;
  healthcheck) cmd_healthcheck ;;
  verify)      cmd_verify ;;
  readonly-old) cmd_readonly_old ;;
  rollback)    cmd_rollback ;;
  done)        cmd_done ;;
  full)        cmd_full ;;
  *) usage ;;
esac
