#!/bin/sh
set -eu
R=/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun
E="$R/.chanlun/review-results/issue1467-f1-proof/stage49/f2-contract-evidence"
test "$(git -C "$R" rev-parse HEAD)" = 2e3551802306b704e759631487a75498b3b08b12
BUILD=$(mktemp -d "$E/.probe-build.XXXXXX")
trap 'rm -rf "$BUILD"' EXIT HUP INT TERM
mkdir -p "$BUILD/Origin"
cd "$R/formal"
export LEAN_PATH="$BUILD"
LEAN_BIN=$(elan which lean)
RUST_BIN=$(command -v rustc)
"$LEAN_BIN" --version
for MOD in SourceAxioms CompleteClassification ChanlunElements CenterStates CenterFull CenterConstruction CenterComplete
do
    "$LEAN_BIN" -o "$BUILD/Origin/$MOD.olean" "Origin/$MOD.lean" > "$BUILD/$MOD.log" 2>&1 || { cat "$BUILD/$MOD.log"; exit 1; }
    printf 'fresh source build: Origin.%s PASS\n' "$MOD"
done
"$LEAN_BIN" --root="$E" -o "$BUILD/NonNormProbe.olean" "$E/NonNormProbe.lean"
"$RUST_BIN" --version
"$RUST_BIN" --edition=2021 "$E/non_norm_probe.rs" -o "$BUILD/non_norm_probe"
"$BUILD/non_norm_probe"
python3 - "$R" "$E" "$BUILD" "$LEAN_BIN" "$RUST_BIN" <<'PY'
import hashlib, json, pathlib, sys
root, evidence, build, lean, rust = map(pathlib.Path, sys.argv[1:])
def digest(path):
    return {"sha256": hashlib.sha256(path.read_bytes()).hexdigest(), "bytes": path.stat().st_size}
modules = []
for module in ("SourceAxioms", "CompleteClassification", "ChanlunElements", "CenterStates", "CenterFull", "CenterConstruction", "CenterComplete"):
    source = root / "formal" / "Origin" / (module + ".lean")
    modules.append({"module": "Origin." + module, "source": str(source.relative_to(root)), "source_digest": digest(source), "olean_digest": digest(build / "Origin" / (module + ".olean"))})
payload = {
    "method": "direct Lean source builds; direct root elaboration and olean emission; #check and #print axioms; not verify_lean_project",
    "head": "2e3551802306b704e759631487a75498b3b08b12",
    "lean_toolchain_file": digest(root / "formal" / "lean-toolchain"),
    "lean_executable": {"path": str(lean.resolve()), **digest(lean.resolve())},
    "rustc_executable": {"path": str(rust.resolve()), **digest(rust.resolve())},
    "fresh_source_modules": modules,
    "probe_source": digest(evidence / "NonNormProbe.lean"),
    "probe_olean": digest(build / "NonNormProbe.olean"),
    "rust_driver": digest(evidence / "non_norm_probe.rs"),
    "rust_binary": digest(build / "non_norm_probe"),
    "lean_root_exit": 0,
    "rust_compile_exit": 0,
    "rust_run_exit": 0,
    "build_outputs_preserved": False,
    "note": "Only digests are preserved; temporary olean and binary outputs are deleted by the EXIT trap."
}
(evidence / "source-build.json").write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")
print("source-build.json: fresh source/olean/probe/tool executable SHA-256 captured")
PY
