#!/usr/bin/env python3
"""#1640 P57：在 CI 临时源码上复验 N57 与 0072 变异，编译红不算断言红。"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess

AUDIT = "rust/src/bin/s_session_v3/audit_owner.rs"
SCRATCH = "rust/src/s_content/capture_allocation/scratch.rs"
CODEC = "rust/src/s_content.rs"
PREFIX = "v3::audit_owner::publication_degrade_verdict_tests::"
TRUTH = PREFIX + "publication_degrade_verdict_only_for_pure_budget_shortfall"
SAME_TEXT = PREFIX + "publication_same_text_without_origin_rejects"
GUARD = "trace.only_resource() && trace.bridges(&e)"
ONLY = "self.ledger == 0 && self.semantic == 0 && (self.budget > 0 || self.allocation > 0)"
MUTANTS = ("0072_a", "0072_b", "N57_no_bridges_0072_c", "0072_d", "0072_e", "0072_f", "N57_typed_count_only")


def replace_one(text: str, old: str, new: str) -> str:
    if text.count(old) != 1:
        raise ValueError(f"变异锚必须唯一：{old!r}，实际 {text.count(old)}")
    return text.replace(old, new, 1)


def mutate(name: str, sources: dict[str, str]) -> dict[str, str]:
    result = sources.copy()
    if name == "0072_a":
        # 0069 原白名单只认裸桥和 S3DriverInvalid；不接受任意上下文。
        whitelist = '(e.to_string() == format!("{}: codec 工作域分配失败", newchan_rust::s_content::capture_allocation::LIMIT) || e.to_string() == invalid(&format!("{}: codec 工作域分配失败", newchan_rust::s_content::capture_allocation::LIMIT)))'
        result[AUDIT] = replace_one(result[AUDIT], GUARD, whitelist)
        result[AUDIT] = replace_one(result[AUDIT], "None if trace.only_resource() => Ok(()),\n                None => Err((ledger()).into()),", "None => { let _ = &trace; Ok(()) },")
    elif name in ("0072_b", "0072_f"):
        result[SCRATCH] = replace_one(result[SCRATCH], ONLY, "self.semantic == 0 && (self.budget > 0 || self.allocation > 0)")
        if name == "0072_f":
            result[AUDIT] = replace_one(result[AUDIT], GUARD, "trace.only_resource()")
    elif name == "N57_no_bridges_0072_c":
        result[AUDIT] = replace_one(result[AUDIT], GUARD, "trace.only_resource()")
    elif name == "0072_d":
        result[CODEC] = replace_one(result[CODEC], "error.scratch_failure = Some(capture_allocation::scratch::traced(failure));", "error.scratch_failure = Some(failure);")
    elif name == "0072_e":
        result[AUDIT] = replace_one(result[AUDIT], "WorkError::Resource(ScratchFailure::Ledger) => Err((ledger()).into()),", "WorkError::Resource(ScratchFailure::Ledger) => Ok(()),")
    elif name == "N57_typed_count_only":
        result[AUDIT] = replace_one(result[AUDIT], GUARD, "trace.only_resource() && trace.bridged > 0")
    else:
        raise ValueError(f"未知变异：{name}")
    return result


def actual_assertion_red(returncode: int, output: str, target: str) -> bool:
    return (
        returncode == 101
        and f"test {target} ... FAILED" in output
        and "test result: FAILED. 0 passed; 1 failed;" in output
        and ("panicked at" in output)
        and "error: could not compile" not in output
    )


def run(root: Path, output: Path, name: str) -> dict:
    paths = (AUDIT, SCRATCH, CODEC)
    original = {p: (root / p).read_text() for p in paths}
    mutated = mutate(name, original)
    targets = [TRUTH]
    if name.startswith("N57_"):
        targets.append(SAME_TEXT)
    receipts = []
    try:
        for path, text in mutated.items():
            if text != original[path]:
                (root / path).write_text(text)
        for target in targets:
            command = [
                "cargo", "test", "--locked", "--features",
                "s_session,s_capture_native_testonly", "--bin", "s_structure_session",
                target, "--jobs", "4", "--", "--exact", "--test-threads", "1",
            ]
            proc = subprocess.run(command, cwd=root / "rust", text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
            log = output / f"{name}-{target.rsplit('::', 1)[-1]}.log"
            log.write_text(proc.stdout)
            passed = actual_assertion_red(proc.returncode, proc.stdout, target)
            receipts.append({"target": target, "returncode": proc.returncode, "assertion_red": passed, "log": str(log), "command": command})
            print(f"{name} {target}: assertion_red={passed} rc={proc.returncode}", flush=True)
            if not passed:
                print(proc.stdout, flush=True)
    finally:
        for path, text in original.items():
            (root / path).write_text(text)
    return {"mutation": name, "passed": all(r["assertion_red"] for r in receipts), "receipts": receipts}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--mutation", choices=MUTANTS, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    result = run(args.root.resolve(), args.output.resolve(), args.mutation)
    (args.output / f"{args.mutation}.json").write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
