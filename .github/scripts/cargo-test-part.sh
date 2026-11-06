#!/usr/bin/env bash
set -euo pipefail

# #1875：`cargo test <args> -- <libtest 参数>` 的分片执行。CI_TEST_PART=i/N 时，先用全部参数 `--list`
#   出会执行的测试名（去重排序），只跑第 i、i+N、i+2N… 个。N 个作业各取一份：并集恰为原集合、互不重叠；
#   每台机器上仍是 RUST_TEST_THREADS 个线程，同机负载与不拆时相同。
# libtest 参数分两类：选名字的（裸过滤词、--skip X、--exact）只用于列举，名单已经体现了它们；
#   决定怎么执行的（--ignored、--include-ignored、--test-threads N、--nocapture 等）列举和执行都带上，
#   这样 ignored 测试在分片后仍按原命令的语义执行或跳过。
# 用法（在 rust/ 下）：cargo-test-part.sh <cargo test 参数…> -- <libtest 参数…>
# 门：完整列表为空即失败。不设或 0/1 时原样执行一条 cargo test。
# 兼容 macOS 自带 bash 3.2：空数组用 ${a[@]+"${a[@]}"} 展开。

cargo_args=()
while (( $# )) && [[ "$1" != "--" ]]; do cargo_args+=("$1"); shift; done
(( $# )) && shift
libtest=("$@")
IFS=/ read -r i n <<<"${CI_TEST_PART:-0/1}"

if (( n <= 1 )); then
  exec cargo test ${cargo_args[@]+"${cargo_args[@]}"} -- ${libtest[@]+"${libtest[@]}"}
fi

run_opts=()
k=0
while (( k < ${#libtest[@]} )); do
  a="${libtest[k]}"
  case "$a" in
    --skip) k=$((k + 2)); continue ;;
    --exact) ;;
    --test-threads|--format|--color|--logfile|-Z|--shuffle-seed|--report-time)
      run_opts+=("$a")
      if (( k + 1 < ${#libtest[@]} )); then run_opts+=("${libtest[k + 1]}"); fi
      k=$((k + 2)); continue ;;
    -*) run_opts+=("$a") ;;
  esac
  k=$((k + 1))
done

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cargo test ${cargo_args[@]+"${cargo_args[@]}"} -- ${libtest[@]+"${libtest[@]}"} --list >"$work/list.raw"
sed -n 's/: test$//p' "$work/list.raw" | LC_ALL=C sort -u >"$work/names"
count=$(wc -l <"$work/names" | tr -d ' ')
if (( count == 0 )); then
  echo "::error title=Empty test list::cargo test ${cargo_args[*]-} -- ${libtest[*]-} listed no tests" >&2
  exit 1
fi
names=()
while IFS= read -r name; do names+=("$name"); done <"$work/names"
part=()
for (( k = i; k < count; k += n )); do part+=("${names[k]}"); done
echo "cargo-test-part: part $i/$n of $count tests"
(( ${#part[@]} == 0 )) && exit 0
exec cargo test ${cargo_args[@]+"${cargo_args[@]}"} -- ${run_opts[@]+"${run_opts[@]}"} --exact "${part[@]}"
