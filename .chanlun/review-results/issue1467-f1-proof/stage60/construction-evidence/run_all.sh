#!/bin/zsh
set -euo pipefail
script_dir=${0:A:h}
repo_dir=${script_dir:h:h:h:h:h}
result_dir=${1:?Supply a NEW absolute output directory}
[[ $result_dir == /* ]] || { print -u2 'Output directory must be absolute'; exit 2; }
[[ ! -e $result_dir ]] || { print -u2 'Output directory must not already exist'; exit 2; }
mkdir -p "$result_dir"
node "$script_dir/build_check.mjs" "$result_dir/run"
node "$script_dir/role_check.mjs" "$result_dir/run" "$result_dir/roles.json"
node "$script_dir/development_check.mjs" "$result_dir/run" "$result_dir/development.json"
node "$script_dir/capture_sources.mjs" "$result_dir/source-evidence.json"
node "$script_dir/prepare_lean.mjs" "$result_dir/run" "$result_dir/lean-project"
LEAN_PATH="$repo_dir/formal/.lake/build/lib/lean" python3 "$repo_dir/.agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py" \
 --project "$result_dir/lean-project" --target-file Stage60Proof.lean --declaration Stage60.exact_root \
 --expected-type 'Stage60.ATOMS ∧ Stage60.FEATURES ∧ Stage60.CORE ∧ Stage60.FORCE' \
 --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean \
 --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake \
 --direct --build-timeout 60 --strict-exit --output "$result_dir/lean-check" \
 > "$result_dir/lean-check.stdout" 2> "$result_dir/lean-check.stderr"
