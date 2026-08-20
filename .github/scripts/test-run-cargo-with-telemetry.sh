#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly helper="$script_dir/run-cargo-with-telemetry.sh"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT
readonly stub_dir="$tmp_dir/stubs"
readonly work_dir="$tmp_dir/work"
mkdir -p "$stub_dir" "$work_dir/target"

cat > "$stub_dir/free" <<'STUB'
#!/usr/bin/env bash
printf 'Mem: stub\n'
STUB
cat > "$stub_dir/df" <<'STUB'
#!/usr/bin/env bash
if [[ " $* " == *" --output=avail "* ]]; then
  printf 'Avail\n%s\n' "${STUB_AVAILABLE_BYTES:-2147483648}"
else
  printf 'Filesystem Type 1B-blocks Used Available Use%% Mounted on\nstub ext4 1 0 %s 0%% /\n' "${STUB_AVAILABLE_BYTES:-2147483648}"
fi
STUB
cat > "$stub_dir/du" <<'STUB'
#!/usr/bin/env bash
count=0
if [[ -f "$STUB_DU_COUNTER" ]]; then
  count="$(cat "$STUB_DU_COUNTER")"
fi
count=$((count + 1))
printf '%s\n' "$count" > "$STUB_DU_COUNTER"
printf '1024\ttarget\n'
if (( count >= 2 )); then
  exit "${STUB_SNAPSHOT_STATUS:-0}"
fi
STUB
cat > "$stub_dir/ps" <<'STUB'
#!/usr/bin/env bash
if (( ${STUB_SAMPLER_STATUS:-0} != 0 )); then
  exit "$STUB_SAMPLER_STATUS"
fi
printf 'cargo\nrustc\nrust-lld\n'
STUB
cat > "$stub_dir/cargo" <<'STUB'
#!/usr/bin/env bash
{
  printf 'cargo\n'
  printf '%s\n' "$@"
} > "$STUB_ARGV_FILE"
sleep 0.15
exit "${STUB_COMMAND_STATUS:-0}"
STUB
chmod +x "$stub_dir"/*

assert_status() {
  local expected_status=$1 actual_status=$2 label=$3 log_file=$4
  if (( actual_status != expected_status )); then
    echo "FAIL $label: expected status $expected_status, got $actual_status" >&2
    cat "$log_file" >&2
    exit 1
  fi
}

run_matrix_case() {
  local command_variant=$1 command_status=$2 sampler_status=$3 snapshot_status=$4
  local expected_status=0 actual_status label log_file argv_file counter_file expected_file
  local -a command_argv
  case "$command_variant" in
    default)
      command_argv=(cargo test --all-targets --jobs 1)
      ;;
    backtest)
      command_argv=(cargo test --lib --features backtest_bin --jobs 1)
      ;;
    *)
      echo "unknown command variant: $command_variant" >&2
      exit 2
      ;;
  esac
  if (( command_status != 0 )); then
    expected_status=$command_status
  elif (( sampler_status != 0 )); then
    expected_status=$sampler_status
  else
    expected_status=$snapshot_status
  fi

  label="$command_variant-command${command_status}-sampler${sampler_status}-snapshot${snapshot_status}"
  log_file="$tmp_dir/$label.log"
  argv_file="$tmp_dir/$label.argv"
  counter_file="$tmp_dir/$label.du-count"
  expected_file="$tmp_dir/$label.expected"
  printf '%s\n' "${command_argv[@]}" > "$expected_file"

  set +e
  (
    cd "$work_dir"
    env \
      PATH="$stub_dir:$PATH" \
      STUB_ARGV_FILE="$argv_file" \
      STUB_DU_COUNTER="$counter_file" \
      STUB_COMMAND_STATUS="$command_status" \
      STUB_SAMPLER_STATUS="$sampler_status" \
      STUB_SNAPSHOT_STATUS="$snapshot_status" \
      CARGO_TELEMETRY_TEST_SAMPLE_SECONDS=0.01 \
      "$helper" "${command_argv[@]}"
  ) >"$log_file" 2>&1
  actual_status=$?
  set -e

  assert_status "$expected_status" "$actual_status" "$label" "$log_file"
  diff -u "$expected_file" "$argv_file"
  grep -Fq 'telemetry_min_free_bytes=2147483648 telemetry_sample_seconds=0.01 command=' "$log_file"
  if (( sampler_status == 0 )); then
    if grep -Fq 'Resource sampler failed' "$log_file" || grep -Fq 'Resource sampler exited unexpectedly' "$log_file"; then
      echo "FAIL $label: actively stopped sampler was reported as an error" >&2
      cat "$log_file" >&2
      exit 1
    fi
  else
    grep -Fq 'Resource sampler exited unexpectedly::status=42' "$log_file"
  fi
}

matrix_count=0
for command_variant in default backtest; do
  for command_status in 0 7; do
    for sampler_status in 0 42; do
      for snapshot_status in 0 9; do
        run_matrix_case "$command_variant" "$command_status" "$sampler_status" "$snapshot_status"
        matrix_count=$((matrix_count + 1))
      done
    done
  done
done
if (( matrix_count != 16 )); then
  echo "FAIL: expected 16 matrix cells, ran $matrix_count" >&2
  exit 1
fi

run_guard_case() {
  local label=$1 available_bytes=$2 expected_status=$3
  local log_file="$tmp_dir/guard-$label.log"
  local argv_file="$tmp_dir/guard-$label.argv"
  local counter_file="$tmp_dir/guard-$label.du-count"
  local actual_status
  set +e
  (
    cd "$work_dir"
    env \
      PATH="$stub_dir:$PATH" \
      STUB_ARGV_FILE="$argv_file" \
      STUB_DU_COUNTER="$counter_file" \
      STUB_AVAILABLE_BYTES="$available_bytes" \
      "$helper" cargo test --all-targets --jobs 1
  ) >"$log_file" 2>&1
  actual_status=$?
  set -e
  assert_status "$expected_status" "$actual_status" "guard-$label" "$log_file"
  if (( expected_status == 0 )); then
    grep -Fq "available_bytes=$available_bytes minimum_bytes=2147483648" "$log_file"
    grep -Fq 'telemetry_min_free_bytes=2147483648 telemetry_sample_seconds=10 command=' "$log_file"
  elif [[ -e "$argv_file" ]]; then
    echo "FAIL guard-$label: command ran after guard rejection" >&2
    cat "$log_file" >&2
    exit 1
  fi
}

run_guard_case exact-boundary 2147483648 0
run_guard_case one-byte-low 2147483647 1
run_guard_case non-numeric not-a-number 1

grep -Fqx 'readonly MIN_FREE_BYTES=2147483648' "$helper"
grep -Fqx 'readonly DEFAULT_SAMPLE_SECONDS=10' "$helper"

for bad_sample in 0 nope; do
  log_file="$tmp_dir/bad-sample-$bad_sample.log"
  set +e
  CARGO_TELEMETRY_TEST_SAMPLE_SECONDS="$bad_sample" "$helper" cargo true >"$log_file" 2>&1
  actual_status=$?
  set -e
  assert_status 64 "$actual_status" "bad-sample-$bad_sample" "$log_file"
done

log_file="$tmp_dir/no-command.log"
set +e
"$helper" >"$log_file" 2>&1
actual_status=$?
set -e
assert_status 64 "$actual_status" no-command "$log_file"

printf 'PASS: 16 status-priority cells, guard boundaries, defaults, configuration validation, and argv preservation\n'
