#!/usr/bin/env bash
# #1467：在临时模块中做声明锁负例，研究文件和冻结基线均不改。
set -euo pipefail
research_dir=$(cd "$(dirname "$0")" && pwd)
repo_dir=$(cd "$research_dir/../../.." && pwd)
scratch_dir=$(mktemp -d "${TMPDIR:-/tmp}/nc1467-lock-test.XXXXXX")
trap 'rm -rf "$scratch_dir"' EXIT
cd "$repo_dir/formal"
lake_path=$(lake env printenv LEAN_PATH)
compile() {
  lake env lean -R "$scratch_dir" -o "$scratch_dir/$1.olean" "$scratch_dir/$1.lean" > "$scratch_dir/build.log"
}
export_lock() {
  lake env env LEAN_PATH="$scratch_dir:$lake_path" lean "$research_dir/ExportStatements.lean" > "$1"
}
different() {
  if cmp -s "$1" "$2"; then
    printf '应检测到差异，却相同：%s\n' "$3" >&2
    exit 1
  fi
  printf '%s: rejected_by_lock_diff\n' "$3"
}
for module in OrderBaseBoundary PrimitiveSemantics ClockAndGeometry DirectionalChangeLemmas; do
  cp "$research_dir/$module.lean" "$scratch_dir/$module.lean"
  compile "$module"
done
export_lock "$scratch_dir/original.lock"
cmp "$research_dir/statements-v1.lock" "$scratch_dir/original.lock"
printf 'clean_fresh_compile: pass\n'
sed 's/: ambiguous x (chosen x) := Or.inl rfl/: ambiguous x (chosen x) := by exact Or.inl rfl/' "$research_dir/OrderBaseBoundary.lean" > "$scratch_dir/OrderBaseBoundary.lean"
compile OrderBaseBoundary
export_lock "$scratch_dir/proof.lock"
cmp "$scratch_dir/original.lock" "$scratch_dir/proof.lock"
printf 'proof_body_only: unchanged\n'
cat "$research_dir/DirectionalChangeLemmas.lean" > "$scratch_dir/fixture.lean"
cat >> "$scratch_dir/fixture.lean" <<'LEAN'

namespace LockCheck
variable {α : Type}
def Meaning : Prop := True
theorem meaningProof : Meaning := True.intro
theorem binder (x : α) : x = x := rfl
end LockCheck
LEAN
cp "$scratch_dir/fixture.lean" "$scratch_dir/DirectionalChangeLemmas.lean"
compile DirectionalChangeLemmas
export_lock "$scratch_dir/fixture.lock"
different "$scratch_dir/original.lock" "$scratch_dir/fixture.lock" declaration_added_outside_original_namespace
sed 's/variable {α : Type}/variable {α : Type 1}/' "$scratch_dir/fixture.lean" > "$scratch_dir/DirectionalChangeLemmas.lean"
compile DirectionalChangeLemmas
export_lock "$scratch_dir/binder.lock"
different "$scratch_dir/fixture.lock" "$scratch_dir/binder.lock" shared_variable_universe_changed
sed -e 's/def Meaning : Prop := True/def Meaning : Prop := 0 = 0/' -e 's/theorem meaningProof : Meaning := True.intro/theorem meaningProof : Meaning := rfl/' "$scratch_dir/fixture.lean" > "$scratch_dir/DirectionalChangeLemmas.lean"
compile DirectionalChangeLemmas
export_lock "$scratch_dir/meaning.lock"
different "$scratch_dir/fixture.lock" "$scratch_dir/meaning.lock" definition_body_changed_theorem_type_unchanged
sed '/^theorem binder /d' "$scratch_dir/fixture.lean" > "$scratch_dir/DirectionalChangeLemmas.lean"
compile DirectionalChangeLemmas
export_lock "$scratch_dir/removed.lock"
different "$scratch_dir/fixture.lock" "$scratch_dir/removed.lock" declaration_removed
for kind in axiom sorry; do
  cp "$research_dir/DirectionalChangeLemmas.lean" "$scratch_dir/DirectionalChangeLemmas.lean"
  if test "$kind" = axiom; then
    printf '\naxiom LockCheck.injected : False\n' >> "$scratch_dir/DirectionalChangeLemmas.lean"
  else
    printf '\ntheorem LockCheck.unfinished : False := by sorry\n' >> "$scratch_dir/DirectionalChangeLemmas.lean"
  fi
  compile DirectionalChangeLemmas
  if export_lock "$scratch_dir/reject.log"; then
    printf '未拒绝 %s\n' "$kind" >&2
    exit 1
  fi
  grep -q '不允许的假定' "$scratch_dir/reject.log"
  printf '%s: rejected_by_axiom_gate\n' "$kind"
done
