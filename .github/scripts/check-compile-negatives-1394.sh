#!/usr/bin/env bash
# #1608：执行 #1394 的两个编译负例，断言它们仍然「编译失败，且失败在负例块本身」。
# 负例只在 `--cfg` 打开时编译，普通构建和 cargo test 都不会碰到；没有本入口时，负例是否仍被拒绝无人检查。
# 判据：rustc 退出非 0；至少一条 error；每条 error 都是 E0308，且落在负例块（cfg 行起 20 行内）。
# 任一不满足即失败：负例意外编译通过，或因别处的无关错误失败，都不能当作「边界仍然拒绝」。
# 用法：在 rust/ 目录下运行。
set -uo pipefail

check() {
  local cfg="$1" file="$2" test_flag="$3"
  local start
  start=$(grep -n "${cfg})" "$file" | grep -m1 '#\[cfg' | cut -d: -f1)
  if [ -z "$start" ]; then
    echo "FAIL ${cfg}: ${file} 中找不到 #[cfg(...${cfg}...)]"; return 1
  fi
  local out rc
  # shellcheck disable=SC2086
  out=$(cargo rustc --locked --features s_session --bin s_structure_session -- ${test_flag} --cfg "$cfg" 2>&1)
  rc=$?
  if [ "$rc" -eq 0 ]; then
    echo "FAIL ${cfg}: 负例编译通过，类型边界不再拒绝旧写法"; return 1
  fi
  local errors locations
  errors=$(printf '%s\n' "$out" | grep -E '^error(\[E[0-9]+\])?: ' | grep -v '^error: could not compile' || true)
  if [ -z "$errors" ]; then
    printf '%s\n' "$out" | tail -40
    echo "FAIL ${cfg}: rustc 失败但没有编译错误（rc=${rc}）"; return 1
  fi
  if printf '%s\n' "$errors" | grep -qv '^error\[E0308\]: '; then
    printf '%s\n' "$out" | tail -60
    echo "FAIL ${cfg}: 出现 E0308 以外的错误，不能证明是负例本身被拒"; return 1
  fi
  locations=$(printf '%s\n' "$out" | grep -A1 -E '^error\[E0308\]: ' | grep -E '^ *--> ' | sed -E 's/^ *--> //')
  local loc path line
  while IFS= read -r loc; do
    path=${loc%%:*}; line=$(printf '%s' "$loc" | cut -d: -f2)
    if [ "$path" != "$file" ] || [ "$line" -lt "$start" ] || [ "$line" -gt $((start + 20)) ]; then
      printf '%s\n' "$out" | tail -60
      echo "FAIL ${cfg}: 错误位置 ${loc} 不在负例块 ${file}:${start}+20 内"; return 1
    fi
  done <<< "$locations"
  echo "PASS ${cfg}: rc=${rc}，$(printf '%s\n' "$errors" | wc -l | tr -d ' ') 条 E0308，均在 ${file}:${start}+20 内"
}

status=0
check issue1394_capture_callback_refusal src/bin/s_session_v2/content_store.rs --test || status=1
check issue1394_closed_owner_refusal src/bin/s_session_v3/audit_store.rs "" || status=1
exit $status
