#!/usr/bin/env bash
# #1467：只读验锁。冻结文件由具名研究版本单独生成；本脚本没有更新锁功能。
set -euo pipefail
research_dir=$(cd "$(dirname "$0")" && pwd)
repo_dir=$(cd "$research_dir/../../.." && pwd)
scratch_dir=$(mktemp -d "${TMPDIR:-/tmp}/nc1467-lock.XXXXXX")
trap 'rm -rf "$scratch_dir"' EXIT
cd "$repo_dir/formal"
lean_prefix=$(lake env lean --print-prefix)
lean_version=$(lake env lean --version)
test "$lean_version" = "$(cat "$research_dir/lean-version-v1.txt")"
lake_path=$(lake env printenv LEAN_PATH)
: > "$scratch_dir/direct-imports.txt"
for module in OrderBaseBoundary PrimitiveSemantics ClockAndGeometry DirectionalChangeLemmas; do
  cp "$research_dir/$module.lean" "$scratch_dir/$module.lean"
  if ! lake env lean -R "$scratch_dir" -o "$scratch_dir/$module.olean" "$scratch_dir/$module.lean" > "$scratch_dir/$module.log"; then
    cat "$scratch_dir/$module.log" >&2
    exit 1
  fi
  printf 'MODULE %s\n' "$module" >> "$scratch_dir/direct-imports.txt"
  lake env lean --deps "$scratch_dir/$module.lean" | sed "s|$lean_prefix/|TOOLCHAIN/|g" >> "$scratch_dir/direct-imports.txt"
done
diff -u "$research_dir/direct-imports-v1.txt" "$scratch_dir/direct-imports.txt"
if ! lake env env LEAN_PATH="$scratch_dir:$lake_path" lean "$research_dir/ExportStatements.lean" > "$scratch_dir/statements.lock"; then
  cat "$scratch_dir/statements.lock" >&2
  exit 1
fi
diff -u "$research_dir/statements-v1.lock" "$scratch_dir/statements.lock"
# 导出器依赖Lean元程序模块，全部导入一起冻结，允许更严格的工具链变动报警。
sed -n 's/^IMPORT //p' "$scratch_dir/statements.lock" | LC_ALL=C sort > "$scratch_dir/imports"
objects=()
while IFS= read -r module; do
  case "$module" in
    OrderBaseBoundary|PrimitiveSemantics|ClockAndGeometry|DirectionalChangeLemmas) continue ;;
  esac
  object="${module//.//}.olean"
  test -f "$lean_prefix/lib/lean/$object"
  for suffix in '' .private .server; do
    if test -f "$lean_prefix/lib/lean/$object$suffix"; then
      objects+=("$object$suffix")
    fi
  done
done < "$scratch_dir/imports"
(cd "$lean_prefix/lib/lean"; printf '%s\0' "${objects[@]}" | xargs -0 shasum -a 256) > "$scratch_dir/imports.sha256"
diff -u "$research_dir/imports-v1.sha256" "$scratch_dir/imports.sha256"
cd "$repo_dir"
if ! shasum -a 256 -c "$research_dir/research-v1.sha256" > "$scratch_dir/source-check.log"; then
  cat "$scratch_dir/source-check.log" >&2
  exit 1
fi
printf 'research-v1: 声明、定义、导入编译对象和研究验收文件均未漂移；假定白名单通过。\n'
