#!/usr/bin/env bash
# 七档 cargo check 告警计数 + fmt 检查；在 rust/ 下运行。
set -u
i=0
for f in "" "--features backtest_bin" "--features s_session" "--features s_session,s_allocation_json_testonly,s_capture_native_testonly" "--features s_session,s_allocation_sqlite_testonly" "--features s_session,s_allocation_json_testonly" "--features s_session,s_capture_native_testonly" "--features s_session,s_allocation_json_testonly,s_allocation_sqlite_testonly" "--features s_session,s_capture_native_testonly,s_allocation_sqlite_testonly" "--features s_session,s_allocation_json_testonly,s_capture_native_testonly,s_allocation_sqlite_testonly"; do
  i=$((i+1))
  out=$(cargo check --all-targets $f --message-format=short 2>&1); rc=$?
  w=$(printf '%s\n' "$out" | grep -E ": warning" | sort -u)
  n=$(printf '%s' "$w" | grep -c . || true)
  e=$(printf '%s\n' "$out" | grep -cE "^error" || true)
  echo "C$i [$f] rc=$rc errors=$e warnings=$n"
  printf '%s\n' "$w" | grep . | sed "s/^/  C$i /"
  printf '%s\n' "$out" | grep -E "^error" | head -5 | sed "s/^/  C$i /"
done
cargo fmt --all -- --check >/tmp/fmt.txt 2>&1; echo "fmt rc=$? diffs=$(grep -c '^Diff in' /tmp/fmt.txt)"; grep '^Diff in' /tmp/fmt.txt | head
../.github/scripts/check-compile-negatives-1394.sh; echo "neg rc=$?"
