#!/bin/sh
set -eu
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo=$(CDPATH= cd -- "$here/../../../../.." && pwd)
out=${1:-/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage57/direct-author/replay}
mkdir -p "$out/project"
for module in BookQuoteProof BookQuoteSpec DirectHullProof DirectHullSpec DirectOutputProof DirectOutputSourcePathProof DirectOutputSourcePathSpec DirectOutputSpec DirectQuoteProof DirectQuoteSpec LocatedProof LocatedSpec MovingQuoteProof MovingQuoteSpec MovingQuoteWireSpec; do
  cp "$repo/.chanlun/review-results/issue1467-f1-proof/$module.lean" "$out/project/$module.lean"
done
cp "$here/DirectLifecycle.lean" "$out/project/DirectLifecycle.lean"
cp "$repo/formal/lean-toolchain" "$out/project/lean-toolchain"
export LEAN_PATH="$repo/formal/.lake/build/lib/lean"
exec /opt/homebrew/opt/python@3.14/bin/python3.14 "$repo/.agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py" \
 --project "$out/project" --contract "$here/witness-contract.json" \
 --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean \
 --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake \
 --direct --build-timeout 60 --strict-exit --output "$out/check"
