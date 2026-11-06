#!/usr/bin/env bash
set -euo pipefail

# #1875：把一条 `cargo test <args> -- <filters>` 拆成 CI_TEST_PROCS 个进程同时跑，测试集合不变。
# 用法（在 rust/ 下）：cargo-test-procs.sh <cargo test 参数…> -- <libtest 过滤词…>
# 做法：先用同样的参数和过滤词 `--list` 出实际会执行的测试名（含 #[ignore]，照旧显示为 ignored），
#   按名字轮流分成 N 份，每份 `--exact` 起一个 cargo test 进程；各进程仍是 RUST_TEST_THREADS 个线程。
#   分进程不共享 fd 表，不放大 #1888 修过的 native_n1 fd 复用竞态。
# 门：列表为空即失败；任何一份失败本步即失败；各份日志按份号整段输出。
# CI_TEST_PROCS<=1，或测试数不足 2N 时，原样执行一条 cargo test。

cargo_args=()
while (( $# )) && [[ "$1" != "--" ]]; do cargo_args+=("$1"); shift; done
(( $# )) && shift
filters=("$@")
procs="${CI_TEST_PROCS:-1}"

if (( procs <= 1 )); then
  exec cargo test "${cargo_args[@]}" -- "${filters[@]}"
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cargo test "${cargo_args[@]}" -- "${filters[@]}" --list >"$work/list.raw"
sed -n 's/: test$//p' "$work/list.raw" | LC_ALL=C sort -u >"$work/names"
mapfile -t names <"$work/names"
if (( ${#names[@]} == 0 )); then
  echo "::error title=Empty test list::cargo test ${cargo_args[*]} -- ${filters[*]} listed no tests" >&2
  exit 1
fi
if (( ${#names[@]} < 2 * procs )); then
  exec cargo test "${cargo_args[@]}" -- --exact "${names[@]}"
fi

echo "cargo-test-procs: ${#names[@]} tests in $procs processes"
pids=()
for (( i = 0; i < procs; i++ )); do
  part=()
  for (( k = i; k < ${#names[@]}; k += procs )); do part+=("${names[k]}"); done
  cargo test "${cargo_args[@]}" -- --exact "${part[@]}" >"$work/part$i.log" 2>&1 &
  pids+=($!)
done
rc=0
for (( i = 0; i < procs; i++ )); do
  wait "${pids[i]}" || rc=1
  echo "::group::cargo-test-procs part $i/$procs"
  cat "$work/part$i.log"
  echo "::endgroup::"
done
exit "$rc"
