"""#1673：tb03c_fin_runtime 驱动的输入 hash、断言集合与手算 oracle 锁（不启动进程、不需 Rust）。"""
import ast
import copy
import hashlib
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))
import tb03c_fin_runtime as run
from s_socket_client import canonical, digest

# 驱动输入 hash 的独立副本：改驱动常量或改冻结件都必须同时改这里（双写，任何一侧漂移即红）。
LOCKED_INPUTS = {
    "tests/fixtures/tb03c_fin_runtime/OPENING-BASE.json": "9cd91d4f174db5293ad5f85925c7e6ac735f9a32eac7d92eb8b394927d694fda",  # DevSkim: ignore DS173237 SHA-256 输入/断言摘要锁，非凭据
    "tests/fixtures/tb03c_fin_runtime/HAND-ORACLE.json": "68fc52b4ec4fc387bd7571a64acf2a0375e844cf8f0e8ebbd5ac0c07eb99ed6a",  # DevSkim: ignore DS173237 SHA-256 输入/断言摘要锁，非凭据
    "tests/fixtures/tb01c/messages.jsonl": "29a202bf4a94e69ed503db54e1e30c52d052616447bbd1c4f37488169cc8ac58",  # DevSkim: ignore DS173237 SHA-256 输入/断言摘要锁，非凭据
    "tests/fixtures/tb01c/clock-plan.json": "3d7d0470f9e6e358e645b3e757b85ed212d45bb0024f9a90b6919bebc4001709",  # DevSkim: ignore DS173237 SHA-256 输入/断言摘要锁，非凭据
    "tests/fixtures/tb01c/profile.json": "2cd50e43e659dc87498eebba76d88deb4cde276cd51c47fa0c083b07dffbdcfc",  # DevSkim: ignore DS173237 SHA-256 输入/断言摘要锁，非凭据
    "tests/fixtures/tb01c/query-resources.json": "36b188ae42be8e24e7fdbb44e7d3a7068200f7c942f50975337d07595ac62e64",  # DevSkim: ignore DS173237 SHA-256 输入/断言摘要锁，非凭据
    "tests/fixtures/tb01c/runtime-plan.json": "18e9b3cd6cb5f17ba715b09405995b68b627bcd696ba457de790dfa9a51125c3",  # DevSkim: ignore DS173237 SHA-256 输入/断言摘要锁，非凭据
    "catalog/signed-catalog.json": "938b0ef59282e689c114cdcb211e2709c86e64c618e4ec0573862bda43508069",  # DevSkim: ignore DS173237 SHA-256 输入/断言摘要锁，非凭据
}
# 断言集合的独立副本（AC：锁驱动断言集合）。
LOCKED_CHECKS_SHA256 = "d86461a5534a0d40a7166192c36fff6c056ace1923331a93db4b51c1c7a4be7e"  # DevSkim: ignore DS173237 SHA-256 输入/断言摘要锁，非凭据


def literal_checks(tree):
    """驱动源码中 ok("<名>", ...) 的全部字面量断言名。"""
    names = set()
    for node in ast.walk(tree):
        if (isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute) and node.func.attr == "ok"
                and node.args and isinstance(node.args[0], ast.Constant) and type(node.args[0].value) is str):
            names.add(node.args[0].value)
        if isinstance(node, ast.Tuple) and node.elts and all(
                isinstance(e, ast.Tuple) and len(e.elts) == 2 and isinstance(e.elts[0], ast.Constant)
                and type(e.elts[0].value) is str and "." in e.elts[0].value for e in node.elts):
            names |= {e.elts[0].value for e in node.elts}
    return names


class Tb03cFinRuntimeContract(unittest.TestCase):
    def test_input_hashes_locked(self):
        self.assertEqual(run.INPUT_HASHES, LOCKED_INPUTS)
        for name, expected in LOCKED_INPUTS.items():
            self.assertEqual(hashlib.sha256((run.ROOT / name).read_bytes()).hexdigest(), expected, name)
        inputs = run.load_inputs()
        self.assertEqual(len(inputs["messages"]), run.S_OPERATIONS)
        self.assertEqual(len(inputs["messages"]), 95)
        self.assertEqual(inputs["messages"][-1]["message_id"], "tb01c-op-0094")

    def test_c6a_resource_caps_are_not_widened(self):
        self.assertEqual(run.QUERY_RESOURCES["max_frame_bytes"], "1048576")
        self.assertEqual(run.QUERY_RESOURCES["max_reply_bytes"], "1048576")
        self.assertEqual(run.QUERY_RESOURCES["max_cached_bytes"], "4194304")
        self.assertEqual(getattr(run.Economy, "FRAME_BYTES", None), 1048576)

    def test_evidence_build_uses_all_three_current_binaries(self):
        calls = []
        class Done:
            returncode = 0
            stdout = ""
            stderr = ""
        def fake_run(args, **_kwargs):
            calls.append(args)
            return Done()
        build = getattr(run, "cargo_build", None)
        self.assertIsNotNone(build)
        build(runner=fake_run)
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0][:4], ["cargo", "build", "--locked", "--features"])
        for binary in ("s_structure_session", "e_economic_session", "b_chong_session"):
            self.assertIn(binary, calls[0])

    def test_s1_oracle_does_not_overwrite_formal_s_binary(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            economy_dir = root / "economy"
            economy_dir.mkdir()
            docs = {name: {"document": name} for name in ("opening", "fill", "gap", "conflict", "valuation")}
            economy = type("Economy", (), {
                "output": economy_dir, "manifest": {"schema_revision": "economic-manifest/1"},
                "input": {}, "plan": {}, "binding": {}, "docs": docs,
            })()
            stack = run.Stack.__new__(run.Stack)
            stack.output, stack.number = root, 0
            stack.record = lambda _entry: None
            seen = {}
            class Done:
                returncode = 0
            def fake_run(_args, **kwargs):
                seen.update(kwargs["env"])
                Path(kwargs["env"]["TB03C_S1_OUTPUT"]).write_bytes(canonical({"E": {}, "B:alpha": {}}))
                return Done()
            original = run.subprocess.run
            run.subprocess.run = fake_run
            previous = os.environ.get("CARGO_TARGET_DIR")
            os.environ["CARGO_TARGET_DIR"] = "/formal-target"
            try:
                run.run_s1_oracle(stack, economy)
            finally:
                run.subprocess.run = original
                if previous is None:
                    os.environ.pop("CARGO_TARGET_DIR", None)
                else:
                    os.environ["CARGO_TARGET_DIR"] = previous
            self.assertEqual(seen["CARGO_TARGET_DIR"], "/formal-target/s1-oracle")

    def test_s1_fixture_contains_the_exact_formal_run_inputs(self):
        docs = {name: {"document": name} for name in ("opening", "fill", "gap", "conflict", "valuation")}
        economy = type("Economy", (), {
            "manifest": {"schema_revision": "economic-manifest/1"},
            "input": {"schema_revision": "tb03b-finance/1"},
            "plan": {"decision_id": "alpha"},
            "binding": {"binding_id": "voice-alpha"},
            "docs": docs,
        })()
        expected = {"manifest": economy.manifest, "input": economy.input, "plan": economy.plan,
                    "binding": economy.binding, "documents": docs}
        actual = getattr(run, "build_s1_fixture", lambda _economy: None)(economy)
        self.assertEqual(actual, expected)

    def test_restart_runtime_and_s1_acceptance_checks_are_registered(self):
        required = {"q.restart_epoch_advanced", "reopen.runtime_drift_detected", "s1.full_terminal_state"}
        self.assertLessEqual(required, set(run.CHECKS["covered"]))

    def test_check_set_locked(self):
        self.assertEqual(hashlib.sha256(canonical(run.CHECKS)).hexdigest(), LOCKED_CHECKS_SHA256)
        for scenario, names in run.CHECKS.items():
            self.assertEqual(len(names), len(set(names)), scenario)
        # 驱动源里每个断言名都已登记；登记的名都在源里有实际调用点（无死登记、无漏登记）。
        used = literal_checks(ast.parse(Path(run.__file__).read_text()))
        registered = set(run.CHECKS["covered"]) | set(run.CHECKS["uncovered"])
        dynamic = {"owner.init", "reopen.recovered"}  # cli() 按 op 选名
        self.assertEqual(used | dynamic, registered, sorted((used | dynamic) ^ registered))

    def test_hand_oracle_recomputed(self):
        oracle = json.loads((run.FIX / "HAND-ORACLE.json").read_text())
        self.assertEqual(run.hand_oracle(oracle["inputs"]), oracle["expected"])
        # 与 S1 Rust 测试同值（tb03c_chain_tests.rs）：锁 30、责任 50、K 80028。
        self.assertEqual((oracle["expected"]["lock"], oracle["expected"]["responsibility_total"],
                          oracle["expected"]["K_after"]), (30, 50, 80028))
        broken = dict(oracle["inputs"], fill_price=15)
        with self.assertRaises(ValueError):
            run.hand_oracle(broken)

    def test_documents_match_s1_construction(self):
        base = json.loads((run.FIX / "OPENING-BASE.json").read_text())
        docs = run.documents(base)
        fill, conflict = docs["fill"]["events"][0], docs["conflict"]["events"][0]
        self.assertEqual(run.fact_key(fill), run.fact_key(conflict))
        self.assertNotEqual(run.fact_hash(docs["fill"]), run.fact_hash(docs["conflict"]))
        self.assertEqual(fill["netted_fill"]["manifest_hash"], docs["fill_manifest"]["manifest_hash"])
        manifest = {k: v for k, v in docs["fill_manifest"].items() if k != "manifest_hash"}
        self.assertEqual(digest(manifest), docs["fill_manifest"]["manifest_hash"])
        self.assertEqual(docs["opening"]["events"][0]["observed_existing_book"]["units"], "4")
        self.assertEqual(base, json.loads((run.FIX / "OPENING-BASE.json").read_text()), "构造不得改写冻结件")

    def test_clock_after_structure(self):
        clock = run.clock_plan()
        self.assertEqual(len(clock["recover_ns"]), run.RECOVER_EPOCHS)
        first = min(int(v) for m in clock["messages"].values() for k in m.values() for v in k)
        self.assertGreater(first, max(int(v) for v in run._numbers(run.load_inputs()["clock"])))
        for m in clock["messages"].values():
            for r in range(run.RECOVER_EPOCHS):
                self.assertLess(int(m["received_ns"][r]), int(m["first_known_ns"][r]))
                self.assertLess(int(m["first_known_ns"][r]), int(m["commit_ns"][r]))

    def test_q_restart_uses_new_query_and_producer_epoch(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            stack = run.Stack.__new__(run.Stack)
            stack.work, stack.output = root / "work", root / "out"
            stack.work.mkdir()
            stack.output.mkdir()
            stack.q_revision, stack.q_started = 1, False
            stack.s = {"query_epoch": "1"}
            stack.s_control = lambda action: {"action": action}
            economy = type("Economy", (), {
                "manifest": {"session_id": "s", "session_generation": "1"},
                "manifest_path": root / "manifest.json",
            })()
            stack.start_q(economy, {"E": "3"})
            config = json.loads((stack.output / "economic-query-r2.json").read_text())
            launcher = json.loads((stack.work / "s-launcher-r2.json").read_text())
            self.assertEqual(config["producer_epoch"], "2")
            self.assertEqual(launcher["query_epoch"], "2")

    @staticmethod
    def _public_view(runtime):
        return {"kind": "EconomicView", "writer_epoch": "1", "runtime": runtime}

    def _public_with_runtime(self, runtime):
        view = self._public_view(runtime)
        class FakeEconomy:
            grant_id = "grant"
            def read(self, _domain, payload):
                if payload["op"] == "ReadView":
                    return copy.deepcopy(view)
                return {"kind": "EconomicFact"}
        return run.public(FakeEconomy(), {"fact_key": "fill"}, {"fact_key": "gap"}, "allocation")

    def test_public_runtime_keeps_storage_facts_but_not_process_identity(self):
        runtime = {"pid": "10", "control_instance_id": "a", "journal_mode": "wal",
                   "synchronous": "2", "fullfsync": "1", "filesystem": "apfs",
                   "read_only": False, "sqlite_version": "3.51.3", "sqlite_version_number": "3051003"}
        before = self._public_with_runtime(runtime)
        self.assertEqual(before[0].get("runtime", {}).get("filesystem"), "apfs")
        self.assertNotIn("pid", before[0].get("runtime", {}))
        volatile = dict(runtime, pid="11", control_instance_id="b")
        self.assertEqual(self._public_with_runtime(volatile), before)

    def test_public_runtime_comparison_rejects_storage_drift(self):
        runtime = {"pid": "10", "control_instance_id": "a", "journal_mode": "wal",
                   "synchronous": "2", "fullfsync": "1", "filesystem": "apfs",
                   "read_only": False, "sqlite_version": "3.51.3", "sqlite_version_number": "3051003"}
        before = self._public_with_runtime(runtime)
        drifted = dict(runtime, filesystem="hfs", read_only=True)
        self.assertNotEqual(self._public_with_runtime(drifted), before)

    def test_paged_http_receives_the_shared_remaining_deadline(self):
        with tempfile.TemporaryDirectory() as tmp:
            stack = run.Stack.__new__(run.Stack)
            stack.output, stack.number, stack.q_epoch = Path(tmp), 0, "2"
            seen = {}
            def fake_http(_economy, _path, _payload, timeout_ms=30000):
                seen["timeout_ms"] = timeout_ms
                return {"ok": True}
            stack.http = fake_http
            economy = type("Economy", (), {
                "manifest": {"session_id": "s", "session_generation": "1"}, "epochs": {"E": "1"},
            })()
            original = run.read_economic_pages
            run.read_economic_pages = lambda _query, fetch, **_kwargs: fetch({"op": "page"}, 1234)
            try:
                stack.paged(economy, {"op": "snapshot"})
            finally:
                run.read_economic_pages = original
            self.assertEqual(seen["timeout_ms"], 1234)

    def test_trust_boundary_statement(self):
        source = Path(run.__file__).read_text()
        self.assertIn("产品绑定 F1/F2 归 TB-04-F #1425", source)
        self.assertIn("未认定存在", source)
        imports = {n.module for n in ast.walk(ast.parse(source)) if isinstance(n, ast.ImportFrom)}
        imports |= {a.name for n in ast.walk(ast.parse(source)) if isinstance(n, ast.Import) for a in n.names}
        self.assertNotIn("tb03c_query_runtime", imports, "冻结驱动不得被依赖")


if __name__ == "__main__":
    unittest.main()
