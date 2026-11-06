#!/usr/bin/env bash
set -euo pipefail

# #1394/#1481：只执行「开某个 TestOnly 特性后才编译出来」的测试。
# 其余同名测试与不带该特性的 s_session 步同体，已在那一步执行（#1588 起为 ci.yml 的 rust-test-s-session 各分片）；
# 这里只补特性新增的那一批。
# 用法（在 rust/ 下）：test-feature-delta.sh <feature>
# 门：两份 --list 都须成功；新增集合为空即失败（只编译不执行、或一个都没编出来，都按无锁计）。

if (( $# != 1 )); then
  echo "usage: $0 <feature>" >&2
  exit 64
fi
readonly feature="$1"
readonly base_features="s_session"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

list_tests() {
  local features="$1" out="$2"
  shift 2
  cargo test --locked --features "$features" "$@" --jobs "${CI_CARGO_JOBS:-1}" -- --list >"$out.raw"
  sed -n 's/: test$//p' "$out.raw" | LC_ALL=C sort >"$out"
}

# #1875：CI_TEST_PROCS>1 时把新增集合按名字轮流分成几份，各起一个 cargo test 进程同时跑（每个进程仍是
#   RUST_TEST_THREADS 个线程；分进程不共享 fd 表，不放大 #1888 的 native_n1 fd 复用竞态）。测试集合与判定不变：
#   每个名字恰好进一份，任何一份失败本步即失败；各份日志按份号顺序整段输出。
run_delta() {
  local target="$1"; shift
  local procs="${CI_TEST_PROCS:-1}" sel
  if [[ "$target" == lib ]]; then sel=(--lib); else sel=(--bin s_structure_session); fi
  if (( procs <= 1 || $# < 2 * procs )); then
    cargo test --locked --features "$base_features,$feature" "${sel[@]}" --jobs "${CI_CARGO_JOBS:-1}" -- --exact "$@"
    return
  fi
  local -a pids=() part
  local i k=0 rc=0
  local names=("$@")
  for (( i = 0; i < procs; i++ )); do
    part=()
    for (( k = i; k < ${#names[@]}; k += procs )); do part+=("${names[k]}"); done
    cargo test --locked --features "$base_features,$feature" "${sel[@]}" --jobs "${CI_CARGO_JOBS:-1}" -- --exact "${part[@]}" \
      >"$work/$target.part$i.log" 2>&1 &
    pids+=($!)
  done
  for (( i = 0; i < procs; i++ )); do
    wait "${pids[i]}" || rc=1
    echo "::group::feature_delta target=$target part $i/$procs"
    cat "$work/$target.part$i.log"
    echo "::endgroup::"
  done
  return "$rc"
}

total=0
for target in lib bin; do
  if [[ "$target" == lib ]]; then
    selector=(--lib)
  else
    selector=(--bin s_structure_session)
  fi
  list_tests "$base_features" "$work/$target.base" "${selector[@]}"
  list_tests "$base_features,$feature" "$work/$target.feature" "${selector[@]}"
  LC_ALL=C comm -13 "$work/$target.base" "$work/$target.feature" >"$work/$target.delta"
  removed="$(LC_ALL=C comm -23 "$work/$target.base" "$work/$target.feature" | wc -l | tr -d ' ')"
  count="$(wc -l <"$work/$target.delta" | tr -d ' ')"
  echo "feature_delta target=$target feature=$feature base=$(wc -l <"$work/$target.base" | tr -d ' ') with_feature=$(wc -l <"$work/$target.feature" | tr -d ' ') delta=$count removed=$removed"
  if (( removed != 0 )); then
    echo "::error title=Feature removes tests::$feature removed $removed tests from $target" >&2
    exit 1
  fi
  if (( count > 0 )); then
    names=()
    while IFS= read -r name; do
      names+=("$name")
    done <"$work/$target.delta"
    run_delta "$target" "${names[@]}"
  fi
  total=$(( total + count ))
done
if (( total == 0 )); then
  echo "::error title=Empty feature delta::$feature compiled no additional tests; nothing was executed" >&2
  exit 1
fi
