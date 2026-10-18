"""#1527 H1 R3：差异范围、固定原件、旧证据和未改函数的只读核验。"""
import ast
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
BASE = "52a86c684768b884aec24a3f794a89a4fe8a3402"
OLD = "44a977568948542aa6d00ecab50e2d468e20b76c"
PRODUCT = "s_session/recovery_admission.py"
CONTROL = "s_session/s_service_control.py"
TESTS = "s_session/tests/issue1527_recovery/test_python_r4.py"


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT)


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def functions(raw):
    source = raw.decode()
    tree = ast.parse(source)
    result = {}
    for node in tree.body:
        if isinstance(node, ast.FunctionDef):
            result[node.name] = ast.get_source_segment(source, node)
        elif isinstance(node, ast.ClassDef):
            for method in node.body:
                if isinstance(method, ast.FunctionDef):
                    result[node.name + "." + method.name] = ast.get_source_segment(source, method)
    return result


def main():
    assert git("rev-parse", "HEAD").decode().strip() == BASE
    assert git("branch", "--show-current").decode().startswith("codex/")
    assert not git("diff", "--cached", "--name-only")
    assert set(git("diff", "--name-only", BASE).decode().splitlines()) == {PRODUCT, TESTS}
    git("diff", "--check", BASE)
    untracked = git("ls-files", "--others", "--exclude-standard").decode().splitlines()
    assert all(p.startswith(str(HERE.relative_to(ROOT)) + "/") for p in untracked)
    hashes = {}
    for path in (PRODUCT, CONTROL, TESTS):
        before = git("show", BASE + ":" + path)
        after = (ROOT / path).read_bytes()
        ast.parse(after)
        hashes[path] = dict(baseline_sha256=digest(before), current_sha256=digest(after))
        if path == CONTROL:
            assert before == after
    old_functions = functions(git("show", BASE + ":" + PRODUCT))
    new_functions = functions((ROOT / PRODUCT).read_bytes())
    changed = {k for k in old_functions if old_functions[k] != new_functions[k]}
    assert changed == {"RecoveryAdmission.require_owner", "RecoveryAdmission.prepare"}
    assert new_functions.keys() - old_functions.keys() == {"_owner_control_object"}
    old_tests = functions(git("show", BASE + ":" + TESTS))
    new_tests = functions((ROOT / TESTS).read_bytes())
    assert all(new_tests[k] == v for k, v in old_tests.items())
    patch = git("diff", "--binary", BASE, "--", PRODUCT, TESTS)
    assert (HERE / "python-h1-r3-repair.patch").read_bytes() == patch
    base_receipt = ROOT.parent / "PYTHON-H1-R3-BASE.json"
    receipt = json.loads(base_receipt.read_text())
    assert receipt["source"] == BASE and receipt["base"] == OLD
    assert receipt["tree"] == git("rev-parse", BASE + "^{tree}").decode().strip()
    prefix = ".chanlun/review-results/issue1527-python-r4-repair-20261018/"
    expected_logs = {p: h for p, h in receipt["files"].items() if p.startswith(prefix) and p.endswith(".log")}
    assert len(expected_logs) == 16
    old_names = set(git("ls-tree", "-r", "--name-only", OLD, "--", prefix).decode().splitlines())
    assert not (expected_logs.keys() & old_names)
    for path, sha in expected_logs.items():
        assert digest((ROOT / path).read_bytes()) == sha
        assert digest(git("show", BASE + ":" + path)) == sha
    for run in json.loads((HERE / "CHECKS.json").read_text())["executions"]:
        for stream in ("stdout", "stderr"):
            artifact = run[stream]
            raw = (HERE / artifact["path"]).read_bytes()
            assert digest(raw) == artifact["sha256"] and len(raw) == artifact["bytes"]
    print(json.dumps(dict(
        result="PASS", baseline=BASE, product=hashes, patch_sha256=digest(patch),
        changed_functions=sorted(changed),
        unchanged_functions=sorted(old_functions.keys() - changed),
        original_test_functions_unchanged=sorted(old_tests),
        previous_16_logs_preserved=expected_logs,
        original_44a_logs_absent=True, service_control_unchanged=True,
        old_tracked_reports_unchanged=True, no_index_change=True,
        base_receipt_sha256=digest(base_receipt.read_bytes())), ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
