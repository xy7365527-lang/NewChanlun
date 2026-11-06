#!/usr/bin/env bash
set -euo pipefail

# #1875：`cargo test <args> -- <filters>` 的分片执行。CI_TEST_PART=i/N 时，先用同样的参数和过滤词
#   `--list` 出会执行的测试名（含 #[ignore]，照旧显示为 ignored），去重排序后只跑第 i、i+N、i+2N… 个（--exact）。
#   N 个作业各取一份：并集恰为原集合、互不重叠；每台机器上仍是 RUST_TEST_THREADS 个线程，同机负载与不拆时相同。
# 用法（在 rust/ 下）：cargo-test-part.sh <cargo test 参数…> -- <libtest 过滤词…>
# 门：完整列表为空即失败。不设或 0/1 时原样执行一条 cargo test。

cargo_args=()
while (( $# )) && [[ "$1" != "--" ]]; do cargo_args+=("$1"); shift; done
(( $# )) && shift
filters=("$@")
IFS=/ read -r i n <<<"${CI_TEST_PART:-0/1}"

if (( n <= 1 )); then
  exec cargo test "${cargo_args[@]}" -- "${filters[@]}"
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cargo test "${cargo_args[@]}" -- "${filters[@]}" --list >"$work/list.raw"
# bash 3.2（macOS 自带）没有 mapfile；空数组在 set -u 下不可展开，故先数行再建数组。
sed -n 's/: test$//p' "$work/list.raw" | LC_ALL=C sort -u >"$work/names"
count=$(wc -l <"$work/names" | tr -d ' ')
names=()
while IFS= read -r name; do names+=("$name"); done <"$work/names"
if (( count == 0 )); then
  echo "::error title=Empty test list::cargo test ${cargo_args[*]} -- ${filters[*]} listed no tests" >&2
  exit 1
fi
part=()
for (( k = i; k < count; k += n )); do part+=("${names[k]}"); done
echo "cargo-test-part: part $i/$n of $count tests"
(( count <= i )) && exit 0
exec cargo test "${cargo_args[@]}" -- --exact "${part[@]}"
