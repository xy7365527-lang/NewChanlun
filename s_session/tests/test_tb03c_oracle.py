"""#1406 TB-03-C：冻结夹具与独立账簿 oracle 的自检（不导入任何生产 reducer）。

手算期望写在 HAND-ORACLE.json；oracle 只读原件字节按 ADR 0024/0031/0032/0034/0036 自行核账。
两者逐步相等、两次运行字节相同，才允许生产测试以它们为对拍基准。
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from copy import deepcopy

ROOT = Path(__file__).resolve().parent
FIXTURES = ROOT / "fixtures" / "tb03c"
sys.path.insert(0, str(ROOT))

import tb03c_oracle  # noqa: E402


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class FrozenFixtureTests(unittest.TestCase):
    def test_manifest_pins_every_fixture_byte(self):
        manifest = json.loads((FIXTURES / "MANIFEST.json").read_text("utf-8"))
        self.assertEqual(manifest["schema_revision"], "tb03c-fixtures/1")
        self.assertEqual(manifest["book_revision"], "tb03c-book/1")
        listed = sorted(manifest["files"])
        on_disk = sorted(str(p.relative_to(FIXTURES)) for p in FIXTURES.rglob("*")
                         if p.is_file() and p.name != "MANIFEST.json" and "__pycache__" not in p.parts)
        self.assertEqual(listed, on_disk)
        for name, digest in manifest["files"].items():
            self.assertEqual(sha256(FIXTURES / name), digest, name)

    def test_generator_reproduces_frozen_raw_documents(self):
        out = subprocess.run([sys.executable, "-B", str(FIXTURES / "generate_fixtures.py"), "--check"],
                             capture_output=True, text=True)
        self.assertEqual(out.returncode, 0, out.stdout + out.stderr)


class OracleAgreesWithHandCalculationTests(unittest.TestCase):
    def test_every_hand_step_matches_independent_oracle(self):
        hand = json.loads((FIXTURES / "HAND-ORACLE.json").read_text("utf-8"))
        result = tb03c_oracle.run(FIXTURES)
        self.assertEqual(result["mismatches"], [])
        self.assertEqual(result["steps"], sum(len(case["steps"]) for case in hand["book_cases"] + hand["e_cases"]))
        self.assertGreaterEqual(result["steps"], 30)

    def test_apportion_sequences_match_hand_tables(self):
        hand = json.loads((FIXTURES / "HAND-ORACLE.json").read_text("utf-8"))
        for case in hand["apportion_cases"]:
            got = [tb03c_oracle.apportion(f, case["weights"], case["tie_rank"])
                   for f in range(case["total"] + 1)]
            self.assertEqual(got, case["sequence"], case["name"])

    def test_frozen_apportion_table_equals_oracle(self):
        cmd = [sys.executable, "-B", str(ROOT / "tb03c_oracle.py"), "--apportion-table"]
        fresh = subprocess.run(cmd, capture_output=True, check=True).stdout
        self.assertEqual((FIXTURES / "APPORTION-ORACLE.json").read_bytes(), fresh)
        rows = json.loads(fresh)["rows"]
        self.assertEqual(len(rows), 2 * (6 ** 2 + 5 ** 3 + 3 ** 4))
        for row in rows:
            q, seq = row["weights"], row["sequence"]
            self.assertEqual(seq[-1], q)
            for f, (prev, cur) in enumerate(zip(seq, seq[1:]), start=1):
                self.assertEqual(sum(cur), f)
                self.assertTrue(all(c >= p for p, c in zip(prev, cur)))
                self.assertTrue(all(f * w // sum(q) <= c <= -((-f * w) // sum(q)) for w, c in zip(q, cur)))

    def test_largest_remainder_counterexample_is_reproduced(self):
        lr = tb03c_oracle.largest_remainder
        self.assertEqual(lr(3, [1, 3, 3], [0, 1, 2]), [1, 1, 1])
        self.assertEqual(lr(4, [1, 3, 3], [0, 1, 2]), [0, 2, 2])

    def test_two_fresh_processes_print_identical_bytes(self):
        cmd = [sys.executable, "-B", str(ROOT / "tb03c_oracle.py"), str(FIXTURES)]
        first = subprocess.run(cmd, capture_output=True, check=True).stdout
        second = subprocess.run(cmd, capture_output=True, check=True).stdout
        self.assertEqual(first, second)
        self.assertEqual(json.loads(first)["mismatches"], [])


class FeeCausalityRegressionTests(unittest.TestCase):
    def setUp(self):
        self.case = json.loads((FIXTURES / "HAND-ORACLE.json").read_text())["e_cases"][0]

    def test_e_resolved_fee_waits_for_b_applied_fill_then_is_idempotent(self):
        steps = self.case["steps"]
        resolve = deepcopy(steps[14])
        resolve["covered"] = resolve["covered"][1:]
        resolve["pending_after"] = ["t-R-open"]
        case = dict(self.case, name="review-M1-resolved-not-applied", steps=[
            *deepcopy([steps[i] for i in [0, 9, 3, 13]]), resolve,
            dict(deepcopy(steps[12]), K=200, q=2),
            dict(deepcopy(steps[15]), K=320, q=3),
            dict(deepcopy(steps[16]), K=323, q=3),
            dict(deepcopy(steps[16]), K=323, q=3),
        ])
        self.assertEqual(tb03c_oracle.run_e_case(FIXTURES, case), (9, []))

    def check_missing_binding(self, field):
        with tempfile.TemporaryDirectory(prefix="tb1406-fee-oracle-") as directory:
            fixtures = Path(directory)
            (fixtures / "raw").mkdir()
            original = json.loads((FIXTURES / "raw/chong-R1.json").read_text())
            changed = deepcopy(original)
            if field == "depends_on":
                changed["events"][3].pop(field)
                changed["evidence_index"]["fragments"].pop("R-fee:dep")
            else:
                changed["events"][3]["known_cost_components"][0].pop(field, None)
            (fixtures / "raw/chong-R1.json").write_text(json.dumps(original))
            (fixtures / "raw/missing.json").write_text(json.dumps(changed))
            steps = self.case["steps"]
            case = dict(self.case, name="review-H1-missing-" + field, steps=[
                deepcopy(steps[0]), deepcopy(steps[9]),
                {"op": "capture", "raw": "raw/missing.json", "pointer": "/events/3", "outcome": "MissingDependency"},
                {"op": "apply", "raw": "raw/missing.json", "pointer": "/events/3", "outcome": "MissingDependency", "K": 200, "q": 2},
            ])
            self.assertEqual(tb03c_oracle.run_e_case(fixtures, case), (4, []))

    def test_fill_fee_requires_dependency(self):
        self.check_missing_binding("depends_on")

    def test_fill_fee_requires_named_fill_binding(self):
        self.check_missing_binding("fill_ref")

    def test_fill_fee_cannot_substitute_dependency_opening_or_allocation(self):
        for variant in ("different_dependency", "opening_as_fill", "wrong_allocation"):
            with self.subTest(variant=variant), tempfile.TemporaryDirectory(prefix="tb1406-fee-binding-") as directory:
                fixtures = Path(directory)
                (fixtures / "raw").mkdir()
                raw = json.loads((FIXTURES / "raw/chong-R1.json").read_text())
                fee = raw["events"][3]
                reference = fee["known_cost_components"][0]["fill_ref"]
                if variant == "wrong_allocation":
                    reference["allocation_id"] = "wrong-allocation"
                else:
                    fee["depends_on"]["external_fact_id"] = "R-open"
                    if variant == "opening_as_fill":
                        reference.update(external_fact_id="R-open", allocation_id="alloc-R-open")
                (fixtures / "raw/chong-R1.json").write_text(json.dumps(raw))
                original = self.case["steps"]
                steps = deepcopy([original[0], original[9]])
                if variant == "wrong_allocation":
                    steps += [deepcopy(original[13]), dict(deepcopy(original[15]), K=320, q=3)]
                capture = dict(deepcopy(original[3]), pending_reason="Unresolved")
                if variant == "different_dependency":
                    capture = {k: v for k, v in capture.items() if k not in ("pending_reason", "scope")}
                    capture["outcome"] = "IdentityConflict"
                steps.append(capture)
                steps.append(dict(deepcopy(original[12]),
                                  outcome="MissingDependency" if variant == "different_dependency" else "IdentityConflict",
                                  K=320 if variant == "wrong_allocation" else 200,
                                  q=3 if variant == "wrong_allocation" else 2))
                if variant != "different_dependency":
                    steps.append(dict(deepcopy(original[7]), outcome="IdentityConflict",
                                      pending_after=sorted(["t-R-open", "t-R-fee"] +
                                                           (["t-R-late"] if variant == "wrong_allocation" else []))))
                case = dict(self.case, name="review-H1-" + variant, steps=steps)
                self.assertEqual(tb03c_oracle.run_e_case(fixtures, case), (len(steps), []))


if __name__ == "__main__":
    unittest.main()
