#!/usr/bin/env bash
set -euo pipefail

# #1141: run one cargo argv behind the fixed 2 GiB disk gate, with fail-loud
# boundary/sampler telemetry. Only the stub test shortens the production 10 s cadence.
readonly MIN_FREE_BYTES=2147483648
readonly DEFAULT_SAMPLE_SECONDS=10
readonly SAMPLE_SECONDS="${CARGO_TELEMETRY_TEST_SAMPLE_SECONDS:-$DEFAULT_SAMPLE_SECONDS}"

usage() {
  echo "usage: $0 <command> [arg ...]" >&2
}

if (( $# == 0 )); then
  usage
  exit 64
fi
if ! [[ "$SAMPLE_SECONDS" =~ ^([0-9]+([.][0-9]+)?|[.][0-9]+)$ && "$SAMPLE_SECONDS" =~ [1-9] ]]; then
  echo "::error title=Invalid telemetry configuration::CARGO_TELEMETRY_TEST_SAMPLE_SECONDS must be a positive number; got $SAMPLE_SECONDS" >&2
  exit 64
fi

readonly -a command_argv=("$@")
printf 'telemetry_min_free_bytes=%s telemetry_sample_seconds=%s command=' "$MIN_FREE_BYTES" "$SAMPLE_SECONDS"
printf ' %q' "${command_argv[@]}"
printf '\n'

boundary_snapshot() {
  local snapshot_status=0 command_status
  free -b || {
    command_status=$?
    if (( snapshot_status == 0 )); then snapshot_status=$command_status; fi
  }
  df -B1 -T . || {
    command_status=$?
    if (( snapshot_status == 0 )); then snapshot_status=$command_status; fi
  }
  du -sx -B1 target || {
    command_status=$?
    if (( snapshot_status == 0 )); then snapshot_status=$command_status; fi
  }
  return "$snapshot_status"
}

sample_resources() {
  while true; do
    echo "sample_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    free -b
    df -B1 -T .
    ps -eo comm= | awk '
      $1 == "cargo" { cargo++ }
      $1 == "rustc" { rustc++ }
      $1 == "rust-lld" { lld++ }
      END { printf "processes cargo=%d rustc=%d rust-lld=%d\n", cargo, rustc, lld }
    '
    sleep "$SAMPLE_SECONDS"
  done
}

initial_snapshot_status=0
boundary_snapshot || initial_snapshot_status=$?
if (( initial_snapshot_status != 0 )); then
  echo "::error title=Initial resource snapshot failed::status=$initial_snapshot_status" >&2
  exit "$initial_snapshot_status"
fi

available_status=0
available_bytes="$(df -B1 --output=avail . | tail -n 1 | tr -d ' ')" || available_status=$?
if (( available_status != 0 )); then
  echo "::error title=Cannot measure free disk::df pipeline status=$available_status" >&2
  exit "$available_status"
fi
echo "available_bytes=$available_bytes minimum_bytes=$MIN_FREE_BYTES"
if ! [[ "$available_bytes" =~ ^[0-9]{1,18}$ ]]; then
  echo "::error title=Cannot measure free disk::df returned available_bytes=$available_bytes" >&2
  exit 1
fi
if (( 10#$available_bytes < 10#$MIN_FREE_BYTES )); then
  echo "::error title=Insufficient disk for Rust tests::available_bytes=$available_bytes minimum_bytes=$MIN_FREE_BYTES" >&2
  exit 1
fi

COMMAND_STATUS=0
sample_resources &
sampler_pid=$!
finish() {
  local sampler_status=0 stop_status=0 wait_status=0 snapshot_status=0
  trap - EXIT

  if kill -0 "$sampler_pid" 2>/dev/null; then
    kill "$sampler_pid" 2>/dev/null || stop_status=$?
    wait "$sampler_pid" 2>/dev/null || wait_status=$?
    if (( stop_status != 0 )); then
      echo "::error title=Failed to stop resource sampler::status=$stop_status" >&2
      sampler_status=$stop_status
    elif (( wait_status != 0 && wait_status != 143 )); then
      echo "::error title=Resource sampler failed while stopping::status=$wait_status" >&2
      sampler_status=$wait_status
    fi
  else
    wait "$sampler_pid" 2>/dev/null || wait_status=$?
    echo "::error title=Resource sampler exited unexpectedly::status=$wait_status" >&2
    if (( wait_status == 0 )); then sampler_status=1; else sampler_status=$wait_status; fi
  fi

  boundary_snapshot || snapshot_status=$?
  if (( snapshot_status != 0 )); then
    echo "::error title=Final resource snapshot failed::status=$snapshot_status" >&2
  fi

  if (( COMMAND_STATUS != 0 )); then exit "$COMMAND_STATUS"; fi
  if (( sampler_status != 0 )); then exit "$sampler_status"; fi
  exit "$snapshot_status"
}
trap finish EXIT

set +e
("${command_argv[@]}")
COMMAND_STATUS=$?
set -e
exit "$COMMAND_STATUS"
