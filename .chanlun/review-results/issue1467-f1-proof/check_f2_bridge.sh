#!/usr/bin/env bash
# #1467：新临时目录编译DC依赖，Origin仅构建具名模块，不改formal源文件。
set -euo pipefail
research_dir=$(cd "$(dirname "$0")" && pwd)
repo_dir=$(cd "$research_dir/../../.." && pwd)
scratch_dir=$(mktemp -d "${TMPDIR:-/tmp}/nc1467-f2-bridge.XXXXXX")
trap 'rm -rf "$scratch_dir"' EXIT
cd "$repo_dir/formal"
test "$(lake env lean --version)" = "$(cat "$research_dir/lean-version-v1.txt")"
shasum -a 256 -c "$research_dir/dc-spec-v1.sha256"
lake build Origin.CenterComplete
lake_path=$(lake env printenv LEAN_PATH)
for module in DCSemantics DirectionalChangeLemmas DCCanonicalProof DCGeometryProof DCOriginBridge; do
  cp "$research_dir/$module.lean" "$scratch_dir/$module.lean"
  if ! lake env env LEAN_PATH="$scratch_dir:$lake_path" lean -R "$scratch_dir" \
      -o "$scratch_dir/$module.olean" "$scratch_dir/$module.lean" > "$scratch_dir/$module.log"; then
    cat "$scratch_dir/$module.log" >&2
    exit 1
  fi
  cat "$scratch_dir/$module.log"
done
lake env env LEAN_PATH="$scratch_dir:$lake_path" lean "$research_dir/F2BridgeAcceptance.lean"
printf 'f2-local-bridge_and_numeric-counterexamples=pass; full_F2_qualification=unproved\n'
