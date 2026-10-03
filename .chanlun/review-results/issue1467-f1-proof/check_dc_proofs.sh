#!/usr/bin/env bash
# #1467：v1研究锁的增补。定义文件先于证明冻结，外部文件强制检查精确目标。
set -euo pipefail
research_dir=$(cd "$(dirname "$0")" && pwd)
repo_dir=$(cd "$research_dir/../../.." && pwd)
scratch_dir=$(mktemp -d "${TMPDIR:-/tmp}/nc1467-dc-verify.XXXXXX")
trap 'rm -rf "$scratch_dir"' EXIT
cd "$repo_dir"
shasum -a 256 -c "$research_dir/dc-proof-contract-v1.sha256"
cd "$repo_dir/formal"
shasum -a 256 -c "$research_dir/dc-spec-v1.sha256"
lean_prefix=$(lake env lean --print-prefix)
test "$(lake env lean --version)" = "$(cat "$research_dir/lean-version-v1.txt")"
if ! (cd "$lean_prefix/lib/lean"; shasum -a 256 -c "$research_dir/imports-v1.sha256") > "$scratch_dir/imports.log"; then
  grep 'FAILED' "$scratch_dir/imports.log" >&2 || true
  exit 1
fi
lake_path=$(lake env printenv LEAN_PATH)
for module in DCSemantics DirectionalChangeLemmas DCCanonicalProof DCGeometryProof DCCausalProof DCWitness; do
  cp "$research_dir/$module.lean" "$scratch_dir/$module.lean"
  if ! lake env env LEAN_PATH="$scratch_dir:$lake_path" lean -R "$scratch_dir" \
      -o "$scratch_dir/$module.olean" "$scratch_dir/$module.lean" > "$scratch_dir/$module.log"; then
    cat "$scratch_dir/$module.log" >&2
    exit 1
  fi
  cat "$scratch_dir/$module.log"
done
lake env env LEAN_PATH="$scratch_dir:$lake_path" lean "$research_dir/DCAcceptance.lean"
printf 'dc-proof-contract-v1: fresh_compile_exact_targets_and_axiom_gate=pass\n'
