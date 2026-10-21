"""#1673：tb03c_fin_runtime 驱动的输入 hash、断言集合与手算 oracle 锁（不启动进程、不需 Rust）。"""
import ast
import hashlib
import json
from pathlib import Path
import sys
import unittest

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))
import tb03c_fin_runtime as run
from s_socket_client import canonical, digest

# 驱动输入 hash 的独立副本：改驱动常量或改冻结件都必须同时改这里（双写，任何一侧漂移即红）。
LOCKED_INPUTS = {
    "tests/fixtures/tb03c_fin_runtime/OPENING-BASE.json": "9cd91d4f174db5293ad5f85925c7e6ac735f9a32eac7d92eb8b394927d694fda",
    "tests/fixtures/tb03c_fin_runtime/HAND-ORACLE.json": "68fc52b4ec4fc387bd7571a64acf2a0375e844cf8f0e8ebbd5ac0c07eb99ed6a",
    "tests/fixtures/tb01c/messages.jsonl": "29a202bf4a94e69ed503db54e1e30c52d052616447bbd1c4f37488169cc8ac58",
    "tests/fixtures/tb01c/clock-plan.json": "3d7d0470f9e6e358e645b3e757b85ed212d45bb0024f9a90b6919bebc4001709",
    "tests/fixtures/tb01c/profile.json": "2cd50e43e659dc87498eebba76d88deb4cde276cd51c47fa0c083b07dffbdcfc",
    "tests/fixtures/tb01c/query-resources.json": "36b188ae42be8e24e7fdbb44e7d3a7068200f7c942f50975337d07595ac62e64",
    "tests/fixtures/tb01c/runtime-plan.json": "18e9b3cd6cb5f17ba715b09405995b68b627bcd696ba457de790dfa9a51125c3",
    "catalog/signed-catalog.json": "938b0ef59282e689c114cdcb211e2709c86e64c618e4ec0573862bda43508069",
}
# 断言集合的独立副本（AC：锁驱动断言集合）。
LOCKED_CHECKS_SHA256 = "08fb4fdeb0a123a0912236cd4d2295a2247ec451abfb03cb8826df0556388287"


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
        self.assertEqual(inputs["messages"][-1]["message_id"], "tb01c-op-0002")

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

    def test_trust_boundary_statement(self):
        source = Path(run.__file__).read_text()
        self.assertIn("产品绑定 F1/F2 归 TB-04-F #1425", source)
        self.assertIn("未认定存在", source)
        imports = {n.module for n in ast.walk(ast.parse(source)) if isinstance(n, ast.ImportFrom)}
        imports |= {a.name for n in ast.walk(ast.parse(source)) if isinstance(n, ast.Import) for a in n.names}
        self.assertNotIn("tb03c_query_runtime", imports, "冻结驱动不得被依赖")


if __name__ == "__main__":
    unittest.main()
