#!/usr/bin/env bash
set -euo pipefail

# #1394/#1481：只执行「开某个 TestOnly 特性后才编译出来」的测试。
# 其余同名测试与不带该特性的 s_session 步同体，已在那一步执行（#1588 起为 ci.yml 的 rust-test-s-session 各分片）；
# 这里只补特性新增的那一批。
# 用法（在 rust/ 下）：
#   test-feature-delta.sh <feature>                         原单参数完整执行入口
#   test-feature-delta.sh <feature> <shard_index> <shard_count>
#   test-feature-delta.sh --verify-partition <feature> <shard_count>
# 分片 index 从 0 起，count >= 5；每片在执行前重新 --list、打印所有片清单并核验全覆盖/互斥。
# #1419 M8C：四条 W2 冷锁固定占片 0..3；其余 (target, 完整测试名) 按 C 排序，
# 以零起序号 n 分到 4 + n % (count - 4)。不读取耗时，不按随机或运行顺序分派。
# 全集为空仍拒门；合法空片核验通过后成功跳过，绝不调用无过滤的 cargo test。
# 门：两份 --list 都须成功；新增集合为空即失败（只编译不执行、或一个都没编出来，都按无锁计）。

usage() {
  echo "usage: $0 <feature> [<shard_index> <shard_count>] | --verify-partition <feature> <shard_count> (count >= 5)" >&2
  exit 64
}

mode=full
if (( $# == 3 )); then
  if [[ "$1" == --verify-partition ]]; then
    mode=verify
    feature="$2"
    shard_count="$3"
    shard_index=0
  else
    mode=shard
    feature="$1"
    shard_index="$2"
    shard_count="$3"
  fi
  # 不接受前导零，避免 Bash 的八进制解释；限制位数以免整数溢出。
  [[ "$shard_count" =~ ^[1-9][0-9]{0,3}$ && "$shard_index" =~ ^(0|[1-9][0-9]{0,3})$ ]] || usage
  (( shard_count >= 5 && shard_index < shard_count )) || usage
elif (( $# == 1 )); then
  feature="$1"
else
  usage
fi
readonly feature mode
readonly base_features="s_session"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

list_tests() {
  local features="$1" out="$2"
  shift 2
  cargo test --locked --features "$features" "$@" --jobs 1 -- --list >"$out.raw"
  sed -n 's/: test$//p' "$out.raw" | LC_ALL=C sort >"$out"
}

partition_delta() {
  local target removed count index name assigned ordinal=0
  local -a selector names
  : >"$work/full.unsorted"
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
      return 1
    fi
    while IFS= read -r name; do
      printf '%s\t%s\n' "$target" "$name" >>"$work/full.unsorted"
    done <"$work/$target.delta"
  done
  LC_ALL=C sort "$work/full.unsorted" >"$work/full"
  if [[ ! -s "$work/full" ]]; then
    echo "::error title=Empty feature delta::$feature compiled no additional tests; nothing was executed" >&2
    return 1
  fi
  for (( index=0; index<shard_count; index++ )); do
    : >"$work/shard.$index"
  done
  while IFS=$'\t' read -r target name; do
    # 完整 target/name 匹配，lib 中的同名项仍按普通项分派。
    case "$target:$name" in
      bin:v3::audit_owner::construction_tests::w2_h1_cold_bounded2_shape_development_window_matches_online) assigned=0 ;;
      bin:v3::audit_owner::construction_tests::w2_h1_cold_cd_second_cursor_continuation_matches_online) assigned=1 ;;
      bin:v3::audit_owner::construction_tests::w2_h1_cold_ef_seed_pair_qualification_and_canonical_matches_online) assigned=2 ;;
      bin:v3::audit_owner::construction_tests::w2_h1_cold_w_undetermined_prior_revision_matches_online) assigned=3 ;;
      *) assigned=$(( 4 + ordinal % (shard_count - 4) )); ordinal=$(( ordinal + 1 )) ;;
    esac
    printf '%s\t%s\n' "$target" "$name" >>"$work/shard.$assigned"
  done <"$work/full"

  : >"$work/union.unsorted"
  for (( index=0; index<shard_count; index++ )); do
    count="$(wc -l <"$work/shard.$index" | tr -d ' ')"
    echo "feature_delta_manifest shard=$index/$shard_count count=$count (target<TAB>full_test_name)"
    cat "$work/shard.$index"
    cat "$work/shard.$index" >>"$work/union.unsorted"
  done
  LC_ALL=C sort "$work/union.unsorted" >"$work/union"
  LC_ALL=C uniq -d "$work/union" >"$work/overlap"
  if [[ -s "$work/overlap" ]] || ! cmp -s "$work/full" "$work/union"; then
    echo "::error title=Invalid feature delta partition::shard union differs from full delta or shards overlap" >&2
    return 1
  fi
  echo "feature_delta_partition verified=true shards=$shard_count total=$(wc -l <"$work/full" | tr -d ' ') overlap=0"
  if [[ "$mode" == verify ]]; then
    return 0
  fi
  if [[ ! -s "$work/shard.$shard_index" ]]; then
    echo "feature_delta_shard shard=$shard_index/$shard_count empty=true; full delta is nonempty and verified; skipping execution"
    return 0
  fi
  for target in lib bin; do
    names=()
    while IFS=$'\t' read -r assigned name; do
      if [[ "$assigned" == "$target" ]]; then
        names+=("$name")
      fi
    done <"$work/shard.$shard_index"
    if (( ${#names[@]} > 0 )); then
      if [[ "$target" == lib ]]; then
        selector=(--lib)
      else
        selector=(--bin s_structure_session)
      fi
      cargo test --locked --features "$base_features,$feature" "${selector[@]}" --jobs 1 -- --exact "${names[@]}"
    fi
  done
}

if [[ "$mode" != full ]]; then
  partition_delta
  exit 0
fi

# 单参数入口保留原目标顺序、差集门与执行命令。
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
    cargo test --locked --features "$base_features,$feature" "${selector[@]}" --jobs 1 -- --exact "${names[@]}"
  fi
  total=$(( total + count ))
done
if (( total == 0 )); then
  echo "::error title=Empty feature delta::$feature compiled no additional tests; nothing was executed" >&2
  exit 1
fi
